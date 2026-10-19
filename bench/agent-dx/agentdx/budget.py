"""Cost estimate and hard budget gate for live agent runs."""
import threading

PILOT_MAX_TRIALS = 60   # absolute ceiling for the pilot; raising it is a code change on purpose

# USD per million tokens. Assumed Sonnet-class list pricing; the live per-trial cost comes from the
# Claude Code JSON (`total_cost_usd`), and the guard projects from observed cost once trials complete.
PRICE = {"input": 3.0, "cache_write": 3.75, "cache_read": 0.30, "output": 15.0}
BASE_CONTEXT_TOKENS = 8000    # Claude Code prompt + restricted tool set; calibrated on 2 smoke trials (~8-9k tokens per turn)


class BudgetExceeded(Exception):
    pass


def estimate_trial(pack_chars, max_turns, expected_turn_fraction=0.6, out_tokens_per_turn=400, growth_per_turn=1500):
    """-> {"expected": usd, "worst": usd}; worst assumes every turn is used."""
    ctx0 = BASE_CONTEXT_TOKENS + pack_chars / 3.5 + 1200

    def cost(turns):
        c = ctx0 * PRICE["cache_write"] / 1e6
        for t in range(int(round(turns))):
            c += (ctx0 + growth_per_turn * t) * PRICE["cache_read"] / 1e6 + out_tokens_per_turn * PRICE["output"] / 1e6
        return c
    return {"expected": cost(max_turns * expected_turn_fraction), "worst": cost(max_turns)}


def estimate_plan(n_trials, pack_chars, max_turns):
    e = estimate_trial(pack_chars, max_turns)
    return {"trials": n_trials, "per_trial_expected": e["expected"], "per_trial_worst": e["worst"],
            "total_expected": e["expected"] * n_trials, "total_worst": e["worst"] * n_trials}


class BudgetGuard:
    """Abort before a trial when it would exceed the trial counter or the dollar budget.
    After >= 3 completed trials the projection uses the observed mean cost instead of the estimate."""

    def __init__(self, budget_usd, max_trials, est_per_trial):
        self.budget, self.max_trials, self.est = budget_usd, min(max_trials, PILOT_MAX_TRIALS), est_per_trial
        self.spent, self.count, self.inflight = 0.0, 0, 0
        self._lock = threading.Lock()

    def next_cost(self):
        return self.spent / self.count if self.count >= 3 else self.est

    def before_trial(self):
        """Reserve a slot for one trial (thread-safe); in-flight trials count at the projected cost."""
        with self._lock:
            if self.count + self.inflight >= self.max_trials:
                raise BudgetExceeded(f"trial counter {self.count}+{self.inflight} in flight reached max-trials {self.max_trials}")
            proj = self.spent + (self.inflight + 1) * self.next_cost()
            if proj > self.budget:
                raise BudgetExceeded(f"spent ${self.spent:.2f} + {self.inflight + 1} trial(s) at ~${self.next_cost():.2f} exceeds budget ${self.budget:.2f}")
            self.inflight += 1

    def record(self, cost_usd):
        with self._lock:
            self.spent += cost_usd or 0.0
            self.count += 1
            self.inflight = max(0, self.inflight - 1)
