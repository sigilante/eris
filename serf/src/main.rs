//! Eris mars (serf): the deterministic Nock computer.
//!
//! Final form runs as a child of the vere king over the newt pipe (see
//! `docs/seam-contract.md`): read 5-byte-framed jammed writs on fd0, emit
//! `%ripe` first then the plea set on fd1, driving nockvm (interpreter + jets +
//! PMA) with the Serf lifecycle.
//!
//! Two subcommands exist today:
//!   * `work` / `boot` — the king's spawn. Not implemented yet: reports the argv
//!     the king handed us (the invocation contract) then exits, so the
//!     spawn/pipe wiring can be smoke-tested via `--serf-bin`.
//!   * `eval <file.noun>` — dev harness: cue a jammed `[subject formula]` noun
//!     and run nockvm's Nock interpreter on it. Proves the mars computes Nock,
//!     independent of the newt seam. Try it on ~/Code/urbit/benchmark nouns.

use std::sync::atomic::AtomicIsize;
use std::sync::Arc;

use nockvm::ext::AtomExt;
use nockvm::hamt::Hamt;
use nockvm::interpreter::{interpret, Context, Error, NockCancelToken, Slogger};
use nockvm::jets::cold::Cold;
use nockvm::jets::hot::Hot;
use nockvm::jets::warm::Warm;
use nockvm::jets::JetDispatchMode;
use nockvm::mem::{NockStack, NOCK_STACK_SIZE_TINY};
use nockvm::noun::{Atom, Noun, D};
use nockvm::serialization::{cue, jam};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("eval") => {
            if let Err(e) = run_eval(args.get(2).map(String::as_str)) {
                eprintln!("eris-serf eval: {e}");
                std::process::exit(1);
            }
        }
        // The king spawns us as `eris-serf work|boot <flags>`; the newt seam is
        // not implemented yet, so report the invocation and exit.
        _ => stub_report(&args),
    }
}

fn stub_report(args: &[String]) {
    eprintln!("eris-serf: spawned ({} args)", args.len());
    for (i, a) in args.iter().enumerate() {
        eprintln!("  argv[{i}] = {a}");
    }
    eprintln!("eris-serf: newt seam not yet implemented; exiting 1.");
    std::process::exit(1);
}

/// A slogger that drops all debug output (we don't surface `%slog` yet).
struct NullSlogger;
impl Slogger for NullSlogger {
    fn slog(&mut self, _stack: &mut NockStack, _pri: u64, _tank: Noun) {}
    fn flog(&mut self, _stack: &mut NockStack, _cord: Noun) {}
}

/// Build a minimal jetless `Context` sufficient for pure Nock (opcodes 0–11).
/// Mirrors nockvm's own `init_context` test helper, but with an empty hot state
/// (jets are optimizations; correctness holds without them) and no scry.
fn init_context() -> Context {
    let mut stack = NockStack::new(NOCK_STACK_SIZE_TINY, 0);
    let arena = stack.arena().clone(); // grab before `stack` moves into Context
    let cold = Cold::new(&mut stack);
    let warm = Warm::new(&mut stack);
    let hot = Hot::init(&mut stack, &[]); // empty: unjetted but correct
    let cache = Hamt::<Noun>::new(&mut stack);
    let test_jets = Hamt::<()>::new(&mut stack);
    Context {
        stack,
        op_budget: None,
        jet_dispatch: JetDispatchMode::Exact,
        slogger: Box::pin(NullSlogger),
        cold,
        warm,
        hot,
        cache,
        scry_stack: D(0),
        trace_info: None,
        running_status: Arc::new(AtomicIsize::new(NockCancelToken::RUNNING_IDLE)),
        test_jets,
        arena,
    }
}

fn run_eval(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.ok_or("usage: eris-serf eval <file.noun>")?;
    let bytes = std::fs::read(path)?;
    eprintln!("eris-serf eval: {path} ({} bytes jammed)", bytes.len());

    let mut ctx = init_context();

    // Cue the jamfile into a `[subject formula]` cell and split it.
    let jammed: Atom = Atom::from_bytes(&mut ctx.stack, &bytes);
    let pair: Noun = cue(&mut ctx.stack, jammed).map_err(fmt_err)?;
    let (subject, formula) = {
        let space = ctx.stack.noun_space();
        let cell = pair
            .in_space(&space)
            .as_cell()
            .map_err(|_| "cued noun is not a [subject formula] cell")?;
        (cell.head().noun(), cell.tail().noun())
    };

    let start = std::time::Instant::now();
    let res: Noun = interpret(&mut ctx, subject, formula).map_err(fmt_err)?;
    let elapsed = start.elapsed();

    print_result(&mut ctx, res);
    eprintln!("eris-serf eval: ok in {elapsed:.3?}");
    Ok(())
}

fn fmt_err(e: Error) -> String {
    format!("nock crash: {e}")
}

fn print_result(ctx: &mut Context, res: Noun) {
    {
        let space = ctx.stack.noun_space();
        let rh = res.in_space(&space);
        if let Ok(atom) = rh.as_atom() {
            if let Ok(v) = atom.as_u64() {
                println!("result = {v}");
                return;
            }
            let le = atom.to_le_bytes();
            let hex: String = le.iter().rev().map(|b| format!("{b:02x}")).collect();
            println!("result = <big atom, {} bytes> 0x{hex}", le.len());
            return;
        }
    }
    // A cell result: report its jammed size.
    let jammed = jam(&mut ctx.stack, res);
    let space = ctx.stack.noun_space();
    let n = jammed.in_space(&space).to_le_bytes().len();
    println!("result = <cell, jammed {n} bytes>");
}
