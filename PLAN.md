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
  real pedagogy still — see README "Current limitations" #10 — the
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
- Secondary dominants (roadmap phase 2): `RomanNumeral::applied_to: Option<ScaleDegree>`
  marks an applied dominant (V/x, V7/x for x in ii/iii/IV/V/vi); `to_chord`
  became `Option<Chord>` (only 2 real call sites, both in `generate.rs` —
  the diatonic path stays infallible, only an applied dominant's root can
  fail to spell). New hard rule `SecondaryDominantResolutionRule` requires
  the next chord's root to match the tonicized target and the chromatic
  tone to resolve up by step in every voice holding it (no inner-voice
  exception yet, unlike `LeadingToneResolutionRule` — README limitation
  #6). Two tie-break tuples (`generate::canonical_rank`,
  `search::path_key`) needed a 4th field (`applied_to`) since every
  applied dominant shares `degree == ScaleDegree::DOMINANT` regardless of
  target — caught by review before it shipped, not by a failing test.
  - The naive scoring version broke a demo pinned since v0.1
    (`tests/spine.rs`'s `ends_on_tonic_with_a_recognized_cadence`,
    `examples/basic.rs`): rewarding a correctly-resolving applied
    dominant as strongly as an authentic cadence let the search
    substitute one for a diatonic chord anywhere it merely *fit* a
    diatonic soprano note, not just where a chromatic tone required it —
    on the fully-diatonic spine melody, `V/V -> V` (dominant-of-the-
    dominant, resolving right back to the plain dominant) outscored the
    correct tonic close, and needed a beam width in the hundreds before
    the correct path resurfaced. Root cause found by actually running
    the search at a width sweep (8 through 1024) and reading `why()`/
    `why_not()` at the divergent position, not by re-deriving the
    expected score by hand — see `tasks/lessons.md`. Fixed by not
    scoring an applied dominant's *introduction* via the diatonic
    harmonic-function table at all (only its *resolution* is scored),
    which restored the correct ending at the default width (32) with no
    change to the default beam width itself.
  - `examples/chorale_benchmark.rs`'s `classify_failure` used to treat
    any non-diatonic soprano tone as automatically unsupported
    (`FailureCategory::ChromaticSoprano`); that stopped being true the
    moment applied dominants existed, so the check now asks whether the
    tone is a chord tone of *any* implemented chord (diatonic or applied
    dominant), not just the plain diatonic scale.
  - The 144-chorale re-run (coverage 91.7%) left 6 chorales classified
    `Other` (undiagnosed). Bisecting them found the *bisection tool*
    had the exact same is_final bug as the spine-melody one above, one
    level removed: it isolated a failure point by harmonizing a
    *truncated* melody, which made the truncation point look
    artificially final to the search, wrongly triggering
    `SecondaryDominantResolutionRule`'s final-position rejection at
    positions that aren't final in the real piece. Fixed by replaying
    the full melody's search up to the actual failure point instead
    (`replay_to_failure` in `examples/chorale_benchmark.rs`, correctly
    `is_final: false` for every position except the melody's true last
    one) — see `tasks/lessons.md`. With that fixed, 3 of the 6
    (Riemenschneider 102, 173, 327) shared one more real bug: the rule
    required resolution at the very *next* position unconditionally, so
    a chromatic tone held/repeated across two notes before resolving
    (common) had nowhere to go on its second occurrence. Fixed:
    prolonging the *same* applied dominant across a repeat no longer
    counts as an unresolved dangling dominant — the obligation to
    resolve only applies once the harmony actually changes away from
    it. Raised coverage from 91.7% to 94.4% (136/144).
  - The remaining 2 of those 6 (Riemenschneider 40, 202) are a real,
    unfixed gap: the soprano is forced into a formal chordal-seventh
    role requiring step-down resolution, but the actual melody leaps a
    third — almost certainly Bach using the note as a decorative
    non-chord (passing) tone, which mokuren has no model for at all.
    Not attempted this pass (`tasks/todo.md`) — a genuinely different
    kind of extension (letting a soprano note sit outside the chord)
    from what secondary dominants added (more chords to choose from).
- Minor mode (roadmap phase 3): `Mode::Minor` added as natural minor
  (matching the key signature); the harmonic-minor-derived V, V7, vii°
  (using the raised leading tone) are an additional chromatic vocabulary
  layer alongside the seven natural-minor diatonic triads, the same
  "extra vocabulary, not a redesign" shape secondary dominants used —
  not a new `Mode` variant, since a piece doesn't change key signature
  to use the raised 7th. `RomanNumeral::applied_to: Option<ScaleDegree>`
  was refactored into `NumeralSource` (`Diatonic` / `AppliedDominant` /
  `HarmonicMinorRaisedSeventh`) so a rule can distinguish *why* a
  numeral is chromatic instead of stacking booleans — advisor-flagged
  before implementing, given the project already got burned once by a
  tie-break field added ad hoc for `applied_to`.
  - Real bug found and fixed before it shipped: `LeadingToneResolutionRule`/
    `LeadingToneDoublingRule` computed "the leading tone" via
    `key.diatonic_pitch_class(LEADING_TONE)`, which is natural minor's
    own *unraised* 7th — meaning neither rule would have recognized or
    enforced resolution of the harmonic-minor *raised* leading tone at
    all. Fixed with `Key::functional_leading_tone()` (major: the plain
    diatonic 7th, unchanged; minor: the raised 7th).
  - `HarmonicFunctionProgressionRule`'s degree→function table gained a
    quality check at degree 7: a major-triad VII (natural minor's
    subtonic) isn't the same chord as a diminished vii° and shouldn't
    score as a dominant-function arrival the way vii°/harmonic-minor
    vii° do.
  - Re-run against an expanded corpus (182 → 348, +166 minor chorales):
    major unchanged at 94.5% (172/182, zero regressions, directly
    diffed); minor 42.8% (71/166) — a first-pass number, predicted in
    writing before running (per an advisor review) to land well below
    major's, since minor has no applied dominants yet and no melodic
    minor (raised 6th) — 77% of minor's failures are exactly a soprano
    tone with no chord in the vocabulary for either reason. Bisected one
    of the smaller rule-conflict failures (Riemenschneider 367, B minor)
    and confirmed it traces to the same root cause (too few dominant-area
    voicing choices), not a distinct bug.
  - Deliberately deferred, same narrower-than-full-theory scoping applied
    dominants used: `vii°7` (its chordal seventh sits on the lowered
    6th — the same degree `ChordalSeventhResolutionRule` already
    produces failures on), applied dominants in minor keys, melodic
    minor. See `tasks/todo.md`.
  - Full detail: `tasks/baseline-v0.4.0-minor-mode.md`, `BENCHMARK.md`.
- Minor applied dominants + melodic minor (re-prioritized ahead of
  adaptive/search-budget research by explicit user directive
  2026-08-11: minor's failure mode was missing candidates outright, not
  search missing an existing one, so vocabulary had the bigger lever).
  Vocabulary chosen from real corpus evidence via a new
  `--minor-gap-report` CLI mode, not copied from major: bisecting the
  v0.4.0 minor failures found 79 chorales needing V(7)/ii, 68 V(7)/V,
  65 the melodic-minor raised 6th, 16 V(7)/IV, 1 V(7)/vi, 0 V(7)/iii —
  100% classified. V/III excluded from the implementation on that
  basis; raised 6th (originally deferred as "melodic minor" to a later
  phase) pulled forward into this pass since applied dominants alone
  would only have fully resolved 16/81 chorales, vs. 65/81 needing the
  raised 6th too.
  - `RomanNumeral::minor_applied_dominant_vocabulary()` (V/x, V7/x for
    x in {ii, IV, V, vi}) and `melodic_minor_vocabulary()` (ii as a
    minor triad, IV as a major triad) both reuse the "same root,
    different quality" trick harmonic minor's V/V7 already established
    — verified by hand before implementing, same as before.
  - Re-run: major unchanged 172/182 (94.5%, zero regressions, directly
    diffed); minor 42.8% → 64.5% (71/166 → 107/166, +36 net). 0
    hard-rule violations maintained. 18 minor chorales regressed from
    the vocabulary roughly doubling again — all 18 confirmed
    beam-width-recoverable (directly retested at width 64–512), the
    same horizon-effect pattern applied dominants first produced for
    major; default width (32) intentionally unchanged.
  - Full detail: `tasks/baseline-v0.5.0-minor-applied-dominants.md`.
- Soprano-rest support (roadmap phase 4): `Melody`/`Composer::harmonize`
  are unchanged — still a plain, rest-free `Vec<Note>`. A new `MelodyLine`
  type (`src/melody.rs`) holds `Note`/`Rest` events; its `phrases()`
  method splits at each rest into independent contiguous note runs,
  harmonized separately through the same unchanged `Composer::harmonize`
  path. Grounded in real data before picking this design (music21 query
  over the actual Riemenschneider-numbered corpus, not assumption): of
  the 75 chorales the old extractor excluded for a soprano rest, most
  have only 1-2 short (single-beat) rests — consistent with a breath
  mark at a phrase boundary, which is exactly what `phrases()` treats it
  as. `examples/chorale_benchmark.rs` moved to fixture format v3 (a
  `REST` token in the `soprano:` block); `tools/music21_chorale_extractor.py`
  no longer skips a rest-containing chorale, emitting `REST` events
  instead. A chorale is only "covered" if *every* phrase harmonizes —
  chosen so the number stays comparable to the pre-rest baselines rather
  than becoming a silently looser metric (a design point raised before
  implementing: see `tasks/lessons.md`).
  - Corpus grew from 144 to 182 chorales (+38, +26%) once the "soprano
    rest" exclusion was removed. Re-run: coverage 94.5% (172/182),
    statistically the same rate as the pre-rest 94.4% (136/144) but over
    a meaningfully larger population, **zero regressions** on the
    original 144 (directly diffed, not assumed).
  - A discrepancy surfaced while grounding the design in real data:
    Riemenschneider 327 ("Jesu, nun sei gepreiset") appeared to have 41
    rests when sampled positionally (`parts[0]`), but the actual
    extractor keys parts by name (`parts["Soprano"]`) — this piece has
    15 total parts (extra instrumental doublings), and its *named*
    Soprano part has zero rests, which is why it correctly appears
    rest-free in every baseline. A reminder that a quick investigative
    script and the real extractor can disagree if they don't select data
    the same way — see `tasks/lessons.md`.
  - Found a third instance of the known chordal-seventh/non-chord-tone
    gap (Riemenschneider 132, in addition to 40 and 202 from the
    v0.2.0 baseline) — only visible now because 132 has a soprano rest
    and was excluded entirely before this feature landed. Evidence the
    gap is a recurring pattern in the repertoire, not a one-off; still
    not attempted (see `tasks/todo.md`).
  - Full detail: `tasks/baseline-v0.3.0-soprano-rest.md`, `BENCHMARK.md`.
- Voice-range investigation (roadmap phase 5): root-caused all 5 of the
  baseline's "voice range" failures to the same cause — a soprano note
  on A5, one step above `VoicePart::Soprano`'s old default ceiling (G5).
  Unlike the generated inner voices (which are only ever offered pitches
  `pitches_in_range` already filtered to their range), the soprano pitch
  is taken directly from the input melody at every position, so a single
  out-of-range soprano note rejects *every* candidate there and kills the
  whole harmonization — not a near-miss worth a tolerance/warning, an
  outright ceiling-too-low bug once real material was tried. Widened to
  A5, justified by this real data rather than picked arbitrarily.

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
