//! Property tests (AGENTS.md section 22: "if possible, consider
//! `proptest`"). Scoped to invariants that hold for *any* input,
//! including spellings the rest of the engine never actually produces
//! (mokuren's own usage stays diatonic — see PLAN.md) — that's the
//! point of a property test: it stresses corners example-based unit
//! tests don't reach.
//!
//! Not attempted here: a literal "transpose + inverse transpose"
//! round trip. mokuren has no `transpose` API yet (converting a raw
//! semitone count back into a spelled `PitchClass` is an enharmonic
//! spelling problem with no single correct answer, and out of v0.1's
//! diatonic-only scope). `chord_pitch_classes_match_interval_semitones`
//! below checks the same kind of "spell it, then verify the semitone
//! math wasn't lost" property using `Chord::pitch_classes`, which does
//! exist.

use mokuren::chord::{Chord, ChordQuality};
use mokuren::key::{Key, Mode, ScaleDegree};
use mokuren::pitch::{Accidental, Interval, NoteLetter, Octave, Pitch, PitchClass};
use proptest::prelude::*;

fn note_letter() -> impl Strategy<Value = NoteLetter> {
    prop_oneof![
        Just(NoteLetter::C),
        Just(NoteLetter::D),
        Just(NoteLetter::E),
        Just(NoteLetter::F),
        Just(NoteLetter::G),
        Just(NoteLetter::A),
        Just(NoteLetter::B),
    ]
}

/// The full accidental range, including doubles — used where the
/// property holds unconditionally (pure arithmetic, no musical
/// "sanity" assumed).
fn any_accidental() -> impl Strategy<Value = Accidental> {
    prop_oneof![
        Just(Accidental::DoubleFlat),
        Just(Accidental::Flat),
        Just(Accidental::Natural),
        Just(Accidental::Sharp),
        Just(Accidental::DoubleSharp),
    ]
}

/// Single accidentals only — what every practical key signature and
/// every chord mokuren's own rules ever produce.
fn single_accidental() -> impl Strategy<Value = Accidental> {
    prop_oneof![
        Just(Accidental::Flat),
        Just(Accidental::Natural),
        Just(Accidental::Sharp)
    ]
}

fn any_pitch_class() -> impl Strategy<Value = PitchClass> {
    (note_letter(), any_accidental()).prop_map(|(l, a)| PitchClass::new(l, a))
}

fn practical_pitch_class() -> impl Strategy<Value = PitchClass> {
    (note_letter(), single_accidental()).prop_map(|(l, a)| PitchClass::new(l, a))
}

fn any_pitch() -> impl Strategy<Value = Pitch> {
    (any_pitch_class(), -1i32..=8).prop_map(|(pc, oct)| Pitch::new(pc, Octave(oct)))
}

/// The only qualities `RomanNumeral::diatonic_vocabulary()` ever
/// produces (`AugmentedTriad`/`MajorSeventh`/.../`DiminishedSeventh`
/// exist on `ChordQuality` purely for future extensibility — see
/// AGENTS.md section 5 — and nothing in this crate constructs a
/// `Chord` with one today).
fn diatonic_chord_quality() -> impl Strategy<Value = ChordQuality> {
    prop_oneof![
        Just(ChordQuality::MajorTriad),
        Just(ChordQuality::MinorTriad),
        Just(ChordQuality::DiminishedTriad),
        Just(ChordQuality::DominantSeventh),
    ]
}

proptest! {
    /// Pitch-class normalization: however many accidentals are stacked
    /// on however unusual a letter, `semitone()` is always a valid
    /// pitch class.
    #[test]
    fn pitch_class_semitone_is_always_normalized(pc in any_pitch_class()) {
        prop_assert!(pc.semitone() < 12);
    }

    /// `Interval::between` sorts its arguments internally, so the
    /// result must not depend on call order.
    #[test]
    fn interval_between_is_order_independent(a in any_pitch(), b in any_pitch()) {
        prop_assert_eq!(Interval::between(a, b), Interval::between(b, a));
    }

    /// Chord spelling round-trip: `Chord::pitch_classes()` spells each
    /// tone by letter, but the semitone value it sounds at must always
    /// match `ChordQuality::interval_semitones()` exactly — spelling
    /// must never silently lose or shift the actual pitch.
    ///
    /// Restricted to `practical_pitch_class()` roots (single accidental
    /// at most, matching any real key's diatonic scale) and
    /// `diatonic_chord_quality()`: the exact (root, quality) space
    /// `RomanNumeral::to_chord` can ever actually produce. A root or
    /// quality outside that space can need an accidental beyond what
    /// `Accidental` represents — that's `spell_above`'s documented
    /// limitation, pinned separately in `src/pitch.rs`, not a property
    /// that holds (or needs to) for chord spelling in general.
    #[test]
    fn chord_pitch_classes_match_interval_semitones(root in practical_pitch_class(), quality in diatonic_chord_quality()) {
        let chord = Chord::new(root, quality);
        let tones = chord.pitch_classes().expect("practical roots with diatonic qualities are always spellable");
        let semitones = quality.interval_semitones();
        prop_assert_eq!(tones.len(), semitones.len());
        for (tone, &offset) in tones.iter().zip(semitones) {
            let expected = (root.semitone() as i32 + offset).rem_euclid(12) as u8;
            prop_assert_eq!(tone.semitone(), expected);
        }
    }

    /// Every scale degree of a practical major key round-trips through
    /// `diatonic_pitch_class` / `degree_of`. `Key::new` is expected to
    /// accept every practical (single-accidental) tonic — this is part
    /// of what "practical" means here.
    #[test]
    fn key_degree_round_trips_for_any_major_tonic(
        tonic in practical_pitch_class(),
        degree in 1u8..=7,
    ) {
        let key = Key::new(tonic, Mode::Major).expect("a practical tonic should always be a valid key");
        let pc = key.diatonic_pitch_class(ScaleDegree(degree));
        prop_assert_eq!(key.degree_of(pc), Some(ScaleDegree(degree)));
    }

    /// A major scale is always seven enharmonically distinct pitch
    /// classes — no two degrees of a practical key collide.
    #[test]
    fn key_scale_has_no_enharmonic_duplicates(tonic in practical_pitch_class()) {
        let key = Key::new(tonic, Mode::Major).expect("a practical tonic should always be a valid key");
        let scale = key.scale();
        for i in 0..scale.len() {
            for j in (i + 1)..scale.len() {
                prop_assert!(!scale[i].is_enharmonic_to(&scale[j]));
            }
        }
    }

    /// `Key::new` is the fail-closed gate for arbitrary tonics — proptest
    /// caught a real case here: a double-sharp tonic's own third can
    /// need a triple-sharp, which isn't as rare an edge as it first
    /// looked (see key.rs history). For *any* pitch class as tonic
    /// (including double-flat/double-sharp, not just
    /// `practical_pitch_class()`), `new` must never panic, and if it
    /// returns `Ok`, every scale degree on that key must genuinely be
    /// safe to look up (validating that the validator actually
    /// validates, not just that it returns *something*).
    #[test]
    fn key_new_either_succeeds_safely_or_fails_cleanly(tonic in any_pitch_class()) {
        if let Ok(key) = Key::new(tonic, Mode::Major) {
            for degree in 1..=7u8 {
                let _ = key.diatonic_pitch_class(ScaleDegree(degree));
            }
        }
    }
}
