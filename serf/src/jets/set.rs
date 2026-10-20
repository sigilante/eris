// `+in` set-door jets for Eris.
//
// Companion to `map.rs`. nockvm's stock dashboard leaves the standard Hoon `+in`
// set door unjetted for the same reason it leaves `+by` unjetted: Nockchain's
// kernels carry a digest-ordered `z-in`/`h-in` set instead. nockvm's own
// `jets/set.rs` does ship stock-ordered `jet_put`/`jet_has`, but they are
// commented out of the dashboard, cover only two of the arms, and the `put`
// dedup is broken (it compares with `raw_equals`, which misses structurally
// equal indirect atoms — the same bug `map.rs` avoids with `unifying_equality`).
//
// A set is a treap `~` | `[n=* l=set r=set]` — the element `n` sits directly in
// the node (no `[key value]`), ordered by the stock `mug`-based `+gor`/`+mor`.
// The algorithms follow hoon.hoon's `+in` (and mirror `map.rs` arm-for-arm);
// cross-checked against Vere's `pkg/noun/jets/.../in_*.c`. Correctness is proven
// by differential tests against a `BTreeSet` oracle plus the treap invariants.
//
// Registered from the serf (see `super::eris_hot_state`); nockvm stays unpatched.
// Pinned to Hoon kelvin 138.
//
// Covers the full Vere-k138 `+in` jet set. Deferred: `+rep`/`+rib` (gate folds
// with a gate-internal accumulator; Vere ships no dedicated jet either) — they
// run unjetted, correctly.

use nockvm::interpreter::Context;
use nockvm::jets::sort::util::{gor, mor};
use nockvm::jets::util::{slam, slot};
use nockvm::jets::{JetErr, Result};
use nockvm::mem::NockStack;
use nockvm::noun::{Noun, NounSpace, D, NO, T, YES};
use nockvm::unifying_equality::unifying_equality;

type JetResult<T> = std::result::Result<T, JetErr>;



// ---------------------------------------------------------------------------
// node plumbing: a node is `[n l r]`; n=2, l=6, r=7.
// ---------------------------------------------------------------------------

#[inline(always)]
fn is_yes(noun: Noun) -> bool {
    unsafe { noun.raw_equals(&YES) }
}

/// Split a node into `(elem, left, right)`.
fn decompose(node: Noun, space: &NounSpace) -> JetResult<(Noun, Noun, Noun)> {
    Ok((slot(node, 2, space)?, slot(node, 6, space)?, slot(node, 7, space)?))
}

/// Build `[elem left right]`.
fn make_node(stack: &mut NockStack, elem: Noun, left: Noun, right: Noun) -> Noun {
    T(stack, &[elem, left, right])
}

/// `+gor`: does `a` sort strictly before `b`?
#[inline(always)]
fn gor_lt(stack: &mut NockStack, a: Noun, b: Noun, space: &NounSpace) -> bool {
    is_yes(gor(stack, a, b, space))
}

/// `+mor`: does `a` outrank `b` (treap heap priority)?
#[inline(always)]
fn mor_lt(stack: &mut NockStack, a: Noun, b: Noun, space: &NounSpace) -> bool {
    is_yes(mor(stack, a, b, space))
}

/// Unifying structural equality, the stock noun `=`.
fn noun_eq(stack: &mut NockStack, a: Noun, b: Noun) -> bool {
    let mut a = a;
    let mut b = b;
    unsafe { unifying_equality(stack, &mut a as *mut Noun, &mut b as *mut Noun) }
}

/// The door's set `a`: sample of the door core at axis 7 of the arm subject.
fn door_set(subject: Noun, space: &NounSpace) -> JetResult<Noun> {
    let door = slot(subject, 7, space)?;
    slot(door, 6, space)
}

// ---------------------------------------------------------------------------
// core treap operations (mirror hoon.hoon `+in`, comparator = stock gor/mor)
// ---------------------------------------------------------------------------

/// `+has:in`: membership.
fn set_has(stack: &mut NockStack, tree: Noun, elem: Noun, space: &NounSpace) -> JetResult<bool> {
    let mut node = tree;
    loop {
        if node.is_atom() {
            return Ok(false);
        }
        let (n, left, right) = decompose(node, space)?;
        if noun_eq(stack, elem, n) {
            return Ok(true);
        }
        node = if gor_lt(stack, elem, n, space) {
            left
        } else {
            right
        };
    }
}

/// `+put:in`: insert, rebalancing to keep the treap canonical.
///
/// `pub(crate)` so `+key:by` (`map.rs`) can build its key set with the same put.
pub(crate) fn set_put(
    stack: &mut NockStack,
    tree: Noun,
    elem: Noun,
    space: &NounSpace,
) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(make_node(stack, elem, D(0), D(0)));
    }
    let (n, left, right) = decompose(tree, space)?;
    if noun_eq(stack, elem, n) {
        return Ok(tree);
    }
    if gor_lt(stack, elem, n, space) {
        let child = set_put(stack, left, elem, space)?;
        let (child_n, child_left, child_right) = decompose(child, space)?;
        if mor_lt(stack, n, child_n, space) {
            Ok(make_node(stack, n, child, right))
        } else {
            let demoted = make_node(stack, n, child_right, right);
            Ok(make_node(stack, child_n, child_left, demoted))
        }
    } else {
        let child = set_put(stack, right, elem, space)?;
        let (child_n, child_left, child_right) = decompose(child, space)?;
        if mor_lt(stack, n, child_n, space) {
            Ok(make_node(stack, n, left, child))
        } else {
            let demoted = make_node(stack, n, left, child_left);
            Ok(make_node(stack, child_n, demoted, child_right))
        }
    }
}

/// `+del:in`: remove, merging the subtrees with `set_join`.
fn set_del(stack: &mut NockStack, tree: Noun, elem: Noun, space: &NounSpace) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(D(0));
    }
    let (n, left, right) = decompose(tree, space)?;
    if noun_eq(stack, elem, n) {
        return set_join(stack, left, right, space);
    }
    if gor_lt(stack, elem, n, space) {
        let child = set_del(stack, left, elem, space)?;
        Ok(make_node(stack, n, child, right))
    } else {
        let child = set_del(stack, right, elem, space)?;
        Ok(make_node(stack, n, left, child))
    }
}

/// Merge two disjoint, gor-ordered subtrees preserving heap order (inner loop
/// of `+del:in`).
fn set_join(stack: &mut NockStack, left: Noun, right: Noun, space: &NounSpace) -> JetResult<Noun> {
    if left.is_atom() {
        return Ok(right);
    }
    if right.is_atom() {
        return Ok(left);
    }
    let (left_n, left_left, left_right) = decompose(left, space)?;
    let (right_n, right_left, right_right) = decompose(right, space)?;
    if mor_lt(stack, left_n, right_n, space) {
        let joined = set_join(stack, left_right, right, space)?;
        Ok(make_node(stack, left_n, left_left, joined))
    } else {
        let joined = set_join(stack, left, right_left, space)?;
        Ok(make_node(stack, right_n, joined, right_right))
    }
}

/// `+bif:in`: split a set into `(before, after)` around `elem`, dropping `elem`.
fn set_bif(
    stack: &mut NockStack,
    tree: Noun,
    elem: Noun,
    space: &NounSpace,
) -> JetResult<(Noun, Noun)> {
    if tree.is_atom() {
        return Ok((D(0), D(0)));
    }
    let (n, left, right) = decompose(tree, space)?;
    if noun_eq(stack, elem, n) {
        return Ok((left, right));
    }
    if gor_lt(stack, elem, n, space) {
        let (below, above) = set_bif(stack, left, elem, space)?;
        Ok((below, make_node(stack, n, above, right)))
    } else {
        let (below, above) = set_bif(stack, right, elem, space)?;
        Ok((make_node(stack, n, left, below), above))
    }
}

/// `+uni:in`: union.
pub(crate) fn set_uni(stack: &mut NockStack, a: Noun, b: Noun, space: &NounSpace) -> JetResult<Noun> {
    if noun_eq(stack, a, b) {
        return Ok(a);
    }
    if b.is_atom() {
        return Ok(a);
    }
    if a.is_atom() {
        return Ok(b);
    }
    let (a_n, a_left, a_right) = decompose(a, space)?;
    let (b_n, b_left, b_right) = decompose(b, space)?;

    if noun_eq(stack, a_n, b_n) {
        let left = set_uni(stack, a_left, b_left, space)?;
        let right = set_uni(stack, a_right, b_right, space)?;
        return Ok(make_node(stack, a_n, left, right));
    }

    if mor_lt(stack, a_n, b_n, space) {
        if gor_lt(stack, b_n, a_n, space) {
            let b_mod = make_node(stack, b_n, b_left, D(0));
            let inner = set_uni(stack, a_left, b_mod, space)?;
            let a_mod = make_node(stack, a_n, inner, a_right);
            set_uni(stack, a_mod, b_right, space)
        } else {
            let b_mod = make_node(stack, b_n, D(0), b_right);
            let inner = set_uni(stack, a_right, b_mod, space)?;
            let a_mod = make_node(stack, a_n, a_left, inner);
            set_uni(stack, a_mod, b_left, space)
        }
    } else if gor_lt(stack, a_n, b_n, space) {
        let a_mod = make_node(stack, a_n, a_left, D(0));
        let inner = set_uni(stack, a_mod, b_left, space)?;
        let b_mod = make_node(stack, b_n, inner, b_right);
        set_uni(stack, a_right, b_mod, space)
    } else {
        let a_mod = make_node(stack, a_n, D(0), a_right);
        let inner = set_uni(stack, a_mod, b_right, space)?;
        let b_mod = make_node(stack, b_n, b_left, inner);
        set_uni(stack, a_left, b_mod, space)
    }
}

/// `+int:in`: intersection.
fn set_int(stack: &mut NockStack, a: Noun, b: Noun, space: &NounSpace) -> JetResult<Noun> {
    if a.is_atom() || b.is_atom() {
        return Ok(D(0));
    }
    let (a_n, a_left, a_right) = decompose(a, space)?;
    let (b_n, b_left, b_right) = decompose(b, space)?;

    if mor_lt(stack, a_n, b_n, space) {
        if noun_eq(stack, b_n, a_n) {
            let left = set_int(stack, a_left, b_left, space)?;
            let right = set_int(stack, a_right, b_right, space)?;
            Ok(make_node(stack, b_n, left, right))
        } else if gor_lt(stack, b_n, a_n, space) {
            let b_mod = make_node(stack, b_n, b_left, D(0));
            let x = set_int(stack, a_left, b_mod, space)?;
            let y = set_int(stack, a, b_right, space)?;
            set_uni(stack, x, y, space)
        } else {
            let b_mod = make_node(stack, b_n, D(0), b_right);
            let x = set_int(stack, a_right, b_mod, space)?;
            let y = set_int(stack, a, b_left, space)?;
            set_uni(stack, x, y, space)
        }
    } else if noun_eq(stack, a_n, b_n) {
        let left = set_int(stack, a_left, b_left, space)?;
        let right = set_int(stack, a_right, b_right, space)?;
        Ok(make_node(stack, b_n, left, right))
    } else if gor_lt(stack, a_n, b_n, space) {
        let a_mod = make_node(stack, a_n, a_left, D(0));
        let x = set_int(stack, a_mod, b_left, space)?;
        let y = set_int(stack, a_right, b, space)?;
        set_uni(stack, x, y, space)
    } else {
        let a_mod = make_node(stack, a_n, D(0), a_right);
        let x = set_int(stack, a_mod, b_right, space)?;
        let y = set_int(stack, a_left, b, space)?;
        set_uni(stack, x, y, space)
    }
}

/// `+dif:in`: difference — elements of `a` not in `b`.
fn set_dif(stack: &mut NockStack, a: Noun, b: Noun, space: &NounSpace) -> JetResult<Noun> {
    if b.is_atom() {
        return Ok(a);
    }
    let (b_n, b_left, b_right) = decompose(b, space)?;
    let (below, above) = set_bif(stack, a, b_n, space)?;
    let left = set_dif(stack, below, b_left, space)?;
    let right = set_dif(stack, above, b_right, space)?;
    set_join(stack, left, right, space)
}

/// `+gas:in`: fold `+put` over a `(list)`, left to right.
fn set_gas(stack: &mut NockStack, tree: Noun, list: Noun, space: &NounSpace) -> JetResult<Noun> {
    let mut acc = tree;
    let mut rest = list;
    loop {
        if rest.is_atom() {
            return Ok(acc);
        }
        let item = slot(rest, 2, space)?;
        let tail = slot(rest, 3, space)?;
        acc = set_put(stack, acc, item, space)?;
        rest = tail;
    }
}

/// `+tap:in`: elements as a `(list)`, reverse-in-order (as Hoon's `+tap`).
fn set_tap(stack: &mut NockStack, tree: Noun, acc: Noun, space: &NounSpace) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(acc);
    }
    let (n, left, right) = decompose(tree, space)?;
    let inner = set_tap(stack, left, acc, space)?;
    let consed = T(stack, &[n, inner]);
    set_tap(stack, right, consed, space)
}

/// `+wyt:in`: the element count.
fn set_wyt(tree: Noun, space: &NounSpace) -> JetResult<u64> {
    if tree.is_atom() {
        return Ok(0);
    }
    let (_n, left, right) = decompose(tree, space)?;
    Ok(1 + set_wyt(left, space)? + set_wyt(right, space)?)
}

/// `+apt:in`: is `a` a well-formed set treap? (ports Vere's `_in_apt`; same as
/// the map check but over bare elements.)
fn set_apt(
    stack: &mut NockStack,
    tree: Noun,
    upper: Option<Noun>,
    lower: Option<Noun>,
    space: &NounSpace,
) -> JetResult<bool> {
    if tree.is_atom() {
        return Ok(true);
    }
    let (n, left, right) = decompose(tree, space)?;
    if let Some(u) = upper {
        if !gor_lt(stack, n, u, space) || noun_eq(stack, n, u) {
            return Ok(false);
        }
    }
    if let Some(l) = lower {
        if !gor_lt(stack, l, n, space) || noun_eq(stack, l, n) {
            return Ok(false);
        }
    }
    if !left.is_atom() {
        let (ln, _, _) = decompose(left, space)?;
        if !mor_lt(stack, n, ln, space) || noun_eq(stack, n, ln) {
            return Ok(false);
        }
        if !set_apt(stack, left, Some(n), lower, space)? {
            return Ok(false);
        }
    }
    if !right.is_atom() {
        let (rn, _, _) = decompose(right, space)?;
        if !mor_lt(stack, n, rn, space) || noun_eq(stack, n, rn) {
            return Ok(false);
        }
        if !set_apt(stack, right, upper, Some(n), space)? {
            return Ok(false);
        }
    }
    Ok(true)
}

// ---------------------------------------------------------------------------
// gate-slamming operations (mirror Vere's `_in_run`/`_in_all`/`_in_any`).
// ---------------------------------------------------------------------------

/// `+run:in`: map a gate over elements into a NEW set. Unlike `+by`'s `run`, the
/// result is rebuilt with `set_put` because a transformed element can land in a
/// different place in the ordering.
fn set_run(
    context: &mut Context,
    tree: Noun,
    gate: Noun,
    acc: Noun,
    space: &NounSpace,
) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(acc);
    }
    let (n, left, right) = decompose(tree, space)?;
    let mapped = slam(context, gate, n)?;
    let acc = set_put(&mut context.stack, acc, mapped, space)?;
    let acc = set_run(context, left, gate, acc, space)?;
    set_run(context, right, gate, acc, space)
}

/// `+all:in`: do all elements satisfy the gate? `%.y` on empty; punts on a
/// non-loobean result.
fn set_all(context: &mut Context, tree: Noun, gate: Noun, space: &NounSpace) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(YES);
    }
    let (n, left, right) = decompose(tree, space)?;
    let r = slam(context, gate, n)?;
    if is_yes(r) {
        let lr = set_all(context, left, gate, space)?;
        if !is_yes(lr) {
            return Ok(lr);
        }
        set_all(context, right, gate, space)
    } else if unsafe { r.raw_equals(&NO) } {
        Ok(NO)
    } else {
        Err(JetErr::Punt)
    }
}

/// `+any:in`: does any element satisfy the gate? `%.n` on empty; punts on a
/// non-loobean result.
fn set_any(context: &mut Context, tree: Noun, gate: Noun, space: &NounSpace) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(NO);
    }
    let (n, left, right) = decompose(tree, space)?;
    let r = slam(context, gate, n)?;
    if is_yes(r) {
        Ok(YES)
    } else if unsafe { r.raw_equals(&NO) } {
        let lr = set_any(context, left, gate, space)?;
        if is_yes(lr) {
            return Ok(YES);
        }
        set_any(context, right, gate, space)
    } else {
        Err(JetErr::Punt)
    }
}

// ---------------------------------------------------------------------------
// jets (the door arms); sample at axis 6, door core (whose sample is the set) at 7.
// The no-sample trap arms `tap`/`apt` read `a` via `door_set` (axis 7->6 = 30).
// `wyt` is the exception: a bare `~% %wyt + ~ |.` trap at parent-axis 3, so its
// `a` is at `u3x_con_2` (axis 14) — see `jet_in_wyt`.
// ---------------------------------------------------------------------------

/// `~(has in a)`: `%.y` iff the element is present.
pub fn jet_in_has(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let elem = slot(subject, 6, &space)?;
    let set = door_set(subject, &space)?;
    Ok(if set_has(&mut context.stack, set, elem, &space)? {
        YES
    } else {
        NO
    })
}

/// `~(put in a)`: insert an element.
pub fn jet_in_put(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let elem = slot(subject, 6, &space)?;
    let set = door_set(subject, &space)?;
    set_put(&mut context.stack, set, elem, &space)
}

/// `~(del in a)`: remove an element.
pub fn jet_in_del(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let elem = slot(subject, 6, &space)?;
    let set = door_set(subject, &space)?;
    set_del(&mut context.stack, set, elem, &space)
}

/// `~(gas in a)`: bulk insert from a `(list)`.
pub fn jet_in_gas(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let list = slot(subject, 6, &space)?;
    let set = door_set(subject, &space)?;
    set_gas(&mut context.stack, set, list, &space)
}

/// `~(uni in a)`: union with another set `b`. Sample is `b`.
pub fn jet_in_uni(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let b = slot(subject, 6, &space)?;
    let a = door_set(subject, &space)?;
    set_uni(&mut context.stack, a, b, &space)
}

/// `~(int in a)`: intersection with another set `b`. Sample is `b`.
pub fn jet_in_int(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let b = slot(subject, 6, &space)?;
    let a = door_set(subject, &space)?;
    set_int(&mut context.stack, a, b, &space)
}

/// `~(dif in a)`: a minus b's elements. Sample is `b`.
pub fn jet_in_dif(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let b = slot(subject, 6, &space)?;
    let a = door_set(subject, &space)?;
    set_dif(&mut context.stack, a, b, &space)
}

/// `~(tap in a)`: elements as a `(list)`.
pub fn jet_in_tap(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let a = door_set(subject, &space)?;
    set_tap(&mut context.stack, a, D(0), &space)
}

/// `~(wyt in a)`: the element count.
pub fn jet_in_wyt(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    // `+wyt:in` is a bare trap (`~%  %wyt  +  ~  |.`): `a` is at `u3x_con_2`
    // (axis 14), NOT `u3x_con_sam` (axis 30 = `door_set`). Vere registers `wyt`
    // at parent-axis 3; every other `+in` arm is axis 7. See map.rs `jet_by_wyt`.
    let a = slot(subject, 14, &space)?;
    Ok(D(set_wyt(a, &space)?))
}

/// `~(apt in a)`: is `a` a well-formed set treap?
pub fn jet_in_apt(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let a = door_set(subject, &space)?;
    Ok(if set_apt(&mut context.stack, a, None, None, &space)? {
        YES
    } else {
        NO
    })
}

/// `~(bif in a)`: split `a` around an element into `[l=(set) r=(set)]`. Sample
/// is the element.
pub fn jet_in_bif(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let elem = slot(subject, 6, &space)?;
    let a = door_set(subject, &space)?;
    let (below, above) = set_bif(&mut context.stack, a, elem, &space)?;
    Ok(T(&mut context.stack, &[below, above]))
}

/// `~(run in a)`: map a gate over elements into a new set. Sample is the gate.
pub fn jet_in_run(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let gate = slot(subject, 6, &space)?;
    let a = door_set(subject, &space)?;
    set_run(context, a, gate, D(0), &space)
}

/// `~(all in a)`: do all elements satisfy the gate? Sample is the gate.
pub fn jet_in_all(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let gate = slot(subject, 6, &space)?;
    let a = door_set(subject, &space)?;
    set_all(context, a, gate, &space)
}

/// `~(any in a)`: does any element satisfy the gate? Sample is the gate.
pub fn jet_in_any(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let gate = slot(subject, 6, &space)?;
    let a = door_set(subject, &space)?;
    set_any(context, a, gate, &space)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nockvm::jets::util::test::{assert_jet_door, init_context};
    use nockvm::noun::D;
    use std::collections::BTreeSet;

    fn direct(noun: Noun) -> u64 {
        noun.as_direct().expect("direct atom").data()
    }

    /// In-order elements; asserts the treap is canonical (gor-ordered elements,
    /// mor-ordered priorities).
    fn verify(stack: &mut NockStack, tree: Noun, space: &NounSpace, out: &mut Vec<u64>) {
        if tree.is_atom() {
            assert!(unsafe { tree.raw_equals(&D(0)) }, "empty set must be ~");
            return;
        }
        let (n, left, right) = decompose(tree, space).expect("node");
        verify(stack, left, space, out);
        out.push(direct(n));
        verify(stack, right, space, out);

        if !left.is_atom() {
            let (ln, _, _) = decompose(left, space).unwrap();
            assert!(gor_lt(stack, ln, n, space), "left must gor-precede parent");
            assert!(mor_lt(stack, n, ln, space), "parent must mor-outrank left child");
        }
        if !right.is_atom() {
            let (rn, _, _) = decompose(right, space).unwrap();
            assert!(gor_lt(stack, n, rn, space), "parent must gor-precede right");
            assert!(mor_lt(stack, n, rn, space), "parent must mor-outrank right child");
        }
    }

    fn assert_matches(stack: &mut NockStack, tree: Noun, space: &NounSpace, model: &BTreeSet<u64>) {
        let mut got = Vec::new();
        verify(stack, tree, space, &mut got);
        got.sort_unstable();
        let want: Vec<u64> = model.iter().copied().collect();
        assert_eq!(got, want, "jet tree elements must equal the model");
    }

    fn from_model(stack: &mut NockStack, space: &NounSpace, model: &BTreeSet<u64>) -> Noun {
        let mut tree = D(0);
        for &e in model {
            tree = set_put(stack, tree, D(e), space).expect("put");
        }
        tree
    }

    fn door(stack: &mut NockStack, set: Noun) -> Noun {
        T(stack, &[D(0), set, D(0)])
    }

    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *seed >> 33
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn has_and_put_small() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let model: BTreeSet<u64> = [5u64, 3, 11, 7].into_iter().collect();
        let set = from_model(&mut context.stack, &space, &model);
        let pay = door(&mut context.stack, set);

        assert_jet_door(context, jet_in_has, D(7), pay, YES);
        assert_jet_door(context, jet_in_has, D(99), pay, NO);

        // dedup: re-putting an element leaves the set unchanged. At the root the
        // treap returns the identical tree (deeper, Hoon rebuilds the path with a
        // structurally-equal result — covered by the differential tests).
        let single = set_put(&mut context.stack, D(0), D(5), &space).unwrap();
        let again = set_put(&mut context.stack, single, D(5), &space).unwrap();
        assert!(
            unsafe { again.raw_equals(&single) },
            "duplicate put at root must be a no-op"
        );
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn put_matches_model_random() {
        let mut seed = 0x11CE5E7u64;
        for _ in 0..24 {
            let context = &mut init_context();
            let space = context.stack.noun_space();
            let mut tree = D(0);
            let mut model = BTreeSet::new();
            for _ in 0..120 {
                let e = lcg(&mut seed) % 400;
                tree = set_put(&mut context.stack, tree, D(e), &space).unwrap();
                model.insert(e);
            }
            assert_matches(&mut context.stack, tree, &space, &model);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn del_matches_model_random() {
        let mut seed = 0xDE1E7Eu64;
        for _ in 0..24 {
            let context = &mut init_context();
            let space = context.stack.noun_space();
            let mut tree = D(0);
            let mut model = BTreeSet::new();
            for _ in 0..100 {
                let e = lcg(&mut seed) % 200;
                tree = set_put(&mut context.stack, tree, D(e), &space).unwrap();
                model.insert(e);
            }
            for _ in 0..120 {
                let e = lcg(&mut seed) % 200;
                tree = set_del(&mut context.stack, tree, D(e), &space).unwrap();
                model.remove(&e);
                assert_matches(&mut context.stack, tree, &space, &model);
            }
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn gas_matches_puts() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let elems = [5u64, 3, 5, 9, 1, 3];
        let mut list = D(0);
        for &e in elems.iter().rev() {
            list = T(&mut context.stack, &[D(e), list]);
        }
        let gassed = set_gas(&mut context.stack, D(0), list, &space).unwrap();
        let model: BTreeSet<u64> = elems.into_iter().collect();
        assert_matches(&mut context.stack, gassed, &space, &model);
    }

    fn two_sets(
        stack: &mut NockStack,
        space: &NounSpace,
        seed: &mut u64,
    ) -> (Noun, Noun, BTreeSet<u64>, BTreeSet<u64>) {
        let mut sa = BTreeSet::new();
        let mut sb = BTreeSet::new();
        for _ in 0..60 {
            sa.insert(lcg(seed) % 150);
            sb.insert(lcg(seed) % 150);
        }
        let a = from_model(stack, space, &sa);
        let b = from_model(stack, space, &sb);
        (a, b, sa, sb)
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn uni_int_dif_match_model() {
        let mut seed = 0x5E70_9500u64;
        for _ in 0..16 {
            let context = &mut init_context();
            let space = context.stack.noun_space();
            let (a, b, sa, sb) = two_sets(&mut context.stack, &space, &mut seed);

            let uni: BTreeSet<u64> = sa.union(&sb).copied().collect();
            let r = set_uni(&mut context.stack, a, b, &space).unwrap();
            assert_matches(&mut context.stack, r, &space, &uni);

            let int: BTreeSet<u64> = sa.intersection(&sb).copied().collect();
            let r = set_int(&mut context.stack, a, b, &space).unwrap();
            assert_matches(&mut context.stack, r, &space, &int);

            let dif: BTreeSet<u64> = sa.difference(&sb).copied().collect();
            let r = set_dif(&mut context.stack, a, b, &space).unwrap();
            assert_matches(&mut context.stack, r, &space, &dif);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn del_and_uni_via_door() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let model: BTreeSet<u64> = [5u64, 3, 9].into_iter().collect();
        let set = from_model(&mut context.stack, &space, &model);
        let pay = door(&mut context.stack, set);

        let want = set_del(&mut context.stack, set, D(3), &space).unwrap();
        assert_jet_door(context, jet_in_del, D(3), pay, want);

        let other: BTreeSet<u64> = [9u64, 12].into_iter().collect();
        let b = from_model(&mut context.stack, &space, &other);
        let want = set_uni(&mut context.stack, set, b, &space).unwrap();
        assert_jet_door(context, jet_in_uni, b, pay, want);
    }

    // --- pure tail arms ------------------------------------------------------

    fn list_elems(tree: Noun, space: &NounSpace) -> Vec<u64> {
        let mut out = Vec::new();
        let mut cur = tree;
        while !cur.is_atom() {
            out.push(direct(slot(cur, 2, space).unwrap()));
            cur = slot(cur, 3, space).unwrap();
        }
        out
    }

    fn in_order(tree: Noun, space: &NounSpace, out: &mut Vec<u64>) {
        if tree.is_atom() {
            return;
        }
        let (n, left, right) = decompose(tree, space).unwrap();
        in_order(left, space, out);
        out.push(direct(n));
        in_order(right, space, out);
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn tap_wyt_apt() {
        let mut seed = 0x5E7A9u64;
        for _ in 0..12 {
            let context = &mut init_context();
            let space = context.stack.noun_space();
            let mut model = BTreeSet::new();
            for _ in 0..50 {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                model.insert((seed >> 40) % 400);
            }
            let set = from_model(&mut context.stack, &space, &model);

            // wyt (helper)
            assert_eq!(set_wyt(set, &space).unwrap(), model.len() as u64);
            // wyt end-to-end: `+wyt:in` is a bare trap, so `a` is at axis 14
            // (u3x_con_2), not `door_set` (axis 30). Build [* * a *] and check.
            let subj = T(&mut context.stack, &[D(0), D(0), set, D(0)]);
            let got_wyt = jet_in_wyt(context, subj).unwrap();
            assert_eq!(direct(got_wyt), model.len() as u64);
            // apt
            assert!(set_apt(&mut context.stack, set, None, None, &space).unwrap());
            // tap == reverse-in-order, and holds exactly the model.
            let list = set_tap(&mut context.stack, set, D(0), &space).unwrap();
            let got = list_elems(list, &space);
            let mut io = Vec::new();
            in_order(set, &space, &mut io);
            io.reverse();
            assert_eq!(got, io, "tap must be reverse-in-order");
            let mut sorted = got.clone();
            sorted.sort_unstable();
            let want: Vec<u64> = model.iter().copied().collect();
            assert_eq!(sorted, want);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn apt_rejects_broken_and_bif_splits() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        // broken: child on the gor-wrong side of its parent.
        let child = T(&mut context.stack, &[D(2), D(0), D(0)]);
        let broken = if gor_lt(&mut context.stack, D(2), D(1), &space) {
            T(&mut context.stack, &[D(1), D(0), child])
        } else {
            T(&mut context.stack, &[D(1), child, D(0)])
        };
        assert!(!set_apt(&mut context.stack, broken, None, None, &space).unwrap());

        // bif around a pivot: below gor-precedes, above follows.
        let model: BTreeSet<u64> = (1..40u64).collect();
        let set = from_model(&mut context.stack, &space, &model);
        let pivot = 23u64;
        let (below, above) = set_bif(&mut context.stack, set, D(pivot), &space).unwrap();
        for (tree, is_below) in [(below, true), (above, false)] {
            let mut io = Vec::new();
            in_order(tree, &space, &mut io);
            for e in io {
                assert_eq!(gor_lt(&mut context.stack, D(e), D(pivot), &space), is_below);
            }
        }
    }

    // --- gate-slamming tail arms --------------------------------------------

    fn gate(stack: &mut NockStack, battery: Noun) -> Noun {
        T(stack, &[battery, D(0), D(0)])
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn run_increments_elements() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let model: BTreeSet<u64> = [1u64, 2, 3, 10, 11].into_iter().collect();
        let set = from_model(&mut context.stack, &space, &model);
        let b = T(&mut context.stack, &[D(4), D(0), D(6)]); // +(sample)
        let g = gate(&mut context.stack, b);
        let res = set_run(context, set, g, D(0), &space).unwrap();
        let want: BTreeSet<u64> = model.iter().map(|&e| e + 1).collect();
        assert_matches(&mut context.stack, res, &space, &want);
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn all_any_over_elements() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let is_zero = |stack: &mut NockStack| {
            let s = T(stack, &[D(0), D(6)]);
            let z = T(stack, &[D(1), D(0)]);
            let b = T(stack, &[D(5), s, z]);
            gate(stack, b)
        };
        let zeros: BTreeSet<u64> = [0u64].into_iter().collect();
        let mixed: BTreeSet<u64> = [0u64, 1, 2].into_iter().collect();
        let s0 = from_model(&mut context.stack, &space, &zeros);
        let sm = from_model(&mut context.stack, &space, &mixed);

        let g = is_zero(&mut context.stack);
        assert!(is_yes(set_all(context, s0, g, &space).unwrap()));
        let g = is_zero(&mut context.stack);
        assert!(!is_yes(set_all(context, sm, g, &space).unwrap()));
        let g = is_zero(&mut context.stack);
        assert!(is_yes(set_any(context, sm, g, &space).unwrap()));
        let g = is_zero(&mut context.stack);
        assert!(!is_yes(set_any(context, D(0), g, &space).unwrap()));
    }
}
