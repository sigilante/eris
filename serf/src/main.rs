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
//! Not yet implemented — this is the build skeleton. For now it reports the argv
//! the king hands it (the invocation contract in `docs/seam-contract.md`) on
//! stderr (which the king forwards), then exits so the spawn/pipe wiring can be
//! smoke-tested via `--serf-bin`.

fn main() {
    let args: Vec<String> = std::env::args().collect();
    eprintln!("eris-serf: spawned ({} args)", args.len());
    for (i, a) in args.iter().enumerate() {
        eprintln!("  argv[{i}] = {a}");
    }
    eprintln!("eris-serf: newt seam not yet implemented; exiting 1.");
    std::process::exit(1);
}
