# CHANGES — structural divergence log

Every deliberate departure from upstream, newest first. This is the companion to
`PROVENANCE.md`: provenance says *where code comes from*; this says *what we did
to it and why*. One entry per structural change. Keep it honest — a reader
updating a pin uses this to know what will break.

Entry format:

```
## YYYY-MM-DD — <short title>
- **Scope:** king | serf | build | both | repo
- **Upstream touched:** vere@<sha> path/to/file  |  (none — Eris-owned)
- **Mechanism:** patch (patches/...) | overlay (king/.. , serf/..) | new code
- **What:** one or two sentences.
- **Why:** the reason this divergence is necessary.
- **Re-apply risk:** what to watch for when the pin is bumped.
```

---

## 2026-10-19 — Repository scaffold
- **Scope:** repo
- **Upstream touched:** none
- **Mechanism:** new code
- **What:** Initial structure — `third_party/` submodules (vere @ `81404b1a`,
  nockchain @ `73877be2`), `patches/{vere,nockchain}/`, `king/`, `serf/`,
  `docs/`. Froze the king↔serf newt contract in `docs/seam-contract.md`.
- **Why:** Establish the vendoring discipline before any code lands: pristine
  pinned upstreams, all divergence as patches or Eris-owned overlays.
- **Re-apply risk:** none yet (no patches).

<!-- Planned first divergences (not yet implemented), for orientation:

## (planned) — King spawns the Rust serf
- **Scope:** king
- **Upstream touched:** vere pkg/vere/lord.c (serf spawn path, ~lord.c:1027-1046
  / :1292-1308 where argv "work"/"boot" is built for re-exec of argv[0])
- **Mechanism:** patch (patches/vere/0001-spawn-eris-serf.patch)
- **What:** Replace the self-re-exec with a spawn of the Eris serf binary.
- **Why:** The mars process is now the Rust serf, not vere's mars.c.
- **Re-apply risk:** lord.c spawn code is stable but the argv/flags
  (`--snap-dir`, `--loom`, `--runtime-config`, `--lmdb-map-size`) evolve; verify
  flag set on each bump.

## (planned) — Drop mars.c from the king build
- **Scope:** build
- **Mechanism:** build config (Eris build omits pkg/vere/mars.c)
- **Why:** vere's serf is replaced; keep its king objects, drop the serf.
-->
