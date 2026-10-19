# serf/ — Eris-owned Rust mars (the serf)

The deterministic Nock computer. An Eris-original Rust crate that:

- depends on **nockvm** (interpreter + jets + PMA) from
  `third_party/nockchain/crates/nockvm/rust/*`;
- carries the **Serf lifecycle** (poke / peek / snapshot / event-log / replay),
  modelled on nockapp's `crates/nockapp/src/kernel/form.rs` but with its tokio
  mpsc transport replaced by the newt pipe;
- implements the **newt shim**: 5-byte-framed jammed-noun IO on fd0/fd1, emitting
  `%ripe` first and answering the writ/plea set in `docs/seam-contract.md`.

We write this crate ourselves against nockvm's public API rather than vendoring
nockapp, so nockvm's source stays pristine in the submodule and the king/serf
boundary logic is Eris's.

## How it consumes nockvm (settled)

`serf/` is its own Cargo workspace. It path-deps nockvm into the pinned
submodule, which is made a **standalone workspace** (fully isolated from the
nockchain monorepo) by `patches/nockchain/0001-standalone-nockvm-ws.patch`. So:

- **Run `patches/apply.sh nockchain` before `cargo build`** — otherwise the
  standalone manifest isn't present and Cargo falls back to the monorepo root.
- The toolchain is pinned in `/rust-toolchain.toml` (`nightly-2026-04-03`, to
  match the nockchain pin).
- `serf/Cargo.lock` + the submodule SHA are the reproducible pin.

See `docs/build.md` for the full rationale and the reconcile-on-bump tool.

Skeleton only for now (`src/main.rs` stubs the entry point); populated with the
newt shim + Serf lifecycle next.
