# mokuren roadmap

## Thesis

mokuren does not try to out-build the existing symbolic-music ecosystem on breadth, corpus size, or notation fidelity — those projects have a decade-plus head start and much larger contributor bases. The plan is to beat them on one specific axis where none of them currently compete at all: **explaining a compositional decision at the level of "here is every alternative I considered, why I rejected it, and by how much it lost."** That's `why()` / `why_not()` (AGENTS.md section 3, section 28) — mokuren's answer to "surpass the competing libraries" is to win on reasoning, not on reach.

## Landscape

| Library | Ecosystem | What it's actually good at | Where it stops short of mokuren's target |
|---|---|---|---|
| [music21](https://github.com/cuthbertLab/music21) | Python | Large-scale corpus analysis, format I/O (MusicXML/MIDI/ABC/Humdrum), and Roman-numeral **analysis** of existing music (`roman.py`) | Roman-numeral objects label music you already have; 4-part realization from Roman numerals is manual — the user picks octaves "with good voice leading in mind." No candidate search, no ranked alternatives, no rejection reasons. |
| [Abjad](https://github.com/Abjad/abjad) | Python | Programmatic construction of LilyPond notation — the reference tool for complex contemporary score engraving | Not a harmony engine at all; it has no concept of a chord being "correct" or "rejected." Pure notation construction. |
| [Tonal](https://github.com/tonaljs/tonal) | JavaScript | Small, fast, pure-functional music-theory *data* utilities (intervals, chords, scales, keys) | Deliberately has no search, no generation, no notion of a "decision." It's a utility library, not an engine — closest analogue to mokuren's `pitch`/`chord`/`key` modules alone. |
| [SCAMP](https://github.com/MarcTheSpark/scamp) | Python | Precise polyphonic tempo/timing control, live and offline playback, notation export — a hub for algorithmic composition's *performance* side | No tonal rule engine; SCAMP deliberately imposes as little as possible on the composer's aesthetic choices, which is the opposite of mokuren's constraint-driven search. |
| [MusPy](https://github.com/salu133445/muspy) | Python | Dataset management, format conversion, and evaluation metrics for training ML music-generation models | An ML data-pipeline toolkit, not a reasoning engine — it doesn't generate or explain, it prepares and measures. |
| [Euterpea](https://www.euterpea.com/) | Haskell | Wide-spectrum DSL spanning high-level composition down to signal-level synthesis, strong in music-computing pedagogy | Broader scope (audio included) but no built-in Common Practice constraint/search engine with structured rejection reasons. |
| [tunes](https://github.com/sqrew/tunes) | Rust | The closest direct peer: scales/chords/progressions plus a real-time synthesis and playback engine (multiple synthesis techniques, sample playback) in one crate | Synthesis-first — mokuren explicitly stays out of audio (AGENTS.md section 19) and goes deep on reasoning instead of breadth of sound-generation features. |

None of these expose "why not `vi`, and by how much did it lose" as a first-class, structured, queryable thing. That gap is the target.

## What mokuren is not trying to win

- **Format breadth.** music21 and MusPy both speak MIDI/MusicXML/ABC/Humdrum today; mokuren doesn't yet (section 18 — structured result and text explanation come first, deliberately).
- **Corpus scale / ML tooling.** MusPy and music21's corpus tools serve a different audience (researchers training or evaluating generative models) that mokuren isn't building for.
- **Notation engraving quality.** Abjad-via-LilyPond is a mature, dedicated tool for that; mokuren has no notation renderer and isn't planning to build one (section 27).
- **Audio/synthesis.** tunes and Euterpea both go there; mokuren's non-goals list (section 19/27) rules this out explicitly.
- **Raw speed claims beyond what's measured.** `criterion` benchmarks now exist (`benches/harmonize.rs`) with real one-off numbers in the README, but that's a single data point on one machine, not a tracked performance story — still not something to build a competitive "fast" claim on (section 23).

## Phased roadmap

### v0.1 — the vertical slice (current)
SATB harmonization of a fixed major-key soprano melody, Common Practice rules, beam search, `explain()`/`why()`/`why_not()`/`diagnostics()` backed by real evaluated candidates. See `PLAN.md` and the README's [Current limitations](README.md#current-limitations) for exactly what's real today versus deferred.

### v0.2 — close the correctness gaps v0.1 shipped with
- ~~Inner-voice exception for leading-tone resolution~~ — done: `LeadingToneResolutionRule` now lets alto/tenor skip down a step or third to complete the chord. Still narrower than real pedagogy (unconditional rather than "only when resolving up would leave the chord incomplete") — see README limitation #7. `ChordalSeventhResolutionRule` intentionally has no such exception (real practice mostly agrees).
- True cadential-6/4 detection with lookahead, replacing the backward-only pedal/passing heuristic (README limitation #1).
- Minor mode (natural/harmonic/melodic) — AGENTS.md section 5 leaves this as "evaluate the need"; a minor-key melody is the forcing function.
- ~~`proptest` coverage~~ — done: `tests/properties.rs` (pitch-class normalization, interval symmetry, chord-spelling round trip, key scale-degree round trip).
- ~~`criterion` benchmarks~~ — done ahead of schedule (moved up from v0.5, since it was cheap and unblocks honest performance language elsewhere): `benches/harmonize.rs`, real numbers in README limitation #8. External validation against a chorale corpus is still v0.5 work — a benchmark harness isn't the same as a correctness/quality baseline.

### v0.3 — widen the harmonic vocabulary
- Secondary dominants (V/V, V7/ii, ...), modal mixture, Neapolitan, augmented sixths (AGENTS.md section 20) — the `RomanNumeral`/`HarmonicFunction` data model was built to extend into these without a redesign (section 5).
- A second `StyleProfile` (e.g. Bach Chorale) to prove the hard/soft rule split and `Style` enum actually generalize past one instance — with its rule *choices* documented, not just its rule *set* (section 20: "don't implement a style as a rigid rule pile without a rationale").

### v0.4 — output that leaves the terminal
- MusicXML export (interoperates with music21/Abjad/notation software — mokuren doesn't need to build a renderer if it can hand off to one that already exists).
- MIDI export.
- `serde` for `HarmonizationResult`/`Decision`, so `why_not()` output is queryable outside the Rust process (a JSON reason trace an LLM-facing layer can consume per section 21's architecture).

### v0.5 — external validation
- Chorale-dataset benchmark per AGENTS.md section 24: strip alto/tenor/bass from a known chorale, harmonize the soprano, and score on rule-violation rate, voice-leading quality, cadence correctness, and solution diversity — explicitly *not* on matching the original note-for-note (section 24's own caveat).
- Track `criterion` numbers over time (regression detection) now that the harness exists (moved up to v0.2) — only after that tracking exists does a *speed* claim (as opposed to the one-off numbers already in the README) belong there.

### v1.0 candidate — counterpoint and melody as first-class targets
- Species counterpoint (two-part, then multi-voice) — a genuinely different search problem from SATB harmonization, sharing the `Rule`/`SearchStrategy` architecture rather than the concrete rules.
- Melody generation (contour, interval distribution, phrase structure) — currently mokuren only *harmonizes* a given melody; generating one is a distinct, larger scope AGENTS.md section 20 flags for later.
- Workspace split (`mokuren-core`, `mokuren-theory`, `mokuren-rules`, ...) once real crate boundaries exist from the above growth — not before (AGENTS.md section 26 is explicit: don't split until the dependencies are real).

## Non-goals (restated from AGENTS.md section 27)

DAW features, high-quality audio synthesis, VST, realtime live coding, deep-learning music generation, LLM-driven composition (an LLM may sit *in front of* mokuren per section 21, never replace its reasoning), full genre coverage, a music21-scale reimplementation, or any GUI (MIDI editor or notation editor). mokuren stays a reasoning engine with a Rust API, not "a music library that does everything."
