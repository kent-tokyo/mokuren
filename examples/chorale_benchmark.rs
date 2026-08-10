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
//! Fixture format v2 — one `.chorale` file per piece. `soprano` carries
//! real onset/pitch/duration (v1 forced every note to a quarter, which
//! silently discarded real chorale rhythm — see tasks/lessons.md).
//! `alto`/`tenor`/`bass` are reference pitches *sampled at each soprano
//! onset*, not an independent onset grid: giving them their own
//! offsets/durations would leak where Bach changed harmony into data a
//! benchmark run is supposed to discover, not read off the input.
//!
//! ```text
//! name: <label>
//! key: <tonic pitch class, e.g. C, F#, Bb>
//! meter: <e.g. 4/4 — carried through, not yet consumed by any rule>
//! soprano:
//! <offset in quarter-note beats> <pitch> <duration, as a fraction of a whole note, e.g. 1/4>
//! ...one line per note, offsets contiguous (no rests — mokuren's Melody can't represent one yet)
//!
//! alto: <optional pitch list, one per soprano onset, e.g. A4 A4 F4 ...>
//! tenor: <optional, same>
//! bass: <optional, same>
//! ```
//!
//! `examples/chorale_benchmark_fixtures/` has synthetic smoke-test
//! fixtures (melodies written for this harness, not real chorales) —
//! run against those to see the report format without needing a corpus.
//!
//! Failures are never lumped into one "coverage" bucket (BENCHMARK.md):
//! each is classified as chromatic-soprano (a pitch class outside the
//! key's diatonic scale — mokuren is diatonic-only), search-exhausted
//! (a wider beam finds a path), a specific rule conflict (identified by
//! bisecting to the shortest failing prefix and inspecting
//! `CandidateGenerator`'s rejection reasons there), or other.

use mokuren::diagnostics::Diagnostics;
use mokuren::generate::{CandidateGenerator, CandidateStatus};
use mokuren::melody::{Duration as NoteDuration, Note, Position};
use mokuren::pitch::Pitch;
use mokuren::prelude::*;
use mokuren::rules::RuleId;
use mokuren::score::{Cadence, Reason};
use mokuren::voice::Voicing;
use std::collections::BTreeMap;
use std::time::Instant;

// ---- Fixture parsing (v2, duration-aware) -----------------------------

struct ChoraleFixture {
    name: String,
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

/// One `offset pitch duration` line inside a `soprano:` block.
struct SopranoEvent {
    offset: f64,
    note: Note,
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
    let pitch: Pitch = pitch
        .parse()
        .map_err(|e| format!("bad pitch in {line:?}: {e}"))?;
    let duration = parse_duration(duration)?;
    Ok(SopranoEvent {
        offset,
        note: Note::new(pitch, duration),
    })
}

/// Builds a contiguous `Melody` from parsed events, verifying each
/// event's offset lines up exactly with the previous one's end — a
/// rest or overlap means this soprano line can't be represented by
/// mokuren's `Melody` (a plain `Vec<Note>`, no rests) and is a data
/// error worth surfacing rather than silently misaligning.
fn build_soprano_melody(events: &[SopranoEvent]) -> std::result::Result<Melody, String> {
    if events.is_empty() {
        return Err("soprano block has no events".to_string());
    }
    let mut expected_offset = events[0].offset;
    for event in events {
        if (event.offset - expected_offset).abs() > 1e-6 {
            return Err(format!(
                "soprano offset {} doesn't follow the previous note's end ({expected_offset}) — a rest, overlap, or out-of-order line, none of which mokuren's Melody can represent",
                event.offset
            ));
        }
        expected_offset += event.note.duration.beats();
    }
    Ok(Melody::new(events.iter().map(|e| e.note).collect()))
}

fn parse_chorale_fixture(text: &str) -> std::result::Result<ChoraleFixture, String> {
    let (mut name, mut key, mut meter) = (None, None, None);
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
        const KNOWN_FIELDS: [&str; 7] =
            ["name", "key", "meter", "soprano", "alto", "tenor", "bass"];
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
                key = Some(
                    Key::new(pc, mokuren::key::Mode::Major)
                        .map_err(|e| format!("key {value:?} is not constructible: {e}"))?,
                );
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
    Ok(ChoraleFixture {
        name: name.ok_or("missing `name:`")?,
        key: key.ok_or("missing `key:`")?,
        soprano: build_soprano_melody(&soprano_events)?,
        reference_alto: alto,
        reference_tenor: tenor,
        reference_bass: bass,
    })
}

// ---- Failure classification --------------------------------------------

/// Widths tried, in order, when a chorale fails at the standard width —
/// also the data behind the beam-width coverage curve in the report.
const RETRY_WIDTHS: [usize; 3] = [64, 128, 256];
const STANDARD_WIDTH: usize = 32;

#[derive(Debug, Clone)]
enum FailureCategory {
    /// A soprano pitch class isn't in the key's diatonic scale — no
    /// diatonic chord can contain it, in any key. Never fixed by a
    /// wider beam; only by adding chromatic harmony (roadmap phase 4).
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

fn is_fully_diatonic(melody: &Melody, key: &Key) -> bool {
    melody
        .notes
        .iter()
        .all(|n| key.degree_of(n.pitch.pitch_class).is_some())
}

fn harmonizes_at_width(fixture: &ChoraleFixture, width: usize) -> bool {
    Composer::new()
        .key(fixture.key)
        .style(Style::CommonPractice)
        .search(BeamSearch::new().width(width))
        .harmonize(fixture.soprano.clone())
        .is_ok()
}

/// Finds the shortest prefix (by note count) of `fixture`'s soprano line
/// that still fails to harmonize at `width` — binary search over melody
/// length, each step a fresh full search. `full_length` must already be
/// known to fail at `width`.
fn shortest_failing_prefix(fixture: &ChoraleFixture, width: usize, full_length: usize) -> usize {
    let (mut lo, mut hi) = (1usize, full_length);
    while lo < hi {
        let mid = (lo + hi) / 2;
        let prefix = Melody::new(fixture.soprano.notes[..mid].to_vec());
        let ok = Composer::new()
            .key(fixture.key)
            .style(Style::CommonPractice)
            .search(BeamSearch::new().width(width))
            .harmonize(prefix)
            .is_ok();
        if ok {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

/// Diagnoses a structural (non-search-breadth) failure: harmonizes the
/// one-shorter prefix (known to succeed, by construction of
/// `shortest_failing_prefix`), takes its winning path's final chord as
/// context, and asks `CandidateGenerator` directly what it thinks of the
/// next soprano note in that context. This is a representative sample
/// (the one context a successful shorter search actually reached), not
/// an exhaustive proof that *no* context could work — sufficient to
/// tell a triage report which rule to look at first.
fn diagnose_structural_failure(fixture: &ChoraleFixture, width: usize) -> FailureCategory {
    let failing_len = shortest_failing_prefix(fixture, width, fixture.soprano.len());
    if failing_len <= 1 {
        // Fails on the very first note: no previous context to inspect.
        return FailureCategory::Other;
    }
    let shorter_prefix = Melody::new(fixture.soprano.notes[..failing_len - 1].to_vec());
    let Ok(shorter_result) = Composer::new()
        .key(fixture.key)
        .style(Style::CommonPractice)
        .search(BeamSearch::new().width(width))
        .harmonize(shorter_prefix)
    else {
        return FailureCategory::Other; // shouldn't happen by construction, but fail safe
    };
    let Some(last_decision) = shorter_result.decisions.last() else {
        return FailureCategory::Other;
    };
    let last_candidate = last_decision.selected_candidate();
    let previous_voicing: Voicing = last_candidate.voicing;
    let previous_rn = last_candidate.roman_numeral;

    let failing_note = fixture.soprano.notes[failing_len - 1];
    let is_final = failing_len == fixture.soprano.len();
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
    if !is_fully_diatonic(&fixture.soprano, &fixture.key) {
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
    name: String,
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
    note_match_fraction: Option<f64>,
    failure_category: Option<FailureCategory>,
}

fn note_match(result: &HarmonizationResult, fixture: &ChoraleFixture) -> Option<f64> {
    let (Some(alto), Some(tenor), Some(bass)) = (
        &fixture.reference_alto,
        &fixture.reference_tenor,
        &fixture.reference_bass,
    ) else {
        return None;
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
    (total > 0).then(|| matched as f64 / total as f64)
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
            name: fixture.name.clone(),
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
            note_match_fraction: None,
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

    let note_match_fraction = note_match(&result, fixture);

    ChoraleMetrics {
        name: fixture.name.clone(),
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
        note_match_fraction,
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
        } else if reason.contains("rest") {
            "soprano rest (Melody can't represent one)"
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

// ---- Report ------------------------------------------------------------

fn build_report(metrics: &[ChoraleMetrics], provenance: &Provenance) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();

    let total = metrics.len();
    let covered = metrics.iter().filter(|m| m.covered).count();
    let covered_metrics: Vec<&ChoraleMetrics> = metrics.iter().filter(|m| m.covered).collect();
    let failed_metrics: Vec<&ChoraleMetrics> = metrics.iter().filter(|m| !m.covered).collect();

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
    let _ = writeln!(out, "- fixtures measured here: {total}");
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

    if !failed_metrics.is_empty() {
        let _ = writeln!(out, "## Failure taxonomy (not lumped into one bucket)\n");
        let mut categories: BTreeMap<String, usize> = BTreeMap::new();
        for m in &failed_metrics {
            let label = match m.failure_category.as_ref().unwrap() {
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
                .filter(|m| matches!(m.failure_category, Some(FailureCategory::SearchExhausted { first_working_width }) if first_working_width == width))
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
    let hard_violations: usize = covered_metrics.iter().map(|m| m.hard_violations).sum();
    let _ = writeln!(
        out,
        "{hard_violations} (should always be 0 by construction — a nonzero count is a bug, not a quality signal)\n"
    );

    if !covered_metrics.is_empty() {
        let total_positions: usize = covered_metrics.iter().map(|m| m.positions).sum();

        let _ = writeln!(out, "## Voice-leading cost\n");
        let mut vlc_per_position: Vec<f64> = covered_metrics
            .iter()
            .map(|m| m.voice_leading_cost_total as f64 / m.positions.max(1) as f64)
            .collect();
        let (median, p90, p95) = summarize(&mut vlc_per_position);
        let _ = writeln!(
            out,
            "Per-chorale average (cost / position): median {median:.2}, p90 {p90:.2}, p95 {p95:.2}\n"
        );

        let _ = writeln!(out, "## Runtime\n");
        let mut runtime_ms: Vec<f64> = covered_metrics
            .iter()
            .map(|m| m.runtime.as_secs_f64() * 1000.0)
            .collect();
        let (median, p90, p95) = summarize(&mut runtime_ms);
        let _ = writeln!(
            out,
            "Per chorale (ms): median {median:.1}, p90 {p90:.1}, p95 {p95:.1}\n"
        );

        let _ = writeln!(out, "## Explanation completeness\n");
        let reasons: usize = covered_metrics
            .iter()
            .map(|m| m.positions_with_reasons)
            .sum();
        let _ = writeln!(
            out,
            "- why() coverage: {:.1}% of positions have at least one Reason",
            100.0 * reasons as f64 / total_positions.max(1) as f64
        );
        let why_not_attempts: usize = covered_metrics.iter().map(|m| m.why_not_attempts).sum();
        let why_not_successes: usize = covered_metrics.iter().map(|m| m.why_not_successes).sum();
        let _ = writeln!(
            out,
            "- why_not() success: {why_not_successes}/{why_not_attempts} ({:.1}%) of positions with a valid alternative\n",
            100.0 * why_not_successes as f64 / why_not_attempts.max(1) as f64
        );

        let _ = writeln!(out, "## Cadence\n");
        let mut cadences: BTreeMap<String, usize> = BTreeMap::new();
        for m in &covered_metrics {
            let label = m
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
            .filter(|m| m.ends_on_tonic_function == Some(true))
            .count();
        let _ = writeln!(
            out,
            "\nEnds on a tonic-function chord (proxy for \"the close is at least plausible,\" not full cadence-correctness verification): {tonic_endings}/{} ({:.1}%)\n",
            covered_metrics.len(),
            100.0 * tonic_endings as f64 / covered_metrics.len().max(1) as f64
        );

        let note_matches: Vec<f64> = covered_metrics
            .iter()
            .filter_map(|m| m.note_match_fraction)
            .collect();
        if !note_matches.is_empty() {
            let avg = note_matches.iter().sum::<f64>() / note_matches.len() as f64;
            let _ = writeln!(
                out,
                "## Original-note match (secondary, diagnostic only — see BENCHMARK.md's non-goal)\n"
            );
            let _ = writeln!(
                out,
                "{:.1}% avg over {} fixture(s) with a reference ATB\n",
                avg * 100.0,
                note_matches.len()
            );
        }
    }

    let _ = writeln!(out, "## Per-chorale\n");
    let _ = writeln!(
        out,
        "| Chorale | Result | Voice-leading cost | Cadence | Runtime (ms) |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|");
    for m in metrics {
        if m.covered {
            let cadence = m
                .final_cadence
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".to_string());
            let _ = writeln!(
                out,
                "| {} | covered | {} | {} | {:.1} |",
                m.name,
                m.voice_leading_cost_total,
                cadence,
                m.runtime.as_secs_f64() * 1000.0
            );
        } else {
            let _ = writeln!(
                out,
                "| {} | NOT COVERED | — | — | — ({}) |",
                m.name,
                m.failure_category.as_ref().unwrap()
            );
        }
    }

    out
}

fn main() {
    let mut dir = None;
    let mut report_path: Option<String> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--report" => report_path = args.next(),
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
            Ok(fixture) => fixtures.push(fixture),
            Err(e) => eprintln!("skipping {}: {e}", path.display()),
        }
    }

    if fixtures.is_empty() {
        eprintln!("no .chorale fixtures found in {dir:?}");
        std::process::exit(1);
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
