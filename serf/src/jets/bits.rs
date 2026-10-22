// Standard atom/bit helper jets for Eris.
//
// nockvm's stock dashboard jets the hot bit arms (`bex can cat cut end lsh met
// rap rep rev rip rsh xeb con dis mix`) but leaves several chapter-`two` helpers
// unjetted. A stock k138 Urbit pill uses them, so we add them here, modeled on
// Vere's C jets (`pkg/noun/jets/c/{swp,pow,aor,muk}.c`, `e/{mat,rub}.c`) and
// hoon.hoon, in nockvm's house style. Registered from the serf (see
// `super::eris_hot_state`) so nockvm's own dashboard stays unpatched; pinned to
// Hoon kelvin 138.
//
// Arms: `swp pow aor mat muk rub`. All plain gates in chapter `two`; each jet
// reads its sample at axis 6 and the arguments at axes 2/3 (and 6/7 for the
// 3-ary `muk`). `mat`/`rub` do arbitrary-precision bit math via ibig (the same
// fork nockvm uses); `muk` is murmur3 (the hash behind `+mug`), zero-padding the
// key to `len` as Vere does.

use ibig::UBig;
use murmur3::murmur3_32_of_slice;
use nockvm::interpreter::Context;
use nockvm::jets::bits::rep;
use nockvm::jets::bits::util::{met, rip};
use nockvm::jets::util::slot;
use nockvm::jets::{JetErr, Result};
use nockvm::mem::NockStack;
use nockvm::noun::{Atom, Noun, NounSpace, D, NO, T, YES};
use nockvm::unifying_equality::unifying_equality;

type JetResult<T> = std::result::Result<T, JetErr>;

// --- small shared helpers ---------------------------------------------------

#[inline(always)]
fn loob(b: bool) -> Noun {
    if b {
        YES
    } else {
        NO
    }
}

/// Unifying structural equality (stock `=`), as in map.rs.
fn noun_eq(stack: &mut NockStack, a: Noun, b: Noun) -> bool {
    let mut a = a;
    let mut b = b;
    unsafe { unifying_equality(stack, &mut a as *mut Noun, &mut b as *mut Noun) }
}

// All `UBig` intermediates below are built with the `_stack` constructors so
// their backing words live on the NockStack arena and are reclaimed when the
// event's frame pops. ibig's plain operators (`<<`, `&`, `+`, …) allocate their
// result `Buffer` on the *system* heap as a `ManuallyDrop<Vec>` that is never
// freed (see `ibig/src/buffer.rs`), so using them here would leak a little heap
// on every call — unbounded over a long-running serf. The arithmetic (and thus
// the bit-exact result) is identical either way; only the allocation site moves.

/// `b & ((1 << width) - 1)` — the low `width` bits (0 when `width` is 0).
///
/// Computed by byte-truncation rather than by building a `(1<<width)-1` mask, so
/// no giant mask is ever materialized. When `width` reaches or exceeds the bit
/// length of `x` the low bits *are* all of `x`, so we return it unchanged — this
/// is what stops `+rub` from allocating a multi-GiB mask to extract a value
/// whose high bits are all zero (dwarf's `rub` allocation-bomb finding).
fn low_bits(stack: &mut NockStack, x: &UBig, width: usize) -> UBig {
    if width == 0 {
        return UBig::from_unsigned_stack(stack, 0u64);
    }
    if width >= x.bit_len() {
        return x.clone_stack(stack);
    }
    let bytes = x.to_le_bytes_stack();
    let full = width / 8;
    let rem = width % 8;
    let take = (full + usize::from(rem > 0)).min(bytes.len());
    let mut buf = bytes[..take].to_vec();
    if rem > 0 && full < buf.len() {
        buf[full] &= (1u8 << rem) - 1;
    }
    UBig::from_le_bytes_stack(stack, &buf)
}

/// `cut [0 [off width]]`: `width` bits of `x` starting at bit `off`.
fn cut_bits(stack: &mut NockStack, x: &UBig, off: usize, width: usize) -> UBig {
    let shifted = UBig::shr_ref_stack(stack, x, off);
    low_bits(stack, &shifted, width)
}

/// Trailing-zero count of a nonzero UBig (its lowest set-bit position).
fn trailing_zeros(x: &UBig) -> usize {
    let bytes = x.to_le_bytes_stack();
    let mut tz = 0usize;
    for &byte in bytes {
        if byte == 0 {
            tz += 8;
        } else {
            tz += byte.trailing_zeros() as usize;
            break;
        }
    }
    tz
}

/// `Some(n)` if `x` fits a `usize`.
fn ubig_usize(x: &UBig) -> Option<usize> {
    let bytes = x.to_le_bytes_stack();
    if bytes.len() > std::mem::size_of::<usize>() {
        return None;
    }
    let mut v: u64 = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        v |= (byte as u64) << (8 * i);
    }
    usize::try_from(v).ok()
}

fn ubig_noun(stack: &mut NockStack, x: &UBig) -> Noun {
    Atom::from_ubig(stack, x).as_noun()
}



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
/// let the (unreachable in practice) Nock handle it. We cap `bloq` well below
/// the word size: `rip`/`rep` work in `2^bloq`-bit blocks, so even a tiny `b`
/// forces a `2^bloq`-bit allocation (dwarf's `swp(28, 5)` NockStack-OOM). A real
/// `+swp` never uses a block wider than a few dozen bits; `bloq >= 20` (a 1-Mib
/// block) is already far past any legitimate use, so we Punt there.
const SWP_BLOQ_MAX: usize = 20;
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
    if bloq >= SWP_BLOQ_MAX {
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
    // Cap the result size: `a^b` has ~`bit_len(a) * b` bits. Without a guard a
    // modest sample like `(pow 2 2^36)` asks for a gibibyte-scale atom and aborts
    // the NockStack (dwarf's `pow` OOM). Punt past ~128 Mib of output and let the
    // Nock run it — which, for a genuinely that-large power, also fails, but in
    // the kernel's own accounting rather than as a jet-induced process abort.
    // (`bit_len(a) <= 1` means base 0 or 1: the result is 0/1, never large.)
    const POW_BITS_MAX: u128 = 1 << 27;
    let base_bits = base.bit_len();
    if base_bits > 1 && (base_bits as u128).saturating_mul(exp as u128) > POW_BITS_MAX {
        return Err(JetErr::Punt);
    }
    let res = base.pow_stack(stack, exp);
    Ok(Atom::from_ubig(stack, &res).as_noun())
}

/// `+aor`: a-order — total order on nouns by raw (little-endian byte) order,
/// recursing head-then-tail into cells; atoms sort before cells. hoon:
/// `|= [a=* b=*]`, cribbed from Vere's `u3qc_aor`.
pub fn jet_aor(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let mut a = slot(sam, 2, &space)?;
    let mut b = slot(sam, 3, &space)?;
    loop {
        if noun_eq(&mut context.stack, a, b) {
            return Ok(YES);
        }
        if !a.is_atom() {
            // a is a cell
            if b.is_atom() {
                return Ok(NO);
            }
            let ha = slot(a, 2, &space)?;
            let hb = slot(b, 2, &space)?;
            if noun_eq(&mut context.stack, ha, hb) {
                a = slot(a, 3, &space)?;
                b = slot(b, 3, &space)?;
            } else {
                a = ha;
                b = hb;
            }
        } else {
            // a is an atom
            if !b.is_atom() {
                return Ok(YES);
            }
            let aa = a.as_atom().map_err(|_| JetErr::Punt)?;
            let ba = b.as_atom().map_err(|_| JetErr::Punt)?;
            let la = met(3, aa, &space);
            let lb = met(3, ba, &space);
            let ah = aa.in_space(&space);
            let bh = ba.in_space(&space);
            let abuf = ah.as_ne_bytes();
            let bbuf = bh.as_ne_bytes();
            for i in 0..la.min(lb) {
                if abuf[i] != bbuf[i] {
                    return Ok(loob(abuf[i] < bbuf[i]));
                }
            }
            return Ok(loob(la < lb));
        }
    }
}

/// `+mat`: jam length-encode of `a` → `[p=bit-width q=encoding]`. hoon:
/// `|= a=@`, cribbed from Vere's `u3qe_mat`.
pub fn jet_mat(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let a = slot(subject, 6, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let stack = &mut context.stack;
    let a_big = UBig::from_le_bytes_stack(stack, a.in_space(&space).as_ne_bytes());

    if a_big.bit_len() == 0 {
        return Ok(T(stack, &[D(1), D(1)]));
    }

    let b = a_big.bit_len() as u64; // (met 0 a)
    let c = (64 - b.leading_zeros()) as usize; // (met 0 b)
    let u = c - 1;

    // w = (bex c): a single bit at position c.
    let one = UBig::from_unsigned_stack(stack, 1u64);
    let w = UBig::shl_stack(stack, one, c);
    // x = (end [0 (dec c)] b): the low `u` bits of `b`.
    let b_big = UBig::from_unsigned_stack(stack, b);
    let x = low_bits(stack, &b_big, u);
    // y = (lsh [0 (dec c)] a): `a` shifted up by `u`, so its low `u` bits are 0.
    let y = UBig::shl_ref_stack(stack, &a_big, u);
    // z = (mix x y): x occupies bits [0,u), y bits [u,..) — disjoint, so mix == add.
    let z = UBig::add_stack(stack, x, y);
    // (cat 0 w z): w occupies bit c, z starts at bit c+1 — disjoint, so cat == add.
    let z_hi = UBig::shl_stack(stack, z, c + 1);
    let q = UBig::add_stack(stack, w, z_hi);

    let p = 2 * (c as u64) + b; // (add (add c c) b)
    let p_noun = Atom::new(stack, p).as_noun();
    let q_noun = ubig_noun(stack, &q);
    Ok(T(stack, &[p_noun, q_noun]))
}

/// `+rub`: jam length-decode — read a length-prefixed atom from bitstream `b`
/// at bit offset `a`, returning `[p=bits-consumed q=value]`. hoon:
/// `|= [a=@ b=@]`, cribbed from Vere's `u3qe_rub`.
pub fn jet_rub(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let a = slot(sam, 2, &space)?
        .as_atom()
        .map_err(|_| JetErr::Punt)?
        .in_space(&space)
        .as_u64()
        .map_err(|_| JetErr::Punt)? as usize;
    let b_atom = slot(sam, 3, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let stack = &mut context.stack;
    let b_big = UBig::from_le_bytes_stack(stack, b_atom.in_space(&space).as_ne_bytes());

    // Count leading zeros from bit `a` to the first 1 bit. If none exists at or
    // above `a`, rub would read past the stream and crash — Punt to the Nock.
    let shifted = UBig::shr_ref_stack(stack, &b_big, a);
    if shifted.bit_len() == 0 {
        return Err(JetErr::Punt);
    }
    let c = trailing_zeros(&shifted); // zeros from `a` to the terminator
    if c == 0 {
        // terminator at bit `a`: the zero-length encoding
        return Ok(T(stack, &[D(1), D(0)]));
    }
    let d = a + c + 1; // bit after the terminator
    let cm1 = c - 1;

    let z = cut_bits(stack, &b_big, d, cm1); // (cut 0 [d (dec c)] b)
    // e_big = (add (bex (dec c)) z): bex's bit is at cm1, z < 2^cm1 — disjoint, add == or.
    let one = UBig::from_unsigned_stack(stack, 1u64);
    let bex = UBig::shl_stack(stack, one, cm1);
    let e_big = UBig::add_stack(stack, bex, z);
    // `cut_bits` below caps a width that exceeds the source's bit length, so an
    // `e` that fits `usize` but dwarfs `b` no longer materializes a giant mask;
    // but a non-`usize` `e` still can't index a cut, so Punt there.
    let e = ubig_usize(&e_big).ok_or(JetErr::Punt)?;

    let two_c = UBig::from_unsigned_stack(stack, 2u64 * c as u64);
    let p = UBig::add_stack(stack, two_c, e_big); // (add (add c c) e)
    let q = cut_bits(stack, &b_big, d + cm1, e); // (cut 0 [(add d (dec c)) e] b)

    let p_noun = ubig_noun(stack, &p);
    let q_noun = ubig_noun(stack, &q);
    Ok(T(stack, &[p_noun, q_noun]))
}

/// `+muk`: standard murmur3 — `MurmurHash3_x86_32(key padded to `len` bytes,
/// seed = low 32 bits of `syd`)`. hoon: `|= [syd=@ len=@ key=@]`, cribbed from
/// Vere's `u3qc_muk` (same hash as `+mug`, via the `murmur3` crate).
pub fn jet_muk(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let syd = slot(sam, 2, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let len = slot(sam, 6, &space)?
        .as_atom()
        .map_err(|_| JetErr::Punt)?
        .in_space(&space)
        .as_u64()
        .map_err(|_| JetErr::Punt)? as usize;
    let key = slot(sam, 7, &space)?.as_atom().map_err(|_| JetErr::Punt)?;

    // seed = (end 5 syd) = low 32 bits of syd
    let seed = {
        let h = syd.in_space(&space);
        let bytes = h.as_ne_bytes();
        let mut s = 0u32;
        for i in 0..4 {
            if i < bytes.len() {
                s |= (bytes[i] as u32) << (8 * i);
            }
        }
        s
    };

    let key_len = met(3, key, &space);
    // `len < key length` is a hard error in Vere; and don't pad to an absurd
    // size. Either way, Punt to the Nock.
    if key_len > len || (len > key_len && len > (1usize << 24)) {
        return Err(JetErr::Punt);
    }

    let mut buf = vec![0u8; len];
    if key_len > 0 {
        let kh = key.in_space(&space);
        let kb = kh.as_ne_bytes();
        let n = key_len.min(kb.len()).min(len);
        buf[..n].copy_from_slice(&kb[..n]);
    }
    let h = murmur3_32_of_slice(&buf, seed);
    Ok(D(h as u64))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nockvm::jets::util::test::{assert_jet, assert_noun_eq, init_context};
    use nockvm::noun::{D, T};

    #[test]
    fn test_aor() {
        let c = &mut init_context();
        // atoms compare low-byte-first; 1 < 2.
        let s = T(&mut c.stack, &[D(1), D(2)]);
        assert_jet(c, jet_aor, s, YES);
        let s = T(&mut c.stack, &[D(2), D(1)]);
        assert_jet(c, jet_aor, s, NO);
        let s = T(&mut c.stack, &[D(5), D(5)]);
        assert_jet(c, jet_aor, s, YES);
        // 256 = [00 01] little-endian: low byte 0 < 1, so 256 sorts before 1.
        let s = T(&mut c.stack, &[D(256), D(1)]);
        assert_jet(c, jet_aor, s, YES);
        // an atom sorts before any cell.
        let cell = T(&mut c.stack, &[D(1), D(2)]);
        let s = T(&mut c.stack, &[D(9), cell]);
        assert_jet(c, jet_aor, s, YES);
        // cells: equal heads -> compare tails (2 < 3).
        let l = T(&mut c.stack, &[D(1), D(2)]);
        let r = T(&mut c.stack, &[D(1), D(3)]);
        let s = T(&mut c.stack, &[l, r]);
        assert_jet(c, jet_aor, s, YES);
    }

    #[test]
    fn test_mat_rub_roundtrip() {
        let c = &mut init_context();
        // mat(0) = [1 1]; and for any a, rub(0, (mat a).q) = [(mat a).p a].
        let want0 = T(&mut c.stack, &[D(1), D(1)]);
        assert_jet(c, jet_mat, D(0), want0);

        for &a in &[0u64, 1, 2, 5, 100, 255, 256, 65_536, 1_000_000, 0xdead_beef] {
            // p = mat(a) via the jet
            let pq = {
                let subj = T(&mut c.stack, &[D(0), D(a), D(0)]);
                jet_mat(c, subj).unwrap()
            };
            let p = slot(pq, 2, &c.stack.noun_space()).unwrap();
            let q = slot(pq, 3, &c.stack.noun_space()).unwrap();
            // rub(0, q) must be [p a]
            let rub_sam = T(&mut c.stack, &[D(0), q]);
            let rub_subj = T(&mut c.stack, &[D(0), rub_sam, D(0)]);
            let got = jet_rub(c, rub_subj).unwrap();
            let want = T(&mut c.stack, &[p, D(a)]);
            assert_noun_eq(&mut c.stack, got, want);
        }
    }

    #[test]
    fn test_muk() {
        let c = &mut init_context();
        // key 0x636261 = bytes [61 62 63] ("abc" LE); muk must match a direct
        // murmur3 of those 3 bytes with seed = low 32 bits of syd.
        let seed = 0xcafe_babeu32;
        let key = 0x0063_6261u64;
        let sam = T(&mut c.stack, &[D(seed as u64), D(3), D(key)]);
        let want = murmur3_32_of_slice(&[0x61, 0x62, 0x63], seed) as u64;
        assert_jet(c, jet_muk, sam, D(want));
        // zero-padding: len 5 over a 3-byte key = murmur of [61 62 63 00 00].
        let sam = T(&mut c.stack, &[D(seed as u64), D(5), D(key)]);
        let want = murmur3_32_of_slice(&[0x61, 0x62, 0x63, 0, 0], seed) as u64;
        assert_jet(c, jet_muk, sam, D(want));
    }

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

    // --- regression tests for the memory-safety fixes -----------------------

    #[test]
    fn test_rub_no_alloc_bomb() {
        let c = &mut init_context();
        // b = 2^40 at offset 0. A naive `(1<<e)-1` mask would allocate a ~2^39-bit
        // (≈68 GiB) value to extract bits that are all zero (dwarf's rub bomb).
        // With the byte-truncating cut the extracted value is just 0, returned
        // instantly.
        let sam = T(&mut c.stack, &[D(0), D(1u64 << 40)]);
        let subj = T(&mut c.stack, &[D(0), sam, D(0)]);
        let got = jet_rub(c, subj).unwrap();
        let q = slot(got, 3, &c.stack.noun_space()).unwrap();
        assert!(unsafe { q.raw_equals(&D(0)) }, "rub q must be 0, no giant mask");
    }

    #[test]
    fn test_pow_caps_huge_exponent() {
        let c = &mut init_context();
        // 2^(2^36): output ~2^37 bits, far over the cap → Punt (not a NockStack OOM).
        let sam = T(&mut c.stack, &[D(2), D(1u64 << 36)]);
        let subj = T(&mut c.stack, &[D(0), sam, D(0)]);
        assert!(matches!(jet_pow(c, subj), Err(JetErr::Punt)));
        // a small power still jets.
        let ok = T(&mut c.stack, &[D(2), D(10)]);
        assert_jet(c, jet_pow, ok, D(1024));
    }

    #[test]
    fn test_swp_caps_huge_bloq() {
        let c = &mut init_context();
        // bloq 28 forces a 2^28-bit block even for a tiny `b` → Punt.
        let sam = T(&mut c.stack, &[D(28), D(5)]);
        let subj = T(&mut c.stack, &[D(0), sam, D(0)]);
        assert!(matches!(jet_swp(c, subj), Err(JetErr::Punt)));
    }
}
