# mokuren

[![CI](https://github.com/kent-tokyo/mokuren/actions/workflows/ci.yml/badge.svg)](https://github.com/kent-tokyo/mokuren/actions/workflows/ci.yml)
[![docs.rs](https://img.shields.io/docsrs/mokuren)](https://docs.rs/mokuren)
[![License](https://img.shields.io/crates/l/mokuren)](#license)

English | [日本語](README_ja.md)

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

- **Tonality**: major and (natural + harmonic) minor, diatonic scale degrees, correctly spelled. Chromatic alterations are limited to applied dominants and harmonic minor's raised leading tone (below) — no modal mixture, Neapolitan, or augmented sixths yet.
- **Chords**: major/minor/diminished triads, dominant seventh, with inversions (triads: root/6/64; V7: root/65/43/42).
- **Roman numerals**: I, ii, iii, IV, V, V7, vi, vii°, plus applied ("secondary") dominants V/x and V7/x for x in {ii, iii, IV, V, vi} — e.g. `V/V`, `V7/vi`. In minor: each natural-minor diatonic triad, plus the harmonic-minor-derived V, V7, vii° (using the raised leading tone). `HarmonicFunction` (tonic/predominant/dominant) is kept as a distinct concept from the numeral itself.
- **Voices**: SATB with conventional default ranges.
- **Hard constraints**: voice range, voice crossing, voice overlap, parallel 5th/8ve/unison, spacing, missing chord tone, leading-tone doubling, leading-tone resolution, chordal-seventh resolution, unprepared six-four, secondary-dominant resolution.
- **Soft preferences**: voice-leading quality (common tones, contrary motion), melodic motion (stepwise reward / leap penalty), harmonic-function progression, cadence support, doubling preference, repeated-chord penalty.
- **Search**: beam search, deterministic tie-breaking, per-position diagnostics.

## Current limitations

Read this before trusting an output. In priority order:

1. **On the full Bach chorale corpus, mokuren harmonizes most major-mode chorales but a minority of minor-mode ones.** A 348-chorale baseline (every Riemenschneider chorale music21 resolves to major or minor, via `tools/music21_chorale_extractor.py`) found **94.5% coverage in major** (172/182) but only **42.8% in minor** (71/166), 0 hard-rule violations in either — full detail in `BENCHMARK.md` and `tasks/baseline-v0.4.0-minor-mode.md`. The gap is real and expected, not a bug: minor mode doesn't have applied dominants yet (major's escape hatch for tonicizing a foreign scale degree) or melodic minor's raised 6th, so 77% of minor's failures are a soprano note with literally no chord in the current vocabulary — the same shape as major's own chromatic-soprano gap before secondary dominants were implemented. Major's remaining 10 failures: 7 search-exhausted (a wider beam finds a path, see item 4) and 3 a genuine unfixed gap — a soprano note forced into a formal chordal-seventh role that must resolve by step, when the real melody leaps a third instead, most likely Bach using the note as a decorative non-chord/passing tone, which mokuren has no model for at all (every soprano note must currently be a full chord tone).
2. **A soprano rest splits a melody into independent phrases rather than being modeled directly.** `Melody`/`Composer::harmonize` still only ever see a plain, rest-free sequence of notes — a new `MelodyLine` type (`src/melody.rs`) holds `Note`/`Rest` events and its `phrases()` method splits at each rest into contiguous note runs, harmonized independently (matching how a breath rest actually functions — a phrase boundary, not a gap inside one harmonic idea). This grew the attemptable corpus from 144 to 182 chorales (+38) with zero regressions on the original 144. What it doesn't do: model a rest that *isn't* a full phrase break (e.g. a rest with real harmonic continuity across it would be split anyway), and voice-leading/parallel-motion rules never see across a phrase boundary (each phrase starts fresh, as if it were its own short piece) — not yet known to matter in practice, but not verified either.
3. **Six-four (second-inversion triad) handling is a backward-looking heuristic**, not true cadential-6/4 detection. It accepts a 6/4 only if the bass is a pedal (same as the previous bass) or approached by step; it cannot look ahead to confirm the chord actually resolves to V, and rejects a 6/4 as the very first chord outright (no previous bass to justify it against). The baseline above found **zero failures caused by this rule specifically** — real, but not the current bottleneck (see `ROADMAP.md`).
4. **Beam search has a horizon effect.** `CadenceSupportRule` only rewards the final position, so a beam that's too narrow can prune away the eventually-best path before it reaches that reward. `BeamSearch::new()` defaults to width 32; the applied-dominant vocabulary (item 6) roughly doubled the number of candidates considered at each position, which made this horizon effect measurably worse — 2 chorales that harmonized at width 32 in the v0.1.0 baseline need a wider beam now (one recovers at width 64, one needs width 512; confirmed directly, not assumed — see `tasks/baseline-v0.2.0-secondary-dominants.md`). The default width was deliberately left at 32 rather than raised to cover the width-512 case, which would make a rare case expensive for everyone — see `tasks/lessons.md` for the related case (a *scoring* bug, not just a width one) found while building this. Widen it for longer or more harmonically ambiguous melodies.
5. **Score weights are v0.1 defaults, not corpus-tuned.** They were adjusted by hand against one melody (see `PLAN.md`) to reach musically sensible behavior. The baseline in item 1 measures behavior against real chorales, but doesn't retune the weights against them — that's still open.
6. **Secondary dominants are narrower than full applied-chord theory.** Only V/x and V7/x (major triad / dominant seventh) are implemented, for x in {ii, iii, IV, V, vi} — no applied leading-tone chords (vii°/x), no applied dominants of applied dominants (no tonicization chains). Resolution requires strict step-up motion in *every* voice holding the chromatic tone, unlike `LeadingToneResolutionRule`'s inner-voice exception (item 9) — real practice sometimes allows an inner voice to skip instead. An applied dominant is also never offered as the final chord of a phrase (it can't resolve there). Not implemented in minor keys yet at all — minor's own chromatic layer is only the harmonic-minor V/V7/vii° above.
7. **No melodic minor** (the raised-6th convention), **no other chromatic harmony** (modal mixture, Neapolitan, augmented sixths), **no MIDI/MusicXML/serde**. All deliberately deferred — see `PLAN.md`.
8. **`why()` at the first position is thin.** With no previous chord, most soft rules have nothing to compare against and contribute no `Reason`; only the doubling preference (which emits no reason text) applies.
9. **Single crate, single style profile.** `Style::CommonPractice` is the only style; `RuleProfile`/`StyleProfile`-style extensibility exists in the types but only one instance is implemented.
10. **Leading-tone resolution's inner-voice exception is narrower than real pedagogy.** `LeadingToneResolutionRule` lets an inner voice (alto/tenor) skip down a step or third to complete the chord instead of resolving up, but the exception is unconditional — real practice reserves it for when resolving up would leave the chord incomplete or force bad doubling, which this rule doesn't check. `ChordalSeventhResolutionRule` and `SecondaryDominantResolutionRule` have no such exception in any voice (real practice mostly agrees for the former; the latter is a v0.1 simplification — item 6).
11. Performance has one measured data point (release build, this machine, one commit) that predates the applied-dominant vocabulary: `cargo bench` — candidate generation for one position ≈480µs; a full 7-note harmonization at the default beam width (32) ≈212ms, scaling roughly linearly with both beam width and melody length — not re-measured against the current vocabulary yet. Not tracked over time or across machines, and not something to build a "fast" claim on beyond the tagline itself (section 23 of `AGENTS.md`). The baseline in item 1 has current real-corpus runtime: median 1.4s/chorale at width 32 (p90 1.9s) — lower than the v0.2.0 baseline's 2.9s median despite the larger corpus (measured on a different run; phrase-splitting, item 2, plausibly contributes for the 38 multi-phrase chorales, but the single-phrase majority did identical work to v0.2.0, so this isn't fully attributed — not investigated further since it's a beneficial direction, not a regression).

## Roadmap

Phased implementation plan: [`PLAN.md`](PLAN.md). Current priority is verification, not new features — see the external chorale benchmark protocol: [`BENCHMARK.md`](BENCHMARK.md).

## Development

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

All three are green as of this commit.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option, the standard dual license for Rust crates.
