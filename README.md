# Eris

A Discordian implementation of the Urbit runtime for use by the most beautiful.

![](./img/kallisti.png)

This repository contains an Urbit runtime environment with dual heritage from
Vere and Ares (NockVM/Sword).  It should be considered experimental and used
only with moons or comets on the live network.

## What it is

Eris runs an Urbit ship along Vere's **urth/mars** split, but swaps the Nock
computer:

- the **urth / king** is stock Vere (`third_party/vere`) — its IO drivers,
  process supervision, and the newt IPC pipe;
- the **mars / serf** is **nockvm** (ex-Ares/Sword, from Nockchain,
  `third_party/nockchain`) — a Rust Nock interpreter with jets and a persistent
  memory arena (PMA).

The king spawns the serf (`eris-serf`) and drives it over Vere's newt protocol.
Both upstreams are pinned git submodules; every divergence is a patch in
`patches/` or Eris-owned code in `serf/` / `king/`. See `PROVENANCE.md`,
`CHANGES.md`, and `docs/{architecture,build,seam-contract}.md`.

Status: **a fakezod boots and runs live under nockvm, king-driven** (mesa +
conn + http up, `pier: live`), with PMA-persisted, jet-warm restart. It is slow
(see "Jet coverage" below) and rough; not for production.

## Prerequisites

- **Rust** `nightly-2026-04-03` (pinned in `rust-toolchain.toml`; matches the
  nockchain pin). `rustup` installs it automatically.
- **Zig** `0.15.2` (matches Vere's pin). `zvm install 0.15.2 && zvm use 0.15.2`.
- **just** (`brew install just` / `cargo install just`) for the orchestrator.
- A C toolchain + the usual Vere build deps (handled by Vere's vendored `ext/*`).
- Clone with submodules: `git clone --recursive`, or
  `git submodule update --init --recursive` in an existing checkout.

### macOS 26 / Xcode SDK 27 note

zig 0.15.2 cannot read the macOS 26/27 SDK's `libSystem.tbd` and fails to link.
`just king` works around this automatically via `tools/xcrun-shim/` (reports an
older `MacOSX15.4.sdk`) + a project-local zig cache; both self-disable on other
hosts. See `docs/build.md`.

## Build

```sh
just apply      # apply the patch series onto the pinned submodules (idempotent)
just build      # = just serf + just king
# or individually:
just serf       # -> serf/target/release/eris-serf
just king       # -> third_party/vere/zig-out/<triple>/eris   (first build is slow)
just serf-bin   # print the serf binary path
just king-bin   # print the king binary path
```

## Pills

Eris needs a **Hoon kelvin 138** Arvo pill (nockvm's jets are pinned to k138).
The repo does not ship pills (they are large). To extract a k138 solid pill from
`urbit/urbit` history (Hoon was 138 from 2023-05 to 2024-04):

```sh
# in an urbit/urbit checkout, e.g. commit 0b3ab43c85 (hoon %138, zuse 411):
git -C <urbit> lfs fetch origin 0b3ab43c85 --include=bin/solid.pill
git -C <urbit> show 0b3ab43c85:bin/solid.pill  # (LFS pointer; the object is now local)
# copy the materialized object to e.g. ~/urbit-hoon138-solid.pill
```

A newer (k135/k140) pill will boot *correctly* but entirely unjetted — far too
slow to be usable — until the jets are re-pinned to that kelvin.

## Running a ship

### Live ship (king-driven)

```sh
just dev PILL=/path/to/urbit-hoon138-solid.pill ship=bus
```

This runs the king (`eris`) pointed at the serf via `--serf-bin`, booting a fake
`~bus`. Equivalently, by hand:

```sh
king=$(just king-bin)
serf=$(just serf-bin)
"$king" -F bus --serf-bin "$serf" \
        --bootstrap /path/to/urbit-hoon138-solid.pill --pier ./pier-bus
```

What happens: the king spawns `eris-serf boot`, which cues the pill, builds Arvo
(~3–4 min — the base-desk compile is the slow part), snapshots to
`pier-bus/eris.pma`, and exits 0; the king then spawns `eris-serf work`, which
restores jet-warm from the PMA, emits `%ripe`, and serves the king's events. A
successful boot ends with `pier (N): live` and an http/conn/mesa listener. It is
a real, if slow, live ship — drive it from the dojo (interactive) or over
`.urb/conn.sock`.

Notes:
- Pick a `ship` whose Ames port is free. `~zod` (port 31337) may already be
  taken by another running ship — `~bus` etc. avoid the clash.
- `-t` disables the terminal (headless); add `-x` to boot-then-exit.
- `just clean-pier` removes the dev pier (destroys the ship's state).

### Dev commands (no king, in-process)

The serf has two standalone subcommands, handy for testing nockvm directly:

```sh
# evaluate a jammed [subject formula] noun and print the result:
eris-serf eval <file.noun>
#   (e.g. the self-contained benchmarks in ~/Code/urbit/benchmark/desk/bar)

# boot an Arvo kernel from a pill in-process (no king/newt/persistence):
eris-serf boot-pill <file.pill>
```

## Persistence

The serf owns persistence (this Vere's king has no disk). `boot` evacuates the
kernel + jet cold-state into `pier/eris.pma` (a growable, sparse mmap arena;
roots recorded in `pier/eris.roots`); `work` reopens it and rebuilds the jet
dashboard so pokes run jetted. `%sync`/`%exit` re-persist live events into the
same PMA. (PMA garbage collection is not yet implemented, so the arena only
grows — fine for short sessions.)

## Jet coverage (why it's slow)

nockvm jets the critical path — arithmetic, bits, list/set/sort, some crypto
(aes/ed/sha), and the Hoon compiler (`mink`/`mule`/`ut`) — which is why a boot
*completes*. But it fires ~97 jet cores where Vere installs ~2179, so most of the
standard library runs as interpreted Nock. Vere boots the same pill in ~12s;
Eris takes a few minutes.

Eris adds its own jets without patching nockvm (registered via
`jets::eris_hot_state`): the standard `+by` map door (`serf/src/jets/map.rs`) and
`+in` set door (`serf/src/jets/set.rs`). **But widening coverage arm-by-arm does
not speed up boot — this was profiled, not assumed.** An opcode census of the
base-desk compile shows ~13 billion interpreted Nock opcodes (op9 *call* ≈ 1.1B);
a sampling profile then showed the real self-time sinks are **nockvm's memory
model** — `noun_preserve` (cross-frame copying), `TaggedPtr::resolve_const`
(per-access pointer resolution), allocation churn, and the raw interpret loop.
The Hoon parser combinators that *structure* the compile (`pfix`/`stew`/`plug`/…)
are already jetted; `+by`/`+in` fire but are noise. So the lever for a usable
ship is **nockvm runtime internals** (preserve/alloc churn, pointer resolution,
gate-slam), or the upstream 64-bit/pointer-compression work — not the stdlib jet
long tail. The map/set jets remain worthwhile for map-heavy *steady-state* ship
operation (gall/clay) and as the porting template. See `docs/architecture.md`.

## Layout

```
third_party/{vere,nockchain}   pristine pinned upstreams (submodules)
patches/{vere,nockchain}/      re-appliable divergences (apply.sh)
serf/                          the Rust mars (eris-serf): eval, boot-pill, boot, work
king/                          Eris-owned C over Vere (currently none needed)
docs/                          architecture, build, the frozen seam contract
PROVENANCE.md / CHANGES.md     where code comes from / what we changed
```
