# Build system

**Core fact that shapes everything:** the king and the serf are **two separate
processes**, so the build never links C and Rust together or unifies the two
toolchains. It produces **two independent binaries** and tells the king where the
serf is. We therefore do *not* build a unified mega-build (no top-level Bazel,
though both upstreams ship it); we reuse each upstream's native build and glue
them with a thin orchestrator.

```
  serf  →  cargo   (Rust, nightly-2026-04-03)   →  eris-serf
  king  →  zig     (vere's build.zig, Zig 0.15.2) →  eris  (king; omits mars.c)
  glue  →  justfile: apply-patches → build-serf → build-king → dev
```

## Serf (Rust) — DONE: self-contained, isolated from the monorepo

Decision (owner): pin nockvm **fully isolated** from the nockchain monorepo
workspace — no reliance on nockchain's root `Cargo.toml` for dependency
inheritance.

Mechanism:
- `patches/nockchain/0001-standalone-nockvm-ws.patch` installs
  `third_party/nockchain/crates/nockvm/Cargo.toml`, a standalone Cargo workspace
  root over the nockvm subtree (`rust/{assert_no_alloc,ibig,nockvm,nockvm_crypto,
  nockvm_macros,murmur3,murmur3/murmur3-sys}`). It re-declares
  `[workspace.package]`, `[workspace.lints]`, and all **35** inherited
  `[workspace.dependencies]`, every version/feature lifted verbatim from the
  pinned root. nockvm then roots at `crates/nockvm/`, not the monorepo.
- **Verified:** `cargo metadata --no-deps` from `crates/nockvm` resolves 7
  members, `workspace_root = …/crates/nockvm`, **zero** monorepo crates in the
  graph, no nested-workspace error.
- The manifest is **generated**, not hand-written:
  `patches/nockchain/regen-standalone-ws.py` reads the pinned root + member
  manifests and emits it deterministically. This is the **reconcile-on-bump
  tool** — after a nockchain pin bump, re-run it, regenerate the patch, and log
  any dep drift in `CHANGES.md`. (nockvm's own `nested-workspace.toml` is stale —
  wrong `thiserror`, missing deps — so it is *not* usable directly; the generator
  supersedes it.)
- `serf/Cargo.toml` is its own workspace; it path-deps nockvm into the (patched)
  submodule. `serf/Cargo.lock` + the submodule SHA are the reproducible pin.
- **Toolchain:** `rust-toolchain.toml` pins `nightly-2026-04-03` to match the
  nockchain pin, so nockvm builds reproducibly.

Build order for the serf: `patches/apply.sh nockchain` **must run before**
`cargo build` in `serf/`, or the standalone manifest isn't present and Cargo
falls back to the monorepo root.

## King (C via Zig) — TODO

Decisions (owner): reuse vere's own `build.zig`; `--serf-bin` flag; pin Zig.

- **Zig 0.15.2**, matching vere @ develop's CI (`mlugg/setup-zig@v2`,
  `version: 0.15.2`; `ext/nasm`'s `0.14.0` is only a floor). Pin it in the
  orchestrator and CI; record alongside the vere pin.
- Reuse `third_party/vere/build.zig` (it already assembles the king from
  `pkg/{c3,ent,ur,noun,vere}` + vendored `ext/{curl,gmp,h2o,libuv,...}`). The
  Eris king build **omits `pkg/vere/mars.c`** (vere's own serf) and produces the
  king binary `eris`.
- **`patches/vere/0001-serf-bin.patch`** (planned): add a `--serf-bin <path>`
  flag and redirect the serf-spawn path (`pkg/vere/lord.c`, argv build at
  `~lord.c:1027-1046` / `:1292-1308`) to exec the external Rust serf instead of
  re-exec'ing `argv[0] work|boot`. `--serf-bin` is a natural fit for that spawn
  site and keeps dev ergonomic (point at a freshly built `eris-serf`).
- `king/` holds any Eris-owned C glue + the build entry; vere source edits stay
  in `patches/vere/`.

## Orchestrator — TODO

A top-level `justfile` (or `Makefile`):
- `apply` → `patches/apply.sh` (both submodules)
- `serf` → `apply` + `cargo build` in `serf/`
- `king` → `apply` + `zig build` in the king
- `build` → `serf` + `king`
- `dev` → boot a fake ship with the king pointed at the built serf via
  `--serf-bin` (model on vere's `boot-fake-ship.sh`).

## Status

| Piece                    | State |
|--------------------------|-------|
| serf pin / isolation     | **done & verified** (`0001-standalone-nockvm-ws.patch`, regen tool, toolchain pin) |
| serf crate skeleton      | stub (`serf/`) — no shim logic yet |
| king build (zig, no-mars)| not started |
| `--serf-bin` patch       | not started |
| orchestrator / `dev`     | not started |
| CI (pins + Zig 0.15.2)   | not started |
