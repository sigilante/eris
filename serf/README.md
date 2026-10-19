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
nockapp, so nockvm stays unpatched and the king/serf boundary logic is Eris's.

How this crate consumes nockvm (path vs. workspace vs. patched) is deferred to
the build-system decision — nockvm leans on workspace inheritance
(`version.workspace`, `ibig.workspace`, a non-member `assert_no_alloc` path dep),
which shapes the options.

Empty for now; populated when the build system is settled.
