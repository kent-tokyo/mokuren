//! External chorale benchmark harness (BENCHMARK.md). Measures whether
//! mokuren's reasoning holds up on melodies it was never tuned against —
//! not whether it matches the original harmonization note-for-note (see
//! BENCHMARK.md's explicit non-goal).
//!
//! No chorale data is vendored in this repository. BENCHMARK.md decided
//! music21 as the canonical external source (2026-08-10) — its Bach
//! chorales carry Margaret Greentree's explicit permission for
//! distribution as part of music21 specifically — but still references
//! it rather than vendoring: `tools/music21_chorale_extractor.py` reads
//! your own local music21 install and writes `.chorale` v2 files
//! nowhere this repository commits them.
//!
//!   cargo run --release --example chorale_benchmark -- path/to/chorales [--report path/to/report.md]
//!
//! Fixture format v3 — one `.chorale` file per piece. `soprano` carries
//! real onset/pitch/duration (v1 forced every note to a quarter, which
//! silently discarded real chorale rhythm; v2 couldn't represent a rest
//! at all — see tasks/lessons.md for both). `alto`/`tenor`/`bass` are
//! reference pitches *sampled at each soprano note onset* (rests have no
//! onset to sample), not an independent onset grid: giving them their
//! own offsets/durations would leak where Bach changed harmony into data
//! a benchmark run is supposed to discover, not read off the input.
//!
//! ```text
//! name: <label>
//! key: <tonic pitch class, e.g. C, F#, Bb>
//! mode: <major | minor — optional, defaults to major>
//! meter: <e.g. 4/4 — carried through, not yet consumed by any rule>
//! soprano:
//! <offset in quarter-note beats> <pitch or REST> <duration, as a fraction of a whole note, e.g. 1/4>
//! ...one line per note or rest, offsets contiguous
//!
//! alto: <optional pitch list, one per soprano *note* onset (rests excluded), e.g. A4 A4 F4 ...>
//! tenor: <optional, same>
//! bass: <optional, same>
//! ```
//!
//! A soprano rest splits the piece into independent phrases (one per
//! contiguous run of notes) via `mokuren::melody::MelodyLine::phrases`,
//! matching how a breath rest actually functions in chorale writing — a
//! phrase boundary, not a gap inside one harmonic idea. Each phrase is
//! harmonized independently through the same `Composer::harmonize` a
//! rest-free chorale uses; a chorale counts as "covered" only if *every*
//! one of its phrases harmonizes (see `aggregate_by_chorale`).
//!
//! `examples/chorale_benchmark_fixtures/` has synthetic smoke-test
//! fixtures (melodies written for this harness, not real chorales) —
//! run against those to see the report format without needing a corpus.
//!
//! Failures are never lumped into one "coverage" bucket (BENCHMARK.md):
//! each is classified as chromatic-soprano (a pitch class with no chord
//! at all in mokuren's vocabulary — neither diatonic nor one of the
//! implemented applied dominants), search-exhausted (a wider beam finds
//! a path), a specific rule conflict (identified by bisecting to the
//! shortest failing prefix and inspecting `CandidateGenerator`'s
//! rejection reasons there), or other.

use mokuren::diagnostics::Diagnostics;
use mokuren::generate::{CandidateGenerator, CandidateStatus};
use mokuren::key::Mode;
use mokuren::melody::{Duration as NoteDuration, MelodyEvent, MelodyLine, Note, Position, Rest};
use mokuren::pitch::Pitch;
use mokuren::prelude::*;
use mokuren::rules::RuleId;
use mokuren::score::{Cadence, Reason};
use mokuren::voice::Voicing;
use std::collections::BTreeMap;
use std::time::Instant;

// ---- Fixture parsing (v3, rest-aware) -----------------------------

struct ChoraleFixture {
    /// Display name — the base chorale name, with a `[phrase i/n]` suffix
    /// when `phrase.1 > 1`.
    name: String,
    /// Chorale identity shared by every phrase split from the same file,
    /// used to re-group phrase-level metrics for chorale-level coverage.
    base_name: String,
    /// (1-based index, total phrase count) within this chorale.
    phrase: (usize, usize),
    key: Key,
    soprano: Melody,
    reference_alto: Option<Vec<Pitch>>,
    reference_tenor: Option<Vec<Pitch>>,
    reference_bass: Option<Vec<Pitch>>,
}

fn parse_pitches(s: &str) -> std::result::Result<Vec<Pitch>, String> {
    s.split_whitespace()
        .map(|tok| tok.parse().map_err(|e| format!("bad pitch {tok:?}: {e}")))
        .collect()
}

/// Parses `n/d` as a fraction of a whole note (`1/4` = quarter = 1.0
/// beat, `3/8` = dotted quarter = 1.5 beats) into a `NoteDuration`.
fn parse_duration(s: &str) -> std::result::Result<NoteDuration, String> {
    let (num, den) = s
        .split_once('/')
        .ok_or_else(|| format!("expected `n/d`, got {s:?}"))?;
    let num: f64 = num.parse().map_err(|_| format!("bad numerator in {s:?}"))?;
    let den: f64 = den
        .parse()
        .map_err(|_| format!("bad denominator in {s:?}"))?;
    let beats = 4.0 * num / den;
    NoteDuration::from_beats(beats)
        .ok_or_else(|| format!("{s:?} ({beats} beats) has no representable Duration"))
}

/// One `offset pitch duration` line inside a `soprano:` block — `pitch`
/// is the literal token `REST` for a rest.
struct SopranoEvent {
    offset: f64,
    event: MelodyEvent,
}

fn parse_soprano_event(line: &str) -> std::result::Result<SopranoEvent, String> {
    let mut tokens = line.split_whitespace();
    let (Some(offset), Some(pitch), Some(duration), None) =
        (tokens.next(), tokens.next(), tokens.next(), tokens.next())
    else {
        return Err(format!("expected `offset pitch duration`, got {line:?}"));
    };
    let offset: f64 = offset
        .parse()
        .map_err(|_| format!("bad offset in {line:?}"))?;
    let duration = parse_duration(duration)?;
    let event = if pitch == "REST" {
        MelodyEvent::Rest(Rest { duration })
    } else {
        let pitch: Pitch = pitch
            .parse()
            .map_err(|e| format!("bad pitch in {line:?}: {e}"))?;
        MelodyEvent::Note(Note::new(pitch, duration))
    };
    Ok(SopranoEvent { offset, event })
}

fn event_duration(event: &MelodyEvent) -> NoteDuration {
    match event {
        MelodyEvent::Note(n) => n.duration,
        MelodyEvent::Rest(r) => r.duration,
    }
}

/// Builds a contiguous `MelodyLine` from parsed events, verifying each
/// event's offset lines up exactly with the previous one's end — an
/// overlap or out-of-order line is a data error worth surfacing rather
/// than silently misaligning. A rest is a legitimate event here (unlike
/// v2's fixture format); it becomes a phrase boundary once `.phrases()`
/// splits this line, not a parse error.
fn build_soprano_line(events: &[SopranoEvent]) -> std::result::Result<MelodyLine, String> {
    if events.is_empty() {
        return Err("soprano block has no events".to_string());
    }
    let mut expected_offset = events[0].offset;
    for event in events {
        if (event.offset - expected_offset).abs() > 1e-6 {
            return Err(format!(
                "soprano offset {} doesn't follow the previous event's end ({expected_offset}) — an overlap or out-of-order line",
                event.offset
            ));
        }
        expected_offset += event_duration(&event.event).beats();
    }
    Ok(MelodyLine::new(events.iter().map(|e| e.event).collect()))
}

fn parse_chorale_fixture(text: &str) -> std::result::Result<Vec<ChoraleFixture>, String> {
    let (mut name, mut tonic, mut mode, mut meter) = (None, None, None, None);
    let (mut alto, mut tenor, mut bass) = (None, None, None);
    let mut soprano_events: Vec<SopranoEvent> = Vec::new();
    let mut in_soprano_block = false;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            in_soprano_block = false;
            continue;
        }
        if line.starts_with('#') {
            continue;
        }
        const KNOWN_FIELDS: [&str; 8] = [
            "name", "key", "mode", "meter", "soprano", "alto", "tenor", "bass",
        ];
        let starts_new_field = line
            .split_once(':')
            .is_some_and(|(field, _)| KNOWN_FIELDS.contains(&field.trim()));
        if in_soprano_block && !starts_new_field {
            soprano_events.push(parse_soprano_event(line)?);
            continue;
        }
        let (field, value) = line
            .split_once(':')
            .ok_or_else(|| format!("expected `field: value`, got {line:?}"))?;
        let value = value.trim();
        in_soprano_block = false;
        match field.trim() {
            "name" => name = Some(value.to_string()),
            "key" => {
                let pc: mokuren::pitch::PitchClass = value
                    .parse()
                    .map_err(|e| format!("bad key {value:?}: {e}"))?;
                tonic = Some(pc);
            }
            // Absent means major — keeps every pre-v3-with-mode fixture
            // (and any hand-written one that doesn't care) parseable
            // without change.
            "mode" => {
                mode = Some(match value {
                    "major" => Mode::Major,
                    "minor" => Mode::Minor,
                    other => return Err(format!("unknown mode {other:?} (expected major/minor)")),
                });
            }
            "meter" => meter = Some(value.to_string()),
            "soprano" => in_soprano_block = true,
            "alto" => alto = Some(parse_pitches(value)?),
            "tenor" => tenor = Some(parse_pitches(value)?),
            "bass" => bass = Some(parse_pitches(value)?),
            other => return Err(format!("unknown field {other:?}")),
        }
    }

    let _meter = meter; // carried through for future use; not consumed by any rule yet.
    let base_name = name.ok_or("missing `name:`")?;
    let tonic = tonic.ok_or("missing `key:`")?;
    let key = Key::new(tonic, mode.unwrap_or(Mode::Major))
        .map_err(|e| format!("key {tonic:?} is not constructible: {e}"))?;
    let line = build_soprano_line(&soprano_events)?;
    let phrases = line.phrases();
    if phrases.is_empty() {
        return Err("soprano block has no notes (only rests)".to_string());
    }

    // Reference alto/tenor/bass are sampled once per soprano *note*
    // (rests have no onset to sample against — see the fixture-format
    // doc comment), so they line up with the concatenation of every
    // phrase's notes in order; slice out each phrase's share by count.
    let mut alto_cursor = 0;
    let mut tenor_cursor = 0;
    let mut bass_cursor = 0;
    let phrase_count = phrases.len();
    let mut fixtures = Vec::with_capacity(phrase_count);
    for (i, phrase) in phrases.into_iter().enumerate() {
        let n = phrase.len();
        let name = if phrase_count == 1 {
            base_name.clone()
        } else {
            format!("{base_name} [phrase {}/{phrase_count}]", i + 1)
        };
        let slice = |cursor: &mut usize, reference: &Option<Vec<Pitch>>| {
            reference.as_ref().map(|v| {
                let start = *cursor;
                let end = (start + n).min(v.len());
                *cursor = end;
                v[start..end].to_vec()
            })
        };
        fixtures.push(ChoraleFixture {
            name,
            base_name: base_name.clone(),
            phrase: (i + 1, phrase_count),
            key,
            soprano: phrase,
            reference_alto: slice(&mut alto_cursor, &alto),
            reference_tenor: slice(&mut tenor_cursor, &tenor),
            reference_bass: slice(&mut bass_cursor, &bass),
        });
    }
    Ok(fixtures)
}

// ---- Failure classification --------------------------------------------

/// Widths tried, in order, when a chorale fails at the standard width —
/// also the data behind the beam-width coverage curve in the report.
/// Widened to 512 (from a v0.1.0 ceiling of 256) after the secondary-
/// dominant vocabulary roughly doubled candidates per position: some
/// chorales that used to succeed at width 32 needed up to 512 to
/// recover, and without this wider ceiling `classify_failure` mislabeled
/// them as `Other` (undiagnosed) rather than `SearchExhausted` — a
/// classification bug, not evidence they were structurally unsolvable.
const RETRY_WIDTHS: [usize; 4] = [64, 128, 256, 512];
const STANDARD_WIDTH: usize = 32;

#[derive(Debug, Clone)]
enum FailureCategory {
    /// A soprano pitch class has no chord at all in mokuren's current
    /// vocabulary — neither diatonic nor one of the implemented applied
    /// dominants (V/x, V7/x). Never fixed by a wider beam; only by
    /// adding more chromatic harmony (modal mixture, Neapolitan, ...).
    ChromaticSoprano,
    /// Fully diatonic, but the standard beam width missed a path a
    /// wider one finds. Not a rule gap — search breadth.
    SearchExhausted { first_working_width: usize },
    /// Fully diatonic, fails even at the widest retried beam. Bisected
    /// to the shortest failing prefix and found one rule dominating
    /// every candidate's rejection there.
    RuleConflict(RuleId),
    /// Fully diatonic, fails even at the widest retried beam, and no
    /// single rule dominates the bisected failure point's rejections.
    Other,
}

impl std::fmt::Display for FailureCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FailureCategory::ChromaticSoprano => write!(f, "chromatic soprano unsupported"),
            FailureCategory::SearchExhausted {
                first_working_width,
            } => {
                write!(f, "search exhausted (works at width {first_working_width})")
            }
            FailureCategory::RuleConflict(rule) => write!(f, "rule conflict ({rule})"),
            FailureCategory::Other => write!(f, "other"),
        }
    }
}

/// A pitch class mokuren's current harmonic vocabulary has *no* chord
/// for at all — neither a diatonic one nor one of the implemented
/// applied dominants (V/x, V7/x for x in ii/iii/IV/V/vi). Applied
/// dominants only cover *some* chromatic alterations (each one's own
/// local leading tone); a soprano tone from an unimplemented category
/// (modal mixture, Neapolitan, augmented sixths, ...) is still
/// genuinely unsupported, so this checks actual chord coverage rather
/// than "is this pitch class in the plain diatonic scale."
fn is_harmonically_unreachable(pitch_class: mokuren::pitch::PitchClass, key: &Key) -> bool {
    if key.degree_of(pitch_class).is_some() {
        return false;
    }
    // The chromatic layer that could still reach this tone differs by
    // mode (applied dominants in major, the harmonic-minor-raised
    // V/V7/vii° in minor — chord.rs) — checking the wrong one here is
    // the exact mistake this crate's own history already made once
    // (applied dominants landing without `classify_failure` updating to
    // match, see tasks/lessons.md), just for minor mode instead of major.
    let extra_vocabulary: Vec<RomanNumeral> = match key.mode {
        Mode::Major => RomanNumeral::applied_dominant_vocabulary(),
        Mode::Minor => RomanNumeral::harmonic_minor_vocabulary().to_vec(),
    };
    !extra_vocabulary.iter().any(|rn| {
        rn.to_chord(key)
            .is_some_and(|chord| chord.contains_pitch_class(pitch_class))
    })
}

fn has_unsupported_chromatic_tone(melody: &Melody, key: &Key) -> bool {
    melody
        .notes
        .iter()
        .any(|n| is_harmonically_unreachable(n.pitch.pitch_class, key))
}

/// Candidate applied-dominant targets to check a minor-key chromatic
/// tone against — the same excluded set (tonic, leading tone) major's
/// `applied_dominant_vocabulary` uses, not a claim these are the only
/// targets that matter in minor (that's exactly what `minor_gap_report`
/// exists to find out from real data, not to assume).
const CANDIDATE_APPLIED_DOMINANT_TARGETS: [mokuren::key::ScaleDegree; 5] = [
    mokuren::key::ScaleDegree::SUPERTONIC,
    mokuren::key::ScaleDegree::MEDIANT,
    mokuren::key::ScaleDegree::SUBDOMINANT,
    mokuren::key::ScaleDegree::DOMINANT,
    mokuren::key::ScaleDegree::SUBMEDIANT,
];

/// What would make an unreachable minor-key soprano pitch class
/// reachable, if anything obvious would — used to decide which
/// chromatic vocabulary is actually worth implementing for minor mode,
/// from real corpus evidence instead of copying major's applied-dominant
/// set wholesale (2026-08-11 user directive: bisect first, implement
/// only what the data supports).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum GapClass {
    /// Matches the melodic-minor raised 6th (a semitone above natural
    /// minor's own submediant) — mokuren has no melodic-minor concept.
    RaisedSixth,
    /// Would be reachable via V/`target` or V7/`target`, if that
    /// applied-dominant target were implemented for minor keys.
    AppliedDominant(mokuren::key::ScaleDegree),
    /// Doesn't match any candidate above — a genuine non-chord tone,
    /// or a target outside the candidate set checked here.
    Other,
}

/// Every classification that would explain `pitch_class` — deliberately
/// not just the first match, since a pitch class can coincide with more
/// than one applied dominant's chord tones; the caller decides which
/// combination of additions covers the most real failures.
fn classify_gap(pitch_class: mokuren::pitch::PitchClass, key: &Key) -> Vec<GapClass> {
    let mut classes = Vec::new();
    let submediant = key.diatonic_pitch_class(mokuren::key::ScaleDegree::SUBMEDIANT);
    if (pitch_class.semitone() as i32 - submediant.semitone() as i32).rem_euclid(12) == 1 {
        classes.push(GapClass::RaisedSixth);
    }
    for &target in &CANDIDATE_APPLIED_DOMINANT_TARGETS {
        let matches = [
            RomanNumeral::applied_dominant(target, mokuren::chord::ChordQuality::MajorTriad),
            RomanNumeral::applied_dominant(target, mokuren::chord::ChordQuality::DominantSeventh),
        ]
        .iter()
        .any(|rn| {
            rn.to_chord(key)
                .is_some_and(|chord| chord.contains_pitch_class(pitch_class))
        });
        if matches {
            classes.push(GapClass::AppliedDominant(target));
        }
    }
    if classes.is_empty() {
        classes.push(GapClass::Other);
    }
    classes
}

fn gap_class_label(class: &GapClass) -> String {
    const NAMES: [&str; 7] = ["I", "ii", "iii", "IV", "V", "vi", "vii"];
    match class {
        GapClass::RaisedSixth => "raised 6th (melodic minor)".to_string(),
        GapClass::AppliedDominant(target) => {
            format!("V(7)/{}", NAMES[(target.0 as usize - 1) % 7])
        }
        GapClass::Other => "other (non-chord tone / unclassified)".to_string(),
    }
}

/// For every minor-key fixture whose soprano has an unreachable
/// chromatic tone, classifies each occurrence and tallies at the
/// *chorale* level (a chorale with multiple different gap classes counts
/// once per class, not once per note) — the real-corpus evidence
/// `tasks/todo.md`'s minor-applied-dominant phase asked for before
/// choosing which targets to implement.
fn minor_gap_report(fixtures: &[ChoraleFixture]) {
    let mut chorale_classes: BTreeMap<String, std::collections::BTreeSet<GapClass>> =
        BTreeMap::new();
    let mut total_minor_chorales: std::collections::BTreeSet<String> = Default::default();
    for fixture in fixtures {
        if fixture.key.mode != Mode::Minor {
            continue;
        }
        total_minor_chorales.insert(fixture.base_name.clone());
        for note in &fixture.soprano.notes {
            if is_harmonically_unreachable(note.pitch.pitch_class, &fixture.key) {
                let classes = classify_gap(note.pitch.pitch_class, &fixture.key);
                chorale_classes
                    .entry(fixture.base_name.clone())
                    .or_default()
                    .extend(classes);
            }
        }
    }

    let mut tally: BTreeMap<GapClass, usize> = BTreeMap::new();
    for classes in chorale_classes.values() {
        for class in classes {
            *tally.entry(class.clone()).or_insert(0) += 1;
        }
    }

    println!(
        "minor-mode chorales with an unreachable chromatic soprano tone: {}/{}",
        chorale_classes.len(),
        total_minor_chorales.len()
    );
    println!("(a chorale can appear under multiple classes if it has more than one kind of gap)\n");
    let mut by_count: Vec<(&GapClass, &usize)> = tally.iter().collect();
    by_count.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
    for (class, count) in by_count {
        println!("{:>4}  {}", count, gap_class_label(class));
    }

    let needs_raised_sixth = chorale_classes
        .values()
        .filter(|classes| classes.contains(&GapClass::RaisedSixth))
        .count();
    let needs_other = chorale_classes
        .values()
        .filter(|classes| classes.contains(&GapClass::Other))
        .count();
    let applied_dominants_alone_would_fully_resolve = chorale_classes
        .values()
        .filter(|classes| {
            classes
                .iter()
                .all(|c| matches!(c, GapClass::AppliedDominant(_)))
        })
        .count();
    println!(
        "\n{applied_dominants_alone_would_fully_resolve}/{} chorales would be *fully* resolved by applied dominants alone (no other gap class present)",
        chorale_classes.len()
    );
    println!(
        "{needs_raised_sixth}/{} chorales have a note needing the raised 6th regardless of applied-dominant coverage",
        chorale_classes.len()
    );
    println!(
        "{needs_other}/{} chorales have a note matching neither candidate (genuine non-chord tone or an untried target)",
        chorale_classes.len()
    );
}

fn harmonizes_at_width(fixture: &ChoraleFixture, width: usize) -> bool {
    Composer::new()
        .key(fixture.key)
        .style(Style::CommonPractice)
        .search(BeamSearch::new().width(width))
        .harmonize(fixture.soprano.clone())
        .is_ok()
}

type NumeralRank = (u8, u8, u8, u8);
type VoicingRank = (i32, i32, i32, i32);
type PathEntry = (Vec<(RomanNumeral, Voicing)>, f64, u32);

/// Canonical (RomanNumeral, Voicing) ranking key — duplicated from
/// `search::path_key`/`generate::canonical_rank` (both private to their
/// modules) since `replay_to_failure` below needs to reproduce
/// `BeamSearch`'s *exact* tie-break chain, not an approximation of it.
fn numeral_voicing_key(rn: &RomanNumeral, v: &Voicing) -> (NumeralRank, VoicingRank) {
    (
        (
            rn.degree.0,
            rn.quality as u8,
            rn.inversion as u8,
            rn.applied_to().map_or(0, |d| d.0),
        ),
        (
            v.soprano.midi(),
            v.alto.midi(),
            v.tenor.midi(),
            v.bass.midi(),
        ),
    )
}

/// Replays beam search over `fixture`'s soprano melody at `width`, one
/// position at a time, and reports the *first* position where the beam
/// empties out — the exact structural failure point — along with the
/// winning path's context reaching it (`None` context at position 0).
/// Returns `None` if the melody never actually gets stuck (shouldn't
/// happen given every caller only invokes this after confirming
/// `harmonizes_at_width(fixture, width)` is `false`).
///
/// This replaces an earlier version that bisected by harmonizing
/// *truncated* melodies directly: whatever prefix length was being
/// tested there had its own last note treated as the final position by
/// the search (`BeamSearch` computes `is_final` from the melody it's
/// given), incorrectly triggering `CadenceSupportRule` and — worse,
/// once `SecondaryDominantResolutionRule` existed — its "no applied
/// dominant at the final position" rejection at a position that isn't
/// actually final in the real piece. That made truncation-based
/// bisection systematically over-report failures wherever the true
/// continuation happened to need an applied dominant, since *every*
/// candidate at an artificially-final position would be rejected
/// outright regardless of context. Replaying `is_final = false` for
/// every position except the real final index (which this function
/// only reaches if the melody doesn't actually get stuck) avoids that
/// entirely — see `tasks/lessons.md`.
///
/// Ranks paths with the *same* tie-break chain as `BeamSearch`
/// (cumulative score, then cumulative voice-leading cost, then
/// canonical Roman-numeral/voicing order) — sorting by score alone
/// isn't enough: on a genuine tie, a different tie-break would keep a
/// different top-`width` set than the real search actually kept,
/// silently diagnosing a context the real run never reached.
fn replay_to_failure(
    fixture: &ChoraleFixture,
    width: usize,
) -> Option<(usize, Option<(RomanNumeral, Voicing)>)> {
    let generator = CandidateGenerator::new(&fixture.key, &Style::CommonPractice);
    let mut diagnostics = Diagnostics::default();
    // (path, cumulative_score, cumulative_voice_leading_cost).
    let mut beam: Vec<PathEntry> = vec![(Vec::new(), 0.0, 0)];
    let len = fixture.soprano.len();
    for index in 0..len {
        let soprano = fixture.soprano.pitch_at(Position(index))?;
        let is_final = index == len - 1;
        let mut next_beam = Vec::new();
        for (path, cumulative, cumulative_vlc) in &beam {
            let previous = path.last().map(|(_, v)| v);
            let previous_rn = path.last().map(|(rn, _)| rn);
            let candidates =
                generator.generate(soprano, previous, previous_rn, is_final, &mut diagnostics);
            for candidate in candidates.into_iter().filter(|c| c.is_valid()) {
                let mut extended = path.clone();
                extended.push((candidate.roman_numeral, candidate.voicing));
                next_beam.push((
                    extended,
                    cumulative + candidate.score.total(),
                    cumulative_vlc + candidate.voice_leading_cost,
                ));
            }
        }
        if next_beam.is_empty() {
            let context = beam
                .into_iter()
                .next()
                .and_then(|(path, _, _)| path.last().copied());
            return Some((index, context));
        }
        next_beam.sort_by(|a, b| {
            b.1.total_cmp(&a.1)
                .then_with(|| a.2.cmp(&b.2))
                .then_with(|| {
                    let a_key: Vec<_> =
                        a.0.iter()
                            .map(|(rn, v)| numeral_voicing_key(rn, v))
                            .collect();
                    let b_key: Vec<_> =
                        b.0.iter()
                            .map(|(rn, v)| numeral_voicing_key(rn, v))
                            .collect();
                    a_key.cmp(&b_key)
                })
        });
        next_beam.truncate(width);
        beam = next_beam;
    }
    None
}

/// Diagnoses a structural (non-search-breadth) failure: replays the
/// real search up to its actual failure point (`replay_to_failure`) and
/// asks `CandidateGenerator` directly what it thinks of the failing
/// note in that exact context. This is a representative sample (the
/// one context a correctly-replayed search actually reaches), not an
/// exhaustive proof that *no* context could work — sufficient to tell a
/// triage report which rule to look at first.
fn diagnose_structural_failure(fixture: &ChoraleFixture, width: usize) -> FailureCategory {
    let Some((failing_index, context)) = replay_to_failure(fixture, width) else {
        return FailureCategory::Other; // shouldn't happen by construction, but fail safe
    };
    let Some((previous_rn, previous_voicing)) = context else {
        // Fails on the very first note: no previous context to inspect.
        return FailureCategory::Other;
    };

    let failing_note = fixture.soprano.notes[failing_index];
    let is_final = failing_index == fixture.soprano.len() - 1;
    let style = Style::CommonPractice;
    let generator = CandidateGenerator::new(&fixture.key, &style);
    let mut diagnostics = Diagnostics::default();
    let candidates = generator.generate(
        failing_note.pitch,
        Some(&previous_voicing),
        Some(&previous_rn),
        is_final,
        &mut diagnostics,
    );

    let mut rule_counts: BTreeMap<RuleId, usize> = BTreeMap::new();
    let mut any_valid = false;
    for candidate in &candidates {
        match &candidate.status {
            CandidateStatus::Valid => any_valid = true,
            CandidateStatus::Rejected(rules) => {
                for rule in rules {
                    *rule_counts.entry(*rule).or_insert(0) += 1;
                }
            }
        }
    }
    if any_valid || candidates.is_empty() {
        // A valid candidate existed in this context (the full search's
        // beam just didn't reach this combination), or no harmonic
        // candidate at all shares a pitch class with this note — the
        // latter should already be caught as chromatic; treat both as
        // "other" here rather than mis-attribute to a rule.
        return FailureCategory::Other;
    }
    match rule_counts.into_iter().max_by_key(|(_, count)| *count) {
        Some((rule, _)) => FailureCategory::RuleConflict(rule),
        None => FailureCategory::Other,
    }
}

fn classify_failure(fixture: &ChoraleFixture) -> FailureCategory {
    if has_unsupported_chromatic_tone(&fixture.soprano, &fixture.key) {
        return FailureCategory::ChromaticSoprano;
    }
    for &width in &RETRY_WIDTHS {
        if harmonizes_at_width(fixture, width) {
            return FailureCategory::SearchExhausted {
                first_working_width: width,
            };
        }
    }
    diagnose_structural_failure(fixture, *RETRY_WIDTHS.last().unwrap())
}

// ---- Measurement ---------------------------------------------------------

struct ChoraleMetrics {
    base_name: String,
    phrase: (usize, usize),
    covered: bool,
    positions: usize,
    hard_violations: usize,
    final_cadence: Option<Cadence>,
    ends_on_tonic_function: Option<bool>,
    voice_leading_cost_total: u32,
    runtime: std::time::Duration,
    positions_with_reasons: usize,
    why_not_attempts: usize,
    why_not_successes: usize,
    note_matched: usize,
    note_total: usize,
    failure_category: Option<FailureCategory>,
}

/// Raw (matched, total) reference-pitch-class agreement counts, not a
/// precomputed fraction — chorale-level aggregation needs to pool these
/// across phrases before dividing, not average already-divided fractions.
fn note_match(result: &HarmonizationResult, fixture: &ChoraleFixture) -> (usize, usize) {
    let (Some(alto), Some(tenor), Some(bass)) = (
        &fixture.reference_alto,
        &fixture.reference_tenor,
        &fixture.reference_bass,
    ) else {
        return (0, 0);
    };
    let mut matched = 0usize;
    let mut total = 0usize;
    for (i, decision) in result.decisions.iter().enumerate() {
        let voicing = decision.selected_candidate().voicing;
        for (chosen, reference) in [
            (voicing.alto, alto.get(i)),
            (voicing.tenor, tenor.get(i)),
            (voicing.bass, bass.get(i)),
        ] {
            if let Some(reference) = reference {
                total += 1;
                if chosen.pitch_class.is_enharmonic_to(&reference.pitch_class) {
                    matched += 1;
                }
            }
        }
    }
    (matched, total)
}

fn measure(fixture: &ChoraleFixture) -> ChoraleMetrics {
    let start = Instant::now();
    let outcome = Composer::new()
        .key(fixture.key)
        .style(Style::CommonPractice)
        .search(BeamSearch::new().width(STANDARD_WIDTH))
        .harmonize(fixture.soprano.clone());
    let runtime = start.elapsed();

    let Ok(result) = outcome else {
        return ChoraleMetrics {
            base_name: fixture.base_name.clone(),
            phrase: fixture.phrase,
            covered: false,
            positions: fixture.soprano.len(),
            hard_violations: 0,
            final_cadence: None,
            ends_on_tonic_function: None,
            voice_leading_cost_total: 0,
            runtime,
            positions_with_reasons: 0,
            why_not_attempts: 0,
            why_not_successes: 0,
            note_matched: 0,
            note_total: 0,
            failure_category: Some(classify_failure(fixture)),
        };
    };

    let hard_violations = result
        .decisions
        .iter()
        .filter(|d| !d.selected_candidate().is_valid())
        .count();
    let voice_leading_cost_total: u32 = result
        .decisions
        .iter()
        .map(|d| d.selected_candidate().voice_leading_cost)
        .sum();
    let positions_with_reasons = result
        .decisions
        .iter()
        .filter(|d| !d.selected_candidate().reasons.is_empty())
        .count();

    let final_cadence = result.decisions.last().and_then(|d| {
        d.selected_candidate().reasons.iter().find_map(|r| match r {
            Reason::CadenceSupport { cadence, .. } => Some(*cadence),
            _ => None,
        })
    });
    let ends_on_tonic_function = Some(
        result
            .decisions
            .last()
            .is_some_and(|d| d.selected().harmonic_function() == HarmonicFunction::Tonic),
    );

    let mut why_not_attempts = 0;
    let mut why_not_successes = 0;
    for (i, decision) in result.decisions.iter().enumerate() {
        if let Some(alt) = decision.alternatives().find(|c| c.is_valid()) {
            why_not_attempts += 1;
            if result.why_not(Position::new(i), alt.roman_numeral).is_ok() {
                why_not_successes += 1;
            }
        }
    }

    let (note_matched, note_total) = note_match(&result, fixture);

    ChoraleMetrics {
        base_name: fixture.base_name.clone(),
        phrase: fixture.phrase,
        covered: true,
        positions: result.decisions.len(),
        hard_violations,
        final_cadence,
        ends_on_tonic_function,
        voice_leading_cost_total,
        runtime,
        positions_with_reasons,
        why_not_attempts,
        why_not_successes,
        note_matched,
        note_total,
        failure_category: None,
    }
}

// ---- Percentiles -----------------------------------------------------

/// Linear-interpolation percentile (the common "R-7" method) over
/// already-collected samples. `p` in `0.0..=1.0`.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let rank = p * (sorted.len() - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    if lo == hi {
        sorted[lo]
    } else {
        let frac = rank - lo as f64;
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}

fn summarize(values: &mut [f64]) -> (f64, f64, f64) {
    values.sort_by(f64::total_cmp);
    (
        percentile(values, 0.5),
        percentile(values, 0.90),
        percentile(values, 0.95),
    )
}

// ---- Provenance ----------------------------------------------------------

struct Provenance {
    mokuren_version: String,
    git_sha: Option<String>,
    music21_version: Option<String>,
    corpus_extracted_count: Option<String>,
    corpus_skipped_count: Option<String>,
    corpus_skip_reasons: BTreeMap<String, usize>,
}

fn git_sha() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8(output.stdout).ok()?.trim().to_string())
}

/// Hand-parses the known scalar fields out of the extractor's
/// manifest.json rather than adding a JSON dependency for a
/// self-controlled, narrow format (tools/music21_chorale_extractor.py
/// is the only writer).
fn read_manifest_field(manifest_text: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\":");
    let start = manifest_text.find(&needle)? + needle.len();
    let rest = manifest_text[start..].trim_start();
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(stripped[..end].to_string())
    } else {
        let end = rest.find([',', '\n', '}']).unwrap_or(rest.len());
        Some(rest[..end].trim().to_string())
    }
}

/// Buckets the `"reason": "..."` strings inside the manifest's
/// `skipped` list by keyword, mirroring the reasons
/// `tools/music21_chorale_extractor.py` actually emits — "対象chorale
/// 数/除外数と除外理由" (exclusion count *and reason*) is the required
/// metric, not just a bare count.
fn skip_reason_breakdown(manifest_text: &str) -> BTreeMap<String, usize> {
    let mut counts: BTreeMap<String, usize> = BTreeMap::new();
    let needle = "\"reason\":";
    let mut rest = manifest_text;
    while let Some(pos) = rest.find(needle) {
        rest = &rest[pos + needle.len()..];
        let Some(reason) = read_manifest_field(&format!("\"reason\":{rest}"), "reason") else {
            break;
        };
        let bucket = if reason.contains("mode is") {
            "minor mode"
        } else if reason.contains("nothing but rests") {
            "soprano is nothing but rests"
        } else if reason.contains("unrepresentable duration") {
            "unrepresentable duration"
        } else if reason.contains("missing") {
            "missing part"
        } else if reason.contains("no sounding note") {
            "ATB gap at a soprano onset"
        } else {
            "other"
        };
        *counts.entry(bucket.to_string()).or_insert(0) += 1;
    }
    counts
}

fn gather_provenance(dir: &str) -> Provenance {
    let manifest_path = std::path::Path::new(dir).join("manifest.json");
    let manifest_text = std::fs::read_to_string(&manifest_path).ok();
    Provenance {
        mokuren_version: env!("CARGO_PKG_VERSION").to_string(),
        git_sha: git_sha(),
        music21_version: manifest_text
            .as_deref()
            .and_then(|t| read_manifest_field(t, "music21_version")),
        corpus_extracted_count: manifest_text
            .as_deref()
            .and_then(|t| read_manifest_field(t, "extracted_count")),
        corpus_skipped_count: manifest_text
            .as_deref()
            .and_then(|t| read_manifest_field(t, "skipped_count")),
        corpus_skip_reasons: manifest_text
            .as_deref()
            .map(skip_reason_breakdown)
            .unwrap_or_default(),
    }
}

// ---- Chorale-level aggregation ------------------------------------------

/// One chorale's metrics, pooled across every phrase a soprano rest
/// split it into (one phrase, the common case, for a rest-free chorale).
/// A chorale counts as `covered` only if *every* phrase harmonized —
/// looser bookkeeping (e.g. "covered if any phrase worked") would make
/// this number silently incomparable to the pre-rest-support baselines,
/// which measured one melody per chorale.
struct ChoraleAggregate<'a> {
    name: &'a str,
    phrase_count: usize,
    covered: bool,
    positions: usize,
    hard_violations: usize,
    final_cadence: Option<Cadence>,
    ends_on_tonic_function: Option<bool>,
    voice_leading_cost_total: u32,
    runtime: std::time::Duration,
    positions_with_reasons: usize,
    why_not_attempts: usize,
    why_not_successes: usize,
    note_matched: usize,
    note_total: usize,
    /// The first failing phrase's category and (index, total), if any.
    failure: Option<(FailureCategory, (usize, usize))>,
}

/// Groups phrase-level metrics by chorale. Relies on every phrase of one
/// chorale being adjacent in `metrics` — true because `parse_chorale_fixture`
/// returns all of a file's phrases together and `main` never reorders
/// `fixtures` before calling `measure`.
fn aggregate_by_chorale(metrics: &[ChoraleMetrics]) -> Vec<ChoraleAggregate<'_>> {
    let mut aggregates: Vec<ChoraleAggregate> = Vec::new();
    for m in metrics {
        let is_new = match aggregates.last() {
            Some(last) => last.name != m.base_name,
            None => true,
        };
        if is_new {
            aggregates.push(ChoraleAggregate {
                name: &m.base_name,
                phrase_count: 0,
                covered: true,
                positions: 0,
                hard_violations: 0,
                final_cadence: None,
                ends_on_tonic_function: None,
                voice_leading_cost_total: 0,
                runtime: std::time::Duration::ZERO,
                positions_with_reasons: 0,
                why_not_attempts: 0,
                why_not_successes: 0,
                note_matched: 0,
                note_total: 0,
                failure: None,
            });
        }
        let agg = aggregates.last_mut().unwrap();
        agg.phrase_count += 1;
        agg.covered &= m.covered;
        agg.positions += m.positions;
        agg.hard_violations += m.hard_violations;
        agg.voice_leading_cost_total += m.voice_leading_cost_total;
        agg.runtime += m.runtime;
        agg.positions_with_reasons += m.positions_with_reasons;
        agg.why_not_attempts += m.why_not_attempts;
        agg.why_not_successes += m.why_not_successes;
        agg.note_matched += m.note_matched;
        agg.note_total += m.note_total;
        if m.covered {
            // The chorale's audible ending is its *last* phrase's ending
            // — later phrases overwrite earlier ones as they're folded in.
            agg.final_cadence = m.final_cadence;
            agg.ends_on_tonic_function = m.ends_on_tonic_function;
        }
        if !m.covered && agg.failure.is_none() {
            agg.failure = Some((m.failure_category.clone().unwrap(), m.phrase));
        }
    }
    aggregates
}

// ---- Report ------------------------------------------------------------

fn build_report(metrics: &[ChoraleMetrics], provenance: &Provenance) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();

    let aggregates = aggregate_by_chorale(metrics);
    let total = aggregates.len();
    let covered = aggregates.iter().filter(|a| a.covered).count();
    let covered_metrics: Vec<&ChoraleAggregate> = aggregates.iter().filter(|a| a.covered).collect();
    let failed_metrics: Vec<&ChoraleAggregate> = aggregates.iter().filter(|a| !a.covered).collect();
    let multi_phrase_chorales = aggregates.iter().filter(|a| a.phrase_count > 1).count();

    let _ = writeln!(out, "# Chorale benchmark report\n");
    let _ = writeln!(out, "## Provenance\n");
    let _ = writeln!(out, "- mokuren version: {}", provenance.mokuren_version);
    let _ = writeln!(
        out,
        "- git commit: {}",
        provenance
            .git_sha
            .as_deref()
            .unwrap_or("(unknown — not a git checkout?)")
    );
    let _ = writeln!(
        out,
        "- music21 version: {}",
        provenance
            .music21_version
            .as_deref()
            .unwrap_or("(no manifest.json found in the input directory)")
    );
    if let Some(extracted) = &provenance.corpus_extracted_count {
        let skipped = provenance.corpus_skipped_count.as_deref().unwrap_or("?");
        let _ = writeln!(
            out,
            "- corpus: {extracted} chorale(s) extracted, {skipped} skipped at extraction time"
        );
        if !provenance.corpus_skip_reasons.is_empty() {
            let total_skipped: usize = provenance.corpus_skip_reasons.values().sum();
            let _ = writeln!(out, "  - exclusion reasons ({total_skipped} total):");
            let mut by_count: Vec<(&String, &usize)> =
                provenance.corpus_skip_reasons.iter().collect();
            by_count.sort_by_key(|(_, count)| std::cmp::Reverse(**count));
            for (reason, count) in by_count {
                let _ = writeln!(out, "    - {reason}: {count}");
            }
        }
    }
    let _ = writeln!(
        out,
        "- chorales measured here: {total} ({} phrase(s) total, after splitting at rests)",
        metrics.len()
    );
    let _ = writeln!(
        out,
        "- standard beam width: {STANDARD_WIDTH} (retry widths for failure classification: {RETRY_WIDTHS:?})\n"
    );

    let _ = writeln!(out, "## Coverage\n");
    let _ = writeln!(
        out,
        "- Coverage: {covered}/{total} ({:.1}%)",
        100.0 * covered as f64 / total.max(1) as f64
    );
    let _ = writeln!(
        out,
        "- Search failure rate: {}/{total} ({:.1}%)\n",
        total - covered,
        100.0 * (total - covered) as f64 / total.max(1) as f64
    );

    if multi_phrase_chorales > 0 {
        let _ = writeln!(
            out,
            "- {multi_phrase_chorales} chorale(s) had a soprano rest and were split into multiple phrases (harmonized independently, each via the same `Composer::harmonize` a rest-free chorale uses); \"covered\" above requires *every* phrase of a chorale to harmonize, so this is comparable to the pre-rest-support baselines, not a looser per-phrase number.\n"
        );
    }

    if !failed_metrics.is_empty() {
        let _ = writeln!(out, "## Failure taxonomy (not lumped into one bucket)\n");
        let mut categories: BTreeMap<String, usize> = BTreeMap::new();
        for a in &failed_metrics {
            let (category, _) = a.failure.as_ref().unwrap();
            let label = match category {
                FailureCategory::ChromaticSoprano => "chromatic soprano unsupported".to_string(),
                FailureCategory::SearchExhausted { .. } => {
                    "search exhausted (wider beam works)".to_string()
                }
                FailureCategory::RuleConflict(rule) => format!("rule conflict: {rule}"),
                FailureCategory::Other => "other / undiagnosed".to_string(),
            };
            *categories.entry(label).or_insert(0) += 1;
        }
        for (category, count) in &categories {
            let _ = writeln!(
                out,
                "- {category}: {count} ({:.1}% of all fixtures, {:.1}% of failures)",
                100.0 * *count as f64 / total.max(1) as f64,
                100.0 * *count as f64 / failed_metrics.len().max(1) as f64
            );
        }

        let _ = writeln!(
            out,
            "\n### Beam-width coverage curve (failures only — successes at width {STANDARD_WIDTH} aren't retried)\n"
        );
        let mut cumulative = covered;
        let _ = writeln!(
            out,
            "- width {STANDARD_WIDTH:>4}: {cumulative}/{total} ({:.1}%)",
            100.0 * cumulative as f64 / total.max(1) as f64
        );
        for &width in &RETRY_WIDTHS {
            let newly_working = failed_metrics
                .iter()
                .filter(|a| matches!(&a.failure, Some((FailureCategory::SearchExhausted { first_working_width }, _)) if *first_working_width == width))
                .count();
            cumulative += newly_working;
            let _ = writeln!(
                out,
                "- width {width:>4}: {cumulative}/{total} ({:.1}%)",
                100.0 * cumulative as f64 / total.max(1) as f64
            );
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "## Hard-rule violations\n");
    let hard_violations: usize = covered_metrics.iter().map(|a| a.hard_violations).sum();
    let _ = writeln!(
        out,
        "{hard_violations} (should always be 0 by construction — a nonzero count is a bug, not a quality signal)\n"
    );

    if !covered_metrics.is_empty() {
        let total_positions: usize = covered_metrics.iter().map(|a| a.positions).sum();

        let _ = writeln!(out, "## Voice-leading cost\n");
        let mut vlc_per_position: Vec<f64> = covered_metrics
            .iter()
            .map(|a| a.voice_leading_cost_total as f64 / a.positions.max(1) as f64)
            .collect();
        let (median, p90, p95) = summarize(&mut vlc_per_position);
        let _ = writeln!(
            out,
            "Per-chorale average (cost / position): median {median:.2}, p90 {p90:.2}, p95 {p95:.2}\n"
        );

        let _ = writeln!(out, "## Runtime\n");
        let mut runtime_ms: Vec<f64> = covered_metrics
            .iter()
            .map(|a| a.runtime.as_secs_f64() * 1000.0)
            .collect();
        let (median, p90, p95) = summarize(&mut runtime_ms);
        let _ = writeln!(
            out,
            "Per chorale (ms, summed across phrases): median {median:.1}, p90 {p90:.1}, p95 {p95:.1}\n"
        );

        let _ = writeln!(out, "## Explanation completeness\n");
        let reasons: usize = covered_metrics
            .iter()
            .map(|a| a.positions_with_reasons)
            .sum();
        let _ = writeln!(
            out,
            "- why() coverage: {:.1}% of positions have at least one Reason",
            100.0 * reasons as f64 / total_positions.max(1) as f64
        );
        let why_not_attempts: usize = covered_metrics.iter().map(|a| a.why_not_attempts).sum();
        let why_not_successes: usize = covered_metrics.iter().map(|a| a.why_not_successes).sum();
        let _ = writeln!(
            out,
            "- why_not() success: {why_not_successes}/{why_not_attempts} ({:.1}%) of positions with a valid alternative\n",
            100.0 * why_not_successes as f64 / why_not_attempts.max(1) as f64
        );

        let _ = writeln!(out, "## Cadence\n");
        let _ = writeln!(
            out,
            "(the chorale's *last phrase*'s cadence — for a multi-phrase chorale this is the piece's actual final cadence, not an average across phrase-internal cadences)\n"
        );
        let mut cadences: BTreeMap<String, usize> = BTreeMap::new();
        for a in &covered_metrics {
            let label = a
                .final_cadence
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".to_string());
            *cadences.entry(label).or_insert(0) += 1;
        }
        let _ = writeln!(out, "Final-cadence distribution:");
        for (cadence, count) in &cadences {
            let _ = writeln!(
                out,
                "- {cadence}: {count} ({:.1}%)",
                100.0 * *count as f64 / covered_metrics.len().max(1) as f64
            );
        }
        let tonic_endings = covered_metrics
            .iter()
            .filter(|a| a.ends_on_tonic_function == Some(true))
            .count();
        let _ = writeln!(
            out,
            "\nEnds on a tonic-function chord (proxy for \"the close is at least plausible,\" not full cadence-correctness verification): {tonic_endings}/{} ({:.1}%)\n",
            covered_metrics.len(),
            100.0 * tonic_endings as f64 / covered_metrics.len().max(1) as f64
        );

        let note_total: usize = covered_metrics.iter().map(|a| a.note_total).sum();
        if note_total > 0 {
            let note_matched: usize = covered_metrics.iter().map(|a| a.note_matched).sum();
            let with_reference = covered_metrics.iter().filter(|a| a.note_total > 0).count();
            let _ = writeln!(
                out,
                "## Original-note match (secondary, diagnostic only — see BENCHMARK.md's non-goal)\n"
            );
            let _ = writeln!(
                out,
                "{:.1}% (pooled across phrases) over {with_reference} fixture(s) with a reference ATB\n",
                100.0 * note_matched as f64 / note_total as f64
            );
        }
    }

    let _ = writeln!(out, "## Per-chorale\n");
    let _ = writeln!(
        out,
        "| Chorale | Result | Phrases | Voice-leading cost | Cadence | Runtime (ms) |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|");
    for a in &aggregates {
        if a.covered {
            let cadence = a
                .final_cadence
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".to_string());
            let _ = writeln!(
                out,
                "| {} | covered | {} | {} | {} | {:.1} |",
                a.name,
                a.phrase_count,
                a.voice_leading_cost_total,
                cadence,
                a.runtime.as_secs_f64() * 1000.0
            );
        } else {
            let (category, (phrase_index, phrase_count)) = a.failure.as_ref().unwrap();
            let _ = writeln!(
                out,
                "| {} | NOT COVERED | {phrase_count} | — | — | — (phrase {phrase_index}/{phrase_count}: {category}) |",
                a.name
            );
        }
    }

    out
}

/// Prints a detailed bisection report for one fixture: the exact
/// structural failure point and context (via `replay_to_failure`, so
/// this matches what `classify_failure` itself sees), candidate count,
/// and rejection-rule tally, at both the standard width and every retry
/// width — so "beam-width independence" (does a wider beam change the
/// failure point at all?) is a directly observed fact, not inferred.
fn bisect_report(fixture: &ChoraleFixture) {
    println!("=== {} ===", fixture.name);
    println!("key: {}", fixture.key);
    println!("soprano length: {} notes", fixture.soprano.len());

    for &width in std::iter::once(&STANDARD_WIDTH).chain(RETRY_WIDTHS.iter()) {
        if harmonizes_at_width(fixture, width) {
            println!("width {width}: SUCCEEDS");
            continue;
        }
        let Some((failing_index, context)) = replay_to_failure(fixture, width) else {
            println!(
                "width {width}: FAILS but replay_to_failure found no stuck position (shouldn't happen by construction)"
            );
            continue;
        };
        println!("width {width}: FAILS at position {failing_index} (0-indexed)");
        let Some((previous_rn, previous_voicing)) = context else {
            println!("  fails on the very first note: no previous context to inspect");
            continue;
        };
        println!(
            "  last successful position: {} (0-indexed)",
            failing_index - 1
        );
        let failing_note = fixture.soprano.notes[failing_index];
        println!(
            "  failing note (position {failing_index}): {}",
            failing_note.pitch
        );
        println!(
            "  context: previous numeral {previous_rn}, previous voicing {previous_voicing:?}"
        );
        let is_final = failing_index == fixture.soprano.len() - 1;
        let generator = CandidateGenerator::new(&fixture.key, &Style::CommonPractice);
        let mut diagnostics = Diagnostics::default();
        let candidates = generator.generate(
            failing_note.pitch,
            Some(&previous_voicing),
            Some(&previous_rn),
            is_final,
            &mut diagnostics,
        );
        println!("  candidates generated: {}", candidates.len());
        let mut rule_counts: BTreeMap<RuleId, usize> = BTreeMap::new();
        let mut any_valid = false;
        for candidate in &candidates {
            match &candidate.status {
                CandidateStatus::Valid => any_valid = true,
                CandidateStatus::Rejected(rules) => {
                    for rule in rules {
                        *rule_counts.entry(*rule).or_insert(0) += 1;
                    }
                }
            }
        }
        println!("  any valid candidate in this context: {any_valid}");
        let mut counts: Vec<_> = rule_counts.into_iter().collect();
        counts.sort_by_key(|b| std::cmp::Reverse(b.1));
        println!("  rejection-rule tally: {counts:?}");
    }
}

fn main() {
    let mut dir = None;
    let mut report_path: Option<String> = None;
    let mut bisect: Option<String> = None;
    let mut minor_gap_report_flag = false;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--report" => report_path = args.next(),
            "--bisect" => bisect = args.next(),
            "--minor-gap-report" => minor_gap_report_flag = true,
            other if dir.is_none() => dir = Some(other.to_string()),
            other => {
                eprintln!("unexpected argument: {other:?}");
                std::process::exit(1);
            }
        }
    }
    let Some(dir) = dir else {
        eprintln!(
            "usage: chorale_benchmark <directory of .chorale fixture files> [--report path/to/report.md]"
        );
        eprintln!();
        eprintln!("No chorale corpus is vendored in this repository — see BENCHMARK.md.");
        eprintln!("Try the synthetic smoke-test fixtures first:");
        eprintln!(
            "  cargo run --release --example chorale_benchmark -- examples/chorale_benchmark_fixtures"
        );
        std::process::exit(1);
    };

    let mut fixtures = Vec::new();
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("could not read directory {dir:?}: {e}");
            std::process::exit(1);
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("chorale") {
            continue;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) => {
                eprintln!("skipping {}: {e}", path.display());
                continue;
            }
        };
        match parse_chorale_fixture(&text) {
            Ok(phrases) => fixtures.extend(phrases),
            Err(e) => eprintln!("skipping {}: {e}", path.display()),
        }
    }

    if fixtures.is_empty() {
        eprintln!("no .chorale fixtures found in {dir:?}");
        std::process::exit(1);
    }

    if let Some(needle) = bisect {
        let matches: Vec<_> = fixtures
            .iter()
            .filter(|f| f.name.contains(&needle))
            .collect();
        if matches.is_empty() {
            eprintln!("no fixture name contains {needle:?}");
            std::process::exit(1);
        }
        for fixture in matches {
            bisect_report(fixture);
        }
        return;
    }

    if minor_gap_report_flag {
        minor_gap_report(&fixtures);
        return;
    }

    eprintln!("measuring {} fixture(s)...", fixtures.len());
    let metrics: Vec<ChoraleMetrics> = fixtures
        .iter()
        .enumerate()
        .map(|(i, f)| {
            eprintln!("  [{}/{}] {}", i + 1, fixtures.len(), f.name);
            measure(f)
        })
        .collect();

    let provenance = gather_provenance(&dir);
    let report = build_report(&metrics, &provenance);

    if let Some(report_path) = report_path {
        if let Err(e) = std::fs::write(&report_path, &report) {
            eprintln!("could not write report to {report_path:?}: {e}");
            std::process::exit(1);
        }
        eprintln!("report written to {report_path}");
    }
    println!("{report}");
}
