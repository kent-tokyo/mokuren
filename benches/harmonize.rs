//! Benchmarks (AGENTS.md section 23). Run with `cargo bench`.
//!
//! Existence of this file is not itself a performance claim — see
//! README.md's "Current limitations": the tagline's "fast" is this
//! project's naming brief, not a measured claim, until these numbers
//! are actually reported somewhere the README cites.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use mokuren::diagnostics::Diagnostics;
use mokuren::generate::CandidateGenerator;
use mokuren::key::Key;
use mokuren::melody::{Duration, Melody, Note};
use mokuren::pitch::{Octave, Pitch, PitchClass};
use mokuren::prelude::*;
use mokuren::rules::Style;
use std::hint::black_box;

fn spine_melody() -> Melody {
    Melody::parse("C4 C4 G4 G4 A4 A4 G4").unwrap()
}

/// Repeats the spine's melodic shape to reach a target length, for
/// scaling benchmarks — not a musically meaningful melody past the
/// first 7 notes, just a fixed-cost stand-in of a given size.
fn melody_of_length(len: usize) -> Melody {
    let pattern = [
        PitchClass::C,
        PitchClass::C,
        PitchClass::G,
        PitchClass::G,
        PitchClass::A,
        PitchClass::A,
        PitchClass::G,
    ];
    let notes = (0..len)
        .map(|i| {
            Note::new(
                Pitch::new(pattern[i % pattern.len()], Octave(4)),
                Duration::Quarter,
            )
        })
        .collect();
    Melody::new(notes)
}

fn bench_candidate_generation(c: &mut Criterion) {
    let key = Key::C_MAJOR;
    let style = Style::CommonPractice;
    let generator = CandidateGenerator::new(&key, &style);
    let soprano = Pitch::new(PitchClass::C, Octave(4));

    c.bench_function("candidate_generation_single_position", |b| {
        b.iter(|| {
            let mut diagnostics = Diagnostics::default();
            let candidates =
                generator.generate(black_box(soprano), None, None, false, &mut diagnostics);
            black_box(candidates)
        })
    });
}

fn bench_beam_width_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("beam_width_scaling");
    for width in [8usize, 16, 32, 64] {
        group.bench_with_input(BenchmarkId::from_parameter(width), &width, |b, &width| {
            b.iter(|| {
                let melody = spine_melody();
                let result = Composer::new()
                    .key(Key::C_MAJOR)
                    .style(Style::CommonPractice)
                    .search(BeamSearch::new().width(width))
                    .harmonize(black_box(melody));
                black_box(result)
            })
        });
    }
    group.finish();
}

fn bench_melody_length_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("melody_length_scaling");
    for len in [4usize, 7, 14, 21] {
        group.bench_with_input(BenchmarkId::from_parameter(len), &len, |b, &len| {
            b.iter(|| {
                let melody = melody_of_length(len);
                let result = Composer::new()
                    .key(Key::C_MAJOR)
                    .style(Style::CommonPractice)
                    .search(BeamSearch::new().width(32))
                    .harmonize(black_box(melody));
                black_box(result)
            })
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_candidate_generation,
    bench_beam_width_scaling,
    bench_melody_length_scaling
);
criterion_main!(benches);
