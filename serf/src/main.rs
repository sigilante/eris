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
use nockvm::jets::hot::{Hot, HotEntry, URBIT_HOT_STATE};
use nockvm::jets::warm::Warm;
use nockvm::jets::JetDispatchMode;
use nockvm::mem::{NockStack, NOCK_STACK_SIZE_TINY, NOCK_STACK_SIZE};
use nockvm::noun::{Atom, Noun, D, T};
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
        Some("boot-pill") => {
            if let Err(e) = run_boot_pill(args.get(2).map(String::as_str)) {
                eprintln!("eris-serf boot-pill: {e}");
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
    // eval: pure-Nock test, no jets needed (correctness holds unjetted).
    build_context(NOCK_STACK_SIZE_TINY, &[])
}

fn build_context(size: usize, hot_entries: &[HotEntry]) -> Context {
    let mut stack = NockStack::new(size, 0);
    let arena = stack.arena().clone(); // grab before `stack` moves into Context
    let cold = Cold::new(&mut stack);
    let warm = Warm::new(&mut stack);
    let hot = Hot::init(&mut stack, hot_entries);
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

// ===================================================================
// boot-pill: build an Arvo kernel from a Vere pill and poke its events
// ===================================================================
//
// Mirrors Vere's u3v_life/u3v_boot + the module/userspace poke sequence
// (see docs/seam-contract.md). In-process, no king/newt: this isolates
// "can nockvm build + drive Arvo (and do the base jets fire)" from the
// transport plumbing. v1 pokes the pill's own `mod`++`use`; synthetic
// %wyrd/%whom/%wack identity events come once the raw path is understood.

const EPOCH_DA: u128 = 170_141_184_475_152_167_957_503_069_145_530_368_000;

fn da_now(stack: &mut NockStack) -> Noun {
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let da: u128 = EPOCH_DA + ((1u128 << 64) * ms) / 1000;
    Atom::from_bytes(stack, &da.to_le_bytes()).as_noun()
}

fn head_of(ctx: &Context, n: Noun) -> Result<Noun, String> {
    let sp = ctx.stack.noun_space();
    n.in_space(&sp)
        .as_cell()
        .map(|c| c.head().noun())
        .map_err(|_| "expected a cell (head)".to_string())
}

fn tail_of(ctx: &Context, n: Noun) -> Result<Noun, String> {
    let sp = ctx.stack.noun_space();
    n.in_space(&sp)
        .as_cell()
        .map(|c| c.tail().noun())
        .map_err(|_| "expected a cell (tail)".to_string())
}

fn atom_str(ctx: &Context, n: Noun) -> String {
    let sp = ctx.stack.noun_space();
    match n.in_space(&sp).as_atom() {
        Ok(a) => String::from_utf8_lossy(&a.to_le_bytes())
            .trim_end_matches('\0')
            .to_string(),
        Err(_) => "<cell>".to_string(),
    }
}

fn jam_size(ctx: &mut Context, n: Noun) -> usize {
    let j = jam(&mut ctx.stack, n);
    let sp = ctx.stack.noun_space();
    j.in_space(&sp).to_le_bytes().len()
}

/// `*[arvo-and-sample [8 [9 23 0 2] 9 2 10 [6 0 7] 0 2]]` — vere `+slam` of the
/// kernel's axis-23 gate with sample `[now ovum]`. Returns `[effects new-arvo]`.
fn poke_once(ctx: &mut Context, arvo: Noun, ovum: Noun) -> Result<Noun, Error> {
    let now = da_now(&mut ctx.stack);
    let sample = T(&mut ctx.stack, &[now, ovum]); // [now ovum]
    let sub = T(&mut ctx.stack, &[arvo, sample]); // [arvo [now ovum]]
    let pul = T(&mut ctx.stack, &[D(9), D(23), D(0), D(2)]); // [9 23 0 2]
    let edt = T(&mut ctx.stack, &[D(6), D(0), D(7)]); // [6 0 7]
    let fol = T(&mut ctx.stack, &[D(8), pul, D(9), D(2), D(10), edt, D(0), D(2)]);
    interpret(ctx, sub, fol)
}

fn poke_list(
    ctx: &mut Context,
    arvo: &mut Noun,
    mut list: Noun,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut i = 0usize;
    while list.is_cell() {
        let ovum = head_of(ctx, list)?;
        let next = tail_of(ctx, list)?;
        let t = std::time::Instant::now();
        match poke_once(ctx, *arvo, ovum) {
            Ok(res) => {
                *arvo = tail_of(ctx, res)?;
                eprintln!("  {label}[{i}] ok in {:.2?}", t.elapsed());
            }
            Err(e) => {
                eprintln!("  {label}[{i}] CRASH after {:.2?}: {e}", t.elapsed());
                return Err(format!("poke {label}[{i}] crashed").into());
            }
        }
        i += 1;
        list = next;
    }
    eprintln!("  {label}: {i} events processed");
    Ok(())
}

fn run_boot_pill(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.ok_or("usage: eris-serf boot-pill <file.pill>")?;
    let bytes = std::fs::read(path)?;
    eprintln!("eris-serf boot-pill: {path} ({} bytes)", bytes.len());

    // Jetted (URBIT_HOT_STATE = nockvm's ~136 base-k138 jets) so the Hoon
    // compiler / stdlib arms accelerate — the whole point of using a k138 pill.
    let mut ctx = build_context(NOCK_STACK_SIZE, URBIT_HOT_STATE);
    eprintln!("  context: {} hot-state jet entries", URBIT_HOT_STATE.len());

    // Cue the pill.
    let t0 = std::time::Instant::now();
    let jammed = Atom::from_bytes(&mut ctx.stack, &bytes);
    let pill = cue(&mut ctx.stack, jammed).map_err(fmt_err)?;
    eprintln!("  cued pill in {:.2?}", t0.elapsed());

    // Destructure [tag typ bot mod use] (unwrapping %cash -> real pill).
    let tag = head_of(&ctx, pill)?;
    let tag_s = atom_str(&ctx, tag);
    eprintln!("  tag = %{tag_s}");
    if tag_s == "ivory" {
        return Err("ivory pill cannot boot a full ship".into());
    }
    let mut rest = tail_of(&ctx, pill)?; // [typ bot mod use]
    if tag_s == "cash" {
        let real = head_of(&ctx, rest)?;
        rest = tail_of(&ctx, real)?;
    }
    let typ = head_of(&ctx, rest)?;
    eprintln!("  type = %{}", atom_str(&ctx, typ));
    let r2 = tail_of(&ctx, rest)?; // [bot mod use]
    let bot = head_of(&ctx, r2)?;
    let r3 = tail_of(&ctx, r2)?; // [mod use]
    let modl = head_of(&ctx, r3)?;
    let usel = tail_of(&ctx, r3)?;

    // Lifecycle: gate = *[bot [2 [0 3] [0 2]]]; arvo = axis 7 of gate.
    let t1 = std::time::Instant::now();
    let p03 = T(&mut ctx.stack, &[D(0), D(3)]);
    let p02 = T(&mut ctx.stack, &[D(0), D(2)]);
    let lyf = T(&mut ctx.stack, &[D(2), p03, p02]);
    let gate = interpret(&mut ctx, bot, lyf).map_err(fmt_err)?;
    let mut arvo = tail_of(&ctx, tail_of(&ctx, gate)?)?;
    let sz = jam_size(&mut ctx, arvo);
    eprintln!(
        "  lifecycle -> base Arvo in {:.2?}  (jam {sz} bytes)",
        t1.elapsed()
    );

    // Poke module events, then userspace events.
    poke_list(&mut ctx, &mut arvo, modl, "mod")?;
    poke_list(&mut ctx, &mut arvo, usel, "use")?;

    eprintln!("eris-serf boot-pill: all events processed.");
    Ok(())
}
