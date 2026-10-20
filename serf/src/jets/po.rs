// `+po` @p-syllable-table jets for Eris: `tod`/`tos` (index → syllable) and
// `ind`/`ins` (syllable → index). `+po` holds the two 256-entry syllable tables
// (prefix and suffix) that, with `+ob`, turn a planet/moon number into a `@p`
// name. nockvm leaves them unjetted; `+scot`/`+slaw` of a `@p` lean on them.
//
// Ported from Vere's `pkg/noun/jets/c/po.c` (the tables are copied verbatim and
// length-checked) in nockvm's house style. `+po` is a child of chapter `qua`:
// path `[k138 one two tri qua po <arm>]`, plain unary gates (arg at axis 6).
//
//   tos / ins  — PREFIX table;  tod / ind — SUFFIX table.
//   tos / tod  — index (@)        → syllable (3-byte `@tas` cord).
//   ins / ind  — syllable (3-byte) → `(unit @)` index (`~` if not a syllable).

use nockvm::ext::AtomExt;
use nockvm::interpreter::Context;
use nockvm::jets::util::slot;
use nockvm::jets::{JetErr, Result};
use nockvm::mem::NockStack;
use nockvm::noun::{Atom, Noun, D, T};

// 256 three-letter syllables, packed. `syllable(T, i) = &T[i*3 .. i*3+3]`.
const PREFIX: &[u8; 768] = b"dozmarbinwansamlitsighidfidlissogdirwacsabwissibrigsoldopmodfoglidhopdardorlorhodfolrintogsilmirholpaslacrovlivdalsatlibtabhanticpidtorbolfosdotlosdilforpilramtirwintadbicdifrocwidbisdasmidloprilnardapmolsanlocnovsitnidtipsicropwitnatpanminritpodmottamtolsavposnapnopsomfinfonbanmorworsipronnorbotwicsocwatdolmagpicdavbidbaltimtasmalligsivtagpadsaldivdactansidfabtarmonranniswolmispallasdismaprabtobrollatlonnodnavfignomnibpagsopralbilhaddocridmocpacravripfaltodtiltinhapmicfanpattaclabmogsimsonpinlomrictapfirhasbosbatpochactidhavsaplindibhosdabbitbarracparloddosbortochilmactomdigfilfasmithobharmighinradmashalraglagfadtopmophabnilnosmilfopfamdatnoldinhatnacrisfotribhocnimlarfitwalrapsarnalmoslandondanladdovrivbacpollaptalpitnambonrostonfodponsovnocsorlavmatmipfip";
const SUFFIX: &[u8; 768] = b"zodnecbudwessevpersutletfulpensytdurwepserwylsunrypsyxdyrnuphebpeglupdepdysputlughecryttyvsydnexlunmeplutseppesdelsulpedtemledtulmetwenbynhexfebpyldulhetmevruttylwydtepbesdexsefwycburderneppurrysrebdennutsubpetrulsynregtydsupsemwynrecmegnetsecmulnymtevwebsummutnyxrextebfushepbenmuswyxsymselrucdecwexsyrwetdylmynmesdetbetbeltuxtugmyrpelsyptermebsetdutdegtexsurfeltudnuxruxrenwytnubmedlytdusnebrumtynseglyxpunresredfunrevrefmectedrusbexlebduxrynnumpyxrygryxfeptyrtustyclegnemfermertenlusnussyltecmexpubrymtucfyllepdebbermughuttunbylsudpemdevlurdefbusbeprunmelpexdytbyttyplevmylwedducfurfexnulluclennerlexrupnedlecrydlydfenwelnydhusrelrudneshesfetdesretdunlernyrsebhulrylludremlysfynwerrycsugnysnyllyndyndemluxfedsedbecmunlyrtesmudnytbyrsenwegfyrmurtelreptegpecnelnevfes";

#[inline(always)]
fn syllable(table: &[u8; 768], idx: usize) -> &[u8] {
    &table[idx * 3..idx * 3 + 3]
}

/// index → syllable: the `idx`th syllable of `table` as a 3-byte `@tas` cord
/// (first letter is the low byte). Out-of-range (`>= 256`) Punts to the Nock
/// (which `+tod`/`+tos` crash on).
fn to_syl(stack: &mut NockStack, table: &[u8; 768], idx: u64) -> Result {
    if idx >= 256 {
        return Err(JetErr::Punt);
    }
    Ok(Atom::from_bytes(stack, syllable(table, idx as usize)).as_noun())
}

/// syllable → `(unit index)`: read the first 3 bytes of `a` and look them up in
/// `table`; `[~ i]` if found, `~` if not (or if `a` is not a 3-letter cord).
fn find_syl(stack: &mut NockStack, table: &[u8; 768], a: Atom, space: &nockvm::noun::NounSpace) -> Noun {
    let mut b = [0u8; 3];
    {
        let h = a.in_space(space);
        let bytes = h.as_ne_bytes();
        for (i, slot_b) in b.iter_mut().enumerate() {
            if i < bytes.len() {
                *slot_b = bytes[i];
            }
        }
    }
    for i in 0..256 {
        if syllable(table, i) == b {
            return T(stack, &[D(0), D(i as u64)]);
        }
    }
    D(0)
}

/// `~(tos po a)`: prefix index → syllable.
pub fn jet_po_tos(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let idx = slot(subject, 6, &space)?
        .as_atom()
        .map_err(|_| JetErr::Punt)?
        .in_space(&space)
        .as_u64()
        .map_err(|_| JetErr::Punt)?;
    to_syl(&mut context.stack, PREFIX, idx)
}

/// `~(tod po a)`: suffix index → syllable.
pub fn jet_po_tod(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let idx = slot(subject, 6, &space)?
        .as_atom()
        .map_err(|_| JetErr::Punt)?
        .in_space(&space)
        .as_u64()
        .map_err(|_| JetErr::Punt)?;
    to_syl(&mut context.stack, SUFFIX, idx)
}

/// `~(ins po a)`: prefix syllable → `(unit index)`.
pub fn jet_po_ins(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let a = slot(subject, 6, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    Ok(find_syl(&mut context.stack, PREFIX, a, &space))
}

/// `~(ind po a)`: suffix syllable → `(unit index)`.
pub fn jet_po_ind(context: &mut Context, subject: Noun) -> Result {
    let space = context.stack.noun_space();
    let a = slot(subject, 6, &space)?.as_atom().map_err(|_| JetErr::Punt)?;
    Ok(find_syl(&mut context.stack, SUFFIX, a, &space))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nockvm::jets::util::test::{assert_jet, init_context};
    use nockvm::noun::{D, T};

    fn cord(stack: &mut NockStack, s: &[u8]) -> Noun {
        Atom::from_bytes(stack, s).as_noun()
    }

    #[test]
    fn test_tables_len() {
        assert_eq!(PREFIX.len(), 768);
        assert_eq!(SUFFIX.len(), 768);
        assert_eq!(syllable(PREFIX, 0), b"doz");
        assert_eq!(syllable(SUFFIX, 0), b"zod");
        assert_eq!(syllable(PREFIX, 255), b"fip");
        assert_eq!(syllable(SUFFIX, 255), b"fes");
    }

    #[test]
    fn test_tos_tod() {
        let c = &mut init_context();
        let want = cord(&mut c.stack, b"doz");
        assert_jet(c, jet_po_tos, D(0), want);
        let want = cord(&mut c.stack, b"zod");
        assert_jet(c, jet_po_tod, D(0), want);
        let want = cord(&mut c.stack, b"mar");
        assert_jet(c, jet_po_tos, D(1), want);
        let want = cord(&mut c.stack, b"fip");
        assert_jet(c, jet_po_tos, D(255), want);
    }

    #[test]
    fn test_ins_ind_roundtrip() {
        let c = &mut init_context();
        // prefix: ins("doz") = [~ 0], ins("mar") = [~ 1]
        let doz = cord(&mut c.stack, b"doz");
        let want = T(&mut c.stack, &[D(0), D(0)]);
        assert_jet(c, jet_po_ins, doz, want);
        let mar = cord(&mut c.stack, b"mar");
        let want = T(&mut c.stack, &[D(0), D(1)]);
        assert_jet(c, jet_po_ins, mar, want);
        // suffix: ind("zod") = [~ 0]
        let zod = cord(&mut c.stack, b"zod");
        let want = T(&mut c.stack, &[D(0), D(0)]);
        assert_jet(c, jet_po_ind, zod, want);
        // not a syllable -> ~
        let bad = cord(&mut c.stack, b"xyz");
        assert_jet(c, jet_po_ins, bad, D(0));
    }
}
