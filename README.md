# mokuren

**mokuren — a fast, explainable symbolic composition engine for exploring music-theoretic decisions.**

mokuren generates and searches Common Practice harmonizations and can tell you *why* it chose one chord over another — not by asking an LLM to narrate after the fact, but by reading a structured reason trace that the search itself produced.

> v0.1 status: one vertical slice works end to end — SATB harmonization of a fixed soprano melody in a major key, with `explain()`, `why()`, and `why_not()` backed by real evaluated candidates. See [Current limitations](#current-limitations) before relying on this for anything beyond that slice.

## What mokuren is

- A **candidate-search harmonizer**: generate diatonic chord + voicing candidates for each note of a melody, score them against Common Practice rules, and search for the best sequence with beam search.
- An **explainable reasoning engine**: every accepted or rejected candidate carries a structured `ScoreBreakdown` and `Vec<Reason>` — not a bare number. `why()` and `why_not()` read directly from that data.
- **Deterministic**: same input, same rules, same output, with an explicit tie-break chain (score → voice-leading cost → canonical Roman-numeral order → canonical voicing order). No `f64::partial_cmp().unwrap()` anywhere in the ranking path.

## What mokuren is not

- Not a DAW, synthesizer, or audio engine — mokuren never touches audio. See section 19/27 of `AGENTS.md`.
- Not a notation editor or MIDI/MusicXML exporter (yet) — v0.1 ends at the structured symbolic result; see the [roadmap](#roadmap).
- Not an LLM-driven composer — an LLM can sit in front of mokuren as a natural-language interface, but mokuren's music-theoretic reasoning never depends on one.
- Not a general "harmonize anything in any style" tool — v0.1 is one style (`Style::CommonPractice`), one mode (major), one voicing (SATB).

## Minimal example

```rust
use mokuren::prelude::*;

fn main() -> Result<()> {
    let melody = Melody::parse("C4 C4 G4 G4 A4 A4 G4")?;

    let result = Composer::new()
        .key(Key::C_MAJOR)
        .style(Style::CommonPractice)
        .voices(Voices::SATB)
        .search(BeamSearch::new().width(32))
        .harmonize(melody)?;

    println!("{}", result.explain());
    Ok(())
}
```

Run it yourself: `cargo run --example basic`.

## Explainability example

```text
Harmonization in C major

Position 0: I (score +0.30)
Position 1: IV64 (score +1.11)
Position 2: V6 (score +1.46)
Position 3: I (score +1.76)
Position 4: vi (score +1.03)
Position 5: IV (score +1.28)
Position 6: I (score +2.21)

Progression: I - IV64 - V6 - I - vi - IV - I
```

Asking why the second chord is `V6`:

```text
Why V6?

+ voice leading: 13 semitones of motion, 0 common tone(s), contrary motion: true: +0.25
+ harmonic function: predominant -> dominant: +0.80

Final local score: 1.46
```

## `why_not` example

```text
Why not iii?

iii was valid and ranked #6.

+ voice leading: 18 semitones of motion, 0 common tone(s), contrary motion: true: +0.25
+ harmonic function: predominant -> tonic: +0.20

Final local score: 0.81
Difference from selected V6: -0.65
```

`why_not` also answers for candidates the beam search never kept alive to the end (their status shows *why* they were rejected — which `RuleId` fired) and for numerals that were never legal at that position at all (a clean error, not a panic).

Diagnostics are aggregate, not per-decision — real counts from the run above, at beam width 32:

```text
Candidates generated: 100864
Candidates retained:  3686
Candidates rejected:  97178

Top rejection reasons:
voice overlap 74772
missing chord tone 67720
voice crossing 51114
unprepared six-four 18360
spacing 16980
```

## Supported theory (v0.1)

- **Tonality**: major only, diatonic scale degrees, correctly spelled (no chromatic alterations).
- **Chords**: major/minor/diminished triads, dominant seventh, with inversions (triads: root/6/64; V7: root/65/43/42).
- **Roman numerals**: I, ii, iii, IV, V, V7, vi, vii°, with `HarmonicFunction` (tonic/predominant/dominant) kept as a distinct concept from the numeral itself.
- **Voices**: SATB with conventional default ranges.
- **Hard constraints**: voice range, voice crossing, voice overlap, parallel 5th/8ve/unison, spacing, missing chord tone, leading-tone doubling, leading-tone resolution, chordal-seventh resolution, unprepared six-four.
- **Soft preferences**: voice-leading quality (common tones, contrary motion), melodic motion (stepwise reward / leap penalty), harmonic-function progression, cadence support, doubling preference, repeated-chord penalty.
- **Search**: beam search, deterministic tie-breaking, per-position diagnostics.

## Current limitations

Read this before trusting an output. In priority order:

1. **On a small real-chorale sample, mokuren failed to harmonize about half of them at all.** A 20-chorale validation run of the harmonic-vocabulary against real Bach chorales (major mode, via `tools/music21_chorale_extractor.py` — see `BENCHMARK.md`) found 50% coverage, not a rate near the ~100% every synthetic test melody had shown. The diagnosed cause: a chromatic (non-diatonic) soprano tone — mokuren's engine is diatonic-only (no secondary dominants, no modal mixture) by design (item 5 below), and real chorale writing uses chromatic tones more than a hand-picked stepwise melody ever exercised. This is not yet the full major-mode baseline (that's a deliberate next step, tracked in `tasks/todo.md`), so treat 50% as a preliminary signal, not a final number — but expect the real one to be well below 100% too.
2. **Six-four (second-inversion triad) handling is a backward-looking heuristic**, not true cadential-6/4 detection. It accepts a 6/4 only if the bass is a pedal (same as the previous bass) or approached by step; it cannot look ahead to confirm the chord actually resolves to V, and rejects a 6/4 as the very first chord outright (no previous bass to justify it against).
3. **Beam search has a horizon effect.** `CadenceSupportRule` only rewards the final position, so a beam that's too narrow can prune away the eventually-best path before it reaches that reward. `BeamSearch::new()` defaults to width 32, which is wide enough for the melodies v0.1 targets; widen it for longer or more harmonically ambiguous melodies.
4. **Score weights are v0.1 defaults, not corpus-tuned.** They were adjusted by hand against one melody (see `PLAN.md`) to reach musically sensible behavior, not yet validated against a full chorale corpus (section 24 of `AGENTS.md`; a preliminary sample exists — see item 1 — the full run is still open).
5. **No minor mode, no chromatic harmony** (secondary dominants, modal mixture, Neapolitan, augmented sixths), **no MIDI/MusicXML/serde**. All deliberately deferred — see `PLAN.md`. Item 1 above is what this costs in practice on real material, not just in theory.
6. **`why()` at the first position is thin.** With no previous chord, most soft rules have nothing to compare against and contribute no `Reason`; only the doubling preference (which emits no reason text) applies.
7. **Single crate, single style profile.** `Style::CommonPractice` is the only style; `RuleProfile`/`StyleProfile`-style extensibility exists in the types but only one instance is implemented.
8. **Leading-tone resolution's inner-voice exception is narrower than real pedagogy.** `LeadingToneResolutionRule` lets an inner voice (alto/tenor) skip down a step or third to complete the chord instead of resolving up, but the exception is unconditional — real practice reserves it for when resolving up would leave the chord incomplete or force bad doubling, which this rule doesn't check. `ChordalSeventhResolutionRule` has no such exception in any voice (real practice mostly agrees).
9. Performance has one measured data point (release build, this machine, one commit): `cargo bench` — candidate generation for one position ≈480µs; a full 7-note harmonization at the default beam width (32) ≈212ms, scaling roughly linearly with both beam width and melody length. Not tracked over time or across machines yet, and not something to build a "fast" claim on beyond the tagline itself (section 23 of `AGENTS.md`).

## Roadmap

Phased implementation plan: [`PLAN.md`](PLAN.md). Longer-term direction and competitive positioning: [`ROADMAP.md`](ROADMAP.md). Current priority is verification, not new features — see ROADMAP's "Verification-first phase" and the external chorale benchmark protocol: [`BENCHMARK.md`](BENCHMARK.md).

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

All three are green as of this commit.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option, the standard dual license for Rust crates.
