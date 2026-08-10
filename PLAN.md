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
- proptest property tests (spec: "可能なら")
- serde, MIDI, MusicXML output
- workspace split into multiple crates (section 26: don't split until real
  dependency boundaries appear)

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
