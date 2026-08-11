//! Chords, Roman numerals, and harmonic function.
//!
//! v0.1 keeps Roman numeral and harmonic function as distinct concepts
//! (AGENTS.md section 6): `RomanNumeral` names *which* diatonic chord,
//! `HarmonicFunction` names its role (tonic/predominant/dominant) so
//! progression evaluation can reason at the function level.

use crate::error::{MokurenError, Result};
use crate::key::{Key, ScaleDegree};
use crate::pitch::{PitchClass, spell_above};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChordQuality {
    MajorTriad,
    MinorTriad,
    DiminishedTriad,
    AugmentedTriad,
    MajorSeventh,
    MinorSeventh,
    DominantSeventh,
    HalfDiminishedSeventh,
    DiminishedSeventh,
}

impl ChordQuality {
    pub fn is_seventh(&self) -> bool {
        matches!(
            self,
            ChordQuality::MajorSeventh
                | ChordQuality::MinorSeventh
                | ChordQuality::DominantSeventh
                | ChordQuality::HalfDiminishedSeventh
                | ChordQuality::DiminishedSeventh
        )
    }

    /// Semitones above the root for each chord tone, root first.
    pub fn interval_semitones(&self) -> &'static [i32] {
        match self {
            ChordQuality::MajorTriad => &[0, 4, 7],
            ChordQuality::MinorTriad => &[0, 3, 7],
            ChordQuality::DiminishedTriad => &[0, 3, 6],
            ChordQuality::AugmentedTriad => &[0, 4, 8],
            ChordQuality::MajorSeventh => &[0, 4, 7, 11],
            ChordQuality::MinorSeventh => &[0, 3, 7, 10],
            ChordQuality::DominantSeventh => &[0, 4, 7, 10],
            ChordQuality::HalfDiminishedSeventh => &[0, 3, 6, 10],
            ChordQuality::DiminishedSeventh => &[0, 3, 6, 9],
        }
    }

    /// Whether this quality is spelled with a lowercase Roman numeral.
    fn is_minor_case(&self) -> bool {
        matches!(
            self,
            ChordQuality::MinorTriad
                | ChordQuality::DiminishedTriad
                | ChordQuality::MinorSeventh
                | ChordQuality::HalfDiminishedSeventh
                | ChordQuality::DiminishedSeventh
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChordInversion {
    Root,
    First,
    Second,
    /// Only meaningful for seventh chords.
    Third,
}

impl ChordInversion {
    /// Index into `Chord::pitch_classes()` that must be in the bass.
    pub fn bass_chord_tone_index(&self) -> usize {
        match self {
            ChordInversion::Root => 0,
            ChordInversion::First => 1,
            ChordInversion::Second => 2,
            ChordInversion::Third => 3,
        }
    }

    fn figured_bass(&self, is_seventh: bool) -> &'static str {
        match (self, is_seventh) {
            (ChordInversion::Root, false) => "",
            (ChordInversion::First, false) => "6",
            (ChordInversion::Second, false) => "64",
            (ChordInversion::Root, true) => "7",
            (ChordInversion::First, true) => "65",
            (ChordInversion::Second, true) => "43",
            (ChordInversion::Third, true) => "42",
            (ChordInversion::Third, false) => "", // not meaningful; treated as root
        }
    }
}

/// Tonic / predominant / dominant. Kept separate from `RomanNumeral` so
/// progression evaluation can reason about function-level motion
/// (T -> PD -> D -> T) without switching on scale degree everywhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum HarmonicFunction {
    Tonic,
    Predominant,
    Dominant,
}

impl fmt::Display for HarmonicFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            HarmonicFunction::Tonic => "tonic",
            HarmonicFunction::Predominant => "predominant",
            HarmonicFunction::Dominant => "dominant",
        };
        write!(f, "{s}")
    }
}

/// Standard tonic/predominant/dominant grouping for a diatonic scale
/// degree (I, iii, vi = tonic; ii, IV = predominant; V, vii° = dominant).
/// Degree 7 needs `quality` too: major mode's vii° (diminished) and
/// minor's harmonic-minor-raised vii° (also diminished) are dominant-
/// function, but minor's *natural* VII (a major triad, the subtonic) is
/// not the same chord and doesn't pull toward the tonic the same way —
/// treated as predominant-ish (a known simplification; real pedagogy is
/// more nuanced about the subtonic's function than this three-way split
/// captures at all, minor or major).
fn harmonic_function_of_degree(degree: ScaleDegree, quality: ChordQuality) -> HarmonicFunction {
    match degree.0 {
        1 | 3 | 6 => HarmonicFunction::Tonic,
        2 | 4 => HarmonicFunction::Predominant,
        5 => HarmonicFunction::Dominant,
        7 => {
            if matches!(
                quality,
                ChordQuality::DiminishedTriad
                    | ChordQuality::DiminishedSeventh
                    | ChordQuality::HalfDiminishedSeventh
            ) {
                HarmonicFunction::Dominant
            } else {
                HarmonicFunction::Predominant
            }
        }
        _ => HarmonicFunction::Tonic,
    }
}

/// What kind of chord a `RomanNumeral` names, beyond its scale degree —
/// distinguishes the three ways a numeral's root/quality can be derived,
/// so a rule can pattern-match on *why* a numeral is chromatic instead of
/// checking one boolean per chromatic feature (which would let
/// meaningless combinations like "applied dominant that's also a raised
/// leading tone" type-check). `Diatonic` covers every plain in-key
/// triad/seventh in both major and natural minor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NumeralSource {
    Diatonic,
    /// Tonicizes `ScaleDegree` (e.g. `V/V`, `V7/vi`) rather than
    /// resolving within the home key — see `applied_dominant`. `degree`
    /// on a numeral with this source is always `ScaleDegree::DOMINANT`:
    /// v0.1 only implements V/x and V7/x, not applied leading-tone
    /// chords (vii°/x) — see ROADMAP.md.
    AppliedDominant(ScaleDegree),
    /// Uses the harmonic-minor raised 7th in place of natural minor's
    /// own (lowered) 7th — see `RomanNumeral::harmonic_minor_vocabulary`.
    /// Only meaningful in a minor key; never produced for major.
    HarmonicMinorRaisedSeventh,
    /// Uses the melodic-minor raised 6th in place of natural minor's own
    /// (lowered) 6th — see `RomanNumeral::melodic_minor_vocabulary`.
    /// Only meaningful in a minor key; never produced for major.
    MelodicMinorRaisedSixth,
}

/// A diatonic Roman numeral, an applied ("secondary") dominant, or a
/// harmonic-minor-altered numeral.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RomanNumeral {
    pub degree: ScaleDegree,
    pub quality: ChordQuality,
    pub inversion: ChordInversion,
    pub source: NumeralSource,
}

impl RomanNumeral {
    pub const fn new(
        degree: ScaleDegree,
        quality: ChordQuality,
        inversion: ChordInversion,
    ) -> Self {
        RomanNumeral {
            degree,
            quality,
            inversion,
            source: NumeralSource::Diatonic,
        }
    }

    pub const fn root_position(degree: ScaleDegree, quality: ChordQuality) -> Self {
        RomanNumeral::new(degree, quality, ChordInversion::Root)
    }

    pub const fn with_inversion(self, inversion: ChordInversion) -> Self {
        RomanNumeral { inversion, ..self }
    }

    /// `Some(target)` if this numeral is an applied dominant, matching
    /// the old `applied_to: Option<ScaleDegree>` field's meaning —
    /// convenience for call sites that only care about that one case.
    pub fn applied_to(&self) -> Option<ScaleDegree> {
        match self.source {
            NumeralSource::AppliedDominant(target) => Some(target),
            _ => None,
        }
    }

    /// The applied dominant (or applied dominant seventh) of `target` —
    /// the major triad or dominant seventh chord a perfect fifth above
    /// `target`'s own diatonic pitch, temporarily tonicizing it. Root
    /// position; use `with_inversion` for others.
    pub const fn applied_dominant(target: ScaleDegree, quality: ChordQuality) -> Self {
        RomanNumeral {
            degree: ScaleDegree::DOMINANT,
            quality,
            inversion: ChordInversion::Root,
            source: NumeralSource::AppliedDominant(target),
        }
    }

    /// The standard applied-dominant set (AGENTS.md section 5's "V/V,
    /// V7/ii" example): V/x and V7/x for every diatonic degree except
    /// the tonic (nothing to tonicize) and the leading tone (too
    /// unstable a target in practice).
    pub fn applied_dominant_vocabulary() -> Vec<RomanNumeral> {
        [
            ScaleDegree::SUPERTONIC,
            ScaleDegree::MEDIANT,
            ScaleDegree::SUBDOMINANT,
            ScaleDegree::DOMINANT,
            ScaleDegree::SUBMEDIANT,
        ]
        .into_iter()
        .flat_map(|target| {
            [
                RomanNumeral::applied_dominant(target, ChordQuality::MajorTriad),
                RomanNumeral::applied_dominant(target, ChordQuality::DominantSeventh),
            ]
        })
        .collect()
    }

    // Diatonic triads in a major key.
    pub const I: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::TONIC, ChordQuality::MajorTriad);
    pub const II: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::SUPERTONIC, ChordQuality::MinorTriad);
    pub const III: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::MEDIANT, ChordQuality::MinorTriad);
    pub const IV: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::SUBDOMINANT, ChordQuality::MajorTriad);
    pub const V: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::DOMINANT, ChordQuality::MajorTriad);
    pub const VI: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::SUBMEDIANT, ChordQuality::MinorTriad);
    pub const VII_DIM: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::LEADING_TONE, ChordQuality::DiminishedTriad);
    pub const V7: RomanNumeral =
        RomanNumeral::root_position(ScaleDegree::DOMINANT, ChordQuality::DominantSeventh);

    /// All seven diatonic triads plus V7, root position — the harmonic
    /// vocabulary of the v0.1 spine (AGENTS.md section 5).
    pub fn diatonic_vocabulary() -> [RomanNumeral; 8] {
        [
            RomanNumeral::I,
            RomanNumeral::II,
            RomanNumeral::III,
            RomanNumeral::IV,
            RomanNumeral::V,
            RomanNumeral::VI,
            RomanNumeral::VII_DIM,
            RomanNumeral::V7,
        ]
    }

    /// Natural minor's seven diatonic triads (i, ii°, III, iv, v, VI,
    /// VII) — every degree's quality differs from major's except the
    /// diminished ii°. `v` here is a minor triad and `VII` a major triad
    /// (the subtonic), both built on natural minor's own *unraised* 7th
    /// degree. The far more common dominant-function alternatives (V,
    /// V7, vii°, using the raised leading tone) are
    /// `harmonic_minor_vocabulary` — offered *alongside* this set, not
    /// replacing it, the same "offer more, let scoring decide" shape
    /// `applied_dominant_vocabulary` already uses.
    pub fn natural_minor_vocabulary() -> [RomanNumeral; 7] {
        [
            RomanNumeral::root_position(ScaleDegree::TONIC, ChordQuality::MinorTriad),
            RomanNumeral::root_position(ScaleDegree::SUPERTONIC, ChordQuality::DiminishedTriad),
            RomanNumeral::root_position(ScaleDegree::MEDIANT, ChordQuality::MajorTriad),
            RomanNumeral::root_position(ScaleDegree::SUBDOMINANT, ChordQuality::MinorTriad),
            RomanNumeral::root_position(ScaleDegree::DOMINANT, ChordQuality::MinorTriad),
            RomanNumeral::root_position(ScaleDegree::SUBMEDIANT, ChordQuality::MajorTriad),
            RomanNumeral::root_position(ScaleDegree::LEADING_TONE, ChordQuality::MajorTriad),
        ]
    }

    /// The harmonic-minor-derived dominant-function chords — V, V7, and
    /// vii° — each built using the raised leading tone in place of
    /// natural minor's own (lowered) 7th (`NumeralSource::HarmonicMinorRaisedSeventh`).
    /// This is what actually gives a minor-key cadence a real leading
    /// tone; without it every "dominant" in a minor key would be the
    /// weak natural-minor `v`/`VII` above. vii°7 (the fully diminished
    /// seventh, whose chordal seventh sits on the *lowered* 6th) is
    /// deliberately not included yet — narrower-than-full-theory first
    /// pass, same scoping `applied_dominant_vocabulary` used for
    /// secondary dominants; see ROADMAP.md.
    pub fn harmonic_minor_vocabulary() -> [RomanNumeral; 3] {
        let altered = |degree, quality| RomanNumeral {
            degree,
            quality,
            inversion: ChordInversion::Root,
            source: NumeralSource::HarmonicMinorRaisedSeventh,
        };
        [
            altered(ScaleDegree::DOMINANT, ChordQuality::MajorTriad),
            altered(ScaleDegree::DOMINANT, ChordQuality::DominantSeventh),
            altered(ScaleDegree::LEADING_TONE, ChordQuality::DiminishedTriad),
        ]
    }

    /// Applied dominants for minor keys — V/x and V7/x for the targets
    /// real corpus data actually needed (`examples/chorale_benchmark.rs
    /// --minor-gap-report`, 2026-08-11): x in {ii, IV, V, vi}. Unlike
    /// major's `applied_dominant_vocabulary`, V/III is *not* included —
    /// zero of the bisected minor chorales needed it, so it's left out
    /// rather than assumed to generalize from major's own target set.
    /// (V/V tonicizes the harmonic-minor dominant itself — a standard
    /// "dominant of the dominant," not a contradiction.)
    pub fn minor_applied_dominant_vocabulary() -> Vec<RomanNumeral> {
        [
            ScaleDegree::SUPERTONIC,
            ScaleDegree::SUBDOMINANT,
            ScaleDegree::DOMINANT,
            ScaleDegree::SUBMEDIANT,
        ]
        .into_iter()
        .flat_map(|target| {
            [
                RomanNumeral::applied_dominant(target, ChordQuality::MajorTriad),
                RomanNumeral::applied_dominant(target, ChordQuality::DominantSeventh),
            ]
        })
        .collect()
    }

    /// The two chords that actually change shape under melodic minor's
    /// raised 6th, in Common Practice minor-key writing: ii becomes a
    /// minor triad (not natural minor's diminished ii°) and IV becomes a
    /// major triad (not natural minor's minor iv) — both by definition,
    /// since the raised 6th is *part of* each chord's own stacked-thirds
    /// structure (ii's fifth; IV's third), the same "just a different
    /// quality at an unchanged root" mechanism `harmonic_minor_vocabulary`
    /// uses for the raised 7th, verified the same way before writing this.
    /// Scoped to only these two — a real, if partial, common-practice
    /// convention (not full melodic-minor: descending motion, and any
    /// other chord touching the submediant, still use the natural 6th)
    /// — because real corpus data showed the raised 6th mattering in 65
    /// of 81 chromatic-soprano minor failures (`--minor-gap-report`,
    /// 2026-08-11), too large to leave deferred alongside vii°7/melodic
    /// minor's other conventions.
    pub fn melodic_minor_vocabulary() -> [RomanNumeral; 2] {
        let altered = |degree, quality| RomanNumeral {
            degree,
            quality,
            inversion: ChordInversion::Root,
            source: NumeralSource::MelodicMinorRaisedSixth,
        };
        [
            altered(ScaleDegree::SUPERTONIC, ChordQuality::MinorTriad),
            altered(ScaleDegree::SUBDOMINANT, ChordQuality::MajorTriad),
        ]
    }

    pub fn harmonic_function(&self) -> HarmonicFunction {
        harmonic_function_of_degree(self.degree, self.quality)
    }

    /// `None` only for an applied dominant whose root would need an
    /// accidental beyond what `Accidental` can represent (see
    /// `pitch::spell_above`) — unreachable for any diatonic numeral,
    /// since `Key::new` already validated those (including, for a minor
    /// key, its raised leading tone). Fails closed, same pattern as
    /// `Chord::pitch_classes`.
    pub fn to_chord(&self, key: &Key) -> Option<Chord> {
        let root = match self.source {
            // A raised-6th numeral (ii, IV) doesn't need a different
            // root either — same reasoning as V/V7 below, just for the
            // submediant instead of the leading tone.
            NumeralSource::Diatonic | NumeralSource::MelodicMinorRaisedSixth => {
                key.diatonic_pitch_class(self.degree)
            }
            // A perfect fifth (4 letter-steps, 7 semitones) above the
            // tonicized target — its own dominant, borrowed.
            NumeralSource::AppliedDominant(target) => {
                spell_above(key.diatonic_pitch_class(target), 4, 7)?
            }
            NumeralSource::HarmonicMinorRaisedSeventh
                if self.degree == ScaleDegree::LEADING_TONE =>
            {
                // vii° is *built on* the raised leading tone.
                key.functional_leading_tone()
            }
            NumeralSource::HarmonicMinorRaisedSeventh => {
                // V/V7: the root (scale degree 5) doesn't change: the
                // raised leading tone appears on its own as the third
                // once `quality` stacks a major third instead of natural
                // minor's own minor third — no special root needed.
                key.diatonic_pitch_class(self.degree)
            }
        };
        Some(Chord {
            root,
            quality: self.quality,
        })
    }

    /// The pitch class this applied dominant resolves to, if it is one.
    pub fn resolution_target(&self, key: &Key) -> Option<PitchClass> {
        self.applied_to().map(|t| key.diatonic_pitch_class(t))
    }

    /// The chromatic tone this applied dominant introduces — its own
    /// local leading tone, a semitone below the resolution target — for
    /// rules that track its resolution the way `LeadingToneResolutionRule`
    /// tracks the diatonic one. `None` if this isn't an applied dominant,
    /// or (unreachably, for the same reason as `to_chord`) unspellable.
    pub fn applied_leading_tone(&self, key: &Key) -> Option<PitchClass> {
        self.applied_to()?;
        let tones = self.to_chord(key)?.pitch_classes().ok()?;
        tones.get(1).copied()
    }
}

impl fmt::Display for RomanNumeral {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const NUMERALS: [&str; 7] = ["I", "II", "III", "IV", "V", "VI", "VII"];
        let base = NUMERALS[(self.degree.0 as usize - 1) % 7];
        let mut text = if self.quality.is_minor_case() {
            base.to_lowercase()
        } else {
            base.to_string()
        };
        if matches!(
            self.quality,
            ChordQuality::DiminishedTriad | ChordQuality::DiminishedSeventh
        ) {
            text.push('°');
        } else if self.quality == ChordQuality::HalfDiminishedSeventh {
            text.push('ø');
        }
        write!(
            f,
            "{text}{}",
            self.inversion.figured_bass(self.quality.is_seventh())
        )?;
        if let Some(target) = self.applied_to() {
            // The target is named, not voiced with its own inversion —
            // "V/ii", never "V/ii6" — matching how the target's own
            // diatonic quality (minor for ii/iii/vi) sets its case.
            let target_base = NUMERALS[(target.0 as usize - 1) % 7];
            let target_text = if matches!(target.0, 2 | 3 | 6) {
                target_base.to_lowercase()
            } else {
                target_base.to_string()
            };
            write!(f, "/{target_text}")?;
        }
        Ok(())
    }
}

/// An absolute chord: root pitch class plus quality. Unlike
/// `RomanNumeral`, a `Chord` doesn't need a `Key` to spell its own tones.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chord {
    pub root: PitchClass,
    pub quality: ChordQuality,
}

impl Chord {
    pub const fn new(root: PitchClass, quality: ChordQuality) -> Self {
        Chord { root, quality }
    }

    /// Chord tones stacked in thirds above the root, correctly spelled.
    ///
    /// Fails closed: if a tone needs an accidental beyond what
    /// `Accidental` can represent (double-flat..double-sharp), this
    /// returns `Err` rather than a silently wrong pitch. Every chord
    /// `RomanNumeral::to_chord` builds from a practical key is safe —
    /// this is only reachable by constructing a `Chord` directly with an
    /// unusual `root` (see `tests/properties.rs`).
    pub fn pitch_classes(&self) -> Result<Vec<PitchClass>> {
        self.quality
            .interval_semitones()
            .iter()
            .enumerate()
            .map(|(i, &semitones)| {
                spell_above(self.root, 2 * i as i32, semitones).ok_or_else(|| {
                    MokurenError::UnrepresentablePitch(format!(
                        "{:?} chord on {} has no representable spelling for tone {i}",
                        self.quality, self.root
                    ))
                })
            })
            .collect()
    }

    /// `None` both when this isn't a seventh chord and when the seventh
    /// can't be spelled — a rule that can't determine the seventh can't
    /// enforce its resolution either way, so both cases mean "nothing to
    /// check here," never a fabricated pitch.
    pub fn chordal_seventh(&self) -> Option<PitchClass> {
        if !self.quality.is_seventh() {
            return None;
        }
        self.pitch_classes().ok().map(|tones| tones[3])
    }

    /// `false` both when `pc` genuinely isn't a chord tone and when the
    /// chord can't be spelled at all — used as a candidate-generation
    /// filter, where "can't confirm this chord tone" and "exclude it"
    /// are the same safe outcome.
    pub fn contains_pitch_class(&self, pc: PitchClass) -> bool {
        self.pitch_classes()
            .is_ok_and(|tones| tones.iter().any(|t| t.is_enharmonic_to(&pc)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::{Accidental, NoteLetter};

    #[test]
    fn c_major_triad_spelling() {
        let chord = RomanNumeral::I.to_chord(&Key::C_MAJOR).unwrap();
        let tones = chord.pitch_classes().unwrap();
        assert_eq!(tones, vec![PitchClass::C, PitchClass::E, PitchClass::G]);
    }

    #[test]
    fn v7_in_c_major_is_g_b_d_f() {
        let chord = RomanNumeral::V7.to_chord(&Key::C_MAJOR).unwrap();
        assert_eq!(
            chord.pitch_classes().unwrap(),
            vec![PitchClass::G, PitchClass::B, PitchClass::D, PitchClass::F]
        );
    }

    #[test]
    fn unspellable_chord_fails_closed_instead_of_returning_a_wrong_pitch() {
        // Same case pinned in src/pitch.rs: a double-flat root combined
        // with a quality whose third needs a triple-flat to spell
        // correctly. Reachable only via `Chord::new` directly — no
        // `RomanNumeral::to_chord` ever produces this root/quality pair.
        let cbb = PitchClass::new(NoteLetter::C, Accidental::DoubleFlat);
        let chord = Chord::new(cbb, ChordQuality::MinorSeventh);
        assert!(matches!(
            chord.pitch_classes(),
            Err(MokurenError::UnrepresentablePitch(_))
        ));
        assert!(!chord.contains_pitch_class(PitchClass::E));
        assert_eq!(chord.chordal_seventh(), None);
    }

    #[test]
    fn vii_dim_in_c_major_is_b_d_f() {
        let chord = RomanNumeral::VII_DIM.to_chord(&Key::C_MAJOR).unwrap();
        assert_eq!(
            chord.pitch_classes().unwrap(),
            vec![PitchClass::B, PitchClass::D, PitchClass::F]
        );
    }

    #[test]
    fn display_matches_textbook_notation() {
        assert_eq!(RomanNumeral::I.to_string(), "I");
        assert_eq!(RomanNumeral::VI.to_string(), "vi");
        assert_eq!(RomanNumeral::VII_DIM.to_string(), "vii°");
        assert_eq!(RomanNumeral::V7.to_string(), "V7");
        assert_eq!(
            RomanNumeral::I
                .with_inversion(ChordInversion::First)
                .to_string(),
            "I6"
        );
        assert_eq!(
            RomanNumeral::V7
                .with_inversion(ChordInversion::First)
                .to_string(),
            "V65"
        );
    }

    #[test]
    fn harmonic_functions_match_textbook_grouping() {
        assert_eq!(RomanNumeral::I.harmonic_function(), HarmonicFunction::Tonic);
        assert_eq!(
            RomanNumeral::II.harmonic_function(),
            HarmonicFunction::Predominant
        );
        assert_eq!(
            RomanNumeral::IV.harmonic_function(),
            HarmonicFunction::Predominant
        );
        assert_eq!(
            RomanNumeral::V.harmonic_function(),
            HarmonicFunction::Dominant
        );
        assert_eq!(
            RomanNumeral::VI.harmonic_function(),
            HarmonicFunction::Tonic
        );
        assert_eq!(
            RomanNumeral::VII_DIM.harmonic_function(),
            HarmonicFunction::Dominant
        );
    }

    #[test]
    fn f_major_iv_chord_uses_bb_not_a_sharp() {
        let chord = RomanNumeral::IV.to_chord(&Key::F_MAJOR).unwrap();
        assert_eq!(chord.root, PitchClass::new(NoteLetter::B, Accidental::Flat));
    }

    #[test]
    fn v_of_v_in_c_major_is_d_major_tonicizing_g() {
        let f_sharp = PitchClass::new(NoteLetter::F, Accidental::Sharp);
        let rn = RomanNumeral::applied_dominant(ScaleDegree::DOMINANT, ChordQuality::MajorTriad);
        let chord = rn.to_chord(&Key::C_MAJOR).unwrap();
        assert_eq!(
            chord.pitch_classes().unwrap(),
            vec![PitchClass::D, f_sharp, PitchClass::A]
        );
        assert_eq!(rn.resolution_target(&Key::C_MAJOR), Some(PitchClass::G));
        assert_eq!(rn.applied_leading_tone(&Key::C_MAJOR), Some(f_sharp));
        assert_eq!(rn.to_string(), "V/V");
    }

    #[test]
    fn v7_of_ii_in_c_major_targets_d_and_displays_lowercase() {
        let rn =
            RomanNumeral::applied_dominant(ScaleDegree::SUPERTONIC, ChordQuality::DominantSeventh);
        assert_eq!(rn.resolution_target(&Key::C_MAJOR), Some(PitchClass::D));
        assert_eq!(rn.to_string(), "V7/ii");
    }

    #[test]
    fn a_minor_i_is_a_minor_triad_not_major() {
        let i = RomanNumeral::natural_minor_vocabulary()[0];
        let chord = i.to_chord(&Key::A_MINOR).unwrap();
        assert_eq!(
            chord.pitch_classes().unwrap(),
            vec![PitchClass::A, PitchClass::C, PitchClass::E]
        );
        assert_eq!(i.to_string(), "i");
    }

    #[test]
    fn a_minor_natural_vii_is_g_major_the_subtonic() {
        let vii = RomanNumeral::natural_minor_vocabulary()[6];
        assert_eq!(vii.degree, ScaleDegree::LEADING_TONE);
        let chord = vii.to_chord(&Key::A_MINOR).unwrap();
        assert_eq!(
            chord.pitch_classes().unwrap(),
            vec![PitchClass::G, PitchClass::B, PitchClass::D]
        );
        assert_eq!(vii.to_string(), "VII");
        assert_eq!(vii.harmonic_function(), HarmonicFunction::Predominant);
    }

    #[test]
    fn a_minor_harmonic_v_and_v7_use_the_raised_leading_tone() {
        let g_sharp = PitchClass::new(NoteLetter::G, Accidental::Sharp);
        let vocab = RomanNumeral::harmonic_minor_vocabulary();
        let v = vocab[0];
        let v7 = vocab[1];
        assert_eq!(
            v.to_chord(&Key::A_MINOR).unwrap().pitch_classes().unwrap(),
            vec![PitchClass::E, g_sharp, PitchClass::B]
        );
        assert_eq!(v.to_string(), "V");
        assert_eq!(v.harmonic_function(), HarmonicFunction::Dominant);
        assert_eq!(
            v7.to_chord(&Key::A_MINOR).unwrap().pitch_classes().unwrap(),
            vec![PitchClass::E, g_sharp, PitchClass::B, PitchClass::D]
        );
    }

    #[test]
    fn a_minor_harmonic_vii_dim_is_built_on_the_raised_leading_tone() {
        let g_sharp = PitchClass::new(NoteLetter::G, Accidental::Sharp);
        let vii_dim = RomanNumeral::harmonic_minor_vocabulary()[2];
        assert_eq!(vii_dim.degree, ScaleDegree::LEADING_TONE);
        let chord = vii_dim.to_chord(&Key::A_MINOR).unwrap();
        assert_eq!(chord.root, g_sharp);
        assert_eq!(
            chord.pitch_classes().unwrap(),
            vec![g_sharp, PitchClass::B, PitchClass::D]
        );
        assert_eq!(vii_dim.to_string(), "vii°");
        assert_eq!(vii_dim.harmonic_function(), HarmonicFunction::Dominant);
    }

    #[test]
    fn harmonic_minor_source_is_distinct_from_applied_dominant_and_diatonic() {
        // The tie-break/canonical-rank shape depends on `source` being a
        // single enum rather than two independent optional markers —
        // pin that a harmonic-minor numeral never reports as an applied
        // dominant, and vice versa (see NumeralSource's doc comment).
        let v = RomanNumeral::harmonic_minor_vocabulary()[0];
        assert_eq!(v.applied_to(), None);
        assert_eq!(v.source, NumeralSource::HarmonicMinorRaisedSeventh);

        let v_of_v =
            RomanNumeral::applied_dominant(ScaleDegree::DOMINANT, ChordQuality::MajorTriad);
        assert_ne!(v_of_v.source, NumeralSource::HarmonicMinorRaisedSeventh);
    }

    #[test]
    fn a_minor_v_of_v_tonicizes_e_the_natural_dominant() {
        // "Dominant of the dominant" — a real, standard technique, not a
        // contradiction with harmonic minor's own V.
        let v_of_v = RomanNumeral::minor_applied_dominant_vocabulary()
            .into_iter()
            .find(|rn| {
                rn.applied_to() == Some(ScaleDegree::DOMINANT)
                    && rn.quality == ChordQuality::MajorTriad
            })
            .unwrap();
        assert_eq!(v_of_v.resolution_target(&Key::A_MINOR), Some(PitchClass::E));
        let chord = v_of_v.to_chord(&Key::A_MINOR).unwrap();
        let d_sharp = PitchClass::new(NoteLetter::D, Accidental::Sharp);
        let f_sharp = PitchClass::new(NoteLetter::F, Accidental::Sharp);
        assert_eq!(
            chord.pitch_classes().unwrap(),
            vec![PitchClass::B, d_sharp, f_sharp]
        );
        assert_eq!(v_of_v.to_string(), "V/V");
    }

    #[test]
    fn minor_applied_dominant_vocabulary_excludes_iii() {
        // Real corpus data (examples/chorale_benchmark.rs
        // --minor-gap-report, 2026-08-11) found zero minor chorales
        // needing V/III — pin that it's not offered, so a future
        // "just copy major's set" edit gets caught here.
        assert!(
            RomanNumeral::minor_applied_dominant_vocabulary()
                .iter()
                .all(|rn| rn.applied_to() != Some(ScaleDegree::MEDIANT))
        );
    }

    #[test]
    fn a_minor_melodic_minor_ii_is_a_minor_triad_not_diminished() {
        let ii = RomanNumeral::melodic_minor_vocabulary()[0];
        assert_eq!(ii.degree, ScaleDegree::SUPERTONIC);
        let f_sharp = PitchClass::new(NoteLetter::F, Accidental::Sharp);
        assert_eq!(
            ii.to_chord(&Key::A_MINOR).unwrap().pitch_classes().unwrap(),
            vec![PitchClass::B, PitchClass::D, f_sharp]
        );
        assert_eq!(ii.to_string(), "ii");
        assert_eq!(ii.harmonic_function(), HarmonicFunction::Predominant);
    }

    #[test]
    fn a_minor_melodic_minor_iv_is_a_major_triad_not_minor() {
        let iv = RomanNumeral::melodic_minor_vocabulary()[1];
        assert_eq!(iv.degree, ScaleDegree::SUBDOMINANT);
        let f_sharp = PitchClass::new(NoteLetter::F, Accidental::Sharp);
        assert_eq!(
            iv.to_chord(&Key::A_MINOR).unwrap().pitch_classes().unwrap(),
            vec![PitchClass::D, f_sharp, PitchClass::A]
        );
        assert_eq!(iv.to_string(), "IV");
        assert_eq!(iv.harmonic_function(), HarmonicFunction::Predominant);
    }
}
