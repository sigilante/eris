// `+by` map-door jets for Eris.
//
// nockvm's stock dashboard (`URBIT_HOT_STATE`) leaves the standard Hoon `+by`
// door unjetted. Nockchain's own kernels do not use it: they carry a
// digest-ordered `z-by`/`h-by` map instead, so the map jets in `zkvm-jetpack`
// order their treaps by a tip5 digest (`+mor-tip`/`+gor-hip`) over a node that
// caches that digest. Those cannot drive a stock k138 pill, whose maps are the
// ordinary `[[key value] left right]` treap ordered by the `mug`-based `+gor`
// and `+mor`. Eris runs that stock pill, so it needs the standard door, and the
// standard door is where a boot spends most of its time (clay, gall, and the
// compiler's environments are all maps).
//
// These jets are the `+by` arms rebuilt against the stock node shape and the
// stock comparators (`+gor`/`+mor`, from `nockvm::jets::sort::util`). The
// algorithms follow hoon.hoon's `+by`, cross-checked against Vere's
// `pkg/noun/jets/.../by_*.c`; the only change from `zkvm-jetpack`'s versions is
// the comparator and the node shape. Equality is the unifying structural test
// (NOT pointer equality — the stock `set.rs` dedup bug is exactly a `raw_equals`
// where it needed this).
//
// They are registered from the serf (see `super::eris_hot_state`) so nockvm's
// own dashboard stays unpatched. Pinned to Hoon kelvin 138.
//
// Covers the full Vere-k138 `+by` jet set plus `got`/`gut`/`mar`. Deferred:
// `+rep`/`+rib` (gate folds whose accumulator lives at a gate-internal axis;
// Vere ships no dedicated jet for them either) — they run unjetted, correctly.

use nockvm::interpreter::Context;
use nockvm::jets::sort::util::{gor, mor};
use nockvm::jets::util::{slam, slot};
use nockvm::jets::{JetErr, Result};
use nockvm::mem::NockStack;
use nockvm::noun::{Noun, NounSpace, D, NO, T, YES};
use nockvm::unifying_equality::unifying_equality;

type JetResult<T> = std::result::Result<T, JetErr>;



// ---------------------------------------------------------------------------
// node plumbing
//
// A map is a treap `~` | `[n=[key value] l=map r=map]`. `~` is the atom 0; every
// non-empty node is a cell. Axes: n=2, l=6, r=7 within a node; key=2, value=3
// within the entry `n`.
// ---------------------------------------------------------------------------

#[inline(always)]
fn is_yes(noun: Noun) -> bool {
    unsafe { noun.raw_equals(&YES) }
}

/// Split a node into `(entry, left, right)`; entry is the `[key value]` cell.
fn decompose(node: Noun, space: &NounSpace) -> JetResult<(Noun, Noun, Noun)> {
    Ok((slot(node, 2, space)?, slot(node, 6, space)?, slot(node, 7, space)?))
}

/// Split an entry into `(key, value)`.
fn entry_kv(entry: Noun, space: &NounSpace) -> JetResult<(Noun, Noun)> {
    Ok((slot(entry, 2, space)?, slot(entry, 3, space)?))
}

fn make_entry(stack: &mut NockStack, key: Noun, value: Noun) -> Noun {
    T(stack, &[key, value])
}

/// Build `[entry left right]`.
fn make_node(stack: &mut NockStack, entry: Noun, left: Noun, right: Noun) -> Noun {
    T(stack, &[entry, left, right])
}

/// `+gor`: does `a` sort strictly before `b` (by `mug`, then `+dor`)?
#[inline(always)]
fn gor_lt(stack: &mut NockStack, a: Noun, b: Noun, space: &NounSpace) -> bool {
    is_yes(gor(stack, a, b, space))
}

/// `+mor`: does `a` outrank `b` (by double-`mug`, then `+dor`)? This is the
/// treap's heap priority.
#[inline(always)]
fn mor_lt(stack: &mut NockStack, a: Noun, b: Noun, space: &NounSpace) -> bool {
    is_yes(mor(stack, a, b, space))
}

/// Unifying structural equality, the stock noun `=`. Mirrors `cold.rs`'s
/// `noun_unifying_equality`.
fn noun_eq(stack: &mut NockStack, a: Noun, b: Noun) -> bool {
    let mut a = a;
    let mut b = b;
    unsafe { unifying_equality(stack, &mut a as *mut Noun, &mut b as *mut Noun) }
}

/// The door's map `a`: sample of the door core at axis 7 of the arm subject.
fn door_map(subject: Noun, space: &NounSpace) -> JetResult<Noun> {
    let door = slot(subject, 7, space)?;
    slot(door, 6, space)
}

// ---------------------------------------------------------------------------
// core treap operations (mirror hoon.hoon `+by`, comparator = stock gor/mor)
// ---------------------------------------------------------------------------

/// `+get:by` as a `(unit value)`: `~` if absent, else `[~ value]`.
fn map_get_unit(stack: &mut NockStack, tree: Noun, key: Noun, space: &NounSpace) -> JetResult<Noun> {
    let mut node = tree;
    loop {
        if node.is_atom() {
            return Ok(D(0));
        }
        let (entry, left, right) = decompose(node, space)?;
        let (node_key, node_value) = entry_kv(entry, space)?;
        if noun_eq(stack, key, node_key) {
            return Ok(T(stack, &[D(0), node_value]));
        }
        node = if gor_lt(stack, key, node_key, space) {
            left
        } else {
            right
        };
    }
}

/// `+put:by`: insert or replace, rebalancing to keep the treap canonical.
fn map_put(
    stack: &mut NockStack,
    tree: Noun,
    key: Noun,
    value: Noun,
    space: &NounSpace,
) -> JetResult<Noun> {
    if tree.is_atom() {
        let entry = make_entry(stack, key, value);
        return Ok(make_node(stack, entry, D(0), D(0)));
    }

    let (entry, left, right) = decompose(tree, space)?;
    let (node_key, node_value) = entry_kv(entry, space)?;

    if noun_eq(stack, key, node_key) {
        if noun_eq(stack, value, node_value) {
            return Ok(tree);
        }
        let entry = make_entry(stack, key, value);
        return Ok(make_node(stack, entry, left, right));
    }

    if gor_lt(stack, key, node_key, space) {
        let child = map_put(stack, left, key, value, space)?;
        let (child_entry, child_left, child_right) = decompose(child, space)?;
        let (child_key, _) = entry_kv(child_entry, space)?;
        if mor_lt(stack, node_key, child_key, space) {
            Ok(make_node(stack, entry, child, right))
        } else {
            let demoted = make_node(stack, entry, child_right, right);
            Ok(make_node(stack, child_entry, child_left, demoted))
        }
    } else {
        let child = map_put(stack, right, key, value, space)?;
        let (child_entry, child_left, child_right) = decompose(child, space)?;
        let (child_key, _) = entry_kv(child_entry, space)?;
        if mor_lt(stack, node_key, child_key, space) {
            Ok(make_node(stack, entry, left, child))
        } else {
            let demoted = make_node(stack, entry, left, child_left);
            Ok(make_node(stack, child_entry, demoted, child_right))
        }
    }
}

/// `+del:by`: remove a key, merging its subtrees with `map_join`.
fn map_del(stack: &mut NockStack, tree: Noun, key: Noun, space: &NounSpace) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(D(0));
    }
    let (entry, left, right) = decompose(tree, space)?;
    let (node_key, _) = entry_kv(entry, space)?;
    if noun_eq(stack, key, node_key) {
        return map_join(stack, left, right, space);
    }
    if gor_lt(stack, key, node_key, space) {
        let child = map_del(stack, left, key, space)?;
        Ok(make_node(stack, entry, child, right))
    } else {
        let child = map_del(stack, right, key, space)?;
        Ok(make_node(stack, entry, left, child))
    }
}

/// Merge two disjoint, gor-ordered subtrees, preserving the heap order. This is
/// the inner loop of `+del:by`.
fn map_join(stack: &mut NockStack, left: Noun, right: Noun, space: &NounSpace) -> JetResult<Noun> {
    if left.is_atom() {
        return Ok(right);
    }
    if right.is_atom() {
        return Ok(left);
    }
    let (left_entry, left_left, left_right) = decompose(left, space)?;
    let (left_key, _) = entry_kv(left_entry, space)?;
    let (right_entry, right_left, right_right) = decompose(right, space)?;
    let (right_key, _) = entry_kv(right_entry, space)?;
    if mor_lt(stack, left_key, right_key, space) {
        let joined = map_join(stack, left_right, right, space)?;
        Ok(make_node(stack, left_entry, left_left, joined))
    } else {
        let joined = map_join(stack, left, right_left, space)?;
        Ok(make_node(stack, right_entry, joined, right_right))
    }
}

/// `+bif:by`: split a map into `(before, after)` around `key`, dropping `key`.
fn map_bif(
    stack: &mut NockStack,
    tree: Noun,
    key: Noun,
    space: &NounSpace,
) -> JetResult<(Noun, Noun)> {
    if tree.is_atom() {
        return Ok((D(0), D(0)));
    }
    let (entry, left, right) = decompose(tree, space)?;
    let (node_key, _) = entry_kv(entry, space)?;
    if noun_eq(stack, key, node_key) {
        return Ok((left, right));
    }
    if gor_lt(stack, key, node_key, space) {
        let (below, above) = map_bif(stack, left, key, space)?;
        Ok((below, make_node(stack, entry, above, right)))
    } else {
        let (below, above) = map_bif(stack, right, key, space)?;
        Ok((make_node(stack, entry, left, below), above))
    }
}

/// `+uni:by`: left-biased union — on a shared key, `b`'s value wins.
fn map_uni(stack: &mut NockStack, a: Noun, b: Noun, space: &NounSpace) -> JetResult<Noun> {
    if b.is_atom() {
        return Ok(a);
    }
    if a.is_atom() {
        return Ok(b);
    }
    let (a_entry, a_left, a_right) = decompose(a, space)?;
    let (a_key, _) = entry_kv(a_entry, space)?;
    let (b_entry, b_left, b_right) = decompose(b, space)?;
    let (b_key, _) = entry_kv(b_entry, space)?;

    if noun_eq(stack, b_key, a_key) {
        let left = map_uni(stack, a_left, b_left, space)?;
        let right = map_uni(stack, a_right, b_right, space)?;
        return Ok(make_node(stack, b_entry, left, right));
    }

    if mor_lt(stack, a_key, b_key, space) {
        if gor_lt(stack, b_key, a_key, space) {
            let b_mod = make_node(stack, b_entry, b_left, D(0));
            let inner = map_uni(stack, a_left, b_mod, space)?;
            let a_mod = make_node(stack, a_entry, inner, a_right);
            map_uni(stack, a_mod, b_right, space)
        } else {
            let b_mod = make_node(stack, b_entry, D(0), b_right);
            let inner = map_uni(stack, a_right, b_mod, space)?;
            let a_mod = make_node(stack, a_entry, a_left, inner);
            map_uni(stack, a_mod, b_left, space)
        }
    } else if gor_lt(stack, a_key, b_key, space) {
        let a_mod = make_node(stack, a_entry, a_left, D(0));
        let inner = map_uni(stack, a_mod, b_left, space)?;
        let b_mod = make_node(stack, b_entry, inner, b_right);
        map_uni(stack, a_right, b_mod, space)
    } else {
        let a_mod = make_node(stack, a_entry, D(0), a_right);
        let inner = map_uni(stack, a_mod, b_right, space)?;
        let b_mod = make_node(stack, b_entry, b_left, inner);
        map_uni(stack, a_left, b_mod, space)
    }
}

/// `+int:by`: intersection — keys present in both; `b`'s value is kept.
fn map_int(stack: &mut NockStack, a: Noun, b: Noun, space: &NounSpace) -> JetResult<Noun> {
    if a.is_atom() || b.is_atom() {
        return Ok(D(0));
    }
    let (a_entry, a_left, a_right) = decompose(a, space)?;
    let (a_key, _) = entry_kv(a_entry, space)?;
    let (b_entry, b_left, b_right) = decompose(b, space)?;
    let (b_key, _) = entry_kv(b_entry, space)?;

    if mor_lt(stack, a_key, b_key, space) {
        if noun_eq(stack, b_key, a_key) {
            let left = map_int(stack, a_left, b_left, space)?;
            let right = map_int(stack, a_right, b_right, space)?;
            Ok(make_node(stack, b_entry, left, right))
        } else if gor_lt(stack, b_key, a_key, space) {
            let b_mod = make_node(stack, b_entry, b_left, D(0));
            let x = map_int(stack, a_left, b_mod, space)?;
            let y = map_int(stack, a, b_right, space)?;
            map_uni(stack, x, y, space)
        } else {
            let b_mod = make_node(stack, b_entry, D(0), b_right);
            let x = map_int(stack, a_right, b_mod, space)?;
            let y = map_int(stack, a, b_left, space)?;
            map_uni(stack, x, y, space)
        }
    } else if noun_eq(stack, a_key, b_key) {
        let left = map_int(stack, a_left, b_left, space)?;
        let right = map_int(stack, a_right, b_right, space)?;
        Ok(make_node(stack, b_entry, left, right))
    } else if gor_lt(stack, a_key, b_key, space) {
        let a_mod = make_node(stack, a_entry, a_left, D(0));
        let x = map_int(stack, a_mod, b_left, space)?;
        let y = map_int(stack, a_right, b, space)?;
        map_uni(stack, x, y, space)
    } else {
        let a_mod = make_node(stack, a_entry, D(0), a_right);
        let x = map_int(stack, a_mod, b_right, space)?;
        let y = map_int(stack, a_left, b, space)?;
        map_uni(stack, x, y, space)
    }
}

/// `+dif:by`: difference — keys in `a` that are not in `b`.
fn map_dif(stack: &mut NockStack, a: Noun, b: Noun, space: &NounSpace) -> JetResult<Noun> {
    if b.is_atom() {
        return Ok(a);
    }
    let (b_entry, b_left, b_right) = decompose(b, space)?;
    let (b_key, _) = entry_kv(b_entry, space)?;
    let (below, above) = map_bif(stack, a, b_key, space)?;
    let left = map_dif(stack, below, b_left, space)?;
    let right = map_dif(stack, above, b_right, space)?;
    map_join(stack, left, right, space)
}

/// `+gas:by`: fold `+put` over a `(list [key value])`, left to right.
fn map_gas(stack: &mut NockStack, tree: Noun, list: Noun, space: &NounSpace) -> JetResult<Noun> {
    let mut acc = tree;
    let mut rest = list;
    loop {
        if rest.is_atom() {
            return Ok(acc);
        }
        let item = slot(rest, 2, space)?;
        let tail = slot(rest, 3, space)?;
        let (key, value) = entry_kv(item, space)?;
        acc = map_put(stack, acc, key, value, space)?;
        rest = tail;
    }
}

/// `+tap:by`: flatten to a `(list [key value])`. Hoon's `+tap` conses
/// `[n.a $(a l.a)]` then recurses right, which yields the entries in
/// reverse-in-order (gor-descending). We reproduce that exact fold.
fn map_tap(stack: &mut NockStack, tree: Noun, acc: Noun, space: &NounSpace) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(acc);
    }
    let (entry, left, right) = decompose(tree, space)?;
    let inner = map_tap(stack, left, acc, space)?;
    let consed = T(stack, &[entry, inner]);
    map_tap(stack, right, consed, space)
}

/// `+key:by`: the `(set)` of keys. Built with the stock set put so the result
/// is a valid, canonically-ordered set (mirrors Vere's `_by_key`).
fn map_key(stack: &mut NockStack, tree: Noun, acc: Noun, space: &NounSpace) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(acc);
    }
    let (entry, left, right) = decompose(tree, space)?;
    let (key, _) = entry_kv(entry, space)?;
    let acc = super::set::set_put(stack, acc, key, space)?;
    let acc = map_key(stack, left, acc, space)?;
    map_key(stack, right, acc, space)
}

/// `+wyt:by`: the number of entries.
fn map_wyt(tree: Noun, space: &NounSpace) -> JetResult<u64> {
    if tree.is_atom() {
        return Ok(0);
    }
    let (_entry, left, right) = decompose(tree, space)?;
    Ok(1 + map_wyt(left, space)? + map_wyt(right, space)?)
}

/// `+apt:by`: is the tree a valid map treap? Keys strictly `+gor`-ordered within
/// `(lower, upper)` bounds and each parent `+mor`-outranking its children.
/// Ports Vere's `_by_apt` (upper = exclusive high bound, lower = exclusive low).
fn map_apt(
    stack: &mut NockStack,
    tree: Noun,
    upper: Option<Noun>,
    lower: Option<Noun>,
    space: &NounSpace,
) -> JetResult<bool> {
    if tree.is_atom() {
        return Ok(true);
    }
    let (entry, left, right) = decompose(tree, space)?;
    let (key, _) = entry_kv(entry, space)?;

    if let Some(u) = upper {
        if !gor_lt(stack, key, u, space) || noun_eq(stack, key, u) {
            return Ok(false);
        }
    }
    if let Some(l) = lower {
        if !gor_lt(stack, l, key, space) || noun_eq(stack, l, key) {
            return Ok(false);
        }
    }
    if !left.is_atom() {
        let (le, _, _) = decompose(left, space)?;
        let (lk, _) = entry_kv(le, space)?;
        if !mor_lt(stack, key, lk, space) || noun_eq(stack, key, lk) {
            return Ok(false);
        }
        if !map_apt(stack, left, Some(key), lower, space)? {
            return Ok(false);
        }
    }
    if !right.is_atom() {
        let (re, _, _) = decompose(right, space)?;
        let (rk, _) = entry_kv(re, space)?;
        if !mor_lt(stack, key, rk, space) || noun_eq(stack, key, rk) {
            return Ok(false);
        }
        if !map_apt(stack, right, upper, Some(key), space)? {
            return Ok(false);
        }
    }
    Ok(true)
}

// ---------------------------------------------------------------------------
// gate-slamming operations: each applies a fixed gate to entries. `slam` runs
// the gate's arm (its battery axis 2) with a fresh sample; these mirror Vere's
// `_by_run`/`_by_urn`/`u3kdb_jab`/`_by_all`/`_by_any`.
// ---------------------------------------------------------------------------

/// `+run:by`: map a gate over values; tree shape is preserved (keys unchanged).
fn map_run(context: &mut Context, tree: Noun, gate: Noun, space: &NounSpace) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(D(0));
    }
    let (entry, left, right) = decompose(tree, space)?;
    let (key, value) = entry_kv(entry, space)?;
    let new_value = slam(context, gate, value)?;
    let new_left = map_run(context, left, gate, space)?;
    let new_right = map_run(context, right, gate, space)?;
    let new_entry = make_entry(&mut context.stack, key, new_value);
    Ok(make_node(&mut context.stack, new_entry, new_left, new_right))
}

/// `+urn:by`: map a gate over the whole `[key value]` entry; shape preserved.
fn map_urn(context: &mut Context, tree: Noun, gate: Noun, space: &NounSpace) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(D(0));
    }
    let (entry, left, right) = decompose(tree, space)?;
    let (key, _) = entry_kv(entry, space)?;
    let new_value = slam(context, gate, entry)?;
    let new_left = map_urn(context, left, gate, space)?;
    let new_right = map_urn(context, right, gate, space)?;
    let new_entry = make_entry(&mut context.stack, key, new_value);
    Ok(make_node(&mut context.stack, new_entry, new_left, new_right))
}

/// `+jab:by`: replace the value at `key` with `(fun value)`. Absent key crashes
/// in Hoon; we punt so the raw arm produces that crash exactly.
fn map_jab(
    context: &mut Context,
    tree: Noun,
    key: Noun,
    fun: Noun,
    space: &NounSpace,
) -> JetResult<Noun> {
    if tree.is_atom() {
        return Err(JetErr::Punt);
    }
    let (entry, left, right) = decompose(tree, space)?;
    let (node_key, node_value) = entry_kv(entry, space)?;
    if noun_eq(&mut context.stack, key, node_key) {
        let new_value = slam(context, fun, node_value)?;
        let new_entry = make_entry(&mut context.stack, node_key, new_value);
        return Ok(make_node(&mut context.stack, new_entry, left, right));
    }
    if gor_lt(&mut context.stack, key, node_key, space) {
        let child = map_jab(context, left, key, fun, space)?;
        Ok(make_node(&mut context.stack, entry, child, right))
    } else {
        let child = map_jab(context, right, key, fun, space)?;
        Ok(make_node(&mut context.stack, entry, left, child))
    }
}

/// `+all:by`: do all values satisfy the gate? Short-circuits on the first `%.n`.
/// Returns `%.y` on the empty map. Punts if the gate returns a non-loobean.
fn map_all(context: &mut Context, tree: Noun, gate: Noun, space: &NounSpace) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(YES);
    }
    let (entry, left, right) = decompose(tree, space)?;
    let (_, value) = entry_kv(entry, space)?;
    let r = slam(context, gate, value)?;
    if is_yes(r) {
        let lr = map_all(context, left, gate, space)?;
        if !is_yes(lr) {
            return Ok(lr);
        }
        map_all(context, right, gate, space)
    } else if unsafe { r.raw_equals(&NO) } {
        Ok(NO)
    } else {
        Err(JetErr::Punt)
    }
}

/// `+any:by`: does any value satisfy the gate? Short-circuits on the first
/// `%.y`. Returns `%.n` on the empty map. Punts on a non-loobean.
fn map_any(context: &mut Context, tree: Noun, gate: Noun, space: &NounSpace) -> JetResult<Noun> {
    if tree.is_atom() {
        return Ok(NO);
    }
    let (entry, left, right) = decompose(tree, space)?;
    let (_, value) = entry_kv(entry, space)?;
    let r = slam(context, gate, value)?;
    if is_yes(r) {
        Ok(YES)
    } else if unsafe { r.raw_equals(&NO) } {
        let lr = map_any(context, left, gate, space)?;
        if is_yes(lr) {
            return Ok(YES);
        }
        map_any(context, right, gate, space)
    } else {
        Err(JetErr::Punt)
    }
}

// ---------------------------------------------------------------------------
// jets (the door arms)
//
// A door arm's subject is `[battery sample context]`: sample at axis 6, the door
// core (whose own sample is the map `a`) at axis 7. `door_map` reads `a` at
// axis 7->6 (= `u3x_con_sam`, axis 30). The no-sample trap arms `tap`/`key`/`apt`
// ignore axis 6 but still find `a` there. `wyt` is the one exception: it is a
// bare `~%  %wyt  +  ~  |.` trap registered at parent-axis 3, so its `a` is one
// axis shallower, at `u3x_con_2` (axis 14) — see `jet_by_wyt`.
// ---------------------------------------------------------------------------

/// `~(get by a)`: `(unit value)`.
pub fn jet_by_get(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let key = slot(subject, 6, &space)?;
    let map = door_map(subject, &space)?;
    map_get_unit(&mut context.stack, map, key, &space)
}

/// `~(got by a)`: the value, crashing if absent. On absence we punt so the raw
/// Nock runs the arm's own `!!` and the behaviour matches exactly.
pub fn jet_by_got(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let key = slot(subject, 6, &space)?;
    let map = door_map(subject, &space)?;
    let unit = map_get_unit(&mut context.stack, map, key, &space)?;
    if unit.is_atom() {
        return Err(JetErr::Punt);
    }
    slot(unit, 3, &space)
}

/// `~(gut by a)`: the value, or the supplied default if absent. Sample is
/// `[key default]`.
pub fn jet_by_gut(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sample = slot(subject, 6, &space)?;
    let key = slot(sample, 2, &space)?;
    let default = slot(sample, 3, &space)?;
    let map = door_map(subject, &space)?;
    let unit = map_get_unit(&mut context.stack, map, key, &space)?;
    if unit.is_atom() {
        Ok(default)
    } else {
        slot(unit, 3, &space)
    }
}

/// `~(has by a)`: `%.y` iff the key is present.
pub fn jet_by_has(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let key = slot(subject, 6, &space)?;
    let map = door_map(subject, &space)?;
    let unit = map_get_unit(&mut context.stack, map, key, &space)?;
    Ok(if unit.is_atom() { NO } else { YES })
}

/// `~(put by a)`: insert/replace. Sample is `[key value]`.
pub fn jet_by_put(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sample = slot(subject, 6, &space)?;
    let key = slot(sample, 2, &space)?;
    let value = slot(sample, 3, &space)?;
    let map = door_map(subject, &space)?;
    map_put(&mut context.stack, map, key, value, &space)
}

/// `~(del by a)`: remove a key. Sample is the key.
pub fn jet_by_del(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let key = slot(subject, 6, &space)?;
    let map = door_map(subject, &space)?;
    map_del(&mut context.stack, map, key, &space)
}

/// `~(mar by a)`: put when the sample unit is `[~ value]`, else del. Sample is
/// `[key (unit value)]`.
pub fn jet_by_mar(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sample = slot(subject, 6, &space)?;
    let key = slot(sample, 2, &space)?;
    let unit = slot(sample, 3, &space)?;
    let map = door_map(subject, &space)?;
    if unit.is_atom() {
        map_del(&mut context.stack, map, key, &space)
    } else {
        let value = slot(unit, 3, &space)?;
        map_put(&mut context.stack, map, key, value, &space)
    }
}

/// `~(gas by a)`: bulk insert from a `(list [key value])`. Sample is the list.
pub fn jet_by_gas(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let list = slot(subject, 6, &space)?;
    let map = door_map(subject, &space)?;
    map_gas(&mut context.stack, map, list, &space)
}

/// `~(uni by a)`: union with another map `b` (b's values win). Sample is `b`.
pub fn jet_by_uni(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let b = slot(subject, 6, &space)?;
    let a = door_map(subject, &space)?;
    map_uni(&mut context.stack, a, b, &space)
}

/// `~(int by a)`: intersection with another map `b` (b's values win). Sample is `b`.
pub fn jet_by_int(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let b = slot(subject, 6, &space)?;
    let a = door_map(subject, &space)?;
    map_int(&mut context.stack, a, b, &space)
}

/// `~(dif by a)`: a minus b's keys. Sample is `b`.
pub fn jet_by_dif(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let b = slot(subject, 6, &space)?;
    let a = door_map(subject, &space)?;
    map_dif(&mut context.stack, a, b, &space)
}

/// `~(tap by a)`: entries as a `(list [key value])`.
pub fn jet_by_tap(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let a = door_map(subject, &space)?;
    map_tap(&mut context.stack, a, D(0), &space)
}

/// `~(key by a)`: the `(set)` of keys.
pub fn jet_by_key(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let a = door_map(subject, &space)?;
    map_key(&mut context.stack, a, D(0), &space)
}

/// `~(wyt by a)`: the entry count.
pub fn jet_by_wyt(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    // `+wyt:by` is a bare trap (`~%  %wyt  +  ~  |.`), so its door core sits one
    // axis shallower than the sample-bearing arms: `a` is at `u3x_con_2` (axis
    // 14), NOT `u3x_con_sam` (axis 30 = `door_map`). Vere registers `wyt` at
    // parent-axis 3 for exactly this reason; every other `+by` arm is axis 7.
    let a = slot(subject, 14, &space)?;
    Ok(D(map_wyt(a, &space)?))
}

/// `~(apt by a)`: is `a` a well-formed map treap?
pub fn jet_by_apt(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let a = door_map(subject, &space)?;
    Ok(if map_apt(&mut context.stack, a, None, None, &space)? {
        YES
    } else {
        NO
    })
}

/// `~(bif by a)`: split `a` around a key into `[l=(map) r=(map)]`. Sample is the
/// key.
pub fn jet_by_bif(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let key = slot(subject, 6, &space)?;
    let a = door_map(subject, &space)?;
    let (below, above) = map_bif(&mut context.stack, a, key, &space)?;
    Ok(T(&mut context.stack, &[below, above]))
}

/// `~(run by a)`: map a gate over values. Sample is the gate.
pub fn jet_by_run(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let gate = slot(subject, 6, &space)?;
    let a = door_map(subject, &space)?;
    map_run(context, a, gate, &space)
}

/// `~(urn by a)`: map a gate over `[key value]` entries. Sample is the gate.
pub fn jet_by_urn(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let gate = slot(subject, 6, &space)?;
    let a = door_map(subject, &space)?;
    map_urn(context, a, gate, &space)
}

/// `~(jab by a)`: adjust the value at a key via a gate. Sample is `[key fun]`.
pub fn jet_by_jab(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sample = slot(subject, 6, &space)?;
    let key = slot(sample, 2, &space)?;
    let fun = slot(sample, 3, &space)?;
    let a = door_map(subject, &space)?;
    map_jab(context, a, key, fun, &space)
}

/// `~(all by a)`: do all values satisfy the gate? Sample is the gate.
pub fn jet_by_all(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let gate = slot(subject, 6, &space)?;
    let a = door_map(subject, &space)?;
    map_all(context, a, gate, &space)
}

/// `~(any by a)`: does any value satisfy the gate? Sample is the gate.
pub fn jet_by_any(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let gate = slot(subject, 6, &space)?;
    let a = door_map(subject, &space)?;
    map_any(context, a, gate, &space)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nockvm::jets::util::test::{assert_jet_door, init_context};
    use nockvm::noun::D;
    use std::collections::BTreeMap;

    // An independent oracle: a `BTreeMap` model, checked against the jet tree by
    // (1) the exact set of `[key value]` pairs it holds and (2) the treap
    // invariants, which pin the shape to the one canonical tree the Hoon builds.

    fn direct(noun: Noun) -> u64 {
        noun.as_direct().expect("direct atom").data()
    }

    /// In-order `[key value]` pairs; also asserts the treap is canonical
    /// (gor-ordered keys, mor-ordered priorities) and returns the root key.
    fn verify(stack: &mut NockStack, tree: Noun, space: &NounSpace, out: &mut Vec<(u64, u64)>) {
        if tree.is_atom() {
            assert!(unsafe { tree.raw_equals(&D(0)) }, "empty map must be ~");
            return;
        }
        let (entry, left, right) = decompose(tree, space).expect("node");
        let (key, value) = entry_kv(entry, space).expect("entry");

        verify(stack, left, space, out);
        out.push((direct(key), direct(value)));
        verify(stack, right, space, out);

        if !left.is_atom() {
            let (le, _, _) = decompose(left, space).unwrap();
            let (lk, _) = entry_kv(le, space).unwrap();
            assert!(gor_lt(stack, lk, key, space), "left key must gor-precede parent");
            assert!(mor_lt(stack, key, lk, space), "parent must mor-outrank left child");
        }
        if !right.is_atom() {
            let (re, _, _) = decompose(right, space).unwrap();
            let (rk, _) = entry_kv(re, space).unwrap();
            assert!(gor_lt(stack, key, rk, space), "parent must gor-precede right key");
            assert!(mor_lt(stack, key, rk, space), "parent must mor-outrank right child");
        }
    }

    fn assert_matches(stack: &mut NockStack, tree: Noun, space: &NounSpace, model: &BTreeMap<u64, u64>) {
        let mut pairs = Vec::new();
        verify(stack, tree, space, &mut pairs);
        let mut got: Vec<(u64, u64)> = pairs;
        got.sort_unstable();
        let mut want: Vec<(u64, u64)> = model.iter().map(|(&k, &v)| (k, v)).collect();
        want.sort_unstable();
        assert_eq!(got, want, "jet tree pairs must equal the model");
    }

    fn from_model(stack: &mut NockStack, space: &NounSpace, model: &BTreeMap<u64, u64>) -> Noun {
        let mut tree = D(0);
        for (&k, &v) in model {
            tree = map_put(stack, tree, D(k), D(v), space).expect("put");
        }
        tree
    }

    // --- door-call unit tests (exercise the axis plumbing) ------------------

    fn door(stack: &mut NockStack, map: Noun) -> Noun {
        // door core: [battery a=map context]; `a` sits at axis 6.
        T(stack, &[D(0), map, D(0)])
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn get_has_on_small_map() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let mut model = BTreeMap::new();
        for (k, v) in [(5u64, 50u64), (3, 30), (11, 110), (7, 70)] {
            model.insert(k, v);
        }
        let map = from_model(&mut context.stack, &space, &model);
        let pay = door(&mut context.stack, map);

        // present key → [~ value]; absent key → ~
        let want = T(&mut context.stack, &[D(0), D(70)]);
        assert_jet_door(context, jet_by_get, D(7), pay, want);
        assert_jet_door(context, jet_by_get, D(99), pay, D(0));
        assert_jet_door(context, jet_by_has, D(3), pay, YES);
        assert_jet_door(context, jet_by_has, D(99), pay, NO);
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn got_present_gut_default() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let mut model = BTreeMap::new();
        model.insert(5u64, 50u64);
        model.insert(9, 90);
        let map = from_model(&mut context.stack, &space, &model);
        let pay = door(&mut context.stack, map);

        assert_jet_door(context, jet_by_got, D(9), pay, D(90));

        // gut: [key default]
        let hit = T(&mut context.stack, &[D(5), D(0)]);
        assert_jet_door(context, jet_by_gut, hit, pay, D(50));
        let miss = T(&mut context.stack, &[D(42), D(1234)]);
        assert_jet_door(context, jet_by_gut, miss, pay, D(1234));
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn got_absent_punts() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let mut model = BTreeMap::new();
        model.insert(5u64, 50u64);
        let map = from_model(&mut context.stack, &space, &model);
        let pay = door(&mut context.stack, map);
        let subject = T(&mut context.stack, &[D(0), D(404), pay]);
        match jet_by_got(context, subject) {
            Err(JetErr::Punt) => {}
            other => panic!("got on absent key must punt, got {other:?}"),
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn put_replaces_value_and_dedups() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        // the stock set.rs dedup bug: putting an equal key must not grow the tree.
        let mut tree = D(0);
        tree = map_put(&mut context.stack, tree, D(5), D(1), &space).unwrap();
        tree = map_put(&mut context.stack, tree, D(5), D(2), &space).unwrap();
        let mut model = BTreeMap::new();
        model.insert(5u64, 2u64);
        assert_matches(&mut context.stack, tree, &space, &model);

        // putting the identical [key value] returns the same tree pointer.
        let again = map_put(&mut context.stack, tree, D(5), D(2), &space).unwrap();
        assert!(unsafe { again.raw_equals(&tree) }, "identical put must be a no-op");
    }

    // --- differential tests against the model -------------------------------

    fn lcg(seed: &mut u64) -> u64 {
        *seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        *seed >> 33
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn put_matches_model_random() {
        let mut seed = 0xD15C0DE5u64;
        for _ in 0..24 {
            let context = &mut init_context();
            let space = context.stack.noun_space();
            let mut tree = D(0);
            let mut model = BTreeMap::new();
            for _ in 0..120 {
                let key = lcg(&mut seed) % 400;
                let value = lcg(&mut seed) % 1000;
                tree = map_put(&mut context.stack, tree, D(key), D(value), &space).unwrap();
                model.insert(key, value);
            }
            assert_matches(&mut context.stack, tree, &space, &model);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn del_matches_model_random() {
        let mut seed = 0xBADF00Du64;
        for _ in 0..24 {
            let context = &mut init_context();
            let space = context.stack.noun_space();
            let mut tree = D(0);
            let mut model = BTreeMap::new();
            // populate
            for _ in 0..100 {
                let key = lcg(&mut seed) % 200;
                let value = lcg(&mut seed) % 1000;
                tree = map_put(&mut context.stack, tree, D(key), D(value), &space).unwrap();
                model.insert(key, value);
            }
            // delete a mix of present and absent keys
            for _ in 0..120 {
                let key = lcg(&mut seed) % 200;
                tree = map_del(&mut context.stack, tree, D(key), &space).unwrap();
                model.remove(&key);
                assert_matches(&mut context.stack, tree, &space, &model);
            }
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn gas_matches_repeated_put() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        // build a (list [key value]) and gas it onto ~; compare to folded puts.
        let pairs = [(5u64, 1u64), (3, 2), (5, 3), (9, 4), (1, 5)];
        let mut list = D(0);
        for &(k, v) in pairs.iter().rev() {
            let item = T(&mut context.stack, &[D(k), D(v)]);
            list = T(&mut context.stack, &[item, list]);
        }
        let gassed = map_gas(&mut context.stack, D(0), list, &space).unwrap();

        let mut model = BTreeMap::new();
        for &(k, v) in pairs.iter() {
            model.insert(k, v); // later duplicate wins, as gas folds left-to-right
        }
        assert_matches(&mut context.stack, gassed, &space, &model);
    }

    fn two_maps(
        stack: &mut NockStack,
        space: &NounSpace,
        seed: &mut u64,
    ) -> (Noun, Noun, BTreeMap<u64, u64>, BTreeMap<u64, u64>) {
        let mut ma = BTreeMap::new();
        let mut mb = BTreeMap::new();
        for _ in 0..60 {
            ma.insert(lcg(seed) % 150, lcg(seed) % 1000);
            mb.insert(lcg(seed) % 150, lcg(seed) % 1000);
        }
        let a = from_model(stack, space, &ma);
        let b = from_model(stack, space, &mb);
        (a, b, ma, mb)
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn uni_int_dif_match_model() {
        let mut seed = 0x5EED1234u64;
        for _ in 0..16 {
            let context = &mut init_context();
            let space = context.stack.noun_space();
            let (a, b, ma, mb) = two_maps(&mut context.stack, &space, &mut seed);

            // uni: b overrides a
            let mut uni = ma.clone();
            for (&k, &v) in &mb {
                uni.insert(k, v);
            }
            let r = map_uni(&mut context.stack, a, b, &space).unwrap();
            assert_matches(&mut context.stack, r, &space, &uni);

            // int: keys in both, b's value
            let mut int = BTreeMap::new();
            for (&k, &v) in &mb {
                if ma.contains_key(&k) {
                    int.insert(k, v);
                }
            }
            let r = map_int(&mut context.stack, a, b, &space).unwrap();
            assert_matches(&mut context.stack, r, &space, &int);

            // dif: keys in a not in b, a's value
            let mut dif = BTreeMap::new();
            for (&k, &v) in &ma {
                if !mb.contains_key(&k) {
                    dif.insert(k, v);
                }
            }
            let r = map_dif(&mut context.stack, a, b, &space).unwrap();
            assert_matches(&mut context.stack, r, &space, &dif);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn mar_puts_and_dels() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let mut model = BTreeMap::new();
        model.insert(5u64, 50u64);
        let map = from_model(&mut context.stack, &space, &model);
        let pay = door(&mut context.stack, map);

        // mar [key [~ value]] → put
        let some = T(&mut context.stack, &[D(0), D(90)]);
        let sample = T(&mut context.stack, &[D(9), some]);
        let want = map_put(&mut context.stack, map, D(9), D(90), &space).unwrap();
        assert_jet_door(context, jet_by_mar, sample, pay, want);

        // mar [key ~] → del
        let sample = T(&mut context.stack, &[D(5), D(0)]);
        let want = map_del(&mut context.stack, map, D(5), &space).unwrap();
        assert_jet_door(context, jet_by_mar, sample, pay, want);
    }

    // --- pure tail arms ------------------------------------------------------

    /// Collect a `(list [key value])` noun into a Vec, preserving order.
    fn list_pairs(tree: Noun, space: &NounSpace) -> Vec<(u64, u64)> {
        let mut out = Vec::new();
        let mut cur = tree;
        while !cur.is_atom() {
            let item = slot(cur, 2, space).unwrap();
            let k = direct(slot(item, 2, space).unwrap());
            let v = direct(slot(item, 3, space).unwrap());
            out.push((k, v));
            cur = slot(cur, 3, space).unwrap();
        }
        out
    }

    /// In-order (gor-ascending) pairs, unsorted.
    fn in_order(tree: Noun, space: &NounSpace, out: &mut Vec<(u64, u64)>) {
        if tree.is_atom() {
            return;
        }
        let (entry, left, right) = decompose(tree, space).unwrap();
        let (k, v) = entry_kv(entry, space).unwrap();
        in_order(left, space, out);
        out.push((direct(k), direct(v)));
        in_order(right, space, out);
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn tap_is_reverse_in_order() {
        let mut seed = 0x7A9u64;
        for _ in 0..12 {
            let context = &mut init_context();
            let space = context.stack.noun_space();
            let mut model = BTreeMap::new();
            for _ in 0..40 {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                model.insert((seed >> 40) % 300, (seed >> 20) % 300);
            }
            let map = from_model(&mut context.stack, &space, &model);
            let list = map_tap(&mut context.stack, map, D(0), &space).unwrap();

            let got = list_pairs(list, &space);
            let mut io = Vec::new();
            in_order(map, &space, &mut io);
            io.reverse();
            assert_eq!(got, io, "tap must be reverse-in-order");
            // and it holds exactly the model's pairs
            let mut sorted = got.clone();
            sorted.sort_unstable();
            let want: Vec<_> = model.iter().map(|(&k, &v)| (k, v)).collect();
            assert_eq!(sorted, want);
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn key_wyt_match_model() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let model: BTreeMap<u64, u64> =
            [(5u64, 1u64), (3, 2), (11, 3), (7, 4), (1, 5)].into_iter().collect();
        let map = from_model(&mut context.stack, &space, &model);

        assert_eq!(map_wyt(map, &space).unwrap(), model.len() as u64);

        // End-to-end through the jet entrypoint: `+wyt` is a bare trap, so `a`
        // lives at axis 14 (u3x_con_2), not `door_map` (axis 30). Build the
        // trap-shaped subject [* * a *] (axis 14 = a) and check the count.
        let subj = T(&mut context.stack, &[D(0), D(0), map, D(0)]);
        let got_wyt = jet_by_wyt(context, subj).unwrap();
        assert_eq!(direct(got_wyt), model.len() as u64);

        // key set: collect its bare elements, compare to the model's keys.
        let keys = map_key(&mut context.stack, map, D(0), &space).unwrap();
        let mut got = Vec::new();
        let mut stack_nodes = vec![keys];
        while let Some(n) = stack_nodes.pop() {
            if n.is_atom() {
                continue;
            }
            got.push(direct(slot(n, 2, &space).unwrap()));
            stack_nodes.push(slot(n, 6, &space).unwrap());
            stack_nodes.push(slot(n, 7, &space).unwrap());
        }
        got.sort_unstable();
        let want: Vec<u64> = model.keys().copied().collect();
        assert_eq!(got, want);
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn apt_accepts_valid_rejects_broken() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let model: BTreeMap<u64, u64> =
            (0..50u64).map(|k| (k.wrapping_mul(2654435761) % 500, k)).collect();
        let map = from_model(&mut context.stack, &space, &model);
        assert!(map_apt(&mut context.stack, map, None, None, &space).unwrap());

        // a hand-built tree that violates gor-order (two arbitrary keys forced
        // into a parent/child relation that gor would not produce).
        let e1 = T(&mut context.stack, &[D(1), D(0)]);
        let e2 = T(&mut context.stack, &[D(2), D(0)]);
        let child = T(&mut context.stack, &[e2, D(0), D(0)]);
        // put child on whichever side gor disagrees with
        let broken = if gor_lt(&mut context.stack, D(2), D(1), &space) {
            // 2 gor-precedes 1, so placing 2 on the RIGHT of 1 is invalid
            T(&mut context.stack, &[e1, D(0), child])
        } else {
            T(&mut context.stack, &[e1, child, D(0)])
        };
        assert!(!map_apt(&mut context.stack, broken, None, None, &space).unwrap());
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn bif_splits_around_key() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let model: BTreeMap<u64, u64> =
            (1..40u64).map(|k| (k, k * 10)).collect();
        let map = from_model(&mut context.stack, &space, &model);
        let pivot = 17u64;

        let (below, above) = map_bif(&mut context.stack, map, D(pivot), &space).unwrap();
        // every key in `below` gor-precedes pivot; every key in `above` follows.
        for (tree, side_is_below) in [(below, true), (above, false)] {
            let mut io = Vec::new();
            in_order(tree, &space, &mut io);
            for (k, _) in io {
                let before = gor_lt(&mut context.stack, D(k), D(pivot), &space);
                assert_eq!(before, side_is_below, "key {k} on wrong side of {pivot}");
            }
        }
        // union of the halves == model minus the pivot.
        let mut all = Vec::new();
        in_order(below, &space, &mut all);
        in_order(above, &space, &mut all);
        all.sort_unstable();
        let want: Vec<_> = model.iter().filter(|(&k, _)| k != pivot).map(|(&k, &v)| (k, v)).collect();
        assert_eq!(all, want);
    }

    // --- gate-slamming tail arms --------------------------------------------
    //
    // Gates built by hand: battery is the arm formula, run with subject
    // `[battery [sample context]]` (sample at axis 6). See `slam`.

    fn gate(stack: &mut NockStack, battery: Noun) -> Noun {
        T(stack, &[battery, D(0), D(0)])
    }
    fn inc_gate(stack: &mut NockStack) -> Noun {
        let b = T(stack, &[D(4), D(0), D(6)]); // +(sample)
        gate(stack, b)
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn run_increments_values() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let model: BTreeMap<u64, u64> = [(5u64, 10u64), (3, 20), (9, 30)].into_iter().collect();
        let map = from_model(&mut context.stack, &space, &model);
        let g = inc_gate(&mut context.stack);
        let res = map_run(context, map, g, &space).unwrap();
        let want: BTreeMap<u64, u64> = model.iter().map(|(&k, &v)| (k, v + 1)).collect();
        assert_matches(&mut context.stack, res, &space, &want);
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn urn_sees_key() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let model: BTreeMap<u64, u64> = [(5u64, 10u64), (3, 20), (9, 30)].into_iter().collect();
        let map = from_model(&mut context.stack, &space, &model);
        // gate returns the key (head of the [key value] sample = axis 12).
        let b = T(&mut context.stack, &[D(0), D(12)]);
        let g = gate(&mut context.stack, b);
        let res = map_urn(context, map, g, &space).unwrap();
        let want: BTreeMap<u64, u64> = model.keys().map(|&k| (k, k)).collect();
        assert_matches(&mut context.stack, res, &space, &want);
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn jab_adjusts_one_value_and_punts_when_absent() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        let model: BTreeMap<u64, u64> = [(5u64, 10u64), (3, 20), (9, 30)].into_iter().collect();
        let map = from_model(&mut context.stack, &space, &model);
        let g = inc_gate(&mut context.stack);
        let res = map_jab(context, map, D(3), g, &space).unwrap();
        let mut want = model.clone();
        *want.get_mut(&3).unwrap() += 1;
        assert_matches(&mut context.stack, res, &space, &want);

        let g2 = inc_gate(&mut context.stack);
        match map_jab(context, map, D(404), g2, &space) {
            Err(JetErr::Punt) => {}
            other => panic!("jab on absent key must punt, got {other:?}"),
        }
    }

    #[test]
    #[cfg_attr(miri, ignore = "memfd_create unsupported in Miri")]
    fn all_any_over_values() {
        let context = &mut init_context();
        let space = context.stack.noun_space();
        // is-zero predicate: =(sample 0)
        let is_zero = |stack: &mut NockStack| {
            let b = {
                let s = T(stack, &[D(0), D(6)]);
                let z = T(stack, &[D(1), D(0)]);
                T(stack, &[D(5), s, z])
            };
            gate(stack, b)
        };

        let all_zero: BTreeMap<u64, u64> = [(5u64, 0u64), (3, 0), (9, 0)].into_iter().collect();
        let mixed: BTreeMap<u64, u64> = [(5u64, 0u64), (3, 7), (9, 0)].into_iter().collect();
        let map0 = from_model(&mut context.stack, &space, &all_zero);
        let mapm = from_model(&mut context.stack, &space, &mixed);

        let g = is_zero(&mut context.stack);
        assert!(is_yes(map_all(context, map0, g, &space).unwrap()), "all zero → all YES");
        let g = is_zero(&mut context.stack);
        assert!(!is_yes(map_all(context, mapm, g, &space).unwrap()), "mixed → all NO");
        let g = is_zero(&mut context.stack);
        assert!(is_yes(map_any(context, mapm, g, &space).unwrap()), "mixed → any YES");
        // empty map: all YES, any NO
        let g = is_zero(&mut context.stack);
        assert!(is_yes(map_all(context, D(0), g, &space).unwrap()));
        let g = is_zero(&mut context.stack);
        assert!(!is_yes(map_any(context, D(0), g, &space).unwrap()));
    }
}
