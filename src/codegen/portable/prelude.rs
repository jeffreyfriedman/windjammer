//! Windjammer-source preludes for the portable (Go/JS/TS) backends.
//!
//! `std::arena` (ADR-0025) is implemented for these targets in Windjammer itself (`prelude/arena.wj`): a program that
//! imports it gets the prelude's items prepended and the import dropped, then both emitters lower it like user code.

use crate::parser::{Item, Parser, Program};

const ARENA_SRC: &str = include_str!("prelude/arena.wj");

/// The program with the `std::arena` prelude merged in, or `None` when it does not import `std::arena`.
pub fn with_arena_prelude<'a>(program: &Program<'a>) -> Option<Program<'a>> {
    if !crate::analyzer::arena_escape::uses_arenas(program) {
        return None;
    }
    let tokens = crate::lexer::Lexer::new(ARENA_SRC).tokenize_with_locations();
    let prelude: Program<'static> =
        Parser::new_with_source(tokens, "std/arena.wj".into(), ARENA_SRC.into()).parse().expect("arena prelude parses");
    let mut items: Vec<Item<'a>> = prelude.items.clone();
    items.extend(program.items.iter().filter(|i| !is_arena_import(i)).cloned());
    Some(Program::from_items_of(items, [program, &prelude]))
}

fn is_arena_import(item: &Item) -> bool {
    match item {
        Item::Use { path, .. } => {
            let p = path.join("::").replace('.', "::");
            p == "std::arena" || p.starts_with("std::arena::")
        }
        _ => false,
    }
}
