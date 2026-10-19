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

use std::io::{Read, Write};
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
use nockvm::mug::mug_u32;
use nockvm::offset::PmaOffsetWords;
use nockvm::pma::{Pma, PmaCopy};
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
        // The king spawns us as `eris-serf boot|work <flags>` over the newt pipe.
        Some("boot") => {
            let snap = flag(&args, "--snap-dir").unwrap_or(".");
            if let Err(e) = run_boot(snap) {
                eprintln!("eris-serf boot: {e}");
                std::process::exit(1);
            }
            // exit 0 == boot success (the king watches the exit status)
        }
        Some("work") => {
            let snap = flag(&args, "--snap-dir").unwrap_or(".");
            if let Err(e) = run_work(snap) {
                eprintln!("eris-serf work: {e}");
                std::process::exit(1);
            }
        }
        _ => stub_report(&args),
    }
}

/// Value of a `--flag <value>` style option from argv, if present.
fn flag<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(String::as_str)
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

/// Poke one ovum `[wire card]` into Arvo, install the new kernel, log timing.
fn poke_ovum(
    ctx: &mut Context,
    arvo: &mut Noun,
    ovum: Noun,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let t = std::time::Instant::now();
    match poke_once(ctx, *arvo, ovum) {
        Ok(res) => {
            *arvo = tail_of(ctx, res)?;
            eprintln!("  {label} ok in {:.2?}", t.elapsed());
            Ok(())
        }
        Err(e) => {
            eprintln!("  {label} CRASH after {:.2?}: {e}", t.elapsed());
            Err(format!("poke {label} crashed").into())
        }
    }
}

fn poke_list(
    ctx: &mut Context,
    arvo: &mut Noun,
    mut list: Noun,
    label: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut i = 0usize;
    while list.is_cell() {
        let ovum = head_of(ctx, list)?;
        let next = tail_of(ctx, list)?;
        poke_ovum(ctx, arvo, ovum, &format!("{label}[{i}]"))?;
        i += 1;
        list = next;
    }
    eprintln!("  {label}: {i} events processed");
    Ok(i)
}

// ---- synthetic boot prelude (mirrors vere mars.c _mars_boot_make) ----

/// A `@tas`/`@t` mote from ASCII bytes (little-endian cord). `""` -> `0` (%$).
fn mote(stack: &mut NockStack, s: &str) -> Noun {
    if s.is_empty() {
        D(0)
    } else {
        Atom::from_bytes(stack, s.as_bytes()).as_noun()
    }
}

/// `[%$ %arvo ~]` — the wire for module (kernel) ova.
fn arvo_wire(stack: &mut NockStack) -> Noun {
    let arvo = mote(stack, "arvo");
    T(stack, &[D(0), arvo, D(0)])
}

/// `[wire [%wyrd [sen ver] kel]]` — runtime kelvin negotiation. `kel` must
/// match the pill's Arvo (zuse 411 / lull 323 / arvo 237 / hoon 138 / nock 4).
fn wyrd_ovum(stack: &mut NockStack) -> Noun {
    let wire = arvo_wire(stack);
    let zuse = mote(stack, "zuse");
    let z = T(stack, &[zuse, D(411)]);
    let lull = mote(stack, "lull");
    let l = T(stack, &[lull, D(323)]);
    let arvo = mote(stack, "arvo");
    let a = T(stack, &[arvo, D(237)]);
    let hoon = mote(stack, "hoon");
    let h = T(stack, &[hoon, D(138)]);
    let nock = mote(stack, "nock");
    let n = T(stack, &[nock, D(4)]);
    let kel = T(stack, &[z, l, a, h, n, D(0)]); // (list weft), null-terminated
    let vere = mote(stack, "vere");
    let pace = mote(stack, "live");
    let vers = mote(stack, "~.eris");
    let ver = T(stack, &[vere, pace, vers, D(0)]); // path [%vere %live ~.eris ~]
    let sen = mote(stack, "0v1s.vu178");
    let senver = T(stack, &[sen, ver]);
    let wyrd = mote(stack, "wyrd");
    let card = T(stack, &[wyrd, senver, kel]);
    T(stack, &[wire, card])
}

/// `[wire [%verb ~ |]]` — verbosity off.
fn verb_ovum(stack: &mut NockStack) -> Noun {
    let wire = arvo_wire(stack);
    let verb = mote(stack, "verb");
    let card = T(stack, &[verb, D(0), D(1)]);
    T(stack, &[wire, card])
}

/// `[wire [%whom ~zod]]` — identity. ~zod = @p 0.
fn whom_ovum(stack: &mut NockStack) -> Noun {
    let wire = arvo_wire(stack);
    let whom = mote(stack, "whom");
    let card = T(stack, &[whom, D(0)]);
    T(stack, &[wire, card])
}

/// `[wire [%wack eny]]` — entropy (34 bytes = 17 half-words, per mars.c).
fn wack_ovum(stack: &mut NockStack) -> Noun {
    let wire = arvo_wire(stack);
    let wack = mote(stack, "wack");
    let eny = Atom::from_bytes(stack, &[0xA5u8; 34]).as_noun();
    let card = T(stack, &[wack, eny]);
    T(stack, &[wire, card])
}

/// `[[%d %term '1' ~] [%boot | [%fake ~zod]]]` — legacy fakezod boot event.
fn boot_ovum(stack: &mut NockStack) -> Noun {
    let d = mote(stack, "d");
    let term = mote(stack, "term");
    let wire = T(stack, &[d, term, D(0x31), D(0)]); // [%d %term '1' ~]
    let fake = mote(stack, "fake");
    let vent = T(stack, &[fake, D(0)]); // [%fake ~zod]
    let boot = mote(stack, "boot");
    let card = T(stack, &[boot, D(1), vent]); // [%boot %.n vent]  (not lite)
    T(stack, &[wire, card])
}

// ===================================================================
// newt seam: king-driven boot/work over the stdio pipe (see
// docs/seam-contract.md). Two process invocations: `boot` consumes one
// %boot writ, builds the kernel, snapshots, and exits 0 (= success);
// `work` loads the snapshot, emits %ripe, then serves writs.
// ===================================================================

fn io_err(msg: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, msg.to_string())
}

/// Read one newt frame (5-byte header: 0x00 + LE u32 len; body = jammed noun).
/// `Ok(None)` on a clean EOF at a frame boundary.
fn read_frame<R: Read>(r: &mut R) -> std::io::Result<Option<Vec<u8>>> {
    let mut hdr = [0u8; 5];
    let mut got = 0;
    while got < 5 {
        match r.read(&mut hdr[got..])? {
            0 if got == 0 => return Ok(None),
            0 => return Err(io_err("eof mid-header")),
            n => got += n,
        }
    }
    if hdr[0] != 0 {
        return Err(io_err("bad newt version byte"));
    }
    let len = u32::from_le_bytes([hdr[1], hdr[2], hdr[3], hdr[4]]) as usize;
    if len == 0 {
        return Err(io_err("zero-length newt frame"));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    Ok(Some(body))
}

fn write_frame<W: Write>(w: &mut W, body: &[u8]) -> std::io::Result<()> {
    let mut hdr = [0u8; 5]; // hdr[0] = 0 version
    hdr[1..5].copy_from_slice(&(body.len() as u32).to_le_bytes());
    w.write_all(&hdr)?;
    w.write_all(body)?;
    w.flush()
}

fn jam_bytes(ctx: &mut Context, n: Noun) -> Vec<u8> {
    let j = jam(&mut ctx.stack, n);
    let sp = ctx.stack.noun_space();
    j.in_space(&sp).to_le_bytes()
}

fn cue_bytes(ctx: &mut Context, bytes: &[u8]) -> Result<Noun, Error> {
    let a = Atom::from_bytes(&mut ctx.stack, bytes);
    cue(&mut ctx.stack, a)
}

/// Frame a plea noun and write it to fd1 (stdout is the plea channel; all
/// human output goes to stderr via eprintln).
fn write_plea(ctx: &mut Context, plea: Noun) -> std::io::Result<()> {
    let bytes = jam_bytes(ctx, plea);
    let mut out = std::io::stdout().lock();
    write_frame(&mut out, &bytes)
}

/// Persist {kernel, jet cold-state} to a PMA in `snap_dir` (+ a tiny roots
/// sidecar). Unlike a jam-file, this carries the `Cold` dashboard forward, so
/// `work` restores with jets warm. Called from `boot` (fresh PMA).
fn save_snapshot(
    ctx: &mut Context,
    snap_dir: &str,
    arvo: Noun,
    eve: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(snap_dir)?;
    let dir = std::path::Path::new(snap_dir);
    let pma_path = dir.join("eris.pma");
    let _ = std::fs::remove_file(&pma_path); // fresh boot snapshot
    let mut pma = Pma::new(NOCK_STACK_SIZE, pma_path).map_err(|e| format!("pma new: {e:?}"))?;

    let mut arvo = arvo;
    // Evacuate kernel + cold jet-state into the PMA (mutates them to offset-form).
    unsafe {
        arvo.copy_to_pma(&ctx.stack, &mut pma);
        ctx.cold.copy_to_pma(&ctx.stack, &mut pma);
    }
    // Record roots AFTER copy (file-relative, stable across restart).
    let arvo_raw = unsafe { arvo.as_raw() };
    let cold_off: u64 = ctx
        .cold
        .pma_offset(&pma)
        .ok_or("cold not in pma after copy")?
        .into();

    pma.persist_metadata();
    pma.sync_all()?;
    pma.sync_file()?;

    let mut buf = [0u8; 24];
    buf[0..8].copy_from_slice(&arvo_raw.to_le_bytes());
    buf[8..16].copy_from_slice(&cold_off.to_le_bytes());
    buf[16..24].copy_from_slice(&eve.to_le_bytes());
    std::fs::write(dir.join("eris.roots"), buf)?;
    Ok(())
}

/// Reopen a PMA snapshot into a fresh, jet-warm `Context`: install the arena,
/// resolve the kernel + `Cold` from recorded offsets, and rebuild `Hot`/`Warm`
/// from the restored cold (so jet dispatch hits on the first poke). Returns the
/// context, the live `Pma` (must outlive the context — its mmap backs the
/// offset nouns), the kernel, and the event number.
fn open_restore(
    snap_dir: &str,
) -> Result<(Context, Pma, Noun, u64), Box<dyn std::error::Error>> {
    let dir = std::path::Path::new(snap_dir);
    let pma = Pma::open(dir.join("eris.pma")).map_err(|e| format!("pma open: {e:?}"))?;
    let roots = std::fs::read(dir.join("eris.roots")).map_err(|e| format!("no roots: {e}"))?;
    if roots.len() < 24 {
        return Err("roots sidecar too short".into());
    }
    let arvo_raw = u64::from_le_bytes(roots[0..8].try_into().unwrap());
    let cold_off = u64::from_le_bytes(roots[8..16].try_into().unwrap());
    let eve = u64::from_le_bytes(roots[16..24].try_into().unwrap());

    let mut stack = NockStack::new(NOCK_STACK_SIZE, 0);
    stack.install_pma_arena(Arc::clone(pma.arena())); // MUST precede resolving PMA nouns
    let arvo = unsafe { Noun::from_raw(arvo_raw) };
    let mut cold =
        unsafe { Cold::from_pma_offset(&pma, PmaOffsetWords::from_words(cold_off)) };

    // Rebuild the derived/code parts from the RESTORED cold.
    let hot = Hot::init(&mut stack, URBIT_HOT_STATE);
    let test_jets = Hamt::<()>::new(&mut stack);
    let warm = Warm::init(&mut stack, &mut cold, &hot, &test_jets, JetDispatchMode::Exact);
    let cache = Hamt::<Noun>::new(&mut stack);
    let arena = stack.arena().clone();
    let ctx = Context {
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
    };
    Ok((ctx, pma, arvo, eve))
}

/// `[%ripe [pro=%2 kel=~] [who=~zod fake=&] [eve mug]]`
fn make_ripe(ctx: &mut Context, arvo: Noun, eve: u64) -> Noun {
    let mug = mug_u32(&mut ctx.stack, arvo) as u64;
    let ripe = mote(&mut ctx.stack, "ripe");
    let prokel = T(&mut ctx.stack, &[D(2), D(0)]); // [%2 ~]
    let whofake = T(&mut ctx.stack, &[D(0), D(0)]); // [~zod &]
    let evemug = T(&mut ctx.stack, &[D(eve), D(mug)]);
    T(&mut ctx.stack, &[ripe, prokel, whofake, evemug])
}

/// `[%sync eve mug]`
fn make_sync(ctx: &mut Context, arvo: Noun, eve: u64) -> Noun {
    let mug = mug_u32(&mut ctx.stack, arvo) as u64;
    let sync = mote(&mut ctx.stack, "sync");
    T(&mut ctx.stack, &[sync, D(eve), D(mug)])
}

/// Handle `[%poke mil ovum]`: slam the kernel, install, reply
/// `[%poke [%.y effects]]` (or `[%poke [%.n ~]]` on a Nock crash).
fn handle_poke(
    ctx: &mut Context,
    arvo: &mut Noun,
    eve: &mut u64,
    writ: Noun,
    _snap_dir: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let r1 = tail_of(ctx, writ)?; // [mil ovum]
    let ovum = tail_of(ctx, r1)?; // [wire card]
    match poke_once(ctx, *arvo, ovum) {
        Ok(res) => {
            let effects = head_of(ctx, res)?;
            *arvo = tail_of(ctx, res)?;
            *eve += 1;
            let poke = mote(&mut ctx.stack, "poke");
            let inner = T(&mut ctx.stack, &[D(0), effects]); // [%.y effects]
            let plea = T(&mut ctx.stack, &[poke, inner]);
            write_plea(ctx, plea)?;
        }
        Err(e) => {
            eprintln!("work: poke crashed: {e}");
            let poke = mote(&mut ctx.stack, "poke");
            let inner = T(&mut ctx.stack, &[D(1), D(0)]); // [%.n ~]
            let plea = T(&mut ctx.stack, &[poke, inner]);
            write_plea(ctx, plea)?;
        }
    }
    Ok(())
}

fn run_boot(snap_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("eris-serf boot: snap-dir={snap_dir}; reading %boot writ from fd0");
    let body = {
        let mut inp = std::io::stdin().lock();
        read_frame(&mut inp)?.ok_or("no %boot writ on fd0")?
    };
    let mut ctx = build_context(NOCK_STACK_SIZE, URBIT_HOT_STATE);
    let writ = cue_bytes(&mut ctx, &body).map_err(fmt_err)?;
    let tag = atom_str(&ctx, head_of(&ctx, writ)?);
    if tag != "boot" {
        return Err(format!("first writ is %{tag}, expected %boot").into());
    }
    let com = tail_of(&ctx, writ)?; // [pill vent more]
    let pill_c = head_of(&ctx, com)?; // [p=@ q=(unit ovum)]
    let p = head_of(&ctx, pill_c)?; // jammed-pill atom
    let pill_bytes = {
        let sp = ctx.stack.noun_space();
        p.in_space(&sp)
            .as_atom()
            .map_err(|_| "pill.p is not an atom")?
            .to_le_bytes()
    };
    eprintln!("eris-serf boot: pill {} bytes; booting", pill_bytes.len());
    let pill = cue_bytes(&mut ctx, &pill_bytes).map_err(fmt_err)?;
    let (arvo, eve) = boot_arvo(&mut ctx, pill)?;
    save_snapshot(&mut ctx, snap_dir, arvo, eve)?;
    eprintln!("eris-serf boot: booted eve={eve}, snapshot saved; exiting 0");
    Ok(())
}

fn run_work(snap_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    // `_pma` must outlive `ctx`: its mmap backs the restored offset-form nouns.
    let (mut ctx, _pma, mut arvo, mut eve) = open_restore(snap_dir)?;
    eprintln!("eris-serf work: restored from PMA (eve={eve}), jets warm");
    let ripe = make_ripe(&mut ctx, arvo, eve);
    write_plea(&mut ctx, ripe)?;
    eprintln!("eris-serf work: %ripe sent (eve={eve})");

    let mut inp = std::io::stdin().lock();
    loop {
        let body = match read_frame(&mut inp)? {
            Some(b) => b,
            None => {
                eprintln!("eris-serf work: fd0 closed; exiting");
                break;
            }
        };
        let writ = cue_bytes(&mut ctx, &body).map_err(fmt_err)?;
        let tag = atom_str(&ctx, head_of(&ctx, writ)?);
        match tag.as_str() {
            "poke" => handle_poke(&mut ctx, &mut arvo, &mut eve, writ, snap_dir)?,
            "peek" => {
                // v1: reply scry-miss [%peek [%.y ~]].
                let pk = mote(&mut ctx.stack, "peek");
                let inr = T(&mut ctx.stack, &[D(0), D(0)]);
                let pl = T(&mut ctx.stack, &[pk, inr]);
                write_plea(&mut ctx, pl)?;
            }
            // v1: ack only. Durable re-snapshot of post-boot live events (which
            // needs same-PMA re-persist) is a follow-up; the boot snapshot is the
            // durable baseline.
            "sync" => {
                let pl = make_sync(&mut ctx, arvo, eve);
                write_plea(&mut ctx, pl)?;
            }
            "live" => {
                let lv = mote(&mut ctx.stack, "live");
                let pl = T(&mut ctx.stack, &[lv, D(0)]); // [%live ~]
                write_plea(&mut ctx, pl)?;
            }
            "exit" => {
                eprintln!("eris-serf work: %exit; exiting 0");
                break;
            }
            other => eprintln!("eris-serf work: unhandled writ %{other}; ignoring"),
        }
    }
    Ok(())
}

/// Build + boot an Arvo kernel from an already-cued Vere pill noun.
/// Returns (arvo, eve) where eve = number of events applied. Shared by the
/// `boot-pill` dev command and the king-driven `boot` subcommand.
fn boot_arvo(ctx: &mut Context, pill: Noun) -> Result<(Noun, u64), Box<dyn std::error::Error>> {
    // Destructure [tag typ bot mod use] (unwrapping %cash -> real pill).
    let tag = head_of(ctx, pill)?;
    let tag_s = atom_str(ctx, tag);
    eprintln!("  tag = %{tag_s}");
    if tag_s == "ivory" {
        return Err("ivory pill cannot boot a full ship".into());
    }
    let mut rest = tail_of(ctx, pill)?; // [typ bot mod use]
    if tag_s == "cash" {
        let real = head_of(ctx, rest)?;
        rest = tail_of(ctx, real)?;
    }
    let typ = head_of(ctx, rest)?;
    eprintln!("  type = %{}", atom_str(ctx, typ));
    let r2 = tail_of(ctx, rest)?; // [bot mod use]
    let bot = head_of(ctx, r2)?;
    let r3 = tail_of(ctx, r2)?; // [mod use]
    let modl = head_of(ctx, r3)?;
    let usel = tail_of(ctx, r3)?;

    // Lifecycle: arvo = axis 7 of *[bot [2 [0 3] [0 2]]].
    let t1 = std::time::Instant::now();
    let p03 = T(&mut ctx.stack, &[D(0), D(3)]);
    let p02 = T(&mut ctx.stack, &[D(0), D(2)]);
    let lyf = T(&mut ctx.stack, &[D(2), p03, p02]);
    let gate = interpret(ctx, bot, lyf).map_err(fmt_err)?;
    let mut arvo = tail_of(ctx, tail_of(ctx, gate)?)?;
    eprintln!("  lifecycle -> base Arvo in {:.2?}", t1.elapsed());

    // Synthetic prelude (wyrd, verb, whom, wack), pill mod, %boot, pill use.
    let mut eve: u64 = 0;
    let ov = wyrd_ovum(&mut ctx.stack);
    poke_ovum(ctx, &mut arvo, ov, "wyrd")?;
    eve += 1;
    let ov = verb_ovum(&mut ctx.stack);
    poke_ovum(ctx, &mut arvo, ov, "verb")?;
    eve += 1;
    let ov = whom_ovum(&mut ctx.stack);
    poke_ovum(ctx, &mut arvo, ov, "whom")?;
    eve += 1;
    let ov = wack_ovum(&mut ctx.stack);
    poke_ovum(ctx, &mut arvo, ov, "wack")?;
    eve += 1;
    eve += poke_list(ctx, &mut arvo, modl, "mod")? as u64;
    let ov = boot_ovum(&mut ctx.stack);
    poke_ovum(ctx, &mut arvo, ov, "boot")?;
    eve += 1;
    eve += poke_list(ctx, &mut arvo, usel, "use")? as u64;
    Ok((arvo, eve))
}

fn run_boot_pill(path: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.ok_or("usage: eris-serf boot-pill <file.pill>")?;
    let bytes = std::fs::read(path)?;
    eprintln!("eris-serf boot-pill: {path} ({} bytes)", bytes.len());
    let mut ctx = build_context(NOCK_STACK_SIZE, URBIT_HOT_STATE);
    eprintln!("  context: {} hot-state jet entries", URBIT_HOT_STATE.len());
    let t0 = std::time::Instant::now();
    let jammed = Atom::from_bytes(&mut ctx.stack, &bytes);
    let pill = cue(&mut ctx.stack, jammed).map_err(fmt_err)?;
    eprintln!("  cued pill in {:.2?}", t0.elapsed());
    let (_arvo, eve) = boot_arvo(&mut ctx, pill)?;
    eprintln!("eris-serf boot-pill: fakezod booted (eve={eve}).");
    Ok(())
}
