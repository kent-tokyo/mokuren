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
- **Next, per the baseline's own data** (ROADMAP.md "Verification-first
  phase" was reordered by this): secondary dominants / chromatic
  non-chord tones (88.7% of failures, 63/144 fixtures) before minor mode,
  and the cadential-6/4 lookahead moved *down* the list — 0 of the 144
  failures traced to that rule. Also newly surfaced, not on the original
  list: 75/371 chorales (20.2%) were excluded because `Melody` can't
  represent a soprano rest — worth scoping as its own item (roadmap
  phase 4), comparable in size to minor mode's exclusion count (143/371,
  38.5%).

## Real correctness gaps (tracked in more detail in README "Current limitations")

- Chromatic soprano tones (secondary dominants, chromatic non-chord
  tones) are unsupported — the dominant baseline failure cause (see
  above). Roadmap phase 2 (moved up from phase 4).
- `Melody` has no `Rest` variant despite `Rest` existing as a type in
  `melody.rs` — 20.2% of the full chorale corpus has a soprano rest and
  can't even be attempted. Roadmap phase 4 (new, surfaced by the baseline).
- 5/144 baseline fixtures failed on a "voice range" rule conflict —
  smaller than the chromatic-soprano cluster but not yet root-caused;
  likely a soprano note at or past `VoicePart::Soprano`'s hardcoded
  default range edge. Roadmap phase 5.
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
  positions — see README limitation #7.
- Beam search's horizon effect (`BeamSearch::new()` defaults to width 32
  specifically because narrower widths pruned away the eventually-best
  path before `CadenceSupportRule`'s end-loaded reward could matter) means
  wider or more harmonically ambiguous melodies may need an explicit wider
  beam. No auto-scaling by melody length exists.
- `criterion` numbers in README are a one-off (one machine, one commit),
  not tracked over time. Roadmap doesn't schedule this until after the
  verification-first phase.
