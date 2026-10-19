//! Eris mars (serf): the deterministic Nock computer.
//!
//! Runs as a child of the vere king over the newt pipe (see
//! `docs/seam-contract.md`): read 5-byte-framed jammed writs on fd0, emit
//! `%ripe` first then the plea set on fd1, `%slog`/`%flog` on fd2. Internally it
//! drives nockvm (interpreter + jets + PMA) with the Serf lifecycle
//! (poke / peek / snapshot / event-log / replay), modelled on nockapp's
//! `crates/nockapp/src/kernel/form.rs` but with the newt transport in place of
//! its tokio mpsc channel.
//!
//! Not yet implemented — this is the build skeleton.

fn main() {
    eprintln!("eris-serf: not yet implemented (see docs/seam-contract.md)");
    std::process::exit(1);
}
