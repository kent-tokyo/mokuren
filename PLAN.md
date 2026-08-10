# mokuren v0.1 — implementation plan

## Strategy

Depth-first, not breadth-first. Build one narrow vertical spine end-to-end
(the section-1 example: `C4 C4 G4 G4 A4 A4 G4`, C major, 4/4 → SATB →
`explain()` / `why_not()`), then thicken. Exhausting each phase in AGENTS.md
section 29 before starting the next would produce types with no working
demo; the spec's own success condition (section 1) is the pipeline, not the
type catalogue.

## Scope for this spine (v0.1 first cut)

In:
- PitchClass, Pitch, Octave, Interval (generic + quality + semitones)
- Key: major mode only, diatonic scale degrees
- Triads I–vii°, V7 with inversions (root, 6, 64, 65, 43, 42)
- HarmonicFunction (Tonic / Predominant / Dominant)
- SATB voices with default ranges
- Hard constraints: voice range, voice crossing, voice overlap, parallel
  5th/8ve/unison, missing chord tone / invalid doubling of the leading tone
- Soft preferences: stepwise motion, common tones, contrary motion, leap
  penalty, repeated-chord penalty
- Rules: leading-tone resolution, chordal-7th resolution, spacing between
  upper voices
- CandidateGenerator / CandidateEvaluator / SearchStrategy (BeamSearch) as
  separate types
- ScoreBreakdown (structured, not a single f64)
- DecisionTrace holding every evaluated alternative (not just the surviving
  beam) with status: `Valid` / `Rejected(RuleId)`, per position
- `why(position)`, `why_not(position, alternative)`, `explain()`,
  `diagnostics()`
- Deterministic tie-breaking per section 16, no `partial_cmp().unwrap()`

Deferred (explicitly out of scope for this pass):
- Natural/harmonic/melodic minor (spec: "必要性を検討")
- Chromatic alterations, secondary dominants, Neapolitan, Ger+6
- direct/hidden fifths & octaves (spec: "評価" — revisit once the spine works)
- serde, MIDI, MusicXML output
- workspace split into multiple crates (section 26: don't split until real
  dependency boundaries appear)

Done since the first pass (Phase 7 "thicken" work, tracked here rather than
opening a second plan doc):
- `UnpreparedSixFourRule` — the initial spine let the search open a phrase
  on an unrestricted I64, which isn't legal Common Practice writing; see
  README "Current limitations" #3 for what this rule does and doesn't cover.
- `tests/properties.rs` (`proptest`, spec: "可能なら"): pitch-class
  normalization, interval symmetry, chord-spelling round trip, and key
  scale-degree round trip. Scoped to the (root, quality) space
  `RomanNumeral::to_chord` actually produces — see that file's doc comment
  for why, and `src/pitch.rs`'s `spell_above`/`accidental_for_offset` for
  the representable-accidental-range limitation the scoping works around.
- `tests/golden.rs`: hand-verified small SATB passages (section 22
  "Golden tests").
- `benches/harmonize.rs` (Criterion, spec section 23): candidate
  generation, beam-width scaling, melody-length scaling. Moved up from
  the original v0.5 slot in ROADMAP.md since it was cheap and unblocks
  reporting real (rather than claimed) numbers in README.
- Leading-tone inner-voice exception: `LeadingToneResolutionRule` now
  lets alto/tenor skip down a step or third to complete the chord
  instead of resolving up, matching the standard textbook relaxation.
  Outer voices (soprano/bass) are unchanged (strict). Narrower than
  real pedagogy still — see README "Current limitations" #9 — the
  exception is unconditional here rather than "only when resolving up
  would leave the chord incomplete."
- Fail-closed pitch spelling: `spell_above`/`accidental_for_offset` used
  to fall back to `Natural` — a silently wrong pitch, not just a wrong
  spelling — when a required accidental exceeded double-flat/double-sharp.
  `Chord::pitch_classes()` now returns `Result` (unreachable via
  `RomanNumeral::to_chord` with any practical key; only reachable by
  constructing a `Chord` directly with an unusual root).
  `Key::diatonic_pitch_class` initially looked provably safe for *any*
  tonic by hand — proptest found a counterexample (a double-sharp tonic
  can need a triple-sharp for its own third), so `Key::new` became a
  validated constructor instead: it's the only public way to build a
  `Key` with an arbitrary tonic, so once one exists every lookup on it
  (`diatonic_pitch_class`, `scale`, `RomanNumeral::to_chord`, and
  everything downstream in the search hot path) stays infallible. No
  `Result` threading through candidate generation.
- `examples/chorale_benchmark.rs` + `BENCHMARK.md`: the roadmap was
  reordered (see ROADMAP.md's "Verification-first phase") to measure
  reasoning quality against unseen melodies before adding more theory
  scope, since README limitation #5 (hand-tuned weights, one melody) is a
  bigger risk than missing features. The harness computes all 7 protocol
  metrics against a duration-aware `.chorale` v2 fixture format (v1
  forced every note to a quarter, silently discarding real chorale
  rhythm — see `tasks/lessons.md`).
- `tools/music21_chorale_extractor.py`: decided music21 as the canonical
  external corpus source (Margaret Greentree's explicit permission for
  Bach-chorale distribution as part of music21 — `BENCHMARK.md` has the
  detail from the other three candidates that didn't clear licensing).
  Samples alto/tenor/bass at soprano onsets only, by construction — no
  independent ATB timeline exists to leak Bach's own harmonic-rhythm
  decisions into a benchmark that's supposed to discover them. Writes a
  `manifest.json` (source/version/numbering/selected IDs/file hashes)
  for reproducibility. Its *output* is not committed — extraction runs
  happen in a scratch directory outside the repo, then are deleted; only
  the code and the findings are kept.
- `Duration` (src/melody.rs) gained dotted variants (`DottedHalf`,
  `DottedQuarter`, `DottedEighth`) — the real chorale data needed them
  and nothing in the rule engine reads `Duration` for any decision, so
  this was a safe, contained extension, not a behavior change.
- v0.1.0 full major-mode baseline (144 chorales, `tasks/baseline-v0.1.0.md`):
  50.7% coverage, 0 hard-rule violations, failure taxonomy attributing
  88.7% of failures to chromatic-soprano tones and 0 to the cadential-6/4
  rule. This is the data point the "benchmark → failure decomposition →
  next feature → re-benchmark" loop runs on now — see ROADMAP.md's
  "Verification-first phase," which was reordered by this finding
  (secondary dominants moved ahead of the 6/4 lookahead, and a new
  soprano-rest `Melody` gap was surfaced that wasn't on the original list).

## Phases (AGENTS.md section 29, adapted to depth-first order)

1. Foundations: Pitch/Interval/Key/Chord/RomanNumeral/Voice/SATB — just
   enough to represent the spine's melody and one harmonization.
2. Rule engine: `Rule` trait, `RuleResult`, hard/soft split, `ScoreBreakdown`.
3. Candidate generation: per-position chord + voicing candidates, with
   generation/rejection/retention counters wired in from the first call site.
4. Search: BeamSearch over the spine, deterministic tie-break.
5. Explainability: DecisionTrace, `explain()`, `why()`, `why_not()`.
6. End-to-end demo: the section-1 example running via the section-17 API
   shape, as an example and an integration test.
7. Thicken: broaden theory coverage, diagnostics detail, golden tests,
   benchmarks — only after 1–6 are green.

## Design decisions worth recording

- Hard constraints reject and record a `RuleId`; soft preferences only
  adjust score. A hard rule must never be expressible as a large penalty —
  that would make diagnostics counts lie.
- DecisionTrace retains all evaluated alternatives at each position (with
  status), not just what survives the beam, because `why_not()` must be
  able to answer for candidates the beam pruned.
- `Melody::parse` is a boundary parser: returns `Result`, never panics.
