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
//!   cargo run --release --example chorale_benchmark -- path/to/chorales
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

use mokuren::melody::{Duration as NoteDuration, Note, Position};
use mokuren::pitch::Pitch;
use mokuren::prelude::*;
use mokuren::score::{Cadence, Reason};
use std::collections::BTreeMap;
use std::time::Instant;

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

struct ChoraleMetrics {
    name: String,
    covered: bool,
    positions: usize,
    hard_violations: usize,
    final_cadence: Option<Cadence>,
    voice_leading_cost_total: u32,
    runtime: std::time::Duration,
    positions_with_reasons: usize,
    why_not_attempts: usize,
    why_not_successes: usize,
    note_match_fraction: Option<f64>,
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
        .search(BeamSearch::new().width(32))
        .harmonize(fixture.soprano.clone());
    let runtime = start.elapsed();

    let Ok(result) = outcome else {
        return ChoraleMetrics {
            name: fixture.name.clone(),
            covered: false,
            positions: fixture.soprano.len(),
            hard_violations: 0,
            final_cadence: None,
            voice_leading_cost_total: 0,
            runtime,
            positions_with_reasons: 0,
            why_not_attempts: 0,
            why_not_successes: 0,
            note_match_fraction: None,
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
        voice_leading_cost_total,
        runtime,
        positions_with_reasons,
        why_not_attempts,
        why_not_successes,
        note_match_fraction,
    }
}

fn print_report(metrics: &[ChoraleMetrics]) {
    let total = metrics.len();
    let covered = metrics.iter().filter(|m| m.covered).count();
    let covered_metrics: Vec<&ChoraleMetrics> = metrics.iter().filter(|m| m.covered).collect();

    println!("Chorale benchmark report ({total} fixture(s))\n");
    println!(
        "Coverage:              {covered}/{total} ({:.1}%)",
        100.0 * covered as f64 / total.max(1) as f64
    );
    println!(
        "Search failure rate:   {}/{total} ({:.1}%)",
        total - covered,
        100.0 * (total - covered) as f64 / total.max(1) as f64
    );

    let hard_violations: usize = covered_metrics.iter().map(|m| m.hard_violations).sum();
    println!(
        "Hard-rule violations:  {hard_violations} (should always be 0 — a nonzero count is a bug, not a quality signal)"
    );

    if !covered_metrics.is_empty() {
        let total_positions: usize = covered_metrics.iter().map(|m| m.positions).sum();
        let total_vlc: u32 = covered_metrics
            .iter()
            .map(|m| m.voice_leading_cost_total)
            .sum();
        println!(
            "Voice-leading cost:    {:.2} avg per position ({total_vlc} total over {total_positions} positions)",
            total_vlc as f64 / total_positions.max(1) as f64
        );

        let total_runtime: std::time::Duration = covered_metrics.iter().map(|m| m.runtime).sum();
        println!(
            "Runtime:               {:.1}ms avg per chorale ({:.1}ms total)",
            total_runtime.as_secs_f64() * 1000.0 / covered_metrics.len() as f64,
            total_runtime.as_secs_f64() * 1000.0
        );

        let reasons: usize = covered_metrics
            .iter()
            .map(|m| m.positions_with_reasons)
            .sum();
        println!(
            "Explanation coverage:  {:.1}% of positions have at least one Reason",
            100.0 * reasons as f64 / total_positions.max(1) as f64
        );
        let why_not_attempts: usize = covered_metrics.iter().map(|m| m.why_not_attempts).sum();
        let why_not_successes: usize = covered_metrics.iter().map(|m| m.why_not_successes).sum();
        println!(
            "why_not() success:     {why_not_successes}/{why_not_attempts} ({:.1}%) of positions with a valid alternative",
            100.0 * why_not_successes as f64 / why_not_attempts.max(1) as f64
        );

        let mut cadences: BTreeMap<String, usize> = BTreeMap::new();
        for m in &covered_metrics {
            let label = m
                .final_cadence
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".to_string());
            *cadences.entry(label).or_insert(0) += 1;
        }
        println!("Final-cadence distribution:");
        for (cadence, count) in &cadences {
            println!("  {cadence:<12} {count}");
        }

        let note_matches: Vec<f64> = covered_metrics
            .iter()
            .filter_map(|m| m.note_match_fraction)
            .collect();
        if !note_matches.is_empty() {
            let avg = note_matches.iter().sum::<f64>() / note_matches.len() as f64;
            println!(
                "\n(secondary, diagnostic only — see BENCHMARK.md's non-goal) Original-note match: {:.1}% avg over {} fixture(s) with a reference ATB",
                avg * 100.0,
                note_matches.len()
            );
        }
    }

    println!("\nPer-chorale:");
    for m in metrics {
        if m.covered {
            let cadence = m
                .final_cadence
                .map(|c| c.to_string())
                .unwrap_or_else(|| "none".to_string());
            println!(
                "  {:<24} covered   vlc={:<5} cadence={:<10} runtime={:.1}ms",
                m.name,
                m.voice_leading_cost_total,
                cadence,
                m.runtime.as_secs_f64() * 1000.0
            );
        } else {
            println!(
                "  {:<24} NOT COVERED (no valid harmonization found)",
                m.name
            );
        }
    }
}

fn main() {
    let dir = match std::env::args().nth(1) {
        Some(dir) => dir,
        None => {
            eprintln!("usage: chorale_benchmark <directory of .chorale fixture files>");
            eprintln!();
            eprintln!("No chorale corpus is vendored in this repository — see BENCHMARK.md.");
            eprintln!("Try the synthetic smoke-test fixtures first:");
            eprintln!(
                "  cargo run --release --example chorale_benchmark -- examples/chorale_benchmark_fixtures"
            );
            std::process::exit(1);
        }
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

    let metrics: Vec<ChoraleMetrics> = fixtures.iter().map(measure).collect();
    print_report(&metrics);
}
