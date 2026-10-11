//! `docs/reference/cli.md` and the `cli_commands` region: generated from the clap definition.

use super::util::table;
use clap::Command;

fn about(c: &Command) -> String {
    c.get_about().map(|a| a.to_string()).unwrap_or_default()
}

fn walk<'a>(c: &'a Command, path: &mut Vec<String>, out: &mut Vec<(String, &'a Command)>) {
    for s in c.get_subcommands() {
        if s.is_hide_set() || s.get_name() == "help" {
            continue;
        }
        path.push(s.get_name().to_string());
        out.push((path.join(" "), s));
        walk(s, path, out);
        path.pop();
    }
}

pub fn commands(cli: &Command) -> Vec<(String, &Command)> {
    let mut out = Vec::new();
    walk(cli, &mut Vec::new(), &mut out);
    out
}

pub fn flags_of(c: &Command) -> Vec<String> {
    c.get_arguments().filter_map(|a| a.get_long().map(|l| l.to_string())).collect()
}

/// `| Command | What it does |` rows for every subcommand.
pub fn command_table(cli: &Command) -> String {
    let rows: Vec<Vec<String>> = commands(cli).iter().map(|(p, c)| vec![format!("`wj {p}`"), about(c)]).collect();
    table(&["Command", "What it does"], &rows)
}

pub fn cli(cli: &Command) -> String {
    let bin = cli.get_name();
    let mut s = format!("# `{bin}` command-line reference\n\nGenerated from the clap definitions in `src/bin/wj_cli_args.rs`.\n\n## Commands\n\n{}\n", command_table(cli));
    for (path, c) in commands(cli) {
        s.push_str(&format!("## `{bin} {path}`\n\n{}\n\n", about(c)));
        let mut usage = c.clone().bin_name(format!("{bin} {path}"));
        s.push_str(&format!("```text\n{}\n```\n\n", usage.render_usage().to_string().trim()));
        let rows: Vec<Vec<String>> = c
            .get_arguments()
            .filter(|a| a.get_id() != "help" && a.get_id() != "version")
            .map(|a| {
                let name = match (a.get_long(), a.get_short()) {
                    (Some(l), Some(sh)) => format!("`-{sh}`, `--{l}`"),
                    (Some(l), None) => format!("`--{l}`"),
                    (None, Some(sh)) => format!("`-{sh}`"),
                    _ => format!("`<{}>`", a.get_id().as_str().to_uppercase()),
                };
                let val = a.get_value_names().map(|v| v.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(" ")).unwrap_or_default();
                let def = a.get_default_values().iter().map(|d| d.to_string_lossy().to_string()).collect::<Vec<_>>().join(",");
                vec![name, val, def, a.get_help().map(|h| h.to_string()).unwrap_or_default()]
            })
            .collect();
        if !rows.is_empty() {
            s.push_str(&table(&["Argument", "Value", "Default", "Description"], &rows));
            s.push('\n');
        }
    }
    s
}
