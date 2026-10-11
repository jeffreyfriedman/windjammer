//! Arena benchmark suite (ADR-0016). Three workloads, each in several variants that compute the same checksum:
//!
//! * particles: spawn N particles per frame, integrate, drop them all at frame end (F frames)
//! * tree:      build a binary tree (AST-like, depth D), sum it, drop it (R rounds)
//! * scratch:   build K small formatted strings per frame, use them, drop them (F frames)
//!
//! Variants: `box`/`vec-new`/`string-new` = plain per-object allocation; `wj-arena` = windjammer_runtime::arena
//! (`Arena<T>` + `Handle<T>`, `Scratch`); `bumpalo` = hand-written Rust + bumpalo; `reuse` = hand-tuned
//! buffer reuse. Reports best-of-R wall time and heap allocations per frame (counting allocator).

use std::alloc::{GlobalAlloc, Layout, System};
use std::fmt::Write as _;
use std::hint::black_box;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use windjammer_runtime::arena::{Arena, Handle, Scratch};

struct Counting;
static ALLOCS: AtomicU64 = AtomicU64::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, n: usize) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::Relaxed);
        System.realloc(p, l, n)
    }
}
#[global_allocator]
static A: Counting = Counting;

#[derive(Clone, Copy)]
struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
}

fn spawn(i: usize, frame: usize) -> Particle {
    let f = (i * 7 + frame * 13) as f32;
    Particle { x: f * 0.01, y: f * 0.02, vx: (i % 5) as f32 - 2.0, vy: (i % 3) as f32, life: 1.0 + (i % 4) as f32 }
}

fn step(p: &mut Particle) {
    p.vy -= 0.98;
    p.x += p.vx * 0.016;
    p.y += p.vy * 0.016;
    p.life -= 0.016;
}

const N: usize = 20_000;
const FRAMES: usize = 60;

fn particles_box() -> f64 {
    let mut sum = 0.0f64;
    for frame in 0..FRAMES {
        let mut ps: Vec<Box<Particle>> = Vec::new();
        for i in 0..N {
            ps.push(Box::new(spawn(i, frame)));
        }
        for _ in 0..4 {
            for p in ps.iter_mut() {
                step(p);
            }
        }
        sum += ps.iter().map(|p| (p.x + p.y + p.life) as f64).sum::<f64>();
    }
    sum
}

fn particles_vec_new() -> f64 {
    let mut sum = 0.0f64;
    for frame in 0..FRAMES {
        let mut ps: Vec<Particle> = Vec::new();
        for i in 0..N {
            ps.push(spawn(i, frame));
        }
        for _ in 0..4 {
            for p in ps.iter_mut() {
                step(p);
            }
        }
        sum += ps.iter().map(|p| (p.x + p.y + p.life) as f64).sum::<f64>();
    }
    sum
}

fn particles_wj_arena() -> f64 {
    let mut sum = 0.0f64;
    let mut arena: Arena<Particle> = Arena::new();
    let mut live: Vec<Handle<Particle>> = Vec::new();
    for frame in 0..FRAMES {
        arena.reset();
        live.clear();
        for i in 0..N {
            live.push(arena.alloc(spawn(i, frame)));
        }
        for _ in 0..4 {
            for p in arena.iter_mut() {
                step(p);
            }
        }
        sum += live.iter().map(|h| { let p = &arena[*h]; (p.x + p.y + p.life) as f64 }).sum::<f64>();
    }
    sum
}

fn particles_bumpalo() -> f64 {
    let mut sum = 0.0f64;
    let mut bump = bumpalo::Bump::new();
    for frame in 0..FRAMES {
        bump.reset();
        let mut ps: bumpalo::collections::Vec<&mut Particle> = bumpalo::collections::Vec::new_in(&bump);
        for i in 0..N {
            ps.push(bump.alloc(spawn(i, frame)));
        }
        for _ in 0..4 {
            for p in ps.iter_mut() {
                step(p);
            }
        }
        sum += ps.iter().map(|p| (p.x + p.y + p.life) as f64).sum::<f64>();
    }
    sum
}

// ---- tree ----------------------------------------------------------------------------------------------

const DEPTH: u32 = 16;
const ROUNDS: usize = 8;

enum BoxNode {
    Leaf(i64),
    Add(Box<BoxNode>, Box<BoxNode>),
}
fn box_build(d: u32, seed: i64) -> BoxNode {
    if d == 0 {
        BoxNode::Leaf(seed % 7)
    } else {
        BoxNode::Add(Box::new(box_build(d - 1, seed * 3 + 1)), Box::new(box_build(d - 1, seed * 5 + 2)))
    }
}
fn box_eval(n: &BoxNode) -> i64 {
    match n {
        BoxNode::Leaf(v) => *v,
        BoxNode::Add(a, b) => box_eval(a).wrapping_add(box_eval(b)),
    }
}
fn tree_box() -> f64 {
    (0..ROUNDS).map(|r| box_eval(&black_box(box_build(DEPTH, r as i64))) as f64).sum()
}

#[derive(Clone, Copy)]
enum HNode {
    Leaf(i64),
    Add(Handle<HNode>, Handle<HNode>),
}
fn h_build(a: &mut Arena<HNode>, d: u32, seed: i64) -> Handle<HNode> {
    if d == 0 {
        a.alloc(HNode::Leaf(seed % 7))
    } else {
        let l = h_build(a, d - 1, seed * 3 + 1);
        let r = h_build(a, d - 1, seed * 5 + 2);
        a.alloc(HNode::Add(l, r))
    }
}
fn h_eval(a: &Arena<HNode>, h: Handle<HNode>) -> i64 {
    match a[h] {
        HNode::Leaf(v) => v,
        HNode::Add(l, r) => h_eval(a, l).wrapping_add(h_eval(a, r)),
    }
}
fn tree_wj_arena() -> f64 {
    let mut a = Arena::new();
    (0..ROUNDS)
        .map(|r| {
            a.reset();
            let root = h_build(&mut a, DEPTH, r as i64);
            h_eval(&a, black_box(root)) as f64
        })
        .sum()
}

enum BNode<'b> {
    Leaf(i64),
    Add(&'b BNode<'b>, &'b BNode<'b>),
}
fn b_build<'b>(bump: &'b bumpalo::Bump, d: u32, seed: i64) -> &'b BNode<'b> {
    if d == 0 {
        bump.alloc(BNode::Leaf(seed % 7))
    } else {
        let l = b_build(bump, d - 1, seed * 3 + 1);
        let r = b_build(bump, d - 1, seed * 5 + 2);
        bump.alloc(BNode::Add(l, r))
    }
}
fn b_eval(n: &BNode) -> i64 {
    match n {
        BNode::Leaf(v) => *v,
        BNode::Add(a, b) => b_eval(a).wrapping_add(b_eval(b)),
    }
}
fn tree_bumpalo() -> f64 {
    let mut bump = bumpalo::Bump::new();
    let mut total = 0.0;
    for r in 0..ROUNDS {
        bump.reset();
        total += b_eval(black_box(b_build(&bump, DEPTH, r as i64))) as f64;
    }
    total
}

// ---- scratch strings -----------------------------------------------------------------------------------

const K: usize = 5_000;

fn scratch_string_new() -> f64 {
    let mut total = 0usize;
    for frame in 0..FRAMES {
        let mut labels: Vec<String> = Vec::new();
        for i in 0..K {
            let mut s = String::new();
            write!(s, "entity#{} hp={} f={}", i, (i * 7 + frame) % 100, frame).unwrap();
            labels.push(s);
        }
        total += labels.iter().map(|s| s.len()).sum::<usize>();
    }
    total as f64
}

fn scratch_wj() -> f64 {
    let mut total = 0usize;
    let mut sc = Scratch::new();
    for frame in 0..FRAMES {
        sc.reset();
        let mut labels = sc.list();
        for i in 0..K {
            let mut s = sc.string();
            write!(s, "entity#{} hp={} f={}", i, (i * 7 + frame) % 100, frame).unwrap();
            labels.push(s.into_bump_str());
        }
        total += labels.iter().map(|s| s.len()).sum::<usize>();
    }
    total as f64
}

fn scratch_reuse() -> f64 {
    // Hand-tuned: one text buffer + offsets, reused every frame.
    let mut total = 0usize;
    let mut buf = String::new();
    let mut ends: Vec<usize> = Vec::new();
    for frame in 0..FRAMES {
        buf.clear();
        ends.clear();
        for i in 0..K {
            write!(buf, "entity#{} hp={} f={}", i, (i * 7 + frame) % 100, frame).unwrap();
            ends.push(buf.len());
        }
        let mut start = 0;
        for e in &ends {
            total += e - start;
            start = *e;
        }
    }
    total as f64
}

fn measure(name: &str, frames: usize, f: fn() -> f64, reps: usize) -> (f64, f64) {
    let check = f();
    let mut best = f64::MAX;
    let mut allocs = 0;
    for _ in 0..reps {
        let a0 = ALLOCS.load(Ordering::Relaxed);
        let t = Instant::now();
        let v = black_box(f());
        best = best.min(t.elapsed().as_secs_f64());
        allocs = ALLOCS.load(Ordering::Relaxed) - a0;
        assert_eq!(v.to_bits(), check.to_bits(), "{name}: nondeterministic");
    }
    println!("{name:<28} {:>9.3} ms   {:>10.1} allocs/frame   checksum {check}", best * 1e3, allocs as f64 / frames as f64);
    (best, check)
}

fn main() {
    let reps: usize = std::env::args().nth(1).and_then(|s| s.parse().ok()).unwrap_or(15);
    println!("# arena benchmark suite (best of {reps}); particles N={N} x {FRAMES} frames, tree depth {DEPTH} x {ROUNDS}, scratch K={K} x {FRAMES}");
    let groups: [(&str, usize, Vec<(&str, fn() -> f64)>); 3] = [
        ("particles", FRAMES, vec![("box (Vec<Box<P>>)", particles_box), ("vec-new (Vec<P>/frame)", particles_vec_new), ("wj-arena (Arena<P>)", particles_wj_arena), ("bumpalo (&mut P)", particles_bumpalo)]),
        ("tree", ROUNDS, vec![("box (Box<Node>)", tree_box), ("wj-arena (Handle<Node>)", tree_wj_arena), ("bumpalo (&Node)", tree_bumpalo)]),
        ("scratch", FRAMES, vec![("string-new (String/label)", scratch_string_new), ("wj-scratch (Scratch)", scratch_wj), ("reuse (1 buffer, hand-tuned)", scratch_reuse)]),
    ];
    for (g, frames, vs) in groups {
        println!("\n## {g}");
        let mut base = None;
        let mut first_check = None;
        for (name, f) in vs {
            let (t, c) = measure(name, frames, f, reps);
            if let Some(fc) = first_check {
                assert_eq!(fc, c, "{g}: variants disagree");
            }
            first_check = Some(c);
            let b = *base.get_or_insert(t);
            println!("{:<28} speedup vs first: {:.2}x", "", b / t);
        }
    }
}
