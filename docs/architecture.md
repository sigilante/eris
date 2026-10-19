# Eris architecture

Eris is a Nock runtime with dual heritage: Urbit's **Vere** supplies the *urth*
(the king — the nondeterministic outside world) and Nockchain's **nockvm**
(ex-Ares/Sword) supplies the *mars* (the serf — the deterministic Nock
computer). The two run as separate processes along Vere's urth/mars split and
talk over the frozen newt seam (`seam-contract.md`).

```
            ┌───────────────────────────────────────────┐
            │  urth / KING   (vere C — third_party/vere)  │
            │                                             │
            │  IO drivers (Ames, Behn, Eyre, Clay, term)  │
            │  process supervision · ivory-lite kernel    │
            │  newt IPC · (king-side pier plumbing)       │
            └───────────────────────────────────────────┘
                     │  fd0 writ ↓     ↑ fd1 plea   (jammed nouns,
                     │                              5-byte framed, proto v2)
            ┌───────────────────────────────────────────┐
            │  mars / SERF   (Eris Rust — serf/)          │
            │                                             │
            │  nockvm interpreter + jets   (from nockchain)│
            │  PMA persistence  ·  event log + replay      │
            │  the Serf lifecycle (poke/peek/snapshot)     │
            └───────────────────────────────────────────┘
```

## Division of responsibility

| Concern                         | Owner | Notes |
|---------------------------------|-------|-------|
| IO / networking / timers / term | king  | Vere's battle-tested drivers, unchanged |
| Process supervision, spawn      | king  | spawns the Rust serf via stock `--serf-bin` (no patch) |
| newt IPC framing + protocol     | both  | contract frozen in `seam-contract.md` |
| Nock 0–12 execution             | serf  | nockvm interpreter |
| Jets (`%fast` dashboard)        | serf  | **must be re-pinned to Arvo's kelvin** |
| Memory (loom/NockStack)         | serf  | serf-internal; king never reads it |
| Persistence (snapshot)          | serf  | PMA; replaces vere's loom snapshot |
| **Event log + replay**          | serf  | serf owns the log; **no `%play`** |
| Determinism (mug, jam/cue, scry)| both  | already compatible (see PROVENANCE) |

## The three real engineering costs (in priority order)

1. **Jet coverage + kelvin re-pin — dominant cost; this is what sank Ares.**
   nockvm's jets are keyed to Hoon **k138** (`jets/hot.rs` `K_138`); Vere's live
   stdlib is **k140** (~278 jet C functions, layers k135–k140). Jets only fire
   against a matching kelvin, so against current Arvo nockvm's jets don't fire at
   all — correct but unjetted, and unjetted Hoon compilation (`ut`) is unusably
   slow. Work: re-pin to the Arvo kelvin and port missing arms, prioritizing the
   `ut` compiler jets (vere `pkg/noun/jets/f/`) and `e/` crypto. Nockchain's own
   ~148 `zkvm-jetpack` jets are Arvo-irrelevant and simply never loaded.

   **Eris adds its own jets** without patching nockvm's dashboard — see
   `jets::eris_hot_state` and `serf/src/jets/`: the `+by` map door (`map.rs`) and
   `+in` set door (`set.rs`), both cribbed from `zkvm-jetpack`'s `z-by`/`z-in`
   with the comparator swapped back to stock `+gor`/`+mor`; PROVENANCE has the
   detail.

   **Measured, 2026-10-20 (k138 solid pill, `boot-pill`):** porting arms is not a
   blind win, and the priority is *not* obvious from the Vere jet count. An
   `nock_opcode_census` of the base-desk compile (`use[0]`, ~191s) shows
   **~13 billion interpreted Nock opcodes** — op0 slot 2.2B, op6 branch 1.85B,
   op5 equals 1.56B, op8 pin 1.24B, **op9 call 1.10B**, op10 edit 862M, op11 hint
   393M — i.e. ~68M interpreted ops/sec grinding through the compiler and its hot
   stdlib helpers. Against that, the entire `+by` door is invoked only ~0.3M times
   (get 138k, put 69k, del 7k, gas 13k, …), so jetting it buys ~no boot speedup
   (it remains worthwhile for map-heavy steady-state work and as the porting
   template).

   **Profiled, 2026-10-20 (`sample` during `use[0]`) — the lever is NOT jets.**
   The authoritative top-of-stack self-time leaders are, in order:
   `mem::noun_preserve` (copying live nouns across NockStack frames on return),
   `noun::TaggedPtr::resolve_const` (resolving tagged / NounSpace pointers on
   every access), raw `interpret`, then allocation (`raw_alloc_in_previous_frame`,
   frame push/pop, `Hamt::preserve`, malloc/memset) and `unifying_equality`. The
   compile is *structured* by the Hoon parser combinators (`pfix`/`stew`/`plug`/
   `cook`/`pose`/`glue`/`stag`/`here`) — but those are **already jetted**, with
   high inclusive and low self time; the cost under their `slam_with_space` calls
   is the preserve + pointer-resolve + alloc + interpret machinery. `+by` and
   `+in` fire but are noise (`set_uni` is the busiest of them). **Conclusion:
   porting more of the stdlib jet long tail will not make boot usable.** The real
   levers are nockvm runtime internals — cut `noun_preserve`/allocation churn,
   make `TaggedPtr`/`NounSpace` resolution cheaper, tighten `slam_with_space` —
   i.e. the same ground as the upstream 64-bit / pointer-compression work
   (PROVENANCE's pin note), which should narrow the ~16x gap to Vere's u3 (loom +
   direct pointers + refcounting; no per-access resolve, no frame-preserve copy).
   Jets stay a correctness/compat tool, and matter for map/set-heavy steady-state
   work — not the boot-speed lever.

2. **The newt shim + wyrd/kelvin handshake.** nockapp's `Serf` already does
   poke/peek/snapshot/log/replay, but over a tokio mpsc channel with in-memory
   noun copies. The shim swaps that transport for the 5-byte jammed-noun pipe and
   emits the plea set. jam/cue and mug are already wire-compatible, so this is
   well-scoped; the one subtlety is advertising a `kel` the king's wyrd handshake
   accepts (ties back to cost #1).

3. **PMA GC.** nockvm's PMA is a monotonic bump arena with copy-on-write persist
   and **no reclamation yet** ("Milestone 4"). A long-running ship accretes
   unreclaimable garbage; vere's `meld`/`pack`/`cram` is mature. Finish PMA GC or
   accept periodic full-replay compaction.

## Non-issues (recorded so they don't get re-litigated)

- 64-bit nouns / non-loom snapshot format: serf-internal, invisible across the
  seam. Only blocks adopting a stock-vere ship's snapshot; log replay migrates.
- SQLite vs LMDB log: both designs put the log on the serf; it's a format choice.
- Hints: the load-bearing set (`%fast`, `%mean`/`%spot`/`%hunk`, `%slog`,
  `%memo`) is handled by nockvm; `%hela` is a stub and `%germ`/`%sole`/`%nara`
  are absent but degrade gracefully (unknown hints are transparent).

## Repository map

- `third_party/vere` — pristine king upstream (pinned submodule).
- `third_party/nockchain` — pristine serf upstream (pinned submodule).
- `king/` — reserved for Eris-owned C glue over vere; currently none needed (the
  king is stock vere, driven by its own `--serf-bin` flag; `mars.c` stays
  compiled but unused). Built as `eris` via `-Dbinary-name` — no patch.
- `serf/` — Eris-owned Rust serf crate (newt shim + Serf over nockvm).
- `patches/` — re-appliable divergences from the pinned upstreams.
- `docs/seam-contract.md` — the frozen king↔serf interface.
- `PROVENANCE.md` / `CHANGES.md` — where code comes from / what we changed.

Build: two independent artifacts (serf via Cargo, king via vere's Zig build)
wired by a `justfile`; the king is stock vere driven by `--serf-bin`. See
`docs/build.md`.
