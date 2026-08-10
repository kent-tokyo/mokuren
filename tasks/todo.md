# Open issues

Things that are known-incomplete, explicitly deferred, or blocked — not a
duplicate of README's "Current limitations" (which documents shipped
behavior's known gaps for a *user*), this is the working list for
*continuing development*. Update as items resolve; don't let this drift
from PLAN.md/ROADMAP.md/BENCHMARK.md, which remain the source of truth for
scope and phasing — this is the flat, scannable version.

## Verification-first phase: baseline done, next feature picked from data

- ~~Chorale corpus source~~ — decided 2026-08-10: **music21** (Margaret
  Greentree's explicit permission for Bach-chorale distribution as part
  of music21 specifically — see BENCHMARK.md), referenced via
  `tools/music21_chorale_extractor.py` against a local install, never
  vendored into this repository.
- ~~Benchmark harness~~ — done: `examples/chorale_benchmark.rs`, fixture
  format v2 (duration-aware), computes all 7 BENCHMARK.md metrics plus
  the secondary note-match one, percentile distributions (median/p90/p95)
  not just means, and a `FailureCategory` classifier (chromatic-soprano
  check, beam-width retry, rule-conflict bisection via
  `CandidateGenerator::generate()` on the shortest failing prefix).
- ~~music21 extraction adapter~~ — done: `tools/music21_chorale_extractor.py`.
- ~~Run the full major-mode baseline~~ — done 2026-08-10: 144/371 chorales
  attemptable, 73 succeeded (50.7% coverage), 0 hard-rule violations.
  Full results, failure taxonomy, and per-chorale table:
  `tasks/baseline-v0.1.0.md`. Summary and reproduction steps: BENCHMARK.md.
- ~~Secondary dominants (roadmap phase 2)~~ — implemented: `RomanNumeral::applied_to`,
  the standard V/x, V7/x set for x in {ii, iii, IV, V, vi}, and
  `SecondaryDominantResolutionRule` (hard rule requiring correct
  resolution). See PLAN.md and `tasks/lessons.md` for the scoring trap
  found and fixed along the way (a naive reward broke the pinned v0.1
  spine-melody demo, which has no chromatic notes at all).
- ~~Investigate the "voice range" rule-conflict cluster (roadmap phase 5)~~
  — root-caused and fixed: all 5 failing chorales had a soprano note on
  A5, above the old default soprano ceiling (G5); widened to A5
  (`src/voice.rs`). See ROADMAP.md phase 5 for detail.
- ~~Re-run and verify the full baseline against secondary dominants +
  soprano-range fix~~ — done 2026-08-11: coverage 50.7% → 91.7%
  (73 → 132/144), 0 hard-rule violations maintained. Regression-checked
  per-chorale against `tasks/baseline-v0.1.0.md`: 4 chorales that used to
  succeed at width 32 don't anymore (vocabulary roughly doubling means
  more beam-slot competition), but all 4 were individually confirmed to
  still succeed at a wider beam (2 at width 64, 2 at width 512) — not a
  new structural failure, the same known beam-width trade-off. Full
  result: `tasks/baseline-v0.2.0-secondary-dominants.md`, summarized in
  BENCHMARK.md.
- **Next, per this baseline's own data**: minor mode is now roadmap
  phase 3, and the cadential-6/4 lookahead stays deprioritized — 0 of
  the original 144 baseline failures traced to that rule. Also open:
  75/371 chorales (20.2%) excluded because `Melody` can't represent a
  soprano rest (roadmap phase 4, comparable in size to minor mode's
  exclusion count of 143/371, 38.5%); and the 6 chorales still failing
  as `Other`/undiagnosed even at width 512 in the new baseline — not yet
  individually root-caused, likely a mix of chromatic tones outside the
  implemented V/x, V7/x set (see README limitation #6) and possibly
  other new interactions. Worth a quick per-chorale look (same bisection
  technique used for the voice-range cluster) before starting minor mode,
  since it's cheap and might reveal another small, high-value fix like
  the soprano-range one.

## Real correctness gaps (tracked in more detail in README "Current limitations")

- Secondary dominants are narrower than full applied-chord theory: no
  applied leading-tone chords (vii°/x), no chained tonicization, strict
  (no inner-voice-exception) resolution. Roadmap phase 2 follow-up, not
  currently scheduled — the 6 chorales still failing as `Other` in the
  v0.2.0-in-progress baseline are candidates for needing this, but not
  yet individually confirmed (see the item above).
- `Melody` has no `Rest` variant despite `Rest` existing as a type in
  `melody.rs` — 20.2% of the full chorale corpus has a soprano rest and
  can't even be attempted. Roadmap phase 4 (new, surfaced by the baseline).
- Minor mode isn't just "add a Mode variant" — `RomanNumeral`'s chord
  qualities are currently hardcoded consts assuming major
  (`RomanNumeral::I` = `MajorTriad`, always). Minor mode requires deciding
  whether quality becomes key/mode-derived instead of intrinsic to
  `RomanNumeral` — an API shape decision. Roadmap phase 3 (sequenced
  after secondary dominants — see above).
- Six-four handling is backward-looking only (pedal/passing bass check),
  can't confirm a true cadential 6/4 resolves to V. Fixing this properly
  needs the rule engine to see forward context, which it currently doesn't
  — an architecture decision (how rules get lookahead), not a mechanical
  fix. Roadmap phase 6 (moved down: 0 baseline failures traced to this).
- `LeadingToneResolutionRule`'s inner-voice exception is unconditional
  (alto/tenor may always skip down a step/third to complete the chord);
  real pedagogy only sanctions this when resolving up would leave the
  chord incomplete or force bad doubling. Not yet re-scoped to check that
  condition.
- Score weights (cadence bonus, harmonic-function transition table, voice
  leading/melodic motion rewards) are hand-tuned against one melody only.
  The baseline now measures behavior against 144 real chorales, but
  doesn't retune the weights against them — still open, and shouldn't be
  hand-tuned further without re-running the benchmark to check the effect.

## Smaller / lower priority

- `why()` at position 0 is thin (no previous chord means most soft rules
  contribute no `Reason`). Not wrong, just a weaker demo than mid-progression
  positions — see README limitation #8.
- Beam search's horizon effect (`BeamSearch::new()` defaults to width 32
  specifically because narrower widths pruned away the eventually-best
  path before `CadenceSupportRule`'s end-loaded reward could matter) means
  wider or more harmonically ambiguous melodies may need an explicit wider
  beam. No auto-scaling by melody length exists. Got measurably worse when
  the applied-dominant vocabulary roughly doubled candidates per position
  (see `tasks/lessons.md`) — fixed there by correcting the score, not by
  raising the default, but worth re-checking after minor mode adds another
  vocabulary jump.
- `criterion` numbers in README are a one-off (one machine, one commit),
  not tracked over time. Roadmap doesn't schedule this until after the
  verification-first phase.
