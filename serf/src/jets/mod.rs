//! Eris-owned jets, layered onto nockvm's stock dashboard.
//!
//! nockvm's `URBIT_HOT_STATE` jets the critical path (arithmetic, bits,
//! list/set/sort, some crypto, and the Hoon compiler) but leaves the long tail
//! of the standard library unjetted, because Nockchain's own kernels don't use
//! it. Eris runs a stock k138 Urbit pill, which leans hard on those arms, so we
//! add our own jets here and register them WITHOUT patching nockvm: the serf
//! builds its contexts from [`eris_hot_state`], which is the stock dashboard
//! followed by these entries. nockvm's submodule stays pristine.
//!
//! The registration paths mirror the stock dashboard's own (commented-out)
//! `+in` door entries — e.g. `[K_138, one, two, in, put]` — placing `+by` as a
//! sibling of `+in` under the second core, at arm axis 1.
//!
//! Currently: the `+by` map door (see [`map`]) and the `+in` set door (see
//! [`set`]).

use either::Either::Left;
use nockvm::jets::hot::{HotEntry, K_138, URBIT_HOT_STATE};

pub mod map;
pub mod set;

/// The `+by` map-door arms. Paths are `[k138 one two by <arm>]`, each jetted at
/// the arm's own formula (axis 1), matching the stock `+in` sibling entries.
pub static BY_JETS: &[HotEntry] = &[
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"get")],
        1,
        map::jet_by_get,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"got")],
        1,
        map::jet_by_got,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"gut")],
        1,
        map::jet_by_gut,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"has")],
        1,
        map::jet_by_has,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"put")],
        1,
        map::jet_by_put,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"del")],
        1,
        map::jet_by_del,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"mar")],
        1,
        map::jet_by_mar,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"gas")],
        1,
        map::jet_by_gas,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"uni")],
        1,
        map::jet_by_uni,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"int")],
        1,
        map::jet_by_int,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"dif")],
        1,
        map::jet_by_dif,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"tap")],
        1,
        map::jet_by_tap,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"key")],
        1,
        map::jet_by_key,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"wyt")],
        1,
        map::jet_by_wyt,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"apt")],
        1,
        map::jet_by_apt,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"bif")],
        1,
        map::jet_by_bif,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"run")],
        1,
        map::jet_by_run,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"urn")],
        1,
        map::jet_by_urn,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"jab")],
        1,
        map::jet_by_jab,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"all")],
        1,
        map::jet_by_all,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"by"), Left(b"any")],
        1,
        map::jet_by_any,
    ),
];

/// The `+in` set-door arms. Paths are `[k138 one two in <arm>]`, each jetted at
/// the arm's own formula (axis 1), matching the stock dashboard's (commented-out)
/// `+in` entries.
pub static IN_JETS: &[HotEntry] = &[
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"has")],
        1,
        set::jet_in_has,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"put")],
        1,
        set::jet_in_put,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"del")],
        1,
        set::jet_in_del,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"gas")],
        1,
        set::jet_in_gas,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"uni")],
        1,
        set::jet_in_uni,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"int")],
        1,
        set::jet_in_int,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"dif")],
        1,
        set::jet_in_dif,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"tap")],
        1,
        set::jet_in_tap,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"wyt")],
        1,
        set::jet_in_wyt,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"apt")],
        1,
        set::jet_in_apt,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"bif")],
        1,
        set::jet_in_bif,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"run")],
        1,
        set::jet_in_run,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"all")],
        1,
        set::jet_in_all,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"in"), Left(b"any")],
        1,
        set::jet_in_any,
    ),
];

/// The stock nockvm dashboard followed by Eris's own jets. Every context the
/// serf builds (`boot`, `work`, and the `boot-pill` dev harness) is seeded from
/// this, so the extra arms are available both while booting and after restore.
pub fn eris_hot_state() -> Vec<HotEntry> {
    let mut entries = URBIT_HOT_STATE.to_vec();
    entries.extend_from_slice(BY_JETS);
    entries.extend_from_slice(IN_JETS);
    entries
}
