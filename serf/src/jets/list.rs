// Standard `+list` helper jets for Eris.
//
// nockvm's stock dashboard (`URBIT_HOT_STATE`) jets the hot list arms it needs
// for Nockchain (`flop lent roll snag snip turn zing reap levy find scag`) but
// leaves the rest of hoon.hoon's chapter-`two` list toolkit unjetted, because
// Nockchain's kernels don't lean on them. A stock k138 Urbit pill does: these
// arms are everywhere in Arvo and userspace Hoon.
//
// This module adds the missing ones, modeled on Vere's C jets
// (`pkg/noun/jets/b/{sort,reel,skid,...}.c`) and hoon.hoon's own definitions,
// in nockvm's house style. They are registered from the serf (see
// `super::eris_hot_state`) so nockvm's own dashboard stays unpatched; pinned to
// Hoon kelvin 138.
//
// Arms: `welp slag need reel skim skip skid murn lien sort`. All are plain
// gates in chapter `two`, so each jet reads its sample at axis 6 and (for the
// binary gates) the two arguments at axes 2 and 3 of the sample. The
// gate-slamming arms (`reel skim skip skid murn lien sort`) apply the sample
// gate `b` with `nockvm::jets::util::slam`, exactly as Vere's `u3j_gate_slam`.
//
// An invalid loobean out of a slammed predicate (anything but `%.y`/`%.n`)
// returns `JetErr::Punt`, falling back to the Nock — never a silent guess.

use nockvm::interpreter::Context;
use nockvm::jets::util::{slam, slot};
use nockvm::jets::{JetErr, Result};
use nockvm::mem::NockStack;
use nockvm::noun::{Noun, NounSpace, D, NO, T, YES};

type JetResult<T> = std::result::Result<T, JetErr>;


// ---------------------------------------------------------------------------
// shared helpers
// ---------------------------------------------------------------------------

#[inline(always)]
fn is_yes(noun: Noun) -> bool {
    unsafe { noun.raw_equals(&YES) }
}

#[inline(always)]
fn is_no(noun: Noun) -> bool {
    unsafe { noun.raw_equals(&NO) }
}

/// Read a loobean produced by a slammed predicate; `Punt` on anything else.
#[inline(always)]
fn loob(r: Noun) -> JetResult<bool> {
    if is_yes(r) {
        Ok(true)
    } else if is_no(r) {
        Ok(false)
    } else {
        Err(JetErr::Punt)
    }
}

/// Collect the elements of a (possibly improper) list head-first. Stops at the
/// first atom, exactly as hoon's `?~` terminates a list walk.
fn elements(list: Noun, space: &NounSpace) -> JetResult<Vec<Noun>> {
    let mut items = Vec::new();
    let mut cur = list;
    while !cur.is_atom() {
        items.push(slot(cur, 2, space)?);
        cur = slot(cur, 3, space)?;
    }
    Ok(items)
}

/// Build the list `[i0 i1 ... in-1 tail]` from `items`, terminated by `tail`
/// (`~`/`D(0)` for a proper list, an arbitrary noun for `+welp`).
fn build(stack: &mut NockStack, items: &[Noun], tail: Noun) -> Noun {
    let mut out = tail;
    for &x in items.iter().rev() {
        out = T(stack, &[x, out]);
    }
    out
}

/// Sample of the arm's gate core: axis 6 of the subject.
#[inline(always)]
fn sample(subject: Noun, space: &NounSpace) -> JetResult<Noun> {
    slot(subject, 6, space)
}

// ---------------------------------------------------------------------------
// pure arms
// ---------------------------------------------------------------------------

/// `+welp`: faceless weld — the elements of `a` followed by `b` (`b` becomes the
/// terminating tail, which need not be `~`). hoon: `|* [* *]`.
pub fn jet_welp(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = sample(subject, &space)?;
    let a = slot(sam, 2, &space)?;
    let b = slot(sam, 3, &space)?;
    let items = elements(a, &space)?;
    Ok(build(&mut context.stack, &items, b))
}

/// `+slag`: the suffix of `b` after dropping `a` elements (`~` if `a` exceeds
/// the length). hoon: `|* [a=@ b=(list)]`.
pub fn jet_slag(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = sample(subject, &space)?;
    let n = slot(sam, 2, &space)?
        .in_space(&space)
        .as_atom()
        .map_err(|_| JetErr::Punt)?
        .as_u64()
        .map_err(|_| JetErr::Punt)?;
    let mut list = slot(sam, 3, &space)?;
    let mut k = n;
    while k != 0 {
        if list.is_atom() {
            return Ok(D(0));
        }
        list = slot(list, 3, &space)?;
        k -= 1;
    }
    Ok(list)
}

/// `+need`: unwrap a `unit`, crashing on `~`. hoon: `|* a=(unit)` — the sample
/// *is* the unit. On `~` we `Punt` so the Nock raises the `%mean 'need'`.
pub fn jet_need(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let unit = sample(subject, &space)?;
    if unit.is_atom() {
        return Err(JetErr::Punt);
    }
    slot(unit, 3, &space)
}

// ---------------------------------------------------------------------------
// gate-slamming arms
// ---------------------------------------------------------------------------

/// `+reel`: right fold. The accumulator is seeded from the gate's own sample
/// tail (`+<+.b`, axis 13), then `(b element acc)` is applied right-to-left.
/// Modeled on Vere's `u3qb_reel`.
pub fn jet_reel(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = sample(subject, &space)?;
    let a = slot(sam, 2, &space)?;
    let gate = slot(sam, 3, &space)?;
    let mut acc = slot(gate, 13, &space)?;
    let items = elements(a, &space)?;
    for &x in items.iter().rev() {
        let s = T(&mut context.stack, &[x, acc]);
        acc = slam(context, gate, s)?;
    }
    Ok(acc)
}

/// Shared filter for `+skim`/`+skip`: keep elements whose predicate loobean
/// equals `keep_on_yes`. Modeled on Vere's `u3qb_skid`.
fn filter(context: &mut Context, subject: Noun, keep_on_yes: bool) -> Result {
    let space = context.stack.noun_space();
    let sam = sample(subject, &space)?;
    let mut list = slot(sam, 2, &space)?;
    let gate = slot(sam, 3, &space)?;
    let mut kept = Vec::new();
    while !list.is_atom() {
        let elem = slot(list, 2, &space)?;
        list = slot(list, 3, &space)?;
        if loob(slam(context, gate, elem)?)? == keep_on_yes {
            kept.push(elem);
        }
    }
    Ok(build(&mut context.stack, &kept, D(0)))
}

/// `+skim`: keep the elements the gate accepts (`%.y`). hoon: `|* [a=(list) b=$-(* ?)]`.
pub fn jet_skim(context: &mut Context, subject: Noun) -> Result {
    filter(context, subject, true)
}

/// `+skip`: keep the elements the gate rejects (`%.n`). hoon: `|* [a=(list) b=$-(* ?)]`.
pub fn jet_skip(context: &mut Context, subject: Noun) -> Result {
    filter(context, subject, false)
}

/// `+skid`: partition into `[p=accepted q=rejected]`, order preserved within
/// each. Modeled on Vere's `u3qb_skid`.
pub fn jet_skid(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = sample(subject, &space)?;
    let mut list = slot(sam, 2, &space)?;
    let gate = slot(sam, 3, &space)?;
    let mut yes = Vec::new();
    let mut no = Vec::new();
    while !list.is_atom() {
        let elem = slot(list, 2, &space)?;
        list = slot(list, 3, &space)?;
        if loob(slam(context, gate, elem)?)? {
            yes.push(elem);
        } else {
            no.push(elem);
        }
    }
    let p = build(&mut context.stack, &yes, D(0));
    let q = build(&mut context.stack, &no, D(0));
    Ok(T(&mut context.stack, &[p, q]))
}

/// `+murn`: maybe-map — apply `(b element)`, a `(unit)`, keeping the `u` of each
/// non-`~` result. hoon: `|* [a=(list) b=$-(* (unit))]`.
pub fn jet_murn(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = sample(subject, &space)?;
    let mut list = slot(sam, 2, &space)?;
    let gate = slot(sam, 3, &space)?;
    let mut kept = Vec::new();
    while !list.is_atom() {
        let elem = slot(list, 2, &space)?;
        list = slot(list, 3, &space)?;
        let unit = slam(context, gate, elem)?;
        if !unit.is_atom() {
            // non-`~`: take `u.c` (axis 3 of the `[~ u=*]` cell).
            kept.push(slot(unit, 3, &space)?);
        }
    }
    Ok(build(&mut context.stack, &kept, D(0)))
}

/// `+lien`: does the gate accept *any* element? (`%.y` on the first hit, else
/// `%.n`). hoon: `|* [a=(list) b=$-(* ?)]`.
pub fn jet_lien(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = sample(subject, &space)?;
    let mut list = slot(sam, 2, &space)?;
    let gate = slot(sam, 3, &space)?;
    while !list.is_atom() {
        let elem = slot(list, 2, &space)?;
        list = slot(list, 3, &space)?;
        if loob(slam(context, gate, elem)?)? {
            return Ok(YES);
        }
    }
    Ok(NO)
}

/// `+sort`: quicksort by comparator gate `b` (`(b x pivot)` sends `x` before the
/// pivot when `%.y`). Pivot is the head, exactly as Vere's `_quicksort` and
/// hoon's skid-based `+sort`. hoon: `|* [a=(list) b=$-([* *] ?)]`.
pub fn jet_sort(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = sample(subject, &space)?;
    let a = slot(sam, 2, &space)?;
    let gate = slot(sam, 3, &space)?;
    let items = elements(a, &space)?;
    let sorted = quicksort(context, gate, items)?;
    Ok(build(&mut context.stack, &sorted, D(0)))
}

/// Recursive quicksort: pivot = first element, `(gate x pivot)` true ⇒ left.
/// Matches Vere's `_quicksort` partition and `sort(lef) ++ [pivot] ++ sort(rit)`.
fn quicksort(context: &mut Context, gate: Noun, items: Vec<Noun>) -> JetResult<Vec<Noun>> {
    if items.len() <= 1 {
        return Ok(items);
    }
    let pivot = items[0];
    let mut lef = Vec::new();
    let mut rit = Vec::new();
    for &x in &items[1..] {
        let s = T(&mut context.stack, &[x, pivot]);
        if loob(slam(context, gate, s)?)? {
            lef.push(x);
        } else {
            rit.push(x);
        }
    }
    let mut out = quicksort(context, gate, lef)?;
    out.push(pivot);
    out.extend(quicksort(context, gate, rit)?);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nockvm::jets::util::test::{assert_jet, init_context};
    use nockvm::noun::{Noun, D, T};

    /// Build a proper list `[e0 e1 ... ~]`.
    fn list(stack: &mut NockStack, elems: &[u64]) -> Noun {
        let mut out = D(0);
        for &e in elems.iter().rev() {
            out = T(stack, &[D(e), out]);
        }
        out
    }

    /// A gate whose battery is `formula`, with sample `~` and context `~`:
    /// `[formula 0 0]`. `slam` runs `formula` against `[formula sample 0]`.
    fn gate(stack: &mut NockStack, formula: Noun) -> Noun {
        T(stack, &[formula, D(0), D(0)])
    }

    // --- pure arms -------------------------------------------------------

    #[test]
    fn test_welp() {
        let c = &mut init_context();
        // (welp ~[1 2] ~[3 4]) -> ~[1 2 3 4]
        let a = list(&mut c.stack, &[1, 2]);
        let b = list(&mut c.stack, &[3, 4]);
        let sam = T(&mut c.stack, &[a, b]);
        let want = list(&mut c.stack, &[1, 2, 3, 4]);
        assert_jet(c, jet_welp, sam, want);
        // empty prefix returns b unchanged
        let b2 = list(&mut c.stack, &[3, 4]);
        let sam2 = T(&mut c.stack, &[D(0), b2]);
        let want2 = list(&mut c.stack, &[3, 4]);
        assert_jet(c, jet_welp, sam2, want2);
    }

    #[test]
    fn test_slag() {
        let c = &mut init_context();
        let b = list(&mut c.stack, &[1, 2, 3, 4]);
        let sam = T(&mut c.stack, &[D(2), b]);
        let want = list(&mut c.stack, &[3, 4]);
        assert_jet(c, jet_slag, sam, want);
        // drop more than length -> ~
        let b2 = list(&mut c.stack, &[1, 2]);
        let sam2 = T(&mut c.stack, &[D(5), b2]);
        assert_jet(c, jet_slag, sam2, D(0));
        // drop 0 -> whole list
        let b3 = list(&mut c.stack, &[7, 8]);
        let sam3 = T(&mut c.stack, &[D(0), b3]);
        let want3 = list(&mut c.stack, &[7, 8]);
        assert_jet(c, jet_slag, sam3, want3);
    }

    #[test]
    fn test_need() {
        let c = &mut init_context();
        // (need [~ 42]) -> 42
        let unit = T(&mut c.stack, &[D(0), D(42)]);
        assert_jet(c, jet_need, unit, D(42));
    }

    // --- gate-slamming arms ---------------------------------------------

    #[test]
    fn test_reel() {
        let c = &mut init_context();
        // A `cons` gate: battery `[0 6]` returns its whole sample `[element acc]`.
        // reel seeds the accumulator from the gate's own sample tail (axis 13),
        // so the gate needs a *cell* default sample `[p q]`; q=`~` is the seed.
        // Folding right, (b e acc)=[e acc] rebuilds the list unchanged:
        //   acc=~; (b 3 ~)=[3 ~]; (b 2 [3 ~])=[2 3 ~]; (b 1 ..)=[1 2 3 ~].
        let a = list(&mut c.stack, &[1, 2, 3]);
        let formula = T(&mut c.stack, &[D(0), D(6)]);
        let seeded_sample = T(&mut c.stack, &[D(0), D(0)]); // [p=~ q=~]; seed = q
        let g = T(&mut c.stack, &[formula, seeded_sample, D(0)]);
        let sam = T(&mut c.stack, &[a, g]);
        let want = list(&mut c.stack, &[1, 2, 3]);
        assert_jet(c, jet_reel, sam, want);
    }

    #[test]
    fn test_skim_skip_skid() {
        let c = &mut init_context();
        // predicate: is the element 0? battery `[5 [0 6] [1 0]]` = =(sample 0).
        // =(a b) yields %.y(0)/%.n(1); here compare whole sample (axis6) to 0.
        let eq0 = |c: &mut Context| -> Noun {
            let zero = T(&mut c.stack, &[D(1), D(0)]);
            let samp = T(&mut c.stack, &[D(0), D(6)]);
            let formula = T(&mut c.stack, &[D(5), samp, zero]);
            gate(&mut c.stack, formula)
        };

        // skim keeps the 0s
        let a = list(&mut c.stack, &[0, 1, 0, 2, 0]);
        let g = eq0(c);
        let sam = T(&mut c.stack, &[a, g]);
        let want = list(&mut c.stack, &[0, 0, 0]);
        assert_jet(c, jet_skim, sam, want);

        // skip keeps the non-0s
        let a2 = list(&mut c.stack, &[0, 1, 0, 2, 0]);
        let g2 = eq0(c);
        let sam2 = T(&mut c.stack, &[a2, g2]);
        let want2 = list(&mut c.stack, &[1, 2]);
        assert_jet(c, jet_skip, sam2, want2);

        // skid partitions [zeros nonzeros]
        let a3 = list(&mut c.stack, &[0, 1, 0, 2]);
        let g3 = eq0(c);
        let sam3 = T(&mut c.stack, &[a3, g3]);
        let p = list(&mut c.stack, &[0, 0]);
        let q = list(&mut c.stack, &[1, 2]);
        let want3 = T(&mut c.stack, &[p, q]);
        assert_jet(c, jet_skid, sam3, want3);
    }

    #[test]
    fn test_lien() {
        let c = &mut init_context();
        let eq0 = |c: &mut Context| -> Noun {
            let zero = T(&mut c.stack, &[D(1), D(0)]);
            let samp = T(&mut c.stack, &[D(0), D(6)]);
            let formula = T(&mut c.stack, &[D(5), samp, zero]);
            gate(&mut c.stack, formula)
        };
        // some element is 0 -> %.y
        let a = list(&mut c.stack, &[1, 2, 0, 3]);
        let g = eq0(c);
        let sam = T(&mut c.stack, &[a, g]);
        assert_jet(c, jet_lien, sam, YES);
        // none is 0 -> %.n
        let a2 = list(&mut c.stack, &[1, 2, 3]);
        let g2 = eq0(c);
        let sam2 = T(&mut c.stack, &[a2, g2]);
        assert_jet(c, jet_lien, sam2, NO);
    }

    #[test]
    fn test_murn() {
        let c = &mut init_context();
        // gate maps element -> (unit): if element is 0 return ~, else [~ element].
        // battery: ?:(=(e 0) ~ [~ e]). Nock:
        //   test `=(e 0)` : [5 [0 6] [1 0]]
        //   then branch `~` : [1 0]
        //   else branch `[~ e]` : [1 0] cons [0 6] -> [[1 0] [0 6]]? build cell.
        // formula = [6 test yes no]
        let test = {
            let samp = T(&mut c.stack, &[D(0), D(6)]);
            let zero = T(&mut c.stack, &[D(1), D(0)]);
            T(&mut c.stack, &[D(5), samp, zero])
        };
        let yes = T(&mut c.stack, &[D(1), D(0)]); // produce ~ (0)
        let no = {
            // [~ e] = [0 <element at axis 6>]; cons via Nock autocons [[1 0] [0 6]]
            let head = T(&mut c.stack, &[D(1), D(0)]);
            let tail = T(&mut c.stack, &[D(0), D(6)]);
            T(&mut c.stack, &[head, tail])
        };
        let formula = T(&mut c.stack, &[D(6), test, yes, no]);
        let g = gate(&mut c.stack, formula);
        let a = list(&mut c.stack, &[0, 5, 0, 7]);
        let sam = T(&mut c.stack, &[a, g]);
        let want = list(&mut c.stack, &[5, 7]);
        assert_jet(c, jet_murn, sam, want);
    }

    #[test]
    fn test_sort() {
        let c = &mut init_context();
        // Raw Nock has no magnitude compare, so we drive the quicksort machinery
        // with two *deterministic* comparators whose output we can predict
        // exactly — this exercises partition (both directions), recursion, and
        // rebuild without needing the stdlib in the gate's context.

        // empty and singleton: comparator never runs.
        let f0 = T(&mut c.stack, &[D(0), D(6)]);
        let cmp0 = gate(&mut c.stack, f0);
        let sam0 = T(&mut c.stack, &[D(0), cmp0]);
        assert_jet(c, jet_sort, sam0, D(0));

        // const-%.y (`[1 0]`): every non-pivot sorts *before* the pivot, which
        // reverses the list. [1 2 3 4] -> [4 3 2 1].
        let fy = T(&mut c.stack, &[D(1), D(0)]);
        let all_yes = gate(&mut c.stack, fy);
        let a = list(&mut c.stack, &[1, 2, 3, 4]);
        let sam = T(&mut c.stack, &[a, all_yes]);
        let want = list(&mut c.stack, &[4, 3, 2, 1]);
        assert_jet(c, jet_sort, sam, want);

        // const-%.n (`[1 1]`): every non-pivot sorts *after* the pivot, which
        // preserves order. [1 2 3 4] -> [1 2 3 4].
        let fn_ = T(&mut c.stack, &[D(1), D(1)]);
        let all_no = gate(&mut c.stack, fn_);
        let a2 = list(&mut c.stack, &[1, 2, 3, 4]);
        let sam2 = T(&mut c.stack, &[a2, all_no]);
        let want2 = list(&mut c.stack, &[1, 2, 3, 4]);
        assert_jet(c, jet_sort, sam2, want2);
    }
}
