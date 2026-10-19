# patches/

Re-appliable divergences from the pinned upstreams in `third_party/`. **This is
the only sanctioned way to modify vendored code.** If a change can instead live
as new Eris-owned code under `king/` or `serf/`, prefer that — a patch is for the
unavoidable case of editing an upstream file in place (e.g. the king's serf-spawn
path in `pkg/vere/lord.c`).

## Rules

1. One patch = one logical change, with a descriptive name:
   `patches/<submodule>/NNNN-short-title.patch` (e.g.
   `patches/vere/0001-spawn-eris-serf.patch`). Numeric prefixes set apply order.
2. Every patch has a matching entry in `CHANGES.md` (what / why / re-apply risk).
3. Patches apply cleanly against the **recorded pin** in `PROVENANCE.md`. When a
   pin is bumped, re-apply, fix rejects, regenerate the patch, and log the churn.
4. Never commit edits directly into a `third_party/` submodule. The submodule
   working tree is scratch; the patch files are the source of truth.

## Generating a patch

```sh
cd third_party/vere
# make your edit, then:
git diff > ../../patches/vere/0001-spawn-eris-serf.patch
git checkout .        # discard the edit from the submodule working tree
```

(Use `git diff --relative` / `git format-patch` if you prefer; keep paths
relative to the submodule root so `apply.sh` can apply with `-p1` from inside it.
For a new file, `git add -N <path>` first so the diff is a proper add.)

### Generated patches

Some patches are produced by a tool, not hand-written — their header says so.
`patches/nockchain/0001-standalone-nockvm-ws.patch` is emitted by
`patches/nockchain/regen-standalone-ws.py` (the reconcile-on-bump tool for the
standalone nockvm workspace). Don't hand-edit it; re-run the generator and
regenerate the patch. See `docs/build.md` and `CHANGES.md`.

## Applying

`./patches/apply.sh vere` (or `nockchain`, or no arg for all) applies the series
in numeric order inside the corresponding submodule. Run it after any pin bump,
and the build should run it before compiling. Applying is **not** committed into
the submodule — it leaves the working tree patched but the gitlink unchanged.
