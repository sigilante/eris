// `+ut`/type-system helper jets for Eris (chapter `pen`).
//
// These are the type constructors and small type/formula operators the Hoon
// compiler leans on. nockvm jets the hot compiler core (`mint`/`mull`/`nest`/
// `mink`/`mole`/`fuse`/`fish`/`crop`) but leaves these secondary ones unjetted.
// Ported from Vere's `pkg/noun/jets/f/*.c` in nockvm's house style; all are
// plain gates in chapter `pen`, path `[k138 one two tri qua pen <arm>]`, so each
// reads its sample at axis 6 (args at 2/3; `flip`/`fork` are unary at 6).
//
// Arms: `cell face core cons flip flan flor fitz fork comb look loot`. They
// operate on Hoon `type`/`hoon` nouns (tagged by motes `%cell %core %face
// %fork %void`), and `look`/`loot` resolve a name in a type's symbol table —
// used by every variable reference the compiler resolves, so the boot exercises
// them directly.

use ibig::UBig;
use nockvm::interpreter::Context;
use nockvm::jets::bits::util::met;
use nockvm::jets::sort::util::gor;
use nockvm::jets::util::slot;
use nockvm::jets::{JetErr, Result};
use nockvm::mem::NockStack;
use nockvm::noun::{Atom, Noun, NounSpace, D, T};
use nockvm::unifying_equality::unifying_equality;

use super::set;

type JetResult<T> = std::result::Result<T, JetErr>;



const fn tas(s: &[u8]) -> u64 {
    let mut v = 0u64;
    let mut i = s.len();
    while i > 0 {
        i -= 1;
        v = (v << 8) | s[i] as u64;
    }
    v
}

const VOID: u64 = tas(b"void");
const CELL: u64 = tas(b"cell");
const CORE: u64 = tas(b"core");
const FACE: u64 = tas(b"face");
const FORK: u64 = tas(b"fork");

#[inline(always)]
fn is(n: Noun, v: u64) -> bool {
    unsafe { n.raw_equals(&D(v)) }
}
#[inline(always)]
fn is_nul(n: Noun) -> bool {
    unsafe { n.raw_equals(&D(0)) }
}
fn noun_eq(stack: &mut NockStack, a: Noun, b: Noun) -> bool {
    let mut a = a;
    let mut b = b;
    unsafe { unifying_equality(stack, &mut a as *mut Noun, &mut b as *mut Noun) }
}
fn gor_lt(stack: &mut NockStack, a: Noun, b: Noun, space: &NounSpace) -> bool {
    unsafe { gor(stack, a, b, space).raw_equals(&D(0)) }
}

// Every `UBig` here is built with the `_stack` constructors so its words live on
// the NockStack arena and are freed with the event's frame; ibig's plain
// operators leak a never-freed heap buffer per call (see bits.rs). `look`/`loot`
// run on every compiler variable resolution, so that leak would be large.

fn ubig(stack: &mut NockStack, n: Noun, space: &NounSpace) -> JetResult<UBig> {
    let a = n.as_atom().map_err(|_| JetErr::Punt)?;
    Ok(UBig::from_le_bytes_stack(stack, a.in_space(space).as_ne_bytes()))
}
fn from_ubig(stack: &mut NockStack, x: &UBig) -> Noun {
    Atom::from_ubig(stack, x).as_noun()
}

/// `+peg`: axis of `b` within axis `a` (`b >= 1`).
fn peg(stack: &mut NockStack, a: &UBig, b: &UBig) -> UBig {
    // `b` must be a real axis (`>= 1`). `bit_len(b) <= 1` means `b` is 0 or 1:
    // for `b == 1` the axis is unchanged (`a`), and for `b == 0` returning `a`
    // avoids the `bit_len - 1` underflow that would otherwise shift by
    // `usize::MAX`. Callers must not pass 0 (`comb` guards for it); this is the
    // backstop (dwarf's `peg` underflow finding).
    let bl = b.bit_len();
    if bl <= 1 {
        return a.clone_stack(stack);
    }
    let d = bl - 1;
    // peg = (lsh [0 d] a) | (end [0 d] b). The top (d-th) bit of `b` is set, so
    // its low `d` bits are `b - 2^d`; and `a << d` has zero low `d` bits, so the
    // splice is a disjoint add.
    let one = UBig::from_unsigned_stack(stack, 1u64);
    let two_d = UBig::shl_stack(stack, one, d);
    let b_owned = b.clone_stack(stack);
    let lo = UBig::sub_stack(stack, b_owned, two_d);
    let hi = UBig::shl_ref_stack(stack, a, d);
    UBig::add_stack(stack, hi, lo)
}

// --- type constructors ------------------------------------------------------

/// `+cell:ut`: `[%cell hed tal]`, or `%void` if either is `%void`.
pub fn jet_cell(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let hed = slot(sam, 2, &space)?;
    let tal = slot(sam, 3, &space)?;
    if is(hed, VOID) || is(tal, VOID) {
        return Ok(D(VOID));
    }
    Ok(T(&mut context.stack, &[D(CELL), hed, tal]))
}

/// `+face:ut`: `[%face sag tip]`, or `%void` if `tip` is `%void`.
pub fn jet_face(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let sag = slot(sam, 2, &space)?;
    let tip = slot(sam, 3, &space)?;
    if is(tip, VOID) {
        return Ok(D(VOID));
    }
    Ok(T(&mut context.stack, &[D(FACE), sag, tip]))
}

/// `+core:ut`: `[%core pac con]`, or `%void` if `pac` is `%void`.
pub fn jet_core(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let pac = slot(sam, 2, &space)?;
    let con = slot(sam, 3, &space)?;
    if is(pac, VOID) {
        return Ok(D(VOID));
    }
    Ok(T(&mut context.stack, &[D(CORE), pac, con]))
}

/// `+cons:ut`: fold two formulas into one — `[1 p q]` when both are literals
/// `[1 p]`/`[1 q]`, else the autocons `[vur sed]`.
pub fn jet_cons(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let vur = slot(sam, 2, &space)?;
    let sed = slot(sam, 3, &space)?;
    // both `[1 _]`?
    if !vur.is_atom() && is(slot(vur, 2, &space)?, 1) && !sed.is_atom() && is(slot(sed, 2, &space)?, 1)
    {
        let p = slot(vur, 3, &space)?;
        let q = slot(sed, 3, &space)?;
        return Ok(T(&mut context.stack, &[D(1), p, q]));
    }
    Ok(T(&mut context.stack, &[vur, sed]))
}

// --- loobean-formula operators ----------------------------------------------

/// `+flip:ut`: negate a loobean formula. `[1 %.y]`<->`[1 %.n]`, else
/// `[6 hel [1 %.n] [1 %.y]]`.
pub fn jet_flip(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let hel = slot(subject, 6, &space)?;
    if !hel.is_atom() && is(slot(hel, 2, &space)?, 1) {
        let x = slot(hel, 3, &space)?;
        return if is_nul(x) {
            Ok(T(&mut context.stack, &[D(1), D(1)]))
        } else if is(x, 1) {
            Ok(T(&mut context.stack, &[D(1), D(0)]))
        } else {
            Err(JetErr::Punt)
        };
    }
    let yes = T(&mut context.stack, &[D(1), D(1)]);
    let no = T(&mut context.stack, &[D(1), D(0)]);
    Ok(T(&mut context.stack, &[D(6), hel, yes, no]))
}

/// `+flan:ut`: loobean AND of two formulas (constant-folding). Vere `u3qf_flan`.
pub fn jet_flan(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let bos = slot(sam, 2, &space)?;
    let nif = slot(sam, 3, &space)?;
    if noun_eq(&mut context.stack, bos, nif) {
        return Ok(bos);
    }
    if !bos.is_atom() && is(slot(bos, 2, &space)?, 1) {
        return if is_nul(slot(bos, 3, &space)?) {
            Ok(nif)
        } else {
            Ok(bos)
        };
    }
    if !nif.is_atom() && is(slot(nif, 2, &space)?, 1) {
        return if is_nul(slot(nif, 3, &space)?) {
            Ok(bos)
        } else {
            Ok(nif)
        };
    }
    let no = T(&mut context.stack, &[D(1), D(1)]);
    Ok(T(&mut context.stack, &[D(6), bos, nif, no]))
}

/// `+flor:ut`: loobean OR of two formulas (constant-folding). Vere `u3qf_flor`.
pub fn jet_flor(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let bos = slot(sam, 2, &space)?;
    let nif = slot(sam, 3, &space)?;
    if noun_eq(&mut context.stack, bos, nif) {
        return Ok(bos);
    }
    if !bos.is_atom() && is(slot(bos, 2, &space)?, 1) {
        return if is_nul(slot(bos, 3, &space)?) {
            Ok(bos)
        } else {
            Ok(nif)
        };
    }
    if !nif.is_atom() && is(slot(nif, 2, &space)?, 1) {
        return if is_nul(slot(nif, 3, &space)?) {
            Ok(nif)
        } else {
            Ok(bos)
        };
    }
    let yes = T(&mut context.stack, &[D(1), D(0)]);
    Ok(T(&mut context.stack, &[D(6), bos, yes, nif]))
}

// --- fitz: aura compatibility -----------------------------------------------

/// `+fitz:ut`: is aura `wix` a sub-aura of (compatible with) `yaz`? This is the
/// aura-nesting test (e.g. `@tas` nests under `@ta`), NOT a Hoon-kelvin check.
/// Vere `u3qf_fitz`.
pub fn jet_fitz(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let yaz = slot(sam, 2, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let wix = slot(sam, 3, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let yb = yaz.in_space(&space).as_ne_bytes().to_vec();
    let wb = wix.in_space(&space).as_ne_bytes().to_vec();
    let yet = met(3, yaz, &space);
    let wet = met(3, wix, &space);

    // _fitz_fiz: trailing uppercase letters must not increase.
    let last_upper = |buf: &[u8], len: usize| -> u8 {
        if len == 0 {
            return 0;
        }
        let b = buf[len - 1];
        if b.is_ascii_uppercase() { b } else { 0 }
    };
    let (yy, wy) = (last_upper(&yb, yet), last_upper(&wb, wet));
    if yy != 0 && wy != 0 && wy > yy {
        return Ok(D(1)); // %.n
    }
    let mn = yet.min(wet);
    for i in 0..mn {
        let yc = *yb.get(i).unwrap_or(&0);
        let wc = *wb.get(i).unwrap_or(&0);
        if i == yet - 1 && yc.is_ascii_uppercase() {
            return Ok(D(0));
        }
        if i == wet - 1 && wc.is_ascii_uppercase() {
            return Ok(D(0));
        }
        if yc != wc {
            return Ok(D(1));
        }
    }
    Ok(D(0)) // %.y
}

// --- fork: build a union type -----------------------------------------------

/// `+fork:ut`: fold a `(list type)` into a fork (union) type. Vere `u3qf_fork`:
/// drop `%void`s, flatten nested forks via set-union, dedup via set-put.
pub fn jet_fork(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let mut yed = slot(subject, 6, &space)?;
    let mut lez = D(0); // the accumulated (set type)
    while !yed.is_atom() {
        let i_yed = slot(yed, 2, &space)?;
        yed = slot(yed, 3, &space)?;
        if is(i_yed, VOID) {
            continue;
        }
        if !i_yed.is_atom() && is(slot(i_yed, 2, &space)?, FORK) {
            // nested fork: union its set
            let inner = slot(i_yed, 3, &space)?;
            lez = set::set_uni(&mut context.stack, lez, inner, &space)?;
        } else {
            lez = set::set_put(&mut context.stack, lez, i_yed, &space)?;
        }
    }
    if is_nul(lez) {
        return Ok(D(VOID));
    }
    // single element [n ~ ~] -> n
    if is_nul(slot(lez, 6, &space)?) && is_nul(slot(lez, 7, &space)?) {
        return slot(lez, 2, &space);
    }
    Ok(T(&mut context.stack, &[D(FORK), lez]))
}

// --- comb: compose two formulas ---------------------------------------------

/// `+comb:ut`: compose Nock formulas `mal` then `buz`. Vere `u3qf_comb`.
pub fn jet_comb(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let mal = slot(sam, 2, &space)?;
    let buz = slot(sam, 3, &space)?;

    // mal = [0 p_mal], p_mal != 0?
    let mal_axis = if !mal.is_atom() && is(slot(mal, 2, &space)?, 0) {
        let p = slot(mal, 3, &space)?;
        if !is_nul(p) {
            Some(p)
        } else {
            None
        }
    } else {
        None
    };

    if let Some(p_mal) = mal_axis {
        let pm = ubig(&mut context.stack, p_mal, &space)?;
        // buz = [0 p_buz], p_buz != 0?
        if !buz.is_atom() && is(slot(buz, 2, &space)?, 0) {
            let p_buz = slot(buz, 3, &space)?;
            if !is_nul(p_buz) {
                let pb = ubig(&mut context.stack, p_buz, &space)?;
                let r = peg(&mut context.stack, &pm, &pb);
                let ax = from_ubig(&mut context.stack, &r);
                return Ok(T(&mut context.stack, &[D(0), ax]));
            }
        }
        // buz = [2 [0 pp] [0 pq]] ? Both inner axes must be real (`!= 0`): a
        // `[0 0]` is not a valid `[0 axis]` formula, so `peg` must not see a 0
        // (which would underflow) — fall through to the generic `[7 mal buz]`.
        if !buz.is_atom() && is(slot(buz, 2, &space)?, 2) {
            let p_buz = slot(buz, 6, &space)?;
            let q_buz = slot(buz, 7, &space)?;
            if !p_buz.is_atom()
                && is(slot(p_buz, 2, &space)?, 0)
                && !q_buz.is_atom()
                && is(slot(q_buz, 2, &space)?, 0)
                && !is_nul(slot(p_buz, 3, &space)?)
                && !is_nul(slot(q_buz, 3, &space)?)
            {
                let pp = ubig(&mut context.stack, slot(p_buz, 3, &space)?, &space)?;
                let pq = ubig(&mut context.stack, slot(q_buz, 3, &space)?, &space)?;
                let r1 = peg(&mut context.stack, &pm, &pp);
                let a1 = from_ubig(&mut context.stack, &r1);
                let r2 = peg(&mut context.stack, &pm, &pq);
                let a2 = from_ubig(&mut context.stack, &r2);
                let e1 = T(&mut context.stack, &[D(0), a1]);
                let e2 = T(&mut context.stack, &[D(0), a2]);
                return Ok(T(&mut context.stack, &[D(2), e1, e2]));
            }
        }
        return Ok(T(&mut context.stack, &[D(7), mal, buz]));
    }

    // bush branch: mal = [p_mal q_mal], p_mal a cell, q_mal == [0 1] -> [8 p_mal buz]
    if !mal.is_atom() {
        let p_mal = slot(mal, 2, &space)?;
        let q_mal = slot(mal, 3, &space)?;
        if !p_mal.is_atom()
            && !q_mal.is_atom()
            && is(slot(q_mal, 2, &space)?, 0)
            && is(slot(q_mal, 3, &space)?, 1)
        {
            return Ok(T(&mut context.stack, &[D(8), p_mal, buz]));
        }
    }

    // buz = [0 1] (identity) -> mal
    if !buz.is_atom() && is(slot(buz, 2, &space)?, 0) && is(slot(buz, 3, &space)?, 1) {
        return Ok(mal);
    }

    Ok(T(&mut context.stack, &[D(7), mal, buz]))
}

// --- look / loot: resolve a name in a type's symbol table -------------------

/// `_look_in`: walk the `(map term X)` `dab` for `cog`, building the axis `axe`.
/// Returns `[~ axe value]` or `~`. Vere `_look_in`.
fn look_in(
    context: &mut Context,
    cog: Noun,
    dab: Noun,
    axe: &UBig,
    space: &NounSpace,
) -> JetResult<Noun> {
    if dab.is_atom() {
        return Ok(D(0));
    }
    let n = slot(dab, 2, space)?;
    let l = slot(dab, 6, space)?;
    let r = slot(dab, 7, space)?;
    if n.is_atom() {
        return Err(JetErr::Punt); // bad look
    }
    let pn = slot(n, 2, space)?;
    let qn = slot(n, 3, space)?;
    let hit = !qn.is_atom() && noun_eq(&mut context.stack, cog, pn);
    let (l_nul, r_nul) = (is_nul(l), is_nul(r));

    // `axe * 2` / `* 3` / `* 6` / `* 7` via `peg`, each as a NockStack UBig.
    let peg_k = |context: &mut Context, k: u64| -> UBig {
        let kb = UBig::from_unsigned_stack(&mut context.stack, k);
        peg(&mut context.stack, axe, &kb)
    };
    let found = |context: &mut Context, a: &UBig| -> Noun {
        let ax = from_ubig(&mut context.stack, a);
        T(&mut context.stack, &[D(0), ax, qn])
    };

    if l_nul && r_nul {
        if hit {
            Ok(found(context, axe))
        } else {
            Ok(D(0))
        }
    } else if l_nul {
        if hit {
            let p = peg_k(context, 2);
            Ok(found(context, &p))
        } else if gor_lt(&mut context.stack, cog, pn, space) {
            Ok(D(0))
        } else {
            let p = peg_k(context, 3);
            look_in(context, cog, r, &p, space)
        }
    } else if r_nul {
        if hit {
            let p = peg_k(context, 2);
            Ok(found(context, &p))
        } else if gor_lt(&mut context.stack, cog, pn, space) {
            let p = peg_k(context, 3);
            look_in(context, cog, l, &p, space)
        } else {
            Ok(D(0))
        }
    } else if hit {
        let p = peg_k(context, 2);
        Ok(found(context, &p))
    } else if gor_lt(&mut context.stack, cog, pn, space) {
        let p = peg_k(context, 6);
        look_in(context, cog, l, &p, space)
    } else {
        let p = peg_k(context, 7);
        look_in(context, cog, r, &p, space)
    }
}

/// `+look:ut`: resolve `cog` in the face map `dab`.
pub fn jet_look(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let cog = slot(sam, 2, &space)?;
    let dab = slot(sam, 3, &space)?;
    let one = UBig::from_unsigned_stack(&mut context.stack, 1u64);
    look_in(context, cog, dab, &one, &space)
}

/// `_loot_in`: like `_look_in` but over a `dom` whose values are themselves
/// looked up via `+look`. Vere `_loot_in`.
///
/// DEVIATION (intentional): hoon.hoon's `tome` is `[p=what q=(map term hoon)]`,
/// and `+loot` looks up in `q.tome` — so we read the inner map at `slot(qn, 3)`.
/// Vere's C `_loot_in` reads `u3t(n_dom)` directly, i.e. treats the node value
/// as a bare map. Against a real k138 pill the Hoon definition is authoritative;
/// a boot crash at `wack` confirmed the bare-map reading is wrong here. We match
/// hoon, not Vere's C. See CHANGES.md / PROVENANCE.md.
fn loot_in(
    context: &mut Context,
    cog: Noun,
    dom: Noun,
    axe: &UBig,
    space: &NounSpace,
) -> JetResult<Noun> {
    if dom.is_atom() {
        return Ok(D(0));
    }
    let n = slot(dom, 2, space)?;
    let l = slot(dom, 6, space)?;
    let r = slot(dom, 7, space)?;
    if n.is_atom() {
        return Err(JetErr::Punt);
    }
    let qn = slot(n, 3, space)?; // tome = [p=what q=(map term hoon)]
    let map = slot(qn, 3, space)?; // q.tome
    let one = UBig::from_unsigned_stack(&mut context.stack, 1u64);
    let yep = look_in(context, cog, map, &one, space)?;
    let (l_nul, r_nul) = (is_nul(l), is_nul(r));

    // `peg(axe, k)` as a NockStack UBig.
    let peg_k = |context: &mut Context, k: u64| -> UBig {
        let kb = UBig::from_unsigned_stack(&mut context.stack, k);
        peg(&mut context.stack, axe, &kb)
    };

    if l_nul && r_nul {
        if is_nul(yep) {
            Ok(D(0))
        } else {
            loot_on_hit(context, yep, axe, space)
        }
    } else if l_nul {
        if is_nul(yep) {
            let p = peg_k(context, 3);
            loot_in(context, cog, r, &p, space)
        } else {
            let p = peg_k(context, 2);
            loot_on_hit(context, yep, &p, space)
        }
    } else if r_nul {
        if is_nul(yep) {
            let p = peg_k(context, 3);
            loot_in(context, cog, l, &p, space)
        } else {
            let p = peg_k(context, 2);
            loot_on_hit(context, yep, &p, space)
        }
    } else if is_nul(yep) {
        let p6 = peg_k(context, 6);
        let pey = loot_in(context, cog, l, &p6, space)?;
        if !is_nul(pey) {
            Ok(pey)
        } else {
            let p7 = peg_k(context, 7);
            loot_in(context, cog, r, &p7, space)
        }
    } else {
        let p = peg_k(context, 2);
        loot_on_hit(context, yep, &p, space)
    }
}

/// On a `+loot` hit: result axis = `peg(base, u.yep.head)`, value = `u.yep.tail`.
/// `yep` is `[~ u_axe u_val]`, so u_axe = `slot(yep,6)`, u_val = `slot(yep,7)`.
fn loot_on_hit(
    context: &mut Context,
    yep: Noun,
    base: &UBig,
    space: &NounSpace,
) -> JetResult<Noun> {
    let u_axe = ubig(&mut context.stack, slot(yep, 6, space)?, space)?;
    let u_val = slot(yep, 7, space)?;
    let r = peg(&mut context.stack, base, &u_axe);
    let ax = from_ubig(&mut context.stack, &r);
    Ok(T(&mut context.stack, &[D(0), ax, u_val]))
}

/// `+loot:ut`: resolve `cog` through the arm map `dom`.
pub fn jet_loot(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let cog = slot(sam, 2, &space)?;
    let dom = slot(sam, 3, &space)?;
    let one = UBig::from_unsigned_stack(&mut context.stack, 1u64);
    loot_in(context, cog, dom, &one, &space)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nockvm::jets::util::test::{assert_jet, assert_noun_eq, init_context};
    use nockvm::noun::{D, T};

    const NOUN: u64 = tas(b"noun");

    #[test]
    fn test_cell_face_core() {
        let c = &mut init_context();
        // cell(%noun, %noun) = [%cell %noun %noun]
        let sam = T(&mut c.stack, &[D(NOUN), D(NOUN)]);
        let want = T(&mut c.stack, &[D(CELL), D(NOUN), D(NOUN)]);
        assert_jet(c, jet_cell, sam, want);
        // cell(%void, %noun) = %void
        let sam = T(&mut c.stack, &[D(VOID), D(NOUN)]);
        assert_jet(c, jet_cell, sam, D(VOID));
        // face(%void-tip) = %void
        let sam = T(&mut c.stack, &[D(NOUN), D(VOID)]);
        assert_jet(c, jet_face, sam, D(VOID));
        // core(%void, con) = %void
        let sam = T(&mut c.stack, &[D(VOID), D(0)]);
        assert_jet(c, jet_core, sam, D(VOID));
    }

    #[test]
    fn test_flip_flan_flor() {
        let c = &mut init_context();
        // flip [1 0] -> [1 1], flip [1 1] -> [1 0]
        let yes = T(&mut c.stack, &[D(1), D(0)]);
        let no = T(&mut c.stack, &[D(1), D(1)]);
        assert_jet(c, jet_flip, yes, no);
        let yes2 = T(&mut c.stack, &[D(1), D(0)]);
        let no2 = T(&mut c.stack, &[D(1), D(1)]);
        assert_jet(c, jet_flip, no2, yes2);
        // flan([1 0], X) -> X  (true && X)
        let t = T(&mut c.stack, &[D(1), D(0)]);
        let x = T(&mut c.stack, &[D(0), D(6)]);
        let sam = T(&mut c.stack, &[t, x]);
        let want = T(&mut c.stack, &[D(0), D(6)]);
        assert_jet(c, jet_flan, sam, want);
        // flor([1 0], X) -> [1 0]  (true || X)
        let t2 = T(&mut c.stack, &[D(1), D(0)]);
        let x2 = T(&mut c.stack, &[D(0), D(6)]);
        let sam = T(&mut c.stack, &[t2, x2]);
        let want = T(&mut c.stack, &[D(1), D(0)]);
        assert_jet(c, jet_flor, sam, want);
    }

    #[test]
    fn test_cons() {
        let c = &mut init_context();
        // cons([1 2], [1 3]) -> [1 2 3]
        let a = T(&mut c.stack, &[D(1), D(2)]);
        let b = T(&mut c.stack, &[D(1), D(3)]);
        let sam = T(&mut c.stack, &[a, b]);
        let want = T(&mut c.stack, &[D(1), D(2), D(3)]);
        assert_jet(c, jet_cons, sam, want);
        // cons([0 2], [1 3]) -> [[0 2] [1 3]] (autocons)
        let a = T(&mut c.stack, &[D(0), D(2)]);
        let b = T(&mut c.stack, &[D(1), D(3)]);
        let sam = T(&mut c.stack, &[a, b]);
        let aa = T(&mut c.stack, &[D(0), D(2)]);
        let bb = T(&mut c.stack, &[D(1), D(3)]);
        let want = T(&mut c.stack, &[aa, bb]);
        assert_jet(c, jet_cons, sam, want);
    }

    #[test]
    fn test_fork() {
        let c = &mut init_context();
        // fork(~) -> %void
        assert_jet(c, jet_fork, D(0), D(VOID));
        // fork(~[%noun]) -> %noun (single element collapses)
        let list = T(&mut c.stack, &[D(NOUN), D(0)]);
        assert_jet(c, jet_fork, list, D(NOUN));
    }

    #[test]
    fn test_look() {
        let c = &mut init_context();
        // dab = single-node map [[cog val] ~ ~]; look(cog) -> [~ 1 val]
        let cog = D(tas(b"a"));
        let val = T(&mut c.stack, &[D(1), D(2)]);
        let n = T(&mut c.stack, &[cog, val]);
        let dab = T(&mut c.stack, &[n, D(0), D(0)]);
        let sam = T(&mut c.stack, &[cog, dab]);
        let got = {
            let subj = T(&mut c.stack, &[D(0), sam, D(0)]);
            jet_look(c, subj).unwrap()
        };
        let valw = T(&mut c.stack, &[D(1), D(2)]);
        let want = T(&mut c.stack, &[D(0), D(1), valw]);
        assert_noun_eq(&mut c.stack, got, want);
        // miss -> ~
        let other = D(tas(b"b"));
        let val2 = T(&mut c.stack, &[D(1), D(2)]);
        let n2 = T(&mut c.stack, &[D(tas(b"a")), val2]);
        let dab2 = T(&mut c.stack, &[n2, D(0), D(0)]);
        let sam2 = T(&mut c.stack, &[other, dab2]);
        let subj2 = T(&mut c.stack, &[D(0), sam2, D(0)]);
        let got2 = jet_look(c, subj2).unwrap();
        assert_noun_eq(&mut c.stack, got2, D(0));
    }
    #[test]
    fn test_loot() {
        let c = &mut init_context();
        // inner face map (tome): foo -> hoon_val [1 42]
        let foo = D(tas(b"foo"));
        let hoon_val = T(&mut c.stack, &[D(1), D(42)]);
        let inner_n = T(&mut c.stack, &[foo, hoon_val]);
        let innermap = T(&mut c.stack, &[inner_n, D(0), D(0)]);
        let tome = T(&mut c.stack, &[D(0), innermap]); // tome = [what map]
        // dom: bat -> tome
        let bat = D(tas(b"bat"));
        let dom_n = T(&mut c.stack, &[bat, tome]);
        let dom = T(&mut c.stack, &[dom_n, D(0), D(0)]);
        // loot(foo, dom) should be [~ 1 [1 42]]
        let sam = T(&mut c.stack, &[foo, dom]);
        let subj = T(&mut c.stack, &[D(0), sam, D(0)]);
        let got = jet_loot(c, subj).unwrap();
        let hv = T(&mut c.stack, &[D(1), D(42)]);
        let want = T(&mut c.stack, &[D(0), D(1), hv]);
        assert_noun_eq(&mut c.stack, got, want);
    }
    #[test]
    fn test_loot_recurse() {
        let c = &mut init_context();
        // tome helper: single face key->val
        fn tome1(c: &mut Context, key: u64, val: Noun) -> Noun {
            let n = T(&mut c.stack, &[D(key), val]);
            let m = T(&mut c.stack, &[n, D(0), D(0)]);
            T(&mut c.stack, &[D(0), m]) // [what map]
        }
        fn domleaf(c: &mut Context, key: u64, tome: Noun) -> Noun {
            let n = T(&mut c.stack, &[D(key), tome]);
            T(&mut c.stack, &[n, D(0), D(0)])
        }
        let foo = tas(b"foo");
        // root tome has baz (miss); left tome has qux (miss); right tome has foo (hit)
        let x = T(&mut c.stack, &[D(1), D(1)]);
        let troot = tome1(c, tas(b"baz"), x);
        let y = T(&mut c.stack, &[D(1), D(2)]);
        let tl = tome1(c, tas(b"qux"), y);
        let hoon = T(&mut c.stack, &[D(1), D(42)]);
        let tr = tome1(c, foo, hoon);
        let lsub = domleaf(c, tas(b"k1"), tl);
        let rsub = domleaf(c, tas(b"k2"), tr);
        let nroot = T(&mut c.stack, &[D(tas(b"bat")), troot]);
        let dom = T(&mut c.stack, &[nroot, lsub, rsub]);
        let sam = T(&mut c.stack, &[D(foo), dom]);
        let subj = T(&mut c.stack, &[D(0), sam, D(0)]);
        let got = jet_loot(c, subj).unwrap();
        // expected: hit in right subtree, axis peg(7,1)=7
        let hv = T(&mut c.stack, &[D(1), D(42)]);
        let want = T(&mut c.stack, &[D(0), D(7), hv]);
        assert_noun_eq(&mut c.stack, got, want);
    }
    #[test]
    fn test_loot_compound_axis() {
        let c = &mut init_context();
        // tome where foo is at root with a right child -> look returns axis peg(1,2)=2
        let foo = tas(b"foo");
        let val = T(&mut c.stack, &[D(1), D(9)]);
        let fooent = T(&mut c.stack, &[D(foo), val]);
        let val2 = T(&mut c.stack, &[D(1), D(8)]);
        let barent = T(&mut c.stack, &[D(tas(b"bar")), val2]);
        let rchild = T(&mut c.stack, &[barent, D(0), D(0)]);
        let innermap = T(&mut c.stack, &[fooent, D(0), rchild]); // [* ~ *]
        let tome = T(&mut c.stack, &[D(0), innermap]); // [what map]
        // dom leaf: bat -> tome
        let domn = T(&mut c.stack, &[D(tas(b"bat")), tome]);
        let dom = T(&mut c.stack, &[domn, D(0), D(0)]);
        let sam = T(&mut c.stack, &[D(foo), dom]);
        let subj = T(&mut c.stack, &[D(0), sam, D(0)]);
        let got = jet_loot(c, subj).unwrap();
        // loot leaf: peg(axe=1, u_axe=2) = 2; value = val
        let hv = T(&mut c.stack, &[D(1), D(9)]);
        let want = T(&mut c.stack, &[D(0), D(2), hv]);
        assert_noun_eq(&mut c.stack, got, want);
    }

    #[test]
    fn test_comb_zero_axis_no_underflow() {
        let c = &mut init_context();
        // mal = [0 1]; buz = [2 [0 0] [0 0]]. The `[2 [0 pp][0 pq]]` fast path
        // must be skipped (axis 0 is invalid), so `peg` never sees a 0 and never
        // underflows (dwarf's peg finding). It falls through to [7 mal buz].
        let mal = T(&mut c.stack, &[D(0), D(1)]);
        let p0 = T(&mut c.stack, &[D(0), D(0)]);
        let q0 = T(&mut c.stack, &[D(0), D(0)]);
        let buz = T(&mut c.stack, &[D(2), p0, q0]);
        let sam = T(&mut c.stack, &[mal, buz]);
        let subj = T(&mut c.stack, &[D(0), sam, D(0)]);
        let got = jet_comb(c, subj).unwrap();
        let head = slot(got, 2, &c.stack.noun_space()).unwrap();
        assert!(unsafe { head.raw_equals(&D(7)) }, "must be [7 mal buz]");
    }
}
