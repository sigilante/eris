// `+scot` / `+slaw` text jets for Eris — render and parse `dime`s (aura-tagged
// atoms) as cords. nockvm leaves them unjetted; a stock k138 pill renders/parses
// `@ud`/`@ux`/`@p`/paths constantly.
//
// Like Vere (`pkg/noun/jets/e/{scot,slaw}.c`), these jet only the common auras
// and fall back (`Punt`) to the Nock for the rest — so partial coverage is
// always correct, never a wrong answer:
//   scot: `%tas %ud %ux %uv %uw`   (Vere's exact jetted set)
//   slaw: `%ud %tas`               (Vere also jets `%da`/`%p`, which need the
//                                    `+ob`/date Hoon helpers — we Punt those)
// Both are chapter-`qua` gates: path `[k138 one two tri qua <arm>]`. `scot` is
// unary (dime `[mod val]` at axis 6); `slaw` is binary (`mod` axis 12, `txt`
// axis 13). A rendered cord stores its first character in the low byte.

use ibig::UBig;
use nockvm::ext::AtomExt;
use nockvm::interpreter::Context;
use nockvm::jets::bits::util::met;
use nockvm::jets::util::slot;
use nockvm::jets::{JetErr, Result};
use nockvm::mem::NockStack;
use nockvm::noun::{Atom, Noun, D, T};

use super::{ob, po};

// `u3s_dit_y`: 0-9, a-z, A-Z, '-', '~' (64 digits; hex uses [0,16), uv [0,32)).
const DIT: &[u8; 64] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ-~";

fn ubig_u64(x: &UBig) -> u64 {
    let bytes = x.to_le_bytes();
    let mut v = 0u64;
    for (i, &b) in bytes.iter().take(8).enumerate() {
        v |= (b as u64) << (8 * i);
    }
    v
}

fn is_zero(x: &UBig) -> bool {
    *x == UBig::from(0u8)
}

/// Split `val` into little-endian `chunk_bits`-wide chunks (`chunk_bits <= 56`).
fn chunks(val: &UBig, chunk_bits: usize) -> Vec<u64> {
    let mut out = Vec::new();
    let mut a = val.clone();
    let mask = UBig::from((1u64 << chunk_bits) - 1);
    while !is_zero(&a) {
        out.push(ubig_u64(&(&a & &mask)));
        a >>= chunk_bits;
    }
    out
}

/// Append `n` base digits of `chunk` (each `bpd` bits), most-significant first,
/// zero-padded to exactly `n` (for a non-leading group).
fn push_padded(out: &mut Vec<u8>, chunk: u64, n: usize, bpd: usize) {
    let mask = (1u64 << bpd) - 1;
    for k in (0..n).rev() {
        out.push(DIT[((chunk >> (k * bpd)) & mask) as usize]);
    }
}

/// Append the digits of the top (most-significant) `chunk`, no leading zeros.
fn push_top(out: &mut Vec<u8>, chunk: u64, n: usize, bpd: usize) {
    let mask = (1u64 << bpd) - 1;
    let mut hi = n;
    while hi > 1 && ((chunk >> ((hi - 1) * bpd)) & mask) == 0 {
        hi -= 1;
    }
    for k in (0..hi).rev() {
        out.push(DIT[((chunk >> (k * bpd)) & mask) as usize]);
    }
}

/// `0x`/`0v`/`0w`-style base render, dot-grouped by `chunk_bits` (`digits`
/// base-`2^bpd` digits per group).
fn etch_based(val: &UBig, prefix: u8, chunk_bits: usize, digits: usize, bpd: usize) -> Vec<u8> {
    let mut out = vec![b'0', prefix];
    if is_zero(val) {
        out.push(b'0');
        return out;
    }
    let cs = chunks(val, chunk_bits);
    let n = cs.len();
    push_top(&mut out, cs[n - 1], digits, bpd);
    for i in (0..n - 1).rev() {
        out.push(b'.');
        push_padded(&mut out, cs[i], digits, bpd);
    }
    out
}

/// `+scot %ud`: decimal, dot-grouped in 3s from the right.
fn etch_ud(val: &UBig) -> Vec<u8> {
    if is_zero(val) {
        return vec![b'0'];
    }
    // groups of 3 decimal digits, least-significant first
    let mut groups: Vec<u64> = Vec::new();
    let mut a = val.clone();
    let k = UBig::from(1000u64);
    while !is_zero(&a) {
        groups.push(ubig_u64(&(&a % &k)));
        a /= &k;
    }
    let n = groups.len();
    let mut out = groups[n - 1].to_string().into_bytes(); // top group, no pad
    for i in (0..n - 1).rev() {
        out.push(b'.');
        out.extend_from_slice(format!("{:03}", groups[i]).as_bytes());
    }
    out
}

/// Read an atom's significant bytes (ASCII, reading order) into a `Vec<u8>`.
/// `as_ne_bytes` is word-padded, so trim to the `met 3` byte length.
fn atom_bytes(a: Atom, space: &nockvm::noun::NounSpace) -> Vec<u8> {
    let n = met(3, a, space);
    a.in_space(space).as_ne_bytes()[..n].to_vec()
}

fn cord(stack: &mut NockStack, s: &[u8]) -> Noun {
    Atom::from_bytes(stack, s).as_noun()
}

/// `+scot`: render `[mod val]`. Jets `%tas %ud %ux %uv %uw`, Punts the rest.
pub fn jet_scot(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let mod_atom = slot(sam, 2, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let val = slot(sam, 3, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let aura = atom_bytes(mod_atom, &space);

    if aura == b"tas" {
        // @tas renders as itself
        return Ok(val.as_noun());
    }
    let val_big = UBig::from_le_bytes(val.in_space(&space).as_ne_bytes());
    let s = match aura.as_slice() {
        b"ud" => etch_ud(&val_big),
        b"ux" => etch_based(&val_big, b'x', 16, 4, 4),
        b"uv" => etch_based(&val_big, b'v', 25, 5, 5),
        b"uw" => etch_based(&val_big, b'w', 30, 5, 6),
        _ => return Err(JetErr::Punt),
    };
    Ok(cord(&mut context.stack, &s))
}

/// `+slaw %ud`: parse the dot-grouped decimal that `+scot %ud` produces. Returns
/// `Some(value)` if well-formed, `None` if the shape is valid-but-unparseable
/// (`~`). The caller Punts on `len == 0`.
fn sift_ud(bytes: &[u8]) -> Option<UBig> {
    let len = bytes.len();
    if len == 0 {
        return None;
    }
    if bytes[0] == b'0' {
        return if len == 1 { Some(UBig::from(0u8)) } else { None };
    }
    let lead = len % 4;
    if lead == 0 {
        return None;
    }
    let thousand = UBig::from(1000u64);
    let mut val = UBig::from(0u8);
    for &c in &bytes[..lead] {
        if !c.is_ascii_digit() {
            return None;
        }
        val = &val * &UBig::from(10u8) + UBig::from((c - b'0') as u64);
    }
    let mut i = lead;
    while i < len {
        if bytes[i] != b'.' || i + 4 > len {
            return None;
        }
        let mut grp = 0u64;
        for &c in &bytes[i + 1..i + 4] {
            if !c.is_ascii_digit() {
                return None;
            }
            grp = grp * 10 + (c - b'0') as u64;
        }
        val = &val * &thousand + UBig::from(grp);
        i += 4;
    }
    Some(val)
}

/// `+slaw %tas`: validate a `@tas` (`[a-z]([a-z0-9-])*`). On success the value
/// is the cord itself.
fn sift_tas(bytes: &[u8]) -> bool {
    if bytes.is_empty() || !bytes[0].is_ascii_lowercase() {
        return false;
    }
    bytes[1..]
        .iter()
        .all(|&c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

// --- `slaw %p`: parse a `@p` ship name ----------------------------------------

/// Read a 3-lowercase-letter syllable at `*cur`, advancing on success.
fn read_syl(bytes: &[u8], cur: &mut usize) -> Option<[u8; 3]> {
    let c = *cur;
    if c + 3 <= bytes.len()
        && bytes[c].is_ascii_lowercase()
        && bytes[c + 1].is_ascii_lowercase()
        && bytes[c + 2].is_ascii_lowercase()
    {
        *cur += 3;
        Some([bytes[c], bytes[c + 1], bytes[c + 2]])
    } else {
        None
    }
}

/// Look up each syllable (galaxy = one suffix; otherwise prefix/suffix
/// alternating), Horner-combine (first syllable most-significant), and for
/// planets/moons (`do_fynd`) descramble via `+ob`. Returns `[~ value]`, or `~`
/// if any syllable is not valid. Mirrors Vere's `combine`/`_parse_p` tail.
fn finish_p(stack: &mut NockStack, syls: &[[u8; 3]], do_fynd: bool) -> Result {
    let n = syls.len();
    let mut value: u128 = 0;
    for (i, s) in syls.iter().enumerate() {
        let idx = if n == 1 {
            po::find_suffix(s)
        } else if i % 2 == 0 {
            po::find_prefix(s)
        } else {
            po::find_suffix(s)
        };
        match idx {
            Some(v) => value = value * 256 + v as u128,
            None => return Ok(D(0)), // not a syllable -> ~
        }
    }
    let out = if do_fynd {
        ob::fynd_value(value as u64) as u128
    } else {
        value
    };
    let val = Atom::from_bytes(stack, &out.to_le_bytes()).as_noun();
    Ok(T(stack, &[D(0), val]))
}

/// `+slaw %p`: parse `~zod` / `~marzod` / `~sampel-palnet` / moons / comets.
/// Structural failure Punts (`u3_none`); a non-syllable yields `~`. Ported from
/// Vere's `_parse_p`, reusing our `+po` and `+ob` jets.
fn parse_p(stack: &mut NockStack, bytes: &[u8]) -> Result {
    let n = bytes.len();
    if bytes.first() != Some(&b'~') {
        return Err(JetErr::Punt);
    }
    let mut cur = 1usize;
    let mut syls: Vec<[u8; 3]> = Vec::new();

    // `-` separator, or structural failure.
    macro_rules! dash {
        () => {{
            if bytes.get(cur) != Some(&b'-') {
                return Err(JetErr::Punt);
            }
            cur += 1;
        }};
    }
    // read one syllable (Punt if absent)
    macro_rules! syl {
        () => {{
            match read_syl(bytes, &mut cur) {
                Some(s) => syls.push(s),
                None => return Err(JetErr::Punt),
            }
        }};
    }

    syl!(); // a
    if cur >= n {
        return finish_p(stack, &syls, false); // galaxy
    }
    syl!(); // b
    if cur >= n {
        return finish_p(stack, &syls, false); // star
    }
    dash!();
    syl!();
    if cur >= n {
        return Err(JetErr::Punt);
    }
    syl!(); // c d
    if cur >= n {
        return finish_p(stack, &syls, true); // planet
    }
    dash!();
    syl!();
    if cur >= n {
        return Err(JetErr::Punt);
    }
    syl!(); // e f
    if cur >= n {
        return finish_p(stack, &syls, true); // short moon
    }
    dash!();
    syl!();
    if cur >= n {
        return Err(JetErr::Punt);
    }
    syl!(); // g h
    if cur >= n {
        return finish_p(stack, &syls, true); // long moon
    }
    // comet: `--` then four more dash-separated pairs
    dash!();
    dash!();
    for pair in 0..4 {
        syl!();
        if cur >= n {
            return Err(JetErr::Punt);
        }
        syl!();
        if pair < 3 {
            dash!();
        }
    }
    if cur < n {
        return Err(JetErr::Punt); // trailing junk
    }
    finish_p(stack, &syls, false) // comet (no descramble)
}

/// `+slaw`: parse `[mod txt]` → `(unit @)`. Jets `%ud %tas %p`, Punts the rest
/// (including `%da`, whose `@da` calendar math Vere delegates to a Hoon arm via
/// `u3j_cook` — nockvm has no `cook`, and reimplementing the calendar here is
/// disproportionate risk for a userspace parser, so `%da` runs correct-but-Nock).
pub fn jet_slaw(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let mod_atom = slot(sam, 2, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let txt = slot(sam, 3, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let aura = atom_bytes(mod_atom, &space);
    let bytes = atom_bytes(txt, &space);

    match aura.as_slice() {
        b"ud" => match sift_ud(&bytes) {
            None if bytes.is_empty() => Err(JetErr::Punt),
            None => Ok(D(0)), // ~ (valid shape, bad parse)
            Some(v) => {
                let val = Atom::from_ubig(&mut context.stack, &v).as_noun();
                Ok(T(&mut context.stack, &[D(0), val]))
            }
        },
        b"tas" => {
            if sift_tas(&bytes) {
                Ok(T(&mut context.stack, &[D(0), txt.as_noun()]))
            } else {
                Ok(D(0))
            }
        }
        b"p" => parse_p(&mut context.stack, &bytes),
        _ => Err(JetErr::Punt),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nockvm::jets::util::test::{assert_jet, assert_noun_eq, init_context};
    use nockvm::noun::{D, T};

    fn cord_n(stack: &mut NockStack, s: &[u8]) -> Noun {
        Atom::from_bytes(stack, s).as_noun()
    }

    /// scot sample is the dime `[mod val]`; assert_jet wraps it as [0 sam 0].
    fn scot(c: &mut Context, aura: &[u8], val: Noun, want: &[u8]) {
        let moda = cord_n(&mut c.stack, aura);
        let sam = T(&mut c.stack, &[moda, val]);
        let want_cord = cord_n(&mut c.stack, want);
        assert_jet(c, jet_scot, sam, want_cord);
    }

    #[test]
    fn test_scot_ud() {
        let c = &mut init_context();
        scot(c, b"ud", D(0), b"0");
        scot(c, b"ud", D(1), b"1");
        scot(c, b"ud", D(100), b"100");
        scot(c, b"ud", D(1000), b"1.000");
        scot(c, b"ud", D(65536), b"65.536");
        scot(c, b"ud", D(1_000_000), b"1.000.000");
    }

    #[test]
    fn test_scot_ux_uv_uw() {
        let c = &mut init_context();
        scot(c, b"ux", D(0), b"0x0");
        scot(c, b"ux", D(0xf), b"0xf");
        scot(c, b"ux", D(0xdead_beef), b"0xdead.beef");
        scot(c, b"uv", D(0), b"0v0");
        scot(c, b"uw", D(0), b"0w0");
        // @tas renders as itself
        let tas = cord_n(&mut c.stack, b"foo");
        scot(c, b"tas", tas, b"foo");
    }

    /// slaw round-trips what scot produced, for %ud.
    #[test]
    fn test_slaw_ud() {
        let c = &mut init_context();
        for &v in &[0u64, 1, 100, 1000, 65536, 1_000_000] {
            // render with scot
            let moda = cord_n(&mut c.stack, b"ud");
            let sam = T(&mut c.stack, &[moda, D(v)]);
            let subj = T(&mut c.stack, &[D(0), sam, D(0)]);
            let rendered = jet_scot(c, subj).unwrap();
            // parse it back with slaw -> [~ v]
            let moda2 = cord_n(&mut c.stack, b"ud");
            let slaw_sam = T(&mut c.stack, &[moda2, rendered]);
            let slaw_subj = T(&mut c.stack, &[D(0), slaw_sam, D(0)]);
            let got = jet_slaw(c, slaw_subj).unwrap();
            let want = T(&mut c.stack, &[D(0), D(v)]);
            assert_noun_eq(&mut c.stack, got, want);
        }
    }

    fn slaw_p(c: &mut Context, name: &[u8], want_val: Noun) {
        let moda = cord_n(&mut c.stack, b"p");
        let txt = cord_n(&mut c.stack, name);
        let sam = T(&mut c.stack, &[moda, txt]);
        let want = T(&mut c.stack, &[D(0), want_val]);
        assert_jet(c, jet_slaw, sam, want);
    }

    #[test]
    fn test_slaw_p() {
        let c = &mut init_context();
        // galaxies: suffix index
        slaw_p(c, b"~zod", D(0));
        slaw_p(c, b"~nec", D(1));
        // star ~marzod = prefix(mar=1)*256 + suffix(zod=0) = 256
        slaw_p(c, b"~marzod", D(256));
        // planet ~marnec-dozzod: combine [mar nec doz zod] = 16_842_752,
        // then descramble via +ob fynd.
        let raw = ((1u64 * 256 + 1) * 256 + 0) * 256 + 0;
        let want = D(ob::fynd_value(raw));
        slaw_p(c, b"~marnec-dozzod", want);
        // not a real syllable -> bare ~ (empty unit), not [~ 0]
        let moda = cord_n(&mut c.stack, b"p");
        let txt = cord_n(&mut c.stack, b"~xyzxyz");
        let sam = T(&mut c.stack, &[moda, txt]);
        assert_jet(c, jet_slaw, sam, D(0));
    }

    #[test]
    fn test_slaw_tas() {
        let c = &mut init_context();
        // valid @tas -> [~ txt]
        let txt = cord_n(&mut c.stack, b"foo-123");
        let moda = cord_n(&mut c.stack, b"tas");
        let sam = T(&mut c.stack, &[moda, txt]);
        let txt2 = cord_n(&mut c.stack, b"foo-123");
        let want = T(&mut c.stack, &[D(0), txt2]);
        assert_jet(c, jet_slaw, sam, want);
        // invalid (leading digit) -> ~
        let bad = cord_n(&mut c.stack, b"9ab");
        let moda2 = cord_n(&mut c.stack, b"tas");
        let sam2 = T(&mut c.stack, &[moda2, bad]);
        assert_jet(c, jet_slaw, sam2, D(0));
    }
}
