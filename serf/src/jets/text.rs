// `+scot` / `+slaw` text jets for Eris — render and parse `dime`s (aura-tagged
// atoms) as cords. nockvm leaves them unjetted; a stock k138 pill renders/parses
// `@ud`/`@ux`/`@p`/paths constantly.
//
// Like Vere (`pkg/noun/jets/e/{scot,slaw}.c`), these jet only the common auras
// and fall back (`Punt`) to the Nock for the rest — so partial coverage is
// always correct, never a wrong answer:
//   scot: `%tas %ud %ux %uv %uw`   (Vere's exact jetted set)
//   slaw: `%ud %tas %p %da`        (Vere's set; `%p` composes our `+po`/`+ob`
//                                    jets and `%da` ports Vere's `+year` date
//                                    math, since nockvm has no `cook` to slam
//                                    the Hoon helper. Any other aura Punts.)
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

// As in bits.rs, every `UBig` intermediate is built with the `_stack`
// constructors so its words land on the NockStack arena and are reclaimed with
// the event's frame; ibig's plain operators would leak a never-freed heap
// `Buffer` on each call. The arithmetic — and the rendered bytes — are identical.

fn ubig_u64(x: &UBig) -> u64 {
    let bytes = x.to_le_bytes_stack();
    let mut v = 0u64;
    for (i, &b) in bytes.iter().take(8).enumerate() {
        v |= (b as u64) << (8 * i);
    }
    v
}

fn is_zero(x: &UBig) -> bool {
    x.bit_len() == 0
}

/// Split `val` into little-endian `chunk_bits`-wide chunks (`chunk_bits <= 56`).
/// The chunk fits a `u64`, so we mask the low word directly instead of building
/// a `UBig` mask.
fn chunks(stack: &mut NockStack, val: &UBig, chunk_bits: usize) -> Vec<u64> {
    let mask = (1u64 << chunk_bits) - 1;
    let mut out = Vec::new();
    let mut a = val.clone_stack(stack);
    while !is_zero(&a) {
        out.push(ubig_u64(&a) & mask);
        a = UBig::shr_stack(stack, a, chunk_bits);
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
fn etch_based(
    stack: &mut NockStack,
    val: &UBig,
    prefix: u8,
    chunk_bits: usize,
    digits: usize,
    bpd: usize,
) -> Vec<u8> {
    let mut out = vec![b'0', prefix];
    if is_zero(val) {
        out.push(b'0');
        return out;
    }
    let cs = chunks(stack, val, chunk_bits);
    let n = cs.len();
    push_top(&mut out, cs[n - 1], digits, bpd);
    for i in (0..n - 1).rev() {
        out.push(b'.');
        push_padded(&mut out, cs[i], digits, bpd);
    }
    out
}

/// `+scot %ud`: decimal, dot-grouped in 3s from the right.
fn etch_ud(stack: &mut NockStack, val: &UBig) -> Vec<u8> {
    if is_zero(val) {
        return vec![b'0'];
    }
    // groups of 3 decimal digits, least-significant first
    let mut groups: Vec<u64> = Vec::new();
    let mut a = val.clone_stack(stack);
    while !is_zero(&a) {
        let k = UBig::from_unsigned_stack(stack, 1000u64);
        let (q, r) = UBig::div_rem_stack(stack, a, k);
        groups.push(ubig_u64(&r));
        a = q;
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
    let stack = &mut context.stack;
    let val_big = UBig::from_le_bytes_stack(stack, val.in_space(&space).as_ne_bytes());
    let s = match aura.as_slice() {
        b"ud" => etch_ud(stack, &val_big),
        b"ux" => etch_based(stack, &val_big, b'x', 16, 4, 4),
        b"uv" => etch_based(stack, &val_big, b'v', 25, 5, 5),
        b"uw" => etch_based(stack, &val_big, b'w', 30, 5, 6),
        _ => return Err(JetErr::Punt),
    };
    Ok(cord(stack, &s))
}

/// `+slaw %ud`: parse the dot-grouped decimal that `+scot %ud` produces. Returns
/// `Some(value)` if well-formed, `None` if the shape is valid-but-unparseable
/// (`~`). The caller Punts on `len == 0`.
fn sift_ud(stack: &mut NockStack, bytes: &[u8]) -> Option<UBig> {
    let len = bytes.len();
    if len == 0 {
        return None;
    }
    if bytes[0] == b'0' {
        return if len == 1 {
            Some(UBig::from_unsigned_stack(stack, 0u64))
        } else {
            None
        };
    }
    let lead = len % 4;
    if lead == 0 {
        return None;
    }
    let mut val = UBig::from_unsigned_stack(stack, 0u64);
    for &c in &bytes[..lead] {
        if !c.is_ascii_digit() {
            return None;
        }
        let ten = UBig::from_unsigned_stack(stack, 10u64);
        val = UBig::mul_stack(stack, val, ten);
        let d = UBig::from_unsigned_stack(stack, (c - b'0') as u64);
        val = UBig::add_stack(stack, val, d);
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
        let thousand = UBig::from_unsigned_stack(stack, 1000u64);
        val = UBig::mul_stack(stack, val, thousand);
        let g = UBig::from_unsigned_stack(stack, grp);
        val = UBig::add_stack(stack, val, g);
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

// --- `slaw %da`: parse an absolute `@da` date -------------------------------
//
// Vere parses the string in C then slams the Hoon `+year` arm (via `u3j_cook`)
// for the `@da` math; nockvm has no `cook`, so we port `+year`/`+yawn`/`+yule`
// (and `+yo`/`+yelp`) directly. `@da` = `(sec << 64) | frac`, where `sec` is
// seconds since the Urbit epoch and `frac` is the MSB-aligned sub-second.

const EPOCH: u128 = 292_277_024_400;
const MOH: [u128; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const MOY: [u128; 12] = [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];

/// `+yelp`: is `yer` a leap year?
fn yelp(yer: u128) -> bool {
    yer.is_multiple_of(4) && (!yer.is_multiple_of(100) || yer.is_multiple_of(400))
}

/// `+yawn`: absolute day number for `[yer mot day]` (1-indexed month/day).
fn yawn(mut yer: u128, mot: u128, day_in: u128) -> u128 {
    let mut day = day_in - 1;
    let cah = if yelp(yer) { MOY } else { MOH };
    for &len in cah.iter().take((mot - 1) as usize) {
        day += len;
    }
    loop {
        if !yer.is_multiple_of(4) {
            let ney = yer - 1;
            day += if yelp(ney) { 366 } else { 365 };
            yer = ney;
        } else if !yer.is_multiple_of(100) {
            let nef = yer - 4;
            day += if yelp(nef) { 1_461 } else { 1_460 };
            yer = nef;
        } else if !yer.is_multiple_of(400) {
            let nec = yer - 100;
            day += if yelp(nec) { 36_525 } else { 36_524 };
            yer = nec;
        } else {
            return day + (yer / 400) * 146_097;
        }
    }
}

/// `+year`: `[ad year month day h m s frac]` → `@da`. `None` if out of range.
#[allow(clippy::too_many_arguments)]
fn year_to_da(
    ad: bool,
    year: u128,
    month: u128,
    day: u128,
    h: u128,
    m: u128,
    s: u128,
    frac: &[u128],
) -> Option<u128> {
    if !(1..=12).contains(&month) || day < 1 || frac.len() > 4 {
        return None;
    }
    // Bound the year so every step below stays within `u128` and the `sec << 64`
    // packing cannot silently truncate. `@da`'s practical range is comfortably
    // inside this; any date past it Punts and the Nock's bignum `+year` runs it.
    // (dwarf's finding: without this, a huge year overflows `u128` in release —
    // wrapping to a *wrong* @da — or panics in debug.)
    const YEAR_MAX: u128 = 1_000_000_000_000; // 10^12
    if year > YEAR_MAX {
        return None;
    }
    let yer = if ad {
        EPOCH + year
    } else {
        EPOCH.checked_sub(year.checked_sub(1)?)?
    };
    let sec = yawn(yer, month, day)
        .checked_mul(86_400)?
        .checked_add(h.checked_mul(3_600)?)?
        .checked_add(m.checked_mul(60)?)?
        .checked_add(s)?;
    // `@da` holds seconds in bits [64, 128): a `sec` that does not fit 64 bits
    // would be truncated by `sec << 64`, so Punt and let the Nock's wide `@da`
    // represent it exactly.
    if sec >> 64 != 0 {
        return None;
    }
    let mut fac: u128 = 0;
    for (i, &w) in frac.iter().enumerate() {
        fac |= w << ((3 - i) * 16);
    }
    Some((sec << 64) | fac)
}

fn parse_num(bytes: &[u8], cur: &mut usize, allow_zero_lead: bool) -> Option<u128> {
    let d0 = *bytes.get(*cur)?;
    if !d0.is_ascii_digit() || (!allow_zero_lead && d0 == b'0') {
        return None;
    }
    let mut v = (d0 - b'0') as u128;
    *cur += 1;
    while let Some(&d) = bytes.get(*cur) {
        if !d.is_ascii_digit() {
            break;
        }
        // Checked: a date string with enough digits would overflow `u128` and
        // (in release) wrap to a wrong value. `None` here Punts to the Nock.
        v = v.checked_mul(10)?.checked_add((d - b'0') as u128)?;
        *cur += 1;
    }
    Some(v)
}

fn parse_month(bytes: &[u8], cur: &mut usize) -> Option<u128> {
    let d0 = *bytes.get(*cur)?;
    if d0 == b'1' {
        if let Some(&d1) = bytes.get(*cur + 1) {
            if (b'0'..=b'2').contains(&d1) {
                *cur += 2;
                return Some(10 + (d1 - b'0') as u128);
            }
        }
        *cur += 1;
        Some(1)
    } else if (b'2'..=b'9').contains(&d0) {
        *cur += 1;
        Some((d0 - b'0') as u128)
    } else {
        None
    }
}

fn hex_digit(b: Option<&u8>) -> Option<u128> {
    match b {
        Some(&c) if c.is_ascii_digit() => Some((c - b'0') as u128),
        Some(&c) if (b'a'..=b'f').contains(&c) => Some((10 + c - b'a') as u128),
        _ => None,
    }
}

/// `+slaw %da`: parse `~292277024401-.1.1` / `...~2000.1.1` / with optional
/// `..hh.mm.ss` time and `..ffff.ffff` sub-second. Any structural failure Punts
/// (`u3_none`); on success returns `[~ @da]`. Ported from Vere's `_parse_da`.
fn parse_da(stack: &mut NockStack, bytes: &[u8]) -> Result {
    if bytes.first() != Some(&b'~') {
        return Err(JetErr::Punt);
    }
    let mut cur = 1usize; // past '~'
    macro_rules! dot {
        () => {{
            if bytes.get(cur) != Some(&b'.') {
                return Err(JetErr::Punt);
            }
            cur += 1;
        }};
    }

    let year = parse_num(bytes, &mut cur, false).ok_or(JetErr::Punt)?;
    let ad = if bytes.get(cur) == Some(&b'-') {
        cur += 1;
        false
    } else {
        true
    };
    dot!();
    let month = parse_month(bytes, &mut cur).ok_or(JetErr::Punt)?;
    dot!();
    let day = parse_num(bytes, &mut cur, false).ok_or(JetErr::Punt)?;

    let (mut h, mut m, mut s) = (0u128, 0u128, 0u128);
    let mut frac: Vec<u128> = Vec::new();
    if cur < bytes.len() {
        dot!();
        dot!(); // `..`
        h = parse_num(bytes, &mut cur, true).ok_or(JetErr::Punt)?;
        dot!();
        m = parse_num(bytes, &mut cur, true).ok_or(JetErr::Punt)?;
        dot!();
        s = parse_num(bytes, &mut cur, true).ok_or(JetErr::Punt)?;
        if cur < bytes.len() {
            dot!();
            dot!(); // `..`
            loop {
                let mut w: u128 = 0;
                for _ in 0..4 {
                    w = w * 16 + hex_digit(bytes.get(cur)).ok_or(JetErr::Punt)?;
                    cur += 1;
                }
                frac.push(w);
                if cur >= bytes.len() {
                    break;
                }
                dot!();
            }
        }
    }

    let da = year_to_da(ad, year, month, day, h, m, s, &frac).ok_or(JetErr::Punt)?;
    let val = Atom::from_bytes(stack, &da.to_le_bytes()).as_noun();
    Ok(T(stack, &[D(0), val]))
}

/// `+slaw`: parse `[mod txt]` → `(unit @)`. Jets `%ud %tas %p %da`; Punts any
/// other aura (runs correct-but-unjetted).
pub fn jet_slaw(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space)?;
    let mod_atom = slot(sam, 2, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let txt = slot(sam, 3, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    let aura = atom_bytes(mod_atom, &space);
    let bytes = atom_bytes(txt, &space);

    match aura.as_slice() {
        b"ud" => match sift_ud(&mut context.stack, &bytes) {
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
        b"da" => parse_da(&mut context.stack, &bytes),
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
    fn test_slaw_da() {
        let c = &mut init_context();
        let atom128 = |stack: &mut NockStack, v: u128| Atom::from_bytes(stack, &v.to_le_bytes()).as_noun();
        let check = |c: &mut Context, s: &[u8], da: u128| {
            let moda = cord_n(&mut c.stack, b"da");
            let txt = cord_n(&mut c.stack, s);
            let sam = T(&mut c.stack, &[moda, txt]);
            let val = atom128(&mut c.stack, da);
            let want = T(&mut c.stack, &[D(0), val]);
            assert_jet(c, jet_slaw, sam, want);
        };
        // The Urbit epoch ~292277024401-.1.1 (BC) is @da 0.
        check(c, b"~292277024401-.1.1", 0);
        // one second later
        check(c, b"~292277024401-.1.1..00.00.01", 1u128 << 64);
        // sub-second: one 16-bit group is MSB-aligned at bit 48
        check(c, b"~292277024401-.1.1..00.00.00..8000", 0x8000u128 << 48);
        // a day later -> 86400 seconds
        check(c, b"~292277024401-.1.2", 86_400u128 << 64);
        // bad shape (missing day) Punts -> falls back to Nock (not asserted here)
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

    #[test]
    fn test_slaw_da_huge_year_punts() {
        let c = &mut init_context();
        // A 16-digit year is past @da's fixed-width range: the old code wrapped
        // `u128`/truncated `sec << 64` to a *wrong* @da (dwarf's finding). It must
        // Punt and let the Nock's wide bignum math run it.
        let moda = cord_n(&mut c.stack, b"da");
        let txt = cord_n(&mut c.stack, b"~9999999999999999.1.1");
        let sam = T(&mut c.stack, &[moda, txt]);
        let subj = T(&mut c.stack, &[D(0), sam, D(0)]);
        assert!(matches!(jet_slaw(c, subj), Err(JetErr::Punt)));
    }
}
