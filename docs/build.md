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

## King (C via Zig) — DONE: stock vere, NO patch needed

The headline finding: **vere already has the worker-binary indirection as a
first-class flag, so the king needs zero C patches.**

- `--serf-bin <path>` is a stock vere option (`pkg/vere/main.c:382` option 11 →
  `u3_Host.wrk_c = strdup(optarg)`, defaulting to the self-binary at `:3297`).
  `wrk_c` is used verbatim as `arg[0]` **and** `ops_u.file` at *both* spawn sites
  — run (`lord.c:993,1026,1083`) and boot (`lord.c:1260,1291,1335`). Nothing
  clobbers it; the `dock`/`pace` self-copy is a separate explicit subcommand.
  So pointing `--serf-bin` at `eris-serf` redirects the spawn end-to-end with no
  source change. `pkg/vere/mars.c` (vere's own serf) simply stays compiled and
  unused.
- Build as `eris` via the stock build option: **`zig build -Drelease
  -Dbinary-name=eris`** (binary name is `-Dbinary-name`, default `urbit`,
  `build.zig:231`). No patch to rename, either.
- **Zig 0.15.2**, matching vere @ develop's CI (`mlugg/setup-zig@v2`,
  `version: 0.15.2`; `ext/nasm`'s `0.14.0` is only a floor).
- Output: `third_party/vere/zig-out/<zig-triple>/eris`
  (`build.zig:608`). Native darwin builds use the system SDK; the `macos_sdk`
  lazy dep is cross-only, and `ext/{curl,gmp,h2o,libuv,...}` are vendored `.path`
  deps — so a native build needs no network (but the first one is slow).
- **`patches/vere/` stays empty.** `king/` is currently just a README; any
  Eris-owned C glue would go there, and any unavoidable future vere edit in
  `patches/vere/`.

### Consequence: the serf must match vere's spawn CLI

Because the king is stock, the compatibility burden moves entirely to the serf.
The king invokes the serf binary as `eris-serf work|boot <flags>`, where the
flags are vere-serf's (`--snap-dir`, `--runtime-config`, `--temporary-cache-size`,
`--loom`, `--toss`, `--persistent-cache-size`, `--snap-time`, `--lmdb-map-size`,
and optional `--play-until` / `--ephemeral-file` / `--win-intr-handle`). The
Eris serf must accept this exact CLI (interpreting or harmlessly ignoring each
flag) before it ever speaks newt. This invocation contract is recorded in
`docs/seam-contract.md`.

## Orchestrator — DONE (`justfile`)

- `just apply` → `patches/apply.sh` (both submodules, idempotent)
- `just serf` → `apply` + `cargo build --release` in `serf/`
- `just king` → `zig build -Drelease -Dbinary-name=eris` in the vere submodule
- `just build` → both
- `just serf-bin` / `just king-bin` → resolved binary paths
- `just dev PILL=/path/to/pill [ship=NAME]` → boot a fake ship, king driving the
  serf via `--serf-bin` (modelled on vere's `boot-fake-ship.sh`; won't boot until
  the serf implements the newt seam)
- `just clean-pier` → remove the dev pier

## Status

| Piece                      | State |
|----------------------------|-------|
| serf pin / isolation       | **done & verified** (`0001-standalone-nockvm-ws.patch`, regen tool, toolchain pin) |
| king build (stock, `eris`) | **done** — `just king` (`-Dbinary-name=eris`, no patch); first real build not yet run |
| `--serf-bin` wiring        | **done** — stock vere flag, no patch |
| orchestrator / `dev`       | **done** (`justfile`) |
| serf crate shim logic      | stub (`serf/`) — the actual work: serf CLI + newt + Serf/nockvm |
| CI (pins + Zig 0.15.2)     | not started |

The next real work is entirely serf-side: make `eris-serf` accept vere's spawn
CLI and implement the newt seam over nockvm's `Serf`.
