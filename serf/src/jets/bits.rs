// Standard atom/bit helper jets for Eris.
//
// nockvm's stock dashboard jets the hot bit arms (`bex can cat cut end lsh met
// rap rep rev rip rsh xeb con dis mix`) but leaves a couple of chapter-`two`
// helpers unjetted. A stock k138 Urbit pill uses them, so we add them here,
// modeled on Vere's C jets (`pkg/noun/jets/c/{swp,pow}.c`) and hoon.hoon, in
// nockvm's house style. Registered from the serf (see `super::eris_hot_state`)
// so nockvm's own dashboard stays unpatched; pinned to Hoon kelvin 138.
//
// Arms: `swp pow`. Both are plain binary gates in chapter `two`: each jet reads
// its sample at axis 6 and the two arguments at axes 2 and 3.

use nockvm::interpreter::Context;
use nockvm::jets::bits::rep;
use nockvm::jets::bits::util::rip;
use nockvm::jets::util::slot;
use nockvm::jets::{JetErr, Result};
use nockvm::mem::NockStack;
use nockvm::noun::{Atom, Noun, NounSpace, D, T};

type JetResult<T> = std::result::Result<T, JetErr>;



/// Reverse a `~`-terminated list of blocks, in place of hoon's `+flop`.
fn flop(stack: &mut NockStack, mut list: Noun, space: &NounSpace) -> JetResult<Noun> {
    let mut out = D(0);
    while !list.is_atom() {
        out = T(stack, &[slot(list, 2, space)?, out]);
        list = slot(list, 3, space)?;
    }
    Ok(out)
}

/// `+swp`: reverse the `a`-block order of atom `b`, i.e. `(rep a (flop (rip a
/// b)))`. hoon: `|= [a=bloq b=@]`. We reuse nockvm's own `rip`/`rep` so the
/// bit-slicing is identical to the stock jets.
///
/// As in Vere's `u3qc_swp`, the bloq `a` must be a cat (direct) under the word
/// size; anything larger would demand an absurd allocation, so we `Punt` and
/// let the (unreachable in practice) Nock handle it.
pub fn jet_swp(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let a = slot(sam, 2, &space)?;
    let bloq = a
        .in_space(&space)
        .as_atom()
        .map_err(|_| JetErr::Punt)?
        .as_u64()
        .map_err(|_| JetErr::Punt)? as usize;
    if bloq >= 32 {
        return Err(JetErr::Punt);
    }
    let b: Atom = slot(sam, 3, &space)?.as_atom().map_err(|_| JetErr::Punt)?;

    let blocks = rip(&mut context.stack, bloq, 1, b, &space)?;
    let flipped = flop(&mut context.stack, blocks, &space)?;
    rep(&mut context.stack, a, flipped, &space)
}

/// `+pow`: unsigned exponent `a^b`. hoon: `|= [a=@ b=@]`, with `pow 0`→`1`.
/// Modeled on Vere's `u3qc_pow` (which requires `b` be a cat); we `Punt` if the
/// exponent does not fit a `usize`, leaving the Nock to run.
pub fn jet_pow(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let a: Atom = slot(sam, 2, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let b = slot(sam, 3, &space)?
        .in_space(&space)
        .as_atom()
        .map_err(|_| JetErr::Punt)?
        .as_u64()
        .map_err(|_| JetErr::Punt)?;
    let exp = usize::try_from(b).map_err(|_| JetErr::Punt)?;

    let stack = &mut context.stack;
    let base = a.in_space(&space).as_ubig(stack);
    let res = base.pow_stack(stack, exp);
    Ok(Atom::from_ubig(stack, &res).as_noun())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nockvm::jets::util::test::{assert_jet, init_context};
    use nockvm::noun::{D, T};

    #[test]
    fn test_pow() {
        let c = &mut init_context();
        // 2^10 = 1024
        let sam = T(&mut c.stack, &[D(2), D(10)]);
        assert_jet(c, jet_pow, sam, D(1024));
        // a^0 = 1
        let sam2 = T(&mut c.stack, &[D(7), D(0)]);
        assert_jet(c, jet_pow, sam2, D(1));
        // 0^0 = 1 (hoon convention)
        let sam3 = T(&mut c.stack, &[D(0), D(0)]);
        assert_jet(c, jet_pow, sam3, D(1));
        // 0^5 = 0
        let sam4 = T(&mut c.stack, &[D(0), D(5)]);
        assert_jet(c, jet_pow, sam4, D(0));
        // 3^4 = 81
        let sam5 = T(&mut c.stack, &[D(3), D(4)]);
        assert_jet(c, jet_pow, sam5, D(81));
    }

    #[test]
    fn test_swp() {
        let c = &mut init_context();
        // bloq 0 (bits): reverse the bit order within the significant bits.
        // 0b1011 (0xb, met=4 bits) -> 0b1101 (0xd).
        let sam = T(&mut c.stack, &[D(0), D(0xb)]);
        assert_jet(c, jet_swp, sam, D(0xd));
        // bloq 3 (bytes): 0xaabbcc (3 bytes, LSB-first aa bb cc) reversed to
        // cc bb aa i.e. 0xaabbcc -> little-endian blocks [cc bb aa] flipped
        // to [aa bb cc] repacked = 0xccbbaa.
        let sam2 = T(&mut c.stack, &[D(3), D(0xaabbcc)]);
        assert_jet(c, jet_swp, sam2, D(0xccbbaa));
        // swp of 0 is 0.
        let sam3 = T(&mut c.stack, &[D(2), D(0)]);
        assert_jet(c, jet_swp, sam3, D(0));
    }
}
