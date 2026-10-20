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

## 2026-10-20 — Serf jets: the `@r` float doors (`+rs`/`+rd`/`+rh`/`+rq`)
- **Scope:** serf
- **Upstream touched:** (none — Eris-owned, `serf/src/jets/flot.rs`)
- **Mechanism:** new module `flot` (the `hoon-float-jets` reference, verbatim),
  registered via `flot::HOON_FLOAT_HOT` in `jets::eris_hot_state()`. New dep
  `sdfloat` (pure-Rust IEEE-754 SoftFloat), rev-pinned to the numerics reference.
- **What:** Jet the four IEEE-754 float doors — `+rh` (f16), `+rs` (f32), `+rd`
  (f64), `+rq` (f128) — ten arms each (`add sub mul div sqt lth lte equ gte
  gth`), 40 jets total, at `[k138 one two tri <door> <arm>]`. The rounding mode
  is read from the door's sample at axis 30 (`%n %u %d %z %a`); operands are the
  low `width` bits of the atom, results correctly-rounded via `sdfloat`. `+fma`
  is hinted in Hoon but has no SoftFloat FMA, so it stays Nock. *Source:* a
  verbatim import of the numerics repo's `hoon-float-jets` crate (branch
  `sigilante/nockapp-float-jets`), whose results are bit-identical to Vere's
  Berkeley-SoftFloat C jets (the numerics project's own Hoon-vector tests prove
  it). 4 new tests (known f16/f32/f64/f128 values) pass; full boot exits 0.
- **Why:** The `@r` gap bucket — the cheapest remaining because the port already
  exists and is validated upstream. Userspace math (Arvo core uses no floats, so
  no boot impact). Per the reference, one known divergence: `+fl` overflow-to-inf
  vs. SoftFloat saturation in some modes (urbit/urbit#7426) — pre-existing, not
  ours.
- **Re-apply risk:** none (additive). On a bump re-check `sdfloat`'s
  `Format`/`SoftFloat`/`Round` API and that the `@r` doors keep their `~/` hints
  at the k138 path. The `sdfloat` git rev is pinned; if it moves, re-verify
  bit-exactness. Same kelvin caveats as the `+by` entry.

## 2026-10-20 — Serf jets: gap buckets 1–3 (`+po`, `+ob`, `scot`/`slaw`, atom helpers)
- **Scope:** serf
- **Upstream touched:** (none — Eris-owned, `serf/src/jets/{bits,list,ob,po,text}.rs`)
- **Mechanism:** new code (new modules `ob`, `po`, `text`; additions to `bits`,
  `list`), registered via `OB_JETS`/`PO_JETS`/`TEXT_JETS` + extensions to
  `LIST_JETS` in `jets::eris_hot_state()`. Two new deps (`ibig`, `murmur3`),
  path-pinned to nockvm's own forks so `UBig`/`Atom` interop and the hash
  matches `+mug`.
- **What:** Closed 13 of the ~60 remaining gap arms vs. Vere, in three buckets:
  - **list/atom helpers** (`bits.rs`+`list.rs`): `aor` (a-order comparator),
    `mat`/`rub` (jam length encode/decode, via ibig bignum — tested as exact
    inverses), `muk` (murmur3, == `+mug`'s hash, with Vere's key zero-padding),
    `mate` (unit merge).
  - **`+ob`** (`ob.rs`): `fein`/`fynd`, the `@p` Feistel scramble/unscramble
    (ported from Vere `e/{fein,fynd}_ob.c`; murmur3 rounds; tested as inverses).
  - **`+po`** (`po.rs`): `ind`/`ins`/`tod`/`tos`, the `@p` syllable tables
    (both 256-entry prefix/suffix tables copied verbatim from Vere `c/po.c` and
    length-checked).
  - **`scot`/`slaw`** (`text.rs`): render/parse `dime`s. Like Vere, jet the
    common auras and `Punt` the rest (always correct): `scot` does
    `%tas %ud %ux %uv %uw`; `slaw` does `%ud %tas %p %da` — Vere's full jetted
    set. `%p` composes the new `+po`/`+ob` jets to parse ship names; `%da` ports
    the `@da` calendar (`+year`/`+yawn`/`+yule`) directly, since nockvm has no
    `cook` to slam Vere's Hoon helper (anchored in tests on the epoch
    `~292277024401-.1.1` = `@da 0`). Round-trip / known-value tested.
  - Registration paths follow the chapter nesting (`one two tri qua ...`): `ob`
    under `tri`, `po`/`scot`/`slaw` under `qua`, the rest direct `two` arms.
  20 new tests (70 jet tests total) pass; full boot-pill reaches `use[0]`.
- **Why:** These were the cheapest, highest-confidence gap closures (pure/table
  logic + `Punt`-safe partial coverage). Per the profiling they are coverage,
  not a boot-speed lever; load-bearing for `@p`/number/path formatting in
  userspace. Deferred (per the gap analysis): `@r` floats, the crypto bucket,
  the `ut` type helpers, and `scot`/`slaw`'s `%da`/`%p` (run unjetted, correct).
- **Re-apply risk:** none (additive). On a nockchain bump re-check the ibig
  (`UBig::{from_le_bytes,to_le_bytes,pow_stack}`, `Atom::{as_ubig,from_ubig}`)
  and `murmur3::murmur3_32_of_slice` APIs, `bits::util::{met,bex,lsh,con}`, and
  that the `+po` tables still match Vere's `c/po.c`. Same kelvin/path caveats as
  the `+by` entry; `muk`'s seed is `(end 5 syd)` = low 32 bits.

## 2026-10-20 — Serf jets: fix `+by`/`+in` `wyt` subject axis (trap, not gate)
- **Scope:** serf
- **Upstream touched:** (none — Eris-owned, `serf/src/jets/{map,set}.rs`)
- **Mechanism:** bugfix to the jets added 2026-10-20 (`+by`/`+in` doors)
- **What:** `jet_by_wyt`/`jet_in_wyt` read the map/set `a` via `door_map`/
  `door_set` (axis 7->6 = `u3x_con_sam`, axis 30) like every other arm. But
  `+wyt` is a bare trap (`=<  $  ~%  %wyt  +  ~  |.`), not a sample-bearing gate:
  Vere registers it at **parent-axis 3** (every other `by`/`in` arm is axis 7),
  and reads `a` at `u3x_con_2` (**axis 14**). Our jets walked axis 30 — a wrong
  subtree — producing a bad count. Fixed both to read axis 14. The unit tests
  passed before because they drove the counting helpers directly and the
  door-shaped `assert_jet_door` subject; added end-to-end tests that build the
  real trap-shaped subject (`a` at axis 14) and call the jet entrypoint (these
  fail against the old axis).
- **Why:** Latent since the `+by`/`+in` commit (9767d4e). Surfaced by the
  full boot-pill validation of the list/bit jets below: the `%boot` poke calls
  `~(wyt by ...)`, so the bad count crashed boot with `Deterministic(Exit)`
  right after `wack`. (Isolated by bisection: stock-only booted; the crash
  tracked to `jet_by_wyt`; Vere's `tree.c` axis column and `d/in_wyt.c`'s
  `u3x_con_2` confirmed the correct axis.)
- **Re-apply risk:** none. If other trap arms are added later, check Vere's
  `tree.c` parent-axis column — axis 3 means `u3x_con_2` (14), axis 7 means
  `u3x_con_sam` (30).

## 2026-10-20 — Serf jets: chapter-`two` `+list` + atom/bit helpers
- **Scope:** serf
- **Upstream touched:** (none — Eris-owned, `serf/src/jets/{list,bits}.rs`)
- **Mechanism:** new code, registered via `LIST_JETS` in `jets::eris_hot_state()`
- **What:** Added 12 unjetted chapter-`two` standard-library arms a stock k138
  pill leans on but nockvm's dashboard omits. `list.rs`: `welp slag need reel
  skim skip skid murn lien sort` (the gate-slamming ones apply the sample gate
  via `jets::util::slam`, exactly as Vere's `u3j_gate_slam`; `sort` is the
  first-pivot quicksort from Vere's `b/sort.c`). `bits.rs`: `swp` (= `(rep a
  (flop (rip a b)))`, reusing nockvm's public `bits::util::rip`/`bits::rep` so
  the bit-slicing is byte-identical to the stock jets) and `pow` (via ibig's
  `UBig::pow_stack`, loom-allocated). All plain gates at `[k138 one two <arm>]`,
  axis 1 — siblings of nockvm's own `flop`/`turn`/`roll`. 10 new tests (37 jet
  tests total) pass. The boot-pill validation of these jets is what surfaced the
  separate latent `wyt` axis bug (entry above); with that fixed the full boot
  reaches the `use[0]` compile with all 47 eris jets (by21+in14+list12)
  registered. The list/bit jets caused no regression themselves (stock-only and
  BY+IN-only boots crashed identically until `wyt` was fixed). **Deferred within
  this bucket:** none — the remaining gap-bucket items (`aor mat muk sqt`, plus
  the crypto/`@r`/`ut`/`ob` buckets) are tracked separately.
- **Why:** These are the cheapest, highest-confidence gap closures vs. Vere
  (pure treap/list walks + `slam`; no new deps — `pow`'s `UBig` type is used by
  inference, never named, so `serf/Cargo.toml` is unchanged). Per the profiling,
  they do not speed boot; they are coverage/correctness for map/set/list-heavy
  steady-state Hoon. Models the Vere-port method for the rest of the long tail.
  (@r floats, the next bucket, have a Rust-jet reference in `../numerics`
  branch `sigilante/nockapp-float-jets` — SoftFloat `hoon-float-jets`.)
- **Re-apply risk:** none (additive, Eris-owned). On a nockchain pin bump,
  re-check that `jets::bits::{rep, util::rip}`, `jets::util::slam`,
  `Noun::as_atom`, and `Atom::{as_ubig, from_ubig}`/`UBig::pow_stack` stay
  public, and the same kelvin/path caveats as the `+by` entry.

## 2026-10-20 — Serf jets: complete the `+by`/`+in` tails (Vere-k138 parity)
- **Scope:** serf
- **Upstream touched:** (none — Eris-owned, `serf/src/jets/{map,set}.rs`)
- **Mechanism:** new code, registered in `jets::eris_hot_state()`
- **What:** Filled out both doors to Vere's full k138 jet set. Added `+by`:
  `tap key wyt apt bif run urn jab all any`; `+in`: `tap wyt apt bif run all any`.
  Pure arms (`tap`/`key`/`wyt`/`apt`/`bif`) are straight treap walks; the
  gate-slamming arms (`run`/`urn`/`jab`/`all`/`any`) apply a fixed gate via
  `jets::util::slam`, ported from Vere's `_by_*`/`_in_*`. `+by key` builds its
  key set with the shared `set::set_put`. 27 jet tests pass — pure arms vs.
  `BTreeMap`/`BTreeSet` oracles + treap invariants; gate arms slam hand-built
  gates (`[4 0 6]` increment, `[0 12]` key, `[5 [0 6] [1 0]]` is-zero) and check
  against models. Full boot still exits 0 (no regression). **Deferred:**
  `+rep`/`+rib` (gate folds whose accumulator lives at a gate-internal axis;
  Vere ships no dedicated jet for them) — they run unjetted, correctly.
- **Why:** Complete the doors for Vere parity and map/set-heavy steady-state
  work. Per the profiling (below), this does not speed up boot; it is coverage
  and correctness, and rounds out the upstreamable contribution.
- **Re-apply risk:** none (additive). The gate-slamming arms depend on
  `jets::util::slam` staying public; same kelvin/path caveats as the `+by` entry.

## 2026-10-20 — Serf jets: the `+in` set door + boot profiled (jets are NOT the boot bottleneck)
- **Scope:** serf
- **Upstream touched:** (none — Eris-owned, `serf/src/jets/set.rs`)
- **Mechanism:** new code (`serf/src/jets/set.rs`), registered via `IN_JETS` in
  `jets::eris_hot_state()`
- **What:** Added the standard `+in` set door — 7 arms (`has put del gas uni int
  dif`) — same construction as `map.rs` (stock `[n l r]` treap, stock
  `+gor`/`+mor`, `unifying_equality` dedup), cribbed from `zoon_jets::h_in_*`.
  15 jet tests pass (map + set differential vs. `BTreeMap`/`BTreeSet` oracles +
  treap invariants). Verified binds+fires on the k138 pill (`set_uni` is the
  most-used of our jets during the base compile).
- **Then profiled the base-desk compile** (`sample` during `use[0]`; the opcode
  census from the `+by` entry framed it). **Finding — decision-changing:** the
  compile bottleneck is **nockvm's memory model, not jet coverage.** The
  authoritative self-time leaders are `mem::noun_preserve` (cross-frame noun
  copying, #1), `noun::TaggedPtr::resolve_const` (per-access pointer/NounSpace
  resolution, #2), raw `interpret`, and allocation churn (`raw_alloc`, frame
  push/pop, malloc/memset). The compile is *structured* by the Hoon parser
  combinators (`pfix/stew/plug/cook/pose/glue/stag/here`), but those are **already
  jetted** and show high inclusive / low self time — the real cost is the
  preserve+resolve+alloc+interpret machinery their slamming drives. The `+by`/`+in`
  doors fire but are ~noise. **Porting more stdlib jets will not speed up boot.**
- **Why:** `+in` is the natural companion to `+by` and load-bearing for sets in
  steady-state (the compiler's own symbol tables, `+nest`). The profiling was the
  stated next step and redirects the whole effort: the lever is nockvm runtime
  internals (reduce `noun_preserve` churn, optimize tagged-pointer resolution and
  `slam_with_space`), or the 64-bit/pointer-compression work upstream — not the
  jet long tail. See `docs/architecture.md`.
- **Re-apply risk:** none (additive, Eris-owned). Same kelvin/public-API caveats as
  the `+by` entry below.

## 2026-10-20 — Serf jets: the `+by` map door (first Eris-owned jet)
- **Scope:** serf
- **Upstream touched:** (none — Eris-owned, `serf/src/jets/`)
- **Mechanism:** new code (`serf/src/jets/{mod.rs,map.rs}`), plus one dep
  (`either = "1.9.0"`, matching nockchain's pin for an identical `HotEntry`)
- **What:** Added the standard Hoon `+by` map door as Eris-owned jets — 11 arms
  (`get got gut has put del mar gas uni int dif`) — rebuilt against the stock
  node shape `[[key value] l r]` and the stock `mug`-based `+gor`/`+mor`
  comparators. Registered from the serf via `jets::eris_hot_state()` (stock
  `URBIT_HOT_STATE` ++ `BY_JETS`); **nockvm's dashboard is not patched**. All
  three context builders (`boot`, `work`, `boot-pill`) now seed from it. Verified:
  9 differential tests pass (jet vs. a `BTreeMap` oracle + canonical-treap
  invariants), the jets bind and fire on the stock k138 pill, and a king-driven
  boot is unaffected (exit 0).
- **Why:** nockvm leaves the stdlib `+by` unjetted because Nockchain uses its own
  digest-ordered `z-by`/`h-by` instead (see PROVENANCE). A stock Urbit pill needs
  the ordinary door. This is also the template for `+in` (sets) and the rest of
  the tail. **Measured caveat:** `+by` is *not* the base-compile bottleneck — an
  opcode census shows that compile is ~13 billion interpreted Nock opcodes
  (op9 call alone ≈ 1.1B) while maps are only ~0.3M calls, so this gives ~no boot
  speedup. The gap vs. Vere (~12s, ~2179 jets) is broad interpreted execution of
  the compiler and its hot stdlib helpers, not any single door. Kept because it
  is correct, is load-bearing for map-heavy *steady-state* ship operation, and
  seeds the porting method. Next lever needs arm-level profiling, not a guess.
- **Re-apply risk:** none (additive, Eris-owned). On a nockchain pin bump, re-check
  that `nockvm::jets::{sort::util::{gor,mor}, util::slot, util::test::*}` and
  `unifying_equality` remain public, and that `HotEntry`/`K_138` are unchanged. If
  the pill's kelvin moves off 138, the registration paths (`K_138`) must move with
  it or the door silently stops binding (no crash, just slow).

## 2026-10-19 — King builds & runs: `eris` (one vere patch + macOS-26 workaround)
- **Scope:** king / build
- **Upstream touched:** vere@`81404b1a` — `build.zig` (git-rev read)
- **Mechanism:** patch (`patches/vere/0001-build-git-rev-optional.patch`);
  `justfile`; build flags
- **What:** Built the king as `eris` (`zig build -Drelease -Dbinary-name=eris`,
  Zig 0.15.2) — Mach-O arm64, `urbit 5.0 live`, exposes `--serf-bin`. The serf
  **spawn** needs no patch (vere's stock `--serf-bin` → `wrk_c` → `arg[0]`/`file`
  at both `lord.c` sites; `mars.c` stays compiled but unused). The **one** patch
  is unrelated: `build.zig` read `.git/logs/HEAD` with `try` (fatal), assuming
  `.git` is a dir — but a submodule gitlink (and a tarball) have no such dir, so
  it died `error.NotDir`; made best-effort. Host workaround for macOS-26/zig-0.15
  (SDK 27 unreadable by zig): `tools/xcrun-shim/` + project-local
  `--global-cache-dir`, baked into `just king`, self-disabling elsewhere. Added
  the `justfile` and the king→serf invocation contract in `docs/seam-contract.md`.
- **Why:** Reuse vere's battle-tested build; keep divergence to one upstreamable
  patch. The compatibility burden lands on the serf (must accept vere's spawn CLI).
- **Re-apply risk:** `0001` is a tiny `build.zig` hunk — re-check it still applies
  on a vere bump (the version-stamping code may move). The macOS workaround is
  host-specific; drop it once vere accepts a newer zig. Re-verify `--serf-bin`,
  `-Dbinary-name`, and the spawn argv on each bump.

## 2026-10-19 — Serf build: isolate nockvm from the monorepo
- **Scope:** serf / build
- **Upstream touched:** nockchain@`73877be2` — adds (does not edit)
  `crates/nockvm/Cargo.toml`
- **Mechanism:** patch (`patches/nockchain/0001-standalone-nockvm-ws.patch`),
  generated by `patches/nockchain/regen-standalone-ws.py`
- **What:** Installs a standalone Cargo workspace root over the nockvm subtree
  (7 members, 35 inherited deps + `[workspace.package]`/`[workspace.lints]`
  lifted verbatim from the pinned root). `serf/Cargo.toml` path-deps nockvm into
  it; `/rust-toolchain.toml` pins `nightly-2026-04-03`. Verified: `cargo
  metadata` roots at `crates/nockvm`, 7 members, zero monorepo leakage.
- **Why:** Total isolation from the nockchain monorepo workspace — the serf must
  not depend on nockchain's 45-crate root manifest for dependency inheritance.
- **Re-apply risk:** nockvm's inherited dep set drifts between pins. On bump,
  **re-run the generator** (don't hand-edit the manifest), regenerate the patch,
  and record any added/removed/retversioned dep here. The generator fails loudly
  if an inherited dep is absent from the new root.

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

