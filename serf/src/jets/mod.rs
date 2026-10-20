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
//! Currently: the `+by` map door (see [`map`]), the `+in` set door (see
//! [`set`]), and the chapter-`two` `+list`/atom helpers (see [`list`],
//! [`bits`]).

use either::Either::Left;
use nockvm::jets::hot::{HotEntry, K_138, URBIT_HOT_STATE};

pub mod bits;
pub mod flot;
pub mod list;
pub mod map;
pub mod ob;
pub mod po;
pub mod set;
pub mod text;
pub mod ut;

macro_rules! ut_arm {
    ($name:literal, $jet:expr) => {
        (
            &[
                K_138,
                Left(b"one"),
                Left(b"two"),
                Left(b"tri"),
                Left(b"qua"),
                Left(b"pen"),
                Left($name),
            ],
            1,
            $jet as nockvm::jets::Jet,
        )
    };
}

/// The `+ut` chapter-`pen` type helpers. Path `[k138 one two tri qua pen
/// <arm>]`, axis 1.
pub static UT_JETS: &[HotEntry] = &[
    ut_arm!(b"cell", ut::jet_cell),
    ut_arm!(b"comb", ut::jet_comb),
    ut_arm!(b"cons", ut::jet_cons),
    ut_arm!(b"core", ut::jet_core),
    ut_arm!(b"face", ut::jet_face),
    ut_arm!(b"fitz", ut::jet_fitz),
    ut_arm!(b"flan", ut::jet_flan),
    ut_arm!(b"flip", ut::jet_flip),
    ut_arm!(b"flor", ut::jet_flor),
    ut_arm!(b"fork", ut::jet_fork),
    ut_arm!(b"look", ut::jet_look),
    ut_arm!(b"loot", ut::jet_loot),
];

/// The chapter-`qua` text arms `scot`/`slaw`: path `[k138 one two tri qua
/// <arm>]`, axis 1.
pub static TEXT_JETS: &[HotEntry] = &[
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"tri"), Left(b"qua"), Left(b"scot")],
        1,
        text::jet_scot,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"tri"), Left(b"qua"), Left(b"slaw")],
        1,
        text::jet_slaw,
    ),
];

/// The `+po` syllable-table arms (`ind ins tod tos`). `+po` is a child of
/// chapter `qua`: path `[k138 one two tri qua po <arm>]`, axis 1.
pub static PO_JETS: &[HotEntry] = &[
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"tri"), Left(b"qua"), Left(b"po"), Left(b"ind")],
        1,
        po::jet_po_ind,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"tri"), Left(b"qua"), Left(b"po"), Left(b"ins")],
        1,
        po::jet_po_ins,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"tri"), Left(b"qua"), Left(b"po"), Left(b"tod")],
        1,
        po::jet_po_tod,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"tri"), Left(b"qua"), Left(b"po"), Left(b"tos")],
        1,
        po::jet_po_tos,
    ),
];

/// The `+ob` @p-obfuscation arms (`fein`/`fynd`). The `+ob` core is a child of
/// chapter `tri`, so the path is `[k138 one two tri ob <arm>]`, axis 1.
pub static OB_JETS: &[HotEntry] = &[
    (
        &[
            K_138,
            Left(b"one"),
            Left(b"two"),
            Left(b"tri"),
            Left(b"ob"),
            Left(b"fein"),
        ],
        1,
        ob::jet_fein,
    ),
    (
        &[
            K_138,
            Left(b"one"),
            Left(b"two"),
            Left(b"tri"),
            Left(b"ob"),
            Left(b"fynd"),
        ],
        1,
        ob::jet_fynd,
    ),
];

/// Chapter-`two` `+list` helper gates (`welp slag need reel skim skip skid murn
/// lien sort`) and atom/bit helpers (`swp pow`). Each is a plain gate, jetted at
/// its own formula (axis 1), path `[k138 one two <arm>]` — siblings of nockvm's
/// stock `flop`/`turn`/`roll` entries.
pub static LIST_JETS: &[HotEntry] = &[
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"welp")],
        1,
        list::jet_welp,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"slag")],
        1,
        list::jet_slag,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"need")],
        1,
        list::jet_need,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"reel")],
        1,
        list::jet_reel,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"skim")],
        1,
        list::jet_skim,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"skip")],
        1,
        list::jet_skip,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"skid")],
        1,
        list::jet_skid,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"murn")],
        1,
        list::jet_murn,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"lien")],
        1,
        list::jet_lien,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"sort")],
        1,
        list::jet_sort,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"swp")],
        1,
        bits::jet_swp,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"pow")],
        1,
        bits::jet_pow,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"aor")],
        1,
        bits::jet_aor,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"mat")],
        1,
        bits::jet_mat,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"rub")],
        1,
        bits::jet_rub,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"muk")],
        1,
        bits::jet_muk,
    ),
    (
        &[K_138, Left(b"one"), Left(b"two"), Left(b"mate")],
        1,
        list::jet_mate,
    ),
];

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
    entries.extend_from_slice(LIST_JETS);
    entries.extend_from_slice(OB_JETS);
    entries.extend_from_slice(PO_JETS);
    entries.extend_from_slice(TEXT_JETS);
    entries.extend_from_slice(flot::HOON_FLOAT_HOT);
    entries.extend_from_slice(UT_JETS);
    entries
}
