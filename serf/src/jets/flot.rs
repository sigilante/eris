//! NockVM jets for hoon-138's IEEE 754 doors `++rh`, `++rs`, `++rd`, `++rq`.
//!
//! Each door is `~%  %<door>  +>  ~` in chapter `%tri`, and its arms `add
//! sub mul div sqt lth lte equ gte gth` carry `~/` hints, so the jets
//! register at `[k.138 one two tri <door> <arm>]` with no Hoon change.
//! `fma` is hinted too but sdfloat has no fused multiply-add, so it is not
//! registered and stays Nock.
//!
//! Semantics, from `++ff`/`++fl` in hoon-138 (verified against sdfloat by the
//! RustFloat Hoon-vector tests): every result is correctly rounded in the
//! door's mode `r` (axis 30 of the gate core: `%n %u %d %z`); the only NaN
//! produced is the canonical one of the width; comparisons are IEEE
//! (`-0 == +0`, anything with NaN is false, i.e. `%.n`); `sqt` of `-0` is
//! `-0` and of a negative is NaN. Operands are read as the low `width` bits
//! of the atom, as `sea:ff` does with `cut`. One known divergence: `++fl`
//! overflows to infinity in every mode where IEEE 754 (and sdfloat)
//! saturates under `%z`, `%d` (positive) and `%u` (negative); see
//! urbit/urbit#7426. Until that lands, those inputs mismatch the Nock.

use either::Either::Left;
use nockvm::ext::AtomExt;
use nockvm::interpreter::Context;
use nockvm::jets::hot::{HotEntry, K_138};
use nockvm::jets::util::slot;
use nockvm::jets::{Jet, JetErr, Result};
use nockvm::noun::{Atom, Noun, NounSpace, D};
use sdfloat::{Format, Round, SoftFloat, F128, F16, F32, F64};

const fn tas(s: &[u8]) -> u64 {
    let mut v = 0u64;
    let mut i = s.len();
    while i > 0 {
        i -= 1;
        v = (v << 8) | s[i] as u64;
    }
    v
}

/// Print `msg` to stderr when `HOON_FLOAT_JET_TRACE` is set.
fn trace(msg: &str) {
    if std::env::var_os("HOON_FLOAT_JET_TRACE").is_some() {
        eprintln!("hoon-float-jet: {msg}");
    }
}

fn atom_u128(n: Noun, space: &NounSpace, width: usize) -> std::result::Result<u128, JetErr> {
    let a = n.in_space(space).as_atom().map_err(|_| JetErr::Punt)?;
    let mut bytes = a.to_le_bytes();
    bytes.resize(16.max(bytes.len()), 0);
    let mut b = [0u8; 16];
    b.copy_from_slice(&bytes[..16]);
    let v = u128::from_le_bytes(b);
    // Low `width` bits only, as `sea:ff` cuts its fields.
    Ok(if width == 16 { v } else { v & ((1u128 << (width * 8)) - 1) })
}

fn rounding(subject: Noun, space: &NounSpace) -> std::result::Result<Round, JetErr> {
    let r = slot(subject, 30, space).map_err(|_| JetErr::Punt)?;
    let v = r.in_space(space).as_atom().map_err(|_| JetErr::Punt)?.as_u64().map_err(|_| JetErr::Punt)?;
    Ok(match v {
        x if x == tas(b"n") => Round::NearEven,
        x if x == tas(b"u") => Round::Up,
        x if x == tas(b"d") => Round::Down,
        x if x == tas(b"z") => Round::Zero,
        x if x == tas(b"a") => Round::Away,
        _ => return Err(JetErr::Punt),
    })
}

fn pack(context: &mut Context, width: usize, bits: u128) -> Noun {
    Atom::from_bytes(&mut context.stack, &bits.to_le_bytes()[..width]).as_noun()
}

/// Loobean: `%.y` is 0, `%.n` is 1.
fn loob(b: bool) -> Noun {
    D(if b { 0 } else { 1 })
}

#[derive(Clone, Copy)]
enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Sqt,
    Lth,
    Lte,
    Equ,
    Gte,
    Gth,
}

fn run<F: Format + SoftFloat>(context: &mut Context, subject: Noun, op: Op) -> Result {
    let width = (F::WIDTH / 8) as usize;
    let space = context.stack.noun_space();
    let sam = slot(subject, 6, &space).map_err(|_| JetErr::Punt)?;
    let (a, b) = match op {
        Op::Sqt => (atom_u128(sam, &space, width)?, 0u128),
        _ => (
            atom_u128(slot(sam, 2, &space).map_err(|_| JetErr::Punt)?, &space, width)?,
            atom_u128(slot(sam, 3, &space).map_err(|_| JetErr::Punt)?, &space, width)?,
        ),
    };
    let (a, b) = (F::from_raw(a), F::from_raw(b));
    match op {
        Op::Lth => return Ok(loob(a.lt(b))),
        Op::Lte => return Ok(loob(a.le(b))),
        Op::Equ => return Ok(loob(a.eq(b))),
        Op::Gte => return Ok(loob(b.le(a))),
        Op::Gth => return Ok(loob(b.lt(a))),
        _ => {}
    }
    let r = rounding(subject, &space)?;
    let out = match op {
        Op::Add => a.add(b, r),
        Op::Sub => a.sub(b, r),
        Op::Mul => a.mul(b, r),
        Op::Div => a.div(b, r),
        Op::Sqt => a.sqrt(r),
        _ => unreachable!(),
    };
    Ok(pack(context, width, out.raw()))
}

macro_rules! door {
    ($door:literal, $f:ty, $($name:ident = $op:ident),*) => {
        $(
            pub fn $name(context: &mut Context, subject: Noun) -> Result {
                trace(concat!($door, "/", stringify!($name)));
                run::<$f>(context, subject, Op::$op)
            }
        )*
    };
}

pub mod rh {
    use super::*;
    door!("rh", F16, add = Add, sub = Sub, mul = Mul, div = Div, sqt = Sqt, lth = Lth, lte = Lte, equ = Equ, gte = Gte, gth = Gth);
}
pub mod rs {
    use super::*;
    door!("rs", F32, add = Add, sub = Sub, mul = Mul, div = Div, sqt = Sqt, lth = Lth, lte = Lte, equ = Equ, gte = Gte, gth = Gth);
}
pub mod rd {
    use super::*;
    door!("rd", F64, add = Add, sub = Sub, mul = Mul, div = Div, sqt = Sqt, lth = Lth, lte = Lte, equ = Equ, gte = Gte, gth = Gth);
}
pub mod rq {
    use super::*;
    door!("rq", F128, add = Add, sub = Sub, mul = Mul, div = Div, sqt = Sqt, lth = Lth, lte = Lte, equ = Equ, gte = Gte, gth = Gth);
}

macro_rules! arm {
    ($door:literal, $arm:literal, $jet:expr) => {
        (
            &[K_138, Left(b"one"), Left(b"two"), Left(b"tri"), Left($door), Left($arm)],
            1,
            $jet as Jet,
        )
    };
}
macro_rules! table {
    ($($door:literal $m:ident),* $(,)?) => {
        &[$(
            arm!($door, b"add", $m::add),
            arm!($door, b"sub", $m::sub),
            arm!($door, b"mul", $m::mul),
            arm!($door, b"div", $m::div),
            arm!($door, b"sqt", $m::sqt),
            arm!($door, b"lth", $m::lth),
            arm!($door, b"lte", $m::lte),
            arm!($door, b"equ", $m::equ),
            arm!($door, b"gte", $m::gte),
            arm!($door, b"gth", $m::gth),
        )*]
    };
}

/// The forty float-door jets.
pub const HOON_FLOAT_HOT: &[HotEntry] = table!(b"rh" rh, b"rs" rs, b"rd" rd, b"rq" rq);

#[cfg(test)]
mod tests {
    use super::*;
    use nockvm::jets::util::test::{assert_noun_eq, init_context};
    use nockvm::noun::T;

    /// A `@r` atom of `width` bytes from raw bits.
    fn ra(stack: &mut nockvm::mem::NockStack, v: u128, width: usize) -> Noun {
        Atom::from_bytes(stack, &v.to_le_bytes()[..width]).as_noun()
    }

    /// Build a door-arm subject `[0 [a b] [0 [%n 0]]]` (rounding `%n` at axis 30)
    /// and run the binary jet.
    fn bin(c: &mut Context, jet: fn(&mut Context, Noun) -> Result, a: u128, b: u128, w: usize) -> Noun {
        let an = ra(&mut c.stack, a, w);
        let bn = ra(&mut c.stack, b, w);
        let sample = T(&mut c.stack, &[an, bn]);
        let door = T(&mut c.stack, &[D(0), D(tas(b"n")), D(0)]);
        let subject = T(&mut c.stack, &[D(0), sample, door]);
        jet(c, subject).unwrap()
    }

    /// Unary (`sqt`): subject `[0 a [0 [%n 0]]]`.
    fn un(c: &mut Context, jet: fn(&mut Context, Noun) -> Result, a: u128, w: usize) -> Noun {
        let an = ra(&mut c.stack, a, w);
        let door = T(&mut c.stack, &[D(0), D(tas(b"n")), D(0)]);
        let subject = T(&mut c.stack, &[D(0), an, door]);
        jet(c, subject).unwrap()
    }

    #[test]
    fn test_rs() {
        let c = &mut init_context();
        // f32: 1.0=0x3f800000 2.0=0x40000000 3.0=0x40400000 4.0=0x40800000 6.0=0x40c00000
        let got = bin(c, rs::add, 0x3f80_0000, 0x3f80_0000, 4);
        let want = ra(&mut c.stack, 0x4000_0000, 4);
        assert_noun_eq(&mut c.stack, got, want);
        let got = bin(c, rs::mul, 0x4000_0000, 0x4040_0000, 4);
        let want = ra(&mut c.stack, 0x40c0_0000, 4);
        assert_noun_eq(&mut c.stack, got, want);
        let got = bin(c, rs::div, 0x40c0_0000, 0x4000_0000, 4);
        let want = ra(&mut c.stack, 0x4040_0000, 4);
        assert_noun_eq(&mut c.stack, got, want);
        let got = un(c, rs::sqt, 0x4080_0000, 4); // sqrt(4.0) = 2.0
        let want = ra(&mut c.stack, 0x4000_0000, 4);
        assert_noun_eq(&mut c.stack, got, want);
        // comparisons: 1.0 < 2.0 -> %.y (0); 1.0 > 2.0 -> %.n (1)
        let lth = bin(c, rs::lth, 0x3f80_0000, 0x4000_0000, 4);
        assert_noun_eq(&mut c.stack, lth, D(0));
        let gth = bin(c, rs::gth, 0x3f80_0000, 0x4000_0000, 4);
        assert_noun_eq(&mut c.stack, gth, D(1));
        let equ = bin(c, rs::equ, 0x4000_0000, 0x4000_0000, 4);
        assert_noun_eq(&mut c.stack, equ, D(0));
    }

    #[test]
    fn test_rh() {
        let c = &mut init_context();
        // f16: 1.0=0x3c00 2.0=0x4000 4.0=0x4400
        let got = bin(c, rh::add, 0x3c00, 0x3c00, 2);
        let want = ra(&mut c.stack, 0x4000, 2);
        assert_noun_eq(&mut c.stack, got, want);
        let got = un(c, rh::sqt, 0x4400, 2); // sqrt(4)=2
        let want = ra(&mut c.stack, 0x4000, 2);
        assert_noun_eq(&mut c.stack, got, want);
    }

    #[test]
    fn test_rd() {
        let c = &mut init_context();
        // f64: 1.0=0x3ff0.. 2.0=0x4000.. 3.0=0x4008.. 6.0=0x4018..
        let got = bin(c, rd::add, 0x3ff0_0000_0000_0000, 0x3ff0_0000_0000_0000, 8);
        let want = ra(&mut c.stack, 0x4000_0000_0000_0000, 8);
        assert_noun_eq(&mut c.stack, got, want);
        let got = bin(c, rd::mul, 0x4000_0000_0000_0000, 0x4008_0000_0000_0000, 8);
        let want = ra(&mut c.stack, 0x4018_0000_0000_0000, 8); // 2*3=6
        assert_noun_eq(&mut c.stack, got, want);
    }

    #[test]
    fn test_rq() {
        let c = &mut init_context();
        // f128: 1.0=0x3fff<<112 2.0=0x4000<<112 4.0=0x4001<<112
        let one = 0x3fff_u128 << 112;
        let two = 0x4000_u128 << 112;
        let four = 0x4001_u128 << 112;
        let got = bin(c, rq::add, one, one, 16);
        let want = ra(&mut c.stack, two, 16);
        assert_noun_eq(&mut c.stack, got, want);
        let got = un(c, rq::sqt, four, 16); // sqrt(4)=2
        let want = ra(&mut c.stack, two, 16);
        assert_noun_eq(&mut c.stack, got, want);
    }
}
