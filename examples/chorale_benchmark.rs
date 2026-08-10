//! External chorale benchmark harness (BENCHMARK.md). Measures whether
//! mokuren's reasoning holds up on melodies it was never tuned against —
//! not whether it matches the original harmonization note-for-note (see
//! BENCHMARK.md's explicit non-goal).
//!
//! No chorale data is vendored in this repository (BENCHMARK.md: license
//! status of every candidate source was checked, none gave a clean
//! "commit this" answer — decided 2026-08-10 to reference, not vendor).
//! Point this at a local directory of chorale fixture files yourself:
//!
//!   cargo run --release --example chorale_benchmark -- path/to/chorales
//!
//! Fixture format — one `.chorale` file per piece, minimal by design
//! (mokuren has no Humdrum/MusicXML reader yet; that's roadmap phase 5,
//! itself paused until this benchmark runs):
//!
//! ```text
//! name: <label>
//! key: <tonic pitch class, e.g. C, F#, Bb>
//! soprano: <mokuren pitch sequence, e.g. C4 C4 G4 G4>
//! alto: <optional, same format, for the secondary note-match metric>
//! tenor: <optional>
//! bass: <optional>
//! ```
//!
//! `examples/chorale_benchmark_fixtures/` has synthetic smoke-test
//! fixtures (melodies written for this harness, not real chorales) —
//! run against those to see the report format without needing a corpus.

use mokuren::melody::Position;
use mokuren::prelude::*;
use mokuren::score::{Cadence, Reason};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

struct ChoraleFixture {
    name: String,
    key: Key,
    soprano: Melody,
    reference_alto: Option<Vec<mokuren::pitch::Pitch>>,
    reference_tenor: Option<Vec<mokuren::pitch::Pitch>>,
    reference_bass: Option<Vec<mokuren::pitch::Pitch>>,
}

fn parse_pitches(s: &str) -> std::result::Result<Vec<mokuren::pitch::Pitch>, String> {
    s.split_whitespace()
        .map(|tok| tok.parse().map_err(|e| format!("bad pitch {tok:?}: {e}")))
        .collect()
}

fn parse_chorale_fixture(text: &str) -> std::result::Result<ChoraleFixture, String> {
    let (mut name, mut key, mut soprano) = (None, None, None);
    let (mut alto, mut tenor, mut bass) = (None, None, None);
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (field, value) = line
            .split_once(':')
            .ok_or_else(|| format!("expected `field: value`, got {line:?}"))?;
        let value = value.trim();
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
            "soprano" => {
                soprano = Some(Melody::parse(value).map_err(|e| format!("bad soprano: {e}"))?)
            }
            "alto" => alto = Some(parse_pitches(value)?),
            "tenor" => tenor = Some(parse_pitches(value)?),
            "bass" => bass = Some(parse_pitches(value)?),
            other => return Err(format!("unknown field {other:?}")),
        }
    }
    Ok(ChoraleFixture {
        name: name.ok_or("missing `name:`")?,
        key: key.ok_or("missing `key:`")?,
        soprano: soprano.ok_or("missing `soprano:`")?,
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
    runtime: Duration,
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

        let total_runtime: Duration = covered_metrics.iter().map(|m| m.runtime).sum();
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
