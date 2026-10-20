// `+ob` @p-obfuscation jets for Eris: `fein` (scramble) and `fynd` (unscramble).
//
// `+ob` is the Feistel cipher Arvo uses to spread adjacent planet/moon numbers
// across the `@p` name space (so neighbouring planets get unrelated names).
// nockvm leaves it unjetted; a stock k138 pill uses it for every planet/moon
// name. Ported from Vere's `pkg/noun/jets/e/{fein_ob,fynd_ob}.c` in nockvm's
// house style; the round (`+fe`/`+fen`) hash is murmur3 (the `murmur3` crate,
// same as `+muk`/`+mug`). `fein`/`fynd` are children of the `+ob` core, itself
// under chapter `tri`: path `[k138 one two tri ob <arm>]`, plain unary gates
// (`pyn` at axis 6).
//
// All arithmetic is over 32-bit halves (`c3_h` in Vere is 32-bit here); the
// intermediate products are held in `u64` to avoid overflow before the final
// cast, exactly as Vere's `(c3_d)` casts do.

use murmur3::murmur3_32_of_slice;
use nockvm::ext::AtomExt;
use nockvm::interpreter::Context;
use nockvm::jets::bits::util::met;
use nockvm::jets::util::slot;
use nockvm::jets::{JetErr, Result};
use nockvm::mem::NockStack;
use nockvm::noun::{Atom, Noun, D};

const A: u64 = 0xffff; // +feis:ob `a`
const B: u64 = 0x1_0000; // +feis:ob `b`
const K: u64 = 0xffff_0000; // +feis:ob `k`

// `+raku:ob` and its flop (`+fynd` runs the rounds in reverse).
const RAK: [u32; 4] = [0xb76d_5eed, 0xee28_1300, 0x85bc_ae01, 0x4b38_7af7];
const KAR: [u32; 4] = [0x4b38_7af7, 0x85bc_ae01, 0xee28_1300, 0xb76d_5eed];

/// `+fe:ob` — four Feistel rounds, correct over `[0x0, 0xfffe.ffff]`.
fn fe(m: u32) -> u32 {
    let mut l = (m as u64) % A;
    let mut r = (m as u64) / A;
    for (j, &seed) in RAK.iter().enumerate() {
        let k = [(r & 0xff) as u8, ((r >> 8) & 0xff) as u8];
        let f = murmur3_32_of_slice(&k, seed) as u64;
        let modulus = if j & 1 == 0 { A } else { B };
        let t = (f + l) % modulus;
        l = r;
        r = t;
    }
    // legendary @max19
    (if r == A { r * A + l } else { l * A + r }) as u32
}

/// `+feis:ob` — `+fe` with the `0x1.0000` offset, correct over `[0x1.0000,
/// 0xffff.ffff]`. Callers only reach this with `m >= 0x1.0000`.
fn feis(m: u32) -> u32 {
    let c = fe((m as u64 - B) as u32) as u64;
    (B + if c < K { c } else { fe(c as u32) as u64 }) as u32
}

/// `+fen:ob` — the inverse four rounds.
fn fen(m: u32) -> u32 {
    let mut l = (m as u64) / A;
    let mut r = (m as u64) % A;
    if l == A {
        std::mem::swap(&mut l, &mut r); // @max19
    }
    for (j, &seed) in KAR.iter().enumerate() {
        let k = [(l & 0xff) as u8, ((l >> 8) & 0xff) as u8];
        let f = murmur3_32_of_slice(&k, seed) as u64;
        let t = if j & 1 == 1 {
            ((r + A) - (f % A)) % A
        } else {
            ((r + B) - (f % B)) % B
        };
        r = l;
        l = t;
    }
    (r * A + l) as u32
}

/// `+tail:ob` — the inverse of `+feis`.
fn tail(m: u32) -> u32 {
    let c = fen((m as u64 - B) as u32) as u64;
    (B + if c < K { c } else { fen(c as u32) as u64 }) as u32
}

/// Shared `fein`/`fynd` dispatch: galaxies/stars (`sor < 2`) and comets
/// (`sor > 4`) pass through unchanged; planets (`sor == 2`) transform the whole
/// 32-bit value; moons (`sor` 3–4) transform only the low 32 bits, and only
/// when they are `>= 0x1.0000`.
fn ob(context: &mut Context, subject: Noun, round: fn(u32) -> u32) -> Result {
    let space = context.stack.noun_space();
    let pyn_noun = slot(subject, 6, &space)?;
    let pyn = pyn_noun.as_atom().map_err(|_| JetErr::Punt)?;
    let sor = met(4, pyn, &space);
    if !(2..=4).contains(&sor) {
        return Ok(pyn_noun);
    }
    let v = pyn.in_space(&space).as_u64().map_err(|_| JetErr::Punt)?;
    if sor == 2 {
        Ok(D(round(v as u32) as u64))
    } else {
        let lo = (v & 0xffff_ffff) as u32;
        if (lo as u64) < B {
            Ok(pyn_noun)
        } else {
            let out = (v & 0xffff_ffff_0000_0000) | (round(lo) as u64);
            Ok(atom_u64(&mut context.stack, out))
        }
    }
}

/// Build an atom from a `u64` (handles values above the direct-atom max).
fn atom_u64(stack: &mut NockStack, v: u64) -> Noun {
    Atom::from_bytes(stack, &v.to_le_bytes()).as_noun()
}

/// `+fynd:ob` on a `u64` value (`slaw %p` descrambles planet/moon names). Same
/// dispatch as the jet, computing `sor` (number of 16-bit shorts) from the
/// value directly. Values above 64 bits (comets) never reach here.
pub fn fynd_value(v: u64) -> u64 {
    let sor = (64 - v.leading_zeros() as usize).div_ceil(16);
    if !(2..=4).contains(&sor) {
        return v;
    }
    if sor == 2 {
        tail(v as u32) as u64
    } else {
        let lo = (v & 0xffff_ffff) as u32;
        if (lo as u64) < B {
            v
        } else {
            (v & 0xffff_ffff_0000_0000) | (tail(lo) as u64)
        }
    }
}

/// `~(fein ob a)`: scramble `pyn`.
pub fn jet_fein(context: &mut Context, subject: Noun) -> Result {
    ob(context, subject, feis)
}

/// `~(fynd ob a)`: unscramble `pyn`.
pub fn jet_fynd(context: &mut Context, subject: Noun) -> Result {
    ob(context, subject, tail)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nockvm::jets::util::test::{assert_jet, init_context};
    use nockvm::noun::{D, T};

    /// Build an atom from a `u64`, valid even above the direct-atom max.
    fn atom(stack: &mut NockStack, v: u64) -> Noun {
        Atom::from_bytes(stack, &v.to_le_bytes()).as_noun()
    }

    /// `fein` then `fynd` is the identity; small values (galaxies/stars) are
    /// unchanged by both.
    #[test]
    fn test_fein_fynd_roundtrip() {
        let c = &mut init_context();
        for &x in &[
            0u64,
            1,
            0xffff,             // star: sor < 2, unchanged
            0x1_0000,           // smallest planet
            0x1234_5678,        // planet
            0xffff_ffff,        // largest planet
            0x1_0000_0001_0000, // moon (low >= 0x1.0000)
            0xdead_beef_cafe_0000u64,
        ] {
            // Call fein directly: subject is [battery=0 sample=pyn context=0],
            // so pyn sits at axis 6. Build atoms via `atom` since moons exceed
            // the direct-atom max.
            let px = atom(&mut c.stack, x);
            let fein_subj = T(&mut c.stack, &[D(0), px, D(0)]);
            let scrambled = jet_fein(c, fein_subj).unwrap();
            let space = c.stack.noun_space();
            let sc = scrambled
                .as_atom()
                .unwrap()
                .in_space(&space)
                .as_u64()
                .unwrap();
            // fynd(fein(x)) == x. assert_jet wraps the sample into [0 sample 0].
            let sc_noun = atom(&mut c.stack, sc);
            let want = atom(&mut c.stack, x);
            assert_jet(c, jet_fynd, sc_noun, want);
        }
    }

    /// Galaxies/stars (<= 16 bits) are unchanged by both arms.
    #[test]
    fn test_small_unchanged() {
        let c = &mut init_context();
        for &x in &[0u64, 1, 255, 256, 0xffff] {
            assert_jet(c, jet_fein, D(x), D(x));
            assert_jet(c, jet_fynd, D(x), D(x));
        }
    }
}
