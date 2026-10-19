# king/ — Eris-owned C over the urth (vere)

The king is Vere's, taken pristine from `third_party/vere`. This directory holds
**only Eris's own C** that sits on top of it:

- the build entry that compiles vere's king objects (`pkg/{c3,ent,ur,noun}` and
  the king half of `pkg/vere`) while **omitting `pkg/vere/mars.c`** (vere's own
  serf, which the Rust serf replaces);
- any small glue the king needs to launch and supervise the Rust serf.

Edits to vere source files themselves do **not** live here — they go in
`patches/vere/` (see `patches/README.md`). The single expected upstream edit is
the serf-spawn redirect in `pkg/vere/lord.c`.

Empty for now; populated when the build system is settled.
