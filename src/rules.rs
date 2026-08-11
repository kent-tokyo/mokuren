//! The Common Practice rule engine (AGENTS.md sections 8-10).
//!
//! Hard constraints and soft preferences are both `Rule` implementations,
//! distinguished only by `severity()`. A candidate is rejected from the
//! search purely because a hard rule's `status` is `Violation` — never by
//! a large negative `penalty`, so diagnostics counts stay honest (a hard
//! violation is never quietly laundered through the score).

use crate::chord::{Chord, ChordInversion, HarmonicFunction, RomanNumeral};
use crate::key::{Key, ScaleDegree};
use crate::pitch::PitchClass;
use crate::score::{Cadence, Reason, Severity};
use crate::voice::{self, MotionType, VoicePart, Voicing};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum RuleId {
    // Hard constraints.
    VoiceRange,
    VoiceCrossing,
    VoiceOverlap,
    ParallelFifths,
    ParallelOctaves,
    ParallelUnisons,
    Spacing,
    MissingChordTone,
    LeadingToneDoubling,
    LeadingToneResolution,
    ChordalSeventhResolution,
    UnpreparedSixFour,
    SecondaryDominantResolution,
    // Soft preferences.
    VoiceLeadingQuality,
    MelodicMotion,
    HarmonicFunctionProgression,
    CadenceSupport,
    DoublingPreference,
    RepeatedChord,
}

impl fmt::Display for RuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            RuleId::VoiceRange => "voice range",
            RuleId::VoiceCrossing => "voice crossing",
            RuleId::VoiceOverlap => "voice overlap",
            RuleId::ParallelFifths => "parallel fifths",
            RuleId::ParallelOctaves => "parallel octaves",
            RuleId::ParallelUnisons => "parallel unisons",
            RuleId::Spacing => "spacing",
            RuleId::MissingChordTone => "missing chord tone",
            RuleId::LeadingToneDoubling => "leading-tone doubling",
            RuleId::LeadingToneResolution => "leading-tone resolution",
            RuleId::ChordalSeventhResolution => "chordal seventh resolution",
            RuleId::UnpreparedSixFour => "unprepared six-four",
            RuleId::SecondaryDominantResolution => "secondary dominant resolution",
            RuleId::VoiceLeadingQuality => "voice leading",
            RuleId::MelodicMotion => "melodic motion",
            RuleId::HarmonicFunctionProgression => "harmonic function progression",
            RuleId::CadenceSupport => "cadence support",
            RuleId::DoublingPreference => "doubling",
            RuleId::RepeatedChord => "repeated chord",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleStatus {
    Pass,
    Warning,
    Violation,
}

/// A rule's verdict on one candidate. `penalty` is a *signed* score
/// contribution for soft rules (reward or penalty); hard rules leave it
/// at 0.0 and communicate exclusively through `status`.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleResult {
    pub status: RuleStatus,
    pub reasons: Vec<Reason>,
    pub penalty: f64,
}

impl RuleResult {
    fn pass() -> Self {
        RuleResult {
            status: RuleStatus::Pass,
            reasons: Vec::new(),
            penalty: 0.0,
        }
    }

    fn violation(rule: RuleId, severity: Severity) -> Self {
        RuleResult {
            status: RuleStatus::Violation,
            reasons: vec![Reason::RuleViolation { rule, severity }],
            penalty: 0.0,
        }
    }

    fn scored(delta: f64, reason: Option<Reason>) -> Self {
        RuleResult {
            status: RuleStatus::Pass,
            reasons: reason.into_iter().collect(),
            penalty: delta,
        }
    }
}

/// Everything a `Rule` needs to judge one chord transition: the previous
/// chord/voicing (if any) and the current one.
pub struct RuleContext<'a> {
    pub key: &'a Key,
    pub previous: Option<&'a Voicing>,
    pub previous_chord: Option<&'a Chord>,
    pub previous_roman_numeral: Option<&'a RomanNumeral>,
    pub current: &'a Voicing,
    pub chord: &'a Chord,
    pub roman_numeral: &'a RomanNumeral,
    pub is_final_position: bool,
}

pub trait Rule {
    fn id(&self) -> RuleId;
    fn severity(&self) -> Severity;
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult;
}

// ---- Hard constraints -----------------------------------------------

pub struct VoiceRangeRule;
impl Rule for VoiceRangeRule {
    fn id(&self) -> RuleId {
        RuleId::VoiceRange
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        if voice::range_violations(ctx.current).is_empty() {
            RuleResult::pass()
        } else {
            RuleResult::violation(self.id(), self.severity())
        }
    }
}

pub struct VoiceCrossingRule;
impl Rule for VoiceCrossingRule {
    fn id(&self) -> RuleId {
        RuleId::VoiceCrossing
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        if voice::voice_crossings(ctx.current).is_empty() {
            RuleResult::pass()
        } else {
            RuleResult::violation(self.id(), self.severity())
        }
    }
}

pub struct VoiceOverlapRule;
impl Rule for VoiceOverlapRule {
    fn id(&self) -> RuleId {
        RuleId::VoiceOverlap
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        let Some(prev) = ctx.previous else {
            return RuleResult::pass();
        };
        if voice::voice_overlaps(prev, ctx.current).is_empty() {
            RuleResult::pass()
        } else {
            RuleResult::violation(self.id(), self.severity())
        }
    }
}

pub struct ParallelFifthsRule;
impl Rule for ParallelFifthsRule {
    fn id(&self) -> RuleId {
        RuleId::ParallelFifths
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        let Some(prev) = ctx.previous else {
            return RuleResult::pass();
        };
        if voice::parallel_fifths(prev, ctx.current).is_empty() {
            RuleResult::pass()
        } else {
            RuleResult::violation(self.id(), self.severity())
        }
    }
}

pub struct ParallelOctavesRule;
impl Rule for ParallelOctavesRule {
    fn id(&self) -> RuleId {
        RuleId::ParallelOctaves
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        let Some(prev) = ctx.previous else {
            return RuleResult::pass();
        };
        if voice::parallel_octaves(prev, ctx.current).is_empty() {
            RuleResult::pass()
        } else {
            RuleResult::violation(self.id(), self.severity())
        }
    }
}

pub struct ParallelUnisonsRule;
impl Rule for ParallelUnisonsRule {
    fn id(&self) -> RuleId {
        RuleId::ParallelUnisons
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        let Some(prev) = ctx.previous else {
            return RuleResult::pass();
        };
        if voice::parallel_unisons(prev, ctx.current).is_empty() {
            RuleResult::pass()
        } else {
            RuleResult::violation(self.id(), self.severity())
        }
    }
}

pub struct SpacingRule;
impl Rule for SpacingRule {
    fn id(&self) -> RuleId {
        RuleId::Spacing
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        if voice::spacing_violations(ctx.current).is_empty() {
            RuleResult::pass()
        } else {
            RuleResult::violation(self.id(), self.severity())
        }
    }
}

pub struct MissingChordToneRule;
impl Rule for MissingChordToneRule {
    fn id(&self) -> RuleId {
        RuleId::MissingChordTone
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        // An unspellable chord can't be verified complete, so it fails
        // closed as a violation rather than passing by default.
        let Ok(tones) = ctx.chord.pitch_classes() else {
            return RuleResult::violation(self.id(), self.severity());
        };
        let voiced: Vec<PitchClass> = VoicePart::all()
            .into_iter()
            .map(|v| ctx.current.pitch(v).pitch_class)
            .collect();
        let complete = tones
            .iter()
            .all(|tone| voiced.iter().any(|v| v.is_enharmonic_to(tone)));
        if complete {
            RuleResult::pass()
        } else {
            RuleResult::violation(self.id(), self.severity())
        }
    }
}

pub struct LeadingToneDoublingRule;
impl Rule for LeadingToneDoublingRule {
    fn id(&self) -> RuleId {
        RuleId::LeadingToneDoubling
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        let leading_tone = ctx.key.functional_leading_tone();
        let count = VoicePart::all()
            .into_iter()
            .filter(|&v| {
                ctx.current
                    .pitch(v)
                    .pitch_class
                    .is_enharmonic_to(&leading_tone)
            })
            .count();
        if count > 1 {
            RuleResult::violation(self.id(), self.severity())
        } else {
            RuleResult::pass()
        }
    }
}

/// A voice sitting on the leading tone of a dominant-function chord must
/// resolve to the tonic-function chord that follows. Outer voices
/// (soprano, bass) must resolve up by step, strictly — the ear tracks
/// the leading tone's pull most closely there. Inner voices (alto,
/// tenor) get the standard textbook exception: they may instead skip
/// down by step or third to complete the destination chord, which is
/// how a complete tonic triad is normally reached at all when every
/// other voice's target is already spoken for (v0.1 applies this one
/// exception; a chordal seventh, by contrast, resolves down by step in
/// every voice — see `ChordalSeventhResolutionRule`).
pub struct LeadingToneResolutionRule;
impl Rule for LeadingToneResolutionRule {
    fn id(&self) -> RuleId {
        RuleId::LeadingToneResolution
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        let (Some(prev), Some(prev_rn)) = (ctx.previous, ctx.previous_roman_numeral) else {
            return RuleResult::pass();
        };
        if prev_rn.harmonic_function() != HarmonicFunction::Dominant
            || ctx.roman_numeral.harmonic_function() != HarmonicFunction::Tonic
        {
            return RuleResult::pass();
        }
        let leading_tone = ctx.key.functional_leading_tone();
        let tonic = ctx.key.diatonic_pitch_class(ScaleDegree::TONIC);
        for voice in VoicePart::all() {
            let prev_pitch = prev.pitch(voice);
            if !prev_pitch.pitch_class.is_enharmonic_to(&leading_tone) {
                continue;
            }
            let curr_pitch = ctx.current.pitch(voice);
            let motion = curr_pitch.midi() - prev_pitch.midi();
            let resolved_up_by_step =
                motion == 1 && curr_pitch.pitch_class.is_enharmonic_to(&tonic);
            let is_inner_voice = matches!(voice, VoicePart::Alto | VoicePart::Tenor);
            // Step (major or minor 2nd) or third (major or minor) down —
            // e.g. the textbook B -> G a third below.
            let skipped_down_to_complete_chord = is_inner_voice
                && (-4..=-1).contains(&motion)
                && ctx.chord.contains_pitch_class(curr_pitch.pitch_class);
            if !resolved_up_by_step && !skipped_down_to_complete_chord {
                return RuleResult::violation(self.id(), self.severity());
            }
        }
        RuleResult::pass()
    }
}

/// A chordal seventh must resolve down by step in the following chord.
pub struct ChordalSeventhResolutionRule;
impl Rule for ChordalSeventhResolutionRule {
    fn id(&self) -> RuleId {
        RuleId::ChordalSeventhResolution
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        let (Some(prev), Some(prev_chord)) = (ctx.previous, ctx.previous_chord) else {
            return RuleResult::pass();
        };
        let Some(seventh) = prev_chord.chordal_seventh() else {
            return RuleResult::pass();
        };
        for voice in VoicePart::all() {
            let prev_pitch = prev.pitch(voice);
            if !prev_pitch.pitch_class.is_enharmonic_to(&seventh) {
                continue;
            }
            let curr_pitch = ctx.current.pitch(voice);
            let moved_down_by_step = (1..=2).contains(&(prev_pitch.midi() - curr_pitch.midi()));
            if !moved_down_by_step {
                return RuleResult::violation(self.id(), self.severity());
            }
        }
        RuleResult::pass()
    }
}

/// A second-inversion triad (the fifth in the bass) is only usable in
/// Common Practice writing as a cadential, passing, or pedal six-four —
/// never as a freely chosen sonority. v0.1 checks this against the
/// *previous* bass only (pedal: same bass; passing: bass approached by
/// step); it can't see forward to confirm a cadential 6/4 resolves to V,
/// so a six-four with no qualifying previous bass — including the very
/// first chord, which has no previous bass at all — is rejected. Doesn't
/// apply to seventh chords, whose second inversion (43) is unrestricted.
pub struct UnpreparedSixFourRule;
impl Rule for UnpreparedSixFourRule {
    fn id(&self) -> RuleId {
        RuleId::UnpreparedSixFour
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        if ctx.chord.quality.is_seventh() || ctx.roman_numeral.inversion != ChordInversion::Second {
            return RuleResult::pass();
        }
        let Some(prev) = ctx.previous else {
            return RuleResult::violation(self.id(), self.severity());
        };
        let bass_step = (prev.bass.midi() - ctx.current.bass.midi()).abs();
        let is_pedal = bass_step == 0;
        let is_passing_or_neighbor = bass_step == 1 || bass_step == 2;
        if is_pedal || is_passing_or_neighbor {
            RuleResult::pass()
        } else {
            RuleResult::violation(self.id(), self.severity())
        }
    }
}

/// An applied/secondary dominant (`RomanNumeral::applied_to` is `Some`)
/// must resolve to the chord it tonicizes: the *next* chord's root must
/// be that target's diatonic pitch class, and the applied dominant's own
/// chromatic tone (its local leading tone, a semitone below the target)
/// must resolve up by step wherever it's voiced — the same pull
/// `LeadingToneResolutionRule` enforces for the diatonic leading tone,
/// applied to the borrowed one. v0.1 requires strict step-up resolution
/// in every voice (no inner-voice exception yet, unlike
/// `LeadingToneResolutionRule` — see README's "Current limitations"),
/// and rejects an applied dominant outright at the final position, since
/// it can never resolve there.
///
/// *Prolonging* the same applied dominant (same `applied_to` target,
/// e.g. a chromatic tone tied or repeated across two notes before
/// moving on) doesn't count as an unresolved dangling dominant — the
/// resolution obligation only applies once the harmony actually changes
/// away from it. Without this, a genuinely common pattern (Bach chorale
/// baseline, Riemenschneider 102: D#5 held across two consecutive
/// quarter notes before resolving up to E5) was structurally
/// unharmonizable: the *second* note of the hold had nowhere to go,
/// since nothing satisfies "resolve right now" while the soprano itself
/// hasn't moved yet.
pub struct SecondaryDominantResolutionRule;
impl Rule for SecondaryDominantResolutionRule {
    fn id(&self) -> RuleId {
        RuleId::SecondaryDominantResolution
    }
    fn severity(&self) -> Severity {
        Severity::Hard
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        if ctx.roman_numeral.applied_to().is_some() && ctx.is_final_position {
            return RuleResult::violation(self.id(), self.severity());
        }
        let (Some(prev), Some(prev_rn)) = (ctx.previous, ctx.previous_roman_numeral) else {
            return RuleResult::pass();
        };
        let Some(target_pc) = prev_rn.resolution_target(ctx.key) else {
            return RuleResult::pass();
        };
        if ctx.roman_numeral.applied_to() == prev_rn.applied_to() {
            return RuleResult::pass();
        }
        if !ctx.chord.root.is_enharmonic_to(&target_pc) {
            return RuleResult::violation(self.id(), self.severity());
        }
        let Some(chromatic_tone) = prev_rn.applied_leading_tone(ctx.key) else {
            return RuleResult::violation(self.id(), self.severity());
        };
        for voice in VoicePart::all() {
            let prev_pitch = prev.pitch(voice);
            if !prev_pitch.pitch_class.is_enharmonic_to(&chromatic_tone) {
                continue;
            }
            let curr_pitch = ctx.current.pitch(voice);
            let motion = curr_pitch.midi() - prev_pitch.midi();
            let resolved_up_by_step =
                motion == 1 && curr_pitch.pitch_class.is_enharmonic_to(&target_pc);
            if !resolved_up_by_step {
                return RuleResult::violation(self.id(), self.severity());
            }
        }
        RuleResult::pass()
    }
}

// ---- Soft preferences -------------------------------------------------

/// Rewards common-tone retention and contrary outer-voice motion —
/// harmonic voice-leading quality, as distinct from per-voice melodic
/// contour (`MelodicMotionRule`).
pub struct VoiceLeadingRule;
impl Rule for VoiceLeadingRule {
    fn id(&self) -> RuleId {
        RuleId::VoiceLeadingQuality
    }
    fn severity(&self) -> Severity {
        Severity::Soft
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        let Some(prev) = ctx.previous else {
            return RuleResult::pass();
        };
        let common_tones = voice::common_tone_count(prev, ctx.current);
        let contrary_motion = voice::classify_motion(
            prev.pitch(VoicePart::Soprano),
            ctx.current.pitch(VoicePart::Soprano),
            prev.pitch(VoicePart::Bass),
            ctx.current.pitch(VoicePart::Bass),
        ) == MotionType::Contrary;
        let total_motion = voice::total_motion(prev, ctx.current);

        let mut delta = common_tones as f64 * 0.15;
        if contrary_motion {
            delta += 0.25;
        }
        let reason = Reason::VoiceLeading {
            total_motion,
            common_tones,
            contrary_motion,
            score_delta: delta,
        };
        RuleResult::scored(delta, Some(reason))
    }
}

/// Rewards stepwise motion and penalizes large leaps, per voice.
pub struct MelodicMotionRule;
impl Rule for MelodicMotionRule {
    fn id(&self) -> RuleId {
        RuleId::MelodicMotion
    }
    fn severity(&self) -> Severity {
        Severity::Soft
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        let Some(prev) = ctx.previous else {
            return RuleResult::pass();
        };
        let mut delta = 0.0;
        for voice in VoicePart::all() {
            let step = (ctx.current.pitch(voice).midi() - prev.pitch(voice).midi()).abs();
            delta += match step {
                0 => 0.0,
                1 | 2 => 0.08,  // stepwise: reward
                3 | 4 => 0.0,   // small leap (third): neutral
                5..=9 => -0.05, // moderate leap: mild penalty
                _ => -0.25,     // leap larger than a major sixth: penalize
            };
        }
        // No `Reason` here: `Reason::VoiceLeading` is already emitted by
        // `VoiceLeadingRule` for this same transition, and a second copy
        // with a different `score_delta` (this rule's, not that one's)
        // would read as a contradiction rather than a second data point.
        // This rule's contribution is still visible via the
        // `melodic_motion` score field and, when negative, the penalties
        // list.
        RuleResult::scored(delta, None)
    }
}

fn harmonic_transition_score(from: HarmonicFunction, to: HarmonicFunction) -> f64 {
    use HarmonicFunction::*;
    match (from, to) {
        (Tonic, Predominant) => 0.6,
        (Tonic, Dominant) => 0.4,
        (Tonic, Tonic) => 0.1,
        (Predominant, Dominant) => 0.8,
        (Predominant, Predominant) => 0.0,
        (Predominant, Tonic) => 0.2, // plagal-ish motion: allowed, mild reward
        (Dominant, Tonic) => 1.2,    // authentic resolution: strongest
        (Dominant, Dominant) => 0.2,
        (Dominant, Predominant) => -0.6, // functional retrogression
    }
}

pub struct HarmonicFunctionProgressionRule;
impl Rule for HarmonicFunctionProgressionRule {
    fn id(&self) -> RuleId {
        RuleId::HarmonicFunctionProgression
    }
    fn severity(&self) -> Severity {
        Severity::Soft
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        let Some(prev_rn) = ctx.previous_roman_numeral else {
            return RuleResult::pass();
        };
        let from = prev_rn.harmonic_function();
        let to = ctx.roman_numeral.harmonic_function();
        let is_correct_secondary_resolution = prev_rn
            .resolution_target(ctx.key)
            .is_some_and(|target_pc| ctx.chord.root.is_enharmonic_to(&target_pc));
        // The diatonic table's `(Dominant, Predominant) => -0.6` models an
        // unwanted functional retrogression (e.g. V -> IV). That's not
        // what's happening for V/ii -> ii or V/IV -> IV:
        // `SecondaryDominantResolutionRule` *requires* this exact
        // transition as a correct, textbook resolution, not a
        // retrogression — override only this one broken table entry.
        // Every other (from, to) pair, including a resolution that lands
        // on a dominant-function target (V/V -> V), keeps the table's
        // existing (already-sensible) score.
        let delta = if is_correct_secondary_resolution && to == HarmonicFunction::Predominant {
            0.3
        } else if ctx.roman_numeral.applied_to().is_some() {
            // Introducing an applied dominant is scored on its own terms
            // — voice leading now, and the resolution reward above once
            // it actually resolves — not via the diatonic table, which
            // would otherwise hand it the same "arriving at the dominant"
            // reward a true V gets (e.g. Predominant -> Dominant => 0.8)
            // for *any* diatonic soprano note it happens to also fit.
            // Without this, the search preferred gratuitously substituting
            // an applied dominant for an equally valid diatonic chord
            // anywhere one was merely possible, not just where a
            // chromatic soprano tone actually required one.
            0.0
        } else {
            harmonic_transition_score(from, to)
        };
        RuleResult::scored(
            delta,
            Some(Reason::HarmonicFunction {
                from,
                to,
                score_delta: delta,
            }),
        )
    }
}

/// Rewards an authentic/plagal/deceptive cadence landing at the final
/// position; a half cadence gets a smaller nod. v0.1 only judges the
/// final position — phrase-internal cadences are future work (section 20).
pub struct CadenceSupportRule;
impl Rule for CadenceSupportRule {
    fn id(&self) -> RuleId {
        RuleId::CadenceSupport
    }
    fn severity(&self) -> Severity {
        Severity::Soft
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        if !ctx.is_final_position {
            return RuleResult::pass();
        }
        let Some(prev_rn) = ctx.previous_roman_numeral else {
            return RuleResult::pass();
        };
        let from = prev_rn.harmonic_function();
        let to = ctx.roman_numeral.harmonic_function();

        let (cadence, delta) = if from == HarmonicFunction::Dominant
            && to == HarmonicFunction::Tonic
            && ctx.roman_numeral.degree == ScaleDegree::TONIC
        {
            // Closing a phrase properly outweighs the accumulated small
            // voice-leading rewards a mediocre-but-smooth path can rack up
            // over the preceding positions — cadence is the clincher, not
            // a tie-breaker (this is a v0.1 weight, not an architectural
            // claim; see PLAN.md).
            match ctx.key.degree_of(ctx.current.soprano.pitch_class) {
                Some(ScaleDegree::TONIC) => (Cadence::Authentic, 4.0),
                _ => (Cadence::Authentic, 3.0),
            }
        } else if from == HarmonicFunction::Dominant
            && ctx.roman_numeral.degree == ScaleDegree::SUBMEDIANT
        {
            (Cadence::Deceptive, 1.0)
        } else if from == HarmonicFunction::Predominant && to == HarmonicFunction::Tonic {
            (Cadence::Plagal, 1.2)
        } else if to == HarmonicFunction::Dominant && from != HarmonicFunction::Dominant {
            // A half cadence is an *arrival* at the dominant (typically
            // from IV/ii, sometimes from I) — not a second dominant-
            // function chord in a row. Requiring `from != Dominant` stops
            // an applied dominant resolving to a plain V (e.g. V/V -> V)
            // from double-dipping: it already scores well as a correct
            // resolution (`HarmonicFunctionProgressionRule`); rewarding
            // the same move again here as a "cadence" is what let a
            // dominant-of-the-dominant chain outscore an actual tonic
            // close for melodies where both are reachable.
            (Cadence::Half, 0.5)
        } else {
            (Cadence::None, 0.0)
        };

        if cadence == Cadence::None {
            return RuleResult::pass();
        }
        RuleResult::scored(
            delta,
            Some(Reason::CadenceSupport {
                cadence,
                score_delta: delta,
            }),
        )
    }
}

fn doubled_pitch_class(v: &Voicing) -> Option<PitchClass> {
    let pcs: Vec<PitchClass> = VoicePart::all()
        .into_iter()
        .map(|voice| v.pitch(voice).pitch_class)
        .collect();
    pcs.iter()
        .find(|pc| {
            pcs.iter()
                .filter(|other| other.is_enharmonic_to(pc))
                .count()
                > 1
        })
        .copied()
}

/// Rewards doubling the root in a triad over doubling the third (the
/// fifth is a neutral middle ground). Seventh chords carry all four
/// tones already, so there's nothing to judge.
pub struct DoublingPreferenceRule;
impl Rule for DoublingPreferenceRule {
    fn id(&self) -> RuleId {
        RuleId::DoublingPreference
    }
    fn severity(&self) -> Severity {
        Severity::Soft
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        if ctx.chord.quality.is_seventh() {
            return RuleResult::pass();
        }
        let Some(doubled) = doubled_pitch_class(ctx.current) else {
            return RuleResult::pass();
        };
        // A soft preference with nothing to score is just a no-op, not a
        // wrong answer — unlike MissingChordToneRule, this isn't a gate.
        let Ok(tones) = ctx.chord.pitch_classes() else {
            return RuleResult::pass();
        };
        let delta = if doubled.is_enharmonic_to(&tones[0]) {
            0.3 // doubled root
        } else if doubled.is_enharmonic_to(&tones[2]) {
            0.05 // doubled fifth
        } else {
            -0.1 // doubled third
        };
        RuleResult::scored(delta, None)
    }
}

/// Mildly discourages repeating the exact same Roman numeral (degree,
/// quality, and inversion) back to back.
pub struct RepeatedChordRule;
impl Rule for RepeatedChordRule {
    fn id(&self) -> RuleId {
        RuleId::RepeatedChord
    }
    fn severity(&self) -> Severity {
        Severity::Soft
    }
    fn evaluate(&self, ctx: &RuleContext) -> RuleResult {
        let Some(prev_rn) = ctx.previous_roman_numeral else {
            return RuleResult::pass();
        };
        if prev_rn == ctx.roman_numeral {
            RuleResult::scored(-0.2, None)
        } else {
            RuleResult::pass()
        }
    }
}

/// A named, reusable rule set (AGENTS.md section 9's `StyleProfile`).
/// v0.1 ships one style; adding another means writing a new `rules()`
/// arm, not touching the rule implementations above.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Style {
    CommonPractice,
}

impl Style {
    pub fn rules(&self) -> Vec<Box<dyn Rule>> {
        match self {
            Style::CommonPractice => vec![
                Box::new(VoiceRangeRule),
                Box::new(VoiceCrossingRule),
                Box::new(VoiceOverlapRule),
                Box::new(ParallelFifthsRule),
                Box::new(ParallelOctavesRule),
                Box::new(ParallelUnisonsRule),
                Box::new(SpacingRule),
                Box::new(MissingChordToneRule),
                Box::new(LeadingToneDoublingRule),
                Box::new(LeadingToneResolutionRule),
                Box::new(ChordalSeventhResolutionRule),
                Box::new(UnpreparedSixFourRule),
                Box::new(SecondaryDominantResolutionRule),
                Box::new(VoiceLeadingRule),
                Box::new(MelodicMotionRule),
                Box::new(HarmonicFunctionProgressionRule),
                Box::new(CadenceSupportRule),
                Box::new(DoublingPreferenceRule),
                Box::new(RepeatedChordRule),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pitch::{Accidental, NoteLetter, Octave, Pitch};

    #[allow(clippy::too_many_arguments)]
    fn ctx<'a>(
        key: &'a Key,
        previous: Option<&'a Voicing>,
        previous_chord: Option<&'a Chord>,
        previous_roman_numeral: Option<&'a RomanNumeral>,
        current: &'a Voicing,
        chord: &'a Chord,
        roman_numeral: &'a RomanNumeral,
        is_final_position: bool,
    ) -> RuleContext<'a> {
        RuleContext {
            key,
            previous,
            previous_chord,
            previous_roman_numeral,
            current,
            chord,
            roman_numeral,
            is_final_position,
        }
    }

    fn v(
        s: (PitchClass, i32),
        a: (PitchClass, i32),
        t: (PitchClass, i32),
        b: (PitchClass, i32),
    ) -> Voicing {
        Voicing::new(
            Pitch::new(s.0, Octave(s.1)),
            Pitch::new(a.0, Octave(a.1)),
            Pitch::new(t.0, Octave(t.1)),
            Pitch::new(b.0, Octave(b.1)),
        )
    }

    #[test]
    fn missing_chord_tone_is_a_hard_violation() {
        let key = Key::C_MAJOR;
        let chord = RomanNumeral::I.to_chord(&key).unwrap();
        // No third (E) anywhere: C,C,G,C.
        let current = v(
            (PitchClass::C, 5),
            (PitchClass::C, 4),
            (PitchClass::G, 3),
            (PitchClass::C, 3),
        );
        let result = MissingChordToneRule.evaluate(&ctx(
            &key,
            None,
            None,
            None,
            &current,
            &chord,
            &RomanNumeral::I,
            false,
        ));
        assert_eq!(result.status, RuleStatus::Violation);
    }

    #[test]
    fn leading_tone_in_outer_voice_must_resolve_up() {
        let key = Key::C_MAJOR;
        let prev_chord = RomanNumeral::V.to_chord(&key).unwrap();
        let chord = RomanNumeral::I.to_chord(&key).unwrap();
        // Previous: soprano holds B4 (leading tone).
        let prev = v(
            (PitchClass::B, 4),
            (PitchClass::D, 4),
            (PitchClass::G, 3),
            (PitchClass::G, 2),
        );
        // Soprano skips down to G4 (a chord tone, and the same shape an
        // inner voice is allowed) instead of resolving up: still a
        // violation, because soprano is an outer voice.
        let bad_curr = v(
            (PitchClass::G, 4),
            (PitchClass::C, 4),
            (PitchClass::E, 3),
            (PitchClass::C, 3),
        );
        let result = LeadingToneResolutionRule.evaluate(&ctx(
            &key,
            Some(&prev),
            Some(&prev_chord),
            Some(&RomanNumeral::V),
            &bad_curr,
            &chord,
            &RomanNumeral::I,
            false,
        ));
        assert_eq!(result.status, RuleStatus::Violation);

        // Soprano resolves up to C5: passes.
        let good_curr = v(
            (PitchClass::C, 5),
            (PitchClass::E, 4),
            (PitchClass::G, 3),
            (PitchClass::C, 3),
        );
        let result = LeadingToneResolutionRule.evaluate(&ctx(
            &key,
            Some(&prev),
            Some(&prev_chord),
            Some(&RomanNumeral::V),
            &good_curr,
            &chord,
            &RomanNumeral::I,
            false,
        ));
        assert_eq!(result.status, RuleStatus::Pass);
    }

    #[test]
    fn minor_key_raised_leading_tone_must_also_resolve_up() {
        // The natural (unraised) 7th, G, wouldn't be caught by this rule
        // at all before `Key::functional_leading_tone` existed — only
        // the *raised* G# (harmonic minor's own leading tone) should be.
        let key = Key::A_MINOR;
        let g_sharp = PitchClass::new(NoteLetter::G, Accidental::Sharp);
        let harmonic_v = RomanNumeral::harmonic_minor_vocabulary()[0];
        let i = RomanNumeral::natural_minor_vocabulary()[0];
        let prev_chord = harmonic_v.to_chord(&key).unwrap();
        let chord = i.to_chord(&key).unwrap();
        // Previous: soprano holds G#4 (the raised leading tone).
        let prev = v(
            (g_sharp, 4),
            (PitchClass::B, 3),
            (PitchClass::E, 3),
            (PitchClass::E, 2),
        );
        // Soprano leaps down to E4 instead of resolving up to A4.
        let bad_curr = v(
            (PitchClass::E, 4),
            (PitchClass::C, 4),
            (PitchClass::A, 3),
            (PitchClass::A, 2),
        );
        let result = LeadingToneResolutionRule.evaluate(&ctx(
            &key,
            Some(&prev),
            Some(&prev_chord),
            Some(&harmonic_v),
            &bad_curr,
            &chord,
            &i,
            false,
        ));
        assert_eq!(result.status, RuleStatus::Violation);

        // Soprano resolves up to A4: passes.
        let good_curr = v(
            (PitchClass::A, 4),
            (PitchClass::C, 4),
            (PitchClass::E, 3),
            (PitchClass::A, 2),
        );
        let result = LeadingToneResolutionRule.evaluate(&ctx(
            &key,
            Some(&prev),
            Some(&prev_chord),
            Some(&harmonic_v),
            &good_curr,
            &chord,
            &i,
            false,
        ));
        assert_eq!(result.status, RuleStatus::Pass);
    }

    #[test]
    fn leading_tone_in_inner_voice_may_skip_down_to_complete_the_chord() {
        let key = Key::C_MAJOR;
        let prev_chord = RomanNumeral::V.to_chord(&key).unwrap();
        let chord = RomanNumeral::I.to_chord(&key).unwrap();
        // Previous: tenor holds B3 (leading tone).
        let prev = v(
            (PitchClass::G, 4),
            (PitchClass::D, 4),
            (PitchClass::B, 3),
            (PitchClass::G, 2),
        );
        // Tenor skips down a third to G3 — the textbook inner-voice
        // exception, needed to complete the tonic triad — passes.
        let skips_down_to_chord_tone = v(
            (PitchClass::C, 5),
            (PitchClass::C, 4),
            (PitchClass::G, 3),
            (PitchClass::C, 3),
        );
        let result = LeadingToneResolutionRule.evaluate(&ctx(
            &key,
            Some(&prev),
            Some(&prev_chord),
            Some(&RomanNumeral::V),
            &skips_down_to_chord_tone,
            &chord,
            &RomanNumeral::I,
            false,
        ));
        assert_eq!(result.status, RuleStatus::Pass);

        // Tenor resolves up to C4: also passes.
        let resolves_up = v(
            (PitchClass::C, 5),
            (PitchClass::E, 4),
            (PitchClass::C, 4),
            (PitchClass::C, 3),
        );
        let result = LeadingToneResolutionRule.evaluate(&ctx(
            &key,
            Some(&prev),
            Some(&prev_chord),
            Some(&RomanNumeral::V),
            &resolves_up,
            &chord,
            &RomanNumeral::I,
            false,
        ));
        assert_eq!(result.status, RuleStatus::Pass);

        // Tenor leaps down to F3 — not a chord tone of I at all, and
        // further than the allowed step/third: still a violation.
        let leaps_to_non_chord_tone = v(
            (PitchClass::C, 5),
            (PitchClass::C, 4),
            (PitchClass::F, 3),
            (PitchClass::C, 3),
        );
        let result = LeadingToneResolutionRule.evaluate(&ctx(
            &key,
            Some(&prev),
            Some(&prev_chord),
            Some(&RomanNumeral::V),
            &leaps_to_non_chord_tone,
            &chord,
            &RomanNumeral::I,
            false,
        ));
        assert_eq!(result.status, RuleStatus::Violation);
    }

    #[test]
    fn authentic_cadence_scores_higher_than_plain_tonic_arrival() {
        let key = Key::C_MAJOR;
        let chord = RomanNumeral::I.to_chord(&key).unwrap();
        let curr = v(
            (PitchClass::C, 5),
            (PitchClass::E, 4),
            (PitchClass::C, 4),
            (PitchClass::C, 3),
        );
        let result = CadenceSupportRule.evaluate(&ctx(
            &key,
            None,
            None,
            Some(&RomanNumeral::V7),
            &curr,
            &chord,
            &RomanNumeral::I,
            true,
        ));
        assert!(result.penalty > 1.0);
        assert!(matches!(
            result.reasons[0],
            Reason::CadenceSupport {
                cadence: Cadence::Authentic,
                ..
            }
        ));
    }

    #[test]
    fn v7_first_inversion_displays_as_v65() {
        assert_eq!(
            RomanNumeral::V7
                .with_inversion(ChordInversion::First)
                .to_string(),
            "V65"
        );
    }
}
