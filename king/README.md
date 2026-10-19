# king/ — Eris-owned C over the urth (vere)

The king is Vere's, taken **stock** from `third_party/vere` — and it turns out
it needs no patch at all. Vere already exposes the worker-binary path as the
`--serf-bin` flag (`main.c` opt 11 → `u3_Host.wrk_c`, used as both `arg[0]` and
the spawn `file` at the run/boot spawn sites in `lord.c`). Eris just points it at
`eris-serf`. `pkg/vere/mars.c` (vere's own serf) stays compiled but unused.

Build: `just king` → `zig build -Drelease -Dbinary-name=eris` in the submodule
(no rename patch either — binary name is a build option). Output:
`third_party/vere/zig-out/<triple>/eris`.

So this directory is **reserved** — no Eris-owned C is needed on the king side.
Any future Eris C glue would live here. The one vere source edit lives in
`patches/vere/0001-build-git-rev-optional.patch` (a build-system fix, unrelated
to the serf redirect — it lets vere build from a submodule gitlink / tarball).

On macOS 26 the king build needs the `tools/xcrun-shim/` + project-local zig
cache workaround baked into `just king`; see `docs/build.md`.
