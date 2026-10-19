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
