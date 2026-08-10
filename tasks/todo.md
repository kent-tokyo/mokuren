# Open issues

Things that are known-incomplete, explicitly deferred, or blocked — not a
duplicate of README's "Current limitations" (which documents shipped
behavior's known gaps for a *user*), this is the working list for
*continuing development*. Update as items resolve; don't let this drift
from PLAN.md/ROADMAP.md/BENCHMARK.md, which remain the source of truth for
scope and phasing — this is the flat, scannable version.

## Blocking the verification-first phase

- ~~Chorale corpus source~~ — decided 2026-08-10: **music21** (Margaret
  Greentree's explicit permission for Bach-chorale distribution as part
  of music21 specifically — see BENCHMARK.md), referenced via
  `tools/music21_chorale_extractor.py` against a local install, never
  vendored into this repository.
- ~~Benchmark harness~~ — done: `examples/chorale_benchmark.rs`, fixture
  format v2 (duration-aware; v1 forced every note to a quarter, silently
  discarding real chorale rhythm), computes all 7 BENCHMARK.md metrics
  plus the secondary note-match one. Validated against both synthetic
  smoke fixtures and 20 real chorales extracted from music21. Not yet
  done: full distributions (percentiles) rather than means for the
  numeric metrics.
- ~~music21 extraction adapter~~ — done: `tools/music21_chorale_extractor.py`,
  samples alto/tenor/bass at soprano onsets only (no independent ATB
  onset grid, so harmonic rhythm can't leak into the input), skips
  chorales with a soprano rest or an unrepresentable duration rather
  than approximating, writes a `manifest.json` with source/version/
  numbering/selected IDs/file hashes for reproducibility.
- **Remaining**: run the actual major-mode baseline (not the 20-chorale
  validation sample already done — see BENCHMARK.md's "First validation
  run"). The validation run found 50% coverage on that small sample, with
  the one diagnosed failure caused by a non-diatonic (chromatic) soprano
  tone mokuren's diatonic-only engine can't harmonize — expect the real
  baseline number to be materially below 100%, which is useful
  information, not a bug to fix before measuring.

## Real correctness gaps (tracked in more detail in README "Current limitations")

- Six-four handling is backward-looking only (pedal/passing bass check),
  can't confirm a true cadential 6/4 resolves to V. Fixing this properly
  needs the rule engine to see forward context, which it currently doesn't
  — an architecture decision (how rules get lookahead), not a mechanical
  fix. Roadmap phase 3.
- `LeadingToneResolutionRule`'s inner-voice exception is unconditional
  (alto/tenor may always skip down a step/third to complete the chord);
  real pedagogy only sanctions this when resolving up would leave the
  chord incomplete or force bad doubling. Not yet re-scoped to check that
  condition.
- Minor mode isn't just "add a Mode variant" — `RomanNumeral`'s chord
  qualities are currently hardcoded consts assuming major
  (`RomanNumeral::I` = `MajorTriad`, always). Minor mode requires deciding
  whether quality becomes key/mode-derived instead of intrinsic to
  `RomanNumeral` — an API shape decision. Roadmap phase 2.
- Score weights (cadence bonus, harmonic-function transition table, voice
  leading/melodic motion rewards) are hand-tuned against one melody only.
  This is the whole reason the verification-first phase exists — don't
  hand-tune further without a benchmark to check against, or the same
  problem just gets re-created with more parameters.

## Smaller / lower priority

- `why()` at position 0 is thin (no previous chord means most soft rules
  contribute no `Reason`). Not wrong, just a weaker demo than mid-progression
  positions — see README limitation #6.
- Beam search's horizon effect (`BeamSearch::new()` defaults to width 32
  specifically because narrower widths pruned away the eventually-best
  path before `CadenceSupportRule`'s end-loaded reward could matter) means
  wider or more harmonically ambiguous melodies may need an explicit wider
  beam. No auto-scaling by melody length exists.
- `criterion` numbers in README are a one-off (one machine, one commit),
  not tracked over time. Roadmap doesn't schedule this until after the
  verification-first phase.
