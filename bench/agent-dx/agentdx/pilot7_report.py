"""Pilot 7 report: per model and arm (task-clustered bootstrap CIs), the cost decomposition, and for the most expensive
model where the dollars go and how much of the Rust-versus-Windjammer difference is program size, thinking, turns or
context. Every figure is computed from the trial records and the price table in pilot7_config.json."""
import os

from . import costdecomp as CD, pilot as PL, stats as S

METRICS = [("pass", lambda t: 1.0 if t["pass_final"] else 0.0, 3), ("cost USD", lambda t: t["usage"]["cost_usd"], 4),
           ("turns", lambda t: t["usage"]["turns"], 1), ("tokens out", lambda t: t["usage"]["tokens_out"], 0),
           ("program lines", lambda t: t["cost_decomposition"]["final_program_lines"], 0)]
PARTS = [("context (fresh + cache write + cache read)", lambda d: d["usd"]["fresh_input"] + d["usd"]["cache_write"] + d["usd"]["cache_read"]),
         ("visible output", lambda d: d["usd"]["output_visible"]),
         ("thinking (estimate)", lambda d: d["usd"]["output_thinking_est"])]


def load_all(out_dir, models):
    return {m: PL.load_trials(os.path.join(out_dir, m)) for m in models}


def load(out_dir, models):
    """Trials that are graded: a timed-out trial is excluded from every metric (its spend is still counted below)."""
    return {m: [t for t in ts if not t.get("excluded_from_grading")] for m, ts in load_all(out_dir, models).items()}


def spend_section(all_trials):
    rows = ["## Spend and exclusions", "", "| model | trials | timeouts (excluded from grading, spend counted) | charged USD |", "|---|---|---|---|"]
    for m, ts in all_trials.items():
        rows.append(f"| {m} | {len(ts)} | {sum(1 for t in ts if t.get('status') == 'timeout')} | "
                    f"{sum((t['usage'].get('charged_usd') if t['usage'].get('charged_usd') is not None else t['usage'].get('cost_usd')) or 0 for t in ts):.4f} |")
    return rows + [""]


def clusters(trials, arm, f):
    by = {}
    for t in trials:
        if t["arm"] == arm:
            by.setdefault(t["task"], []).append(f(t))
    return by


def fmt(ci, nd):
    return "n/a" if ci is None else f"{ci[0]:.{nd}f} [{ci[1]:.{nd}f}, {ci[2]:.{nd}f}]"


def build(out_dir, cfg, n_boot=2000):
    models, arms = list(cfg["models"]), cfg["arms"]
    data = load(out_dir, models)
    every = load_all(out_dir, models)
    seeds = sorted({t.get("seed") for ts in every.values() for t in ts if "seed" in t}, key=str)
    versions = sorted({t.get("wj_version", "") for ts in every.values() for t in ts})
    md = ["# Pilot 7: models x arms", "", f"Order seed: {', '.join(map(str, seeds)) or 'not recorded'}. wj: {'; '.join(versions) or 'not recorded'}.", "",
          f"Price table: {cfg['prices']['_date']} ({cfg['prices']['_source']})", ""]
    for m in models:
        tr = data[m]
        if not tr:
            md += [f"## {m}", "", "no trials", ""]
            continue
        md += [f"## {m} ({len(tr)} trials)", "", "| metric | " + " | ".join(arms) + " |", "|---|" + "---|" * len(arms)]
        for name, f, nd in METRICS:
            md.append(f"| {name} | " + " | ".join(fmt(S.bootstrap_ci(list(clusters(tr, a, f).values()), n_boot), nd) for a in arms) + " |")
        for name, f in PARTS:
            md.append(f"| USD: {name} | " + " | ".join(fmt(S.bootstrap_ci(list(clusters(tr, a, lambda t: f(t['cost_decomposition'])).values()), n_boot), 4) for a in arms) + " |")
        cal = [t["cost_decomposition"]["usd_calibration"] for t in tr if t["cost_decomposition"].get("usd_calibration")]
        md += ["", f"Computed over reported cost (price-table check): mean {sum(cal) / len(cal):.2f} over {len(cal)} trials." if cal else "", ""]
        base = arms[0]
        md += [f"Paired differences, {base} minus each arm (cost USD, task-paired CI):", ""]
        for a in arms[1:]:
            ci = S.paired_diff_ci(clusters(tr, base, lambda t: t["usage"]["cost_usd"]), clusters(tr, a, lambda t: t["usage"]["cost_usd"]), n_boot)
            md.append(f"- {base} - {a}: {fmt(ci, 4)}")
        md.append("")
    md += spend_section(every)
    top = models[-1]
    md += decomposition_section(top, data[top], arms, cfg["prices"][top])
    return "\n".join(md) + "\n"


def decomposition_section(m, trials, arms, price):
    if not trials:
        return []
    md = [f"## Where the dollars go ({m})", ""]
    for a in arms:
        ds = [t["cost_decomposition"] for t in trials if t["arm"] == a]
        if not ds:
            continue
        tot = sum(d["usd_total_computed"] for d in ds) or 1.0
        shares = ", ".join(f"{name} {100 * sum(f(d) for d in ds) / tot:.0f}%" for name, f in PARTS)
        md.append(f"- {a}: {shares}; context per turn ${sum(d['usd_context_per_turn'] or 0 for d in ds) / len(ds):.4f}")
    base = [t["cost_decomposition"] for t in trials if t["arm"] == arms[0]]
    md += ["", f"Attribution of the mean difference (arm minus {arms[0]}): context, visible output (of which the program text), thinking, turns, program lines:", ""]
    for a in arms[1:]:
        other = [t["cost_decomposition"] for t in trials if t["arm"] == a]
        if base and other:
            md.append(f"- {a}: " + ", ".join(f"{k} {v:+.4f}" for k, v in CD.attribute_difference(base, other, price).items()))
    v = CD.token_saving_value(price)
    md += ["", f"What a pure token saving is worth at {m} prices: 1k fewer output tokens ${v['per_1k_output_tokens']:.4f}; "
           f"1k fewer context tokens re-read on one turn ${v['per_1k_context_tokens_over_turns']:.4f} (multiply by the turns that re-read it).", ""]
    return md
