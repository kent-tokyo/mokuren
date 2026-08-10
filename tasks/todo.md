# Open issues

Things that are known-incomplete, explicitly deferred, or blocked — not a
duplicate of README's "Current limitations" (which documents shipped
behavior's known gaps for a *user*), this is the working list for
*continuing development*. Update as items resolve; don't let this drift
from PLAN.md/ROADMAP.md/BENCHMARK.md, which remain the source of truth for
scope and phasing — this is the flat, scannable version.

## Blocking the verification-first phase

- **Chorale corpus: specific source not yet chosen.** Decision on *approach*
  is made (2026-08-10: "Reference, don't vendor" — see BENCHMARK.md). Still
  open: which specific source (music21 corpus / craigsapp / jthickstun /
  CCARH direct, or something else) the benchmark harness points at by
  default. This is the one remaining blocker on running the benchmark
  against real data — everything else for roadmap phase 1 is ready.
- ~~Benchmark harness~~ — done: `examples/chorale_benchmark.rs`, a simple
  `.chorale` fixture format (mokuren has no Humdrum `**kern`/MusicXML
  reader yet — that's roadmap phase 5, paused until this benchmark runs),
  computes all 7 BENCHMARK.md metrics plus the secondary note-match one,
  verified against synthetic smoke-test fixtures in
  `examples/chorale_benchmark_fixtures/`. Not yet done: full distributions
  (percentiles) rather than means for the numeric metrics.

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
  positions — see README limitation #4.
- Beam search's horizon effect (`BeamSearch::new()` defaults to width 32
  specifically because narrower widths pruned away the eventually-best
  path before `CadenceSupportRule`'s end-loaded reward could matter) means
  wider or more harmonically ambiguous melodies may need an explicit wider
  beam. No auto-scaling by melody length exists.
- `criterion` numbers in README are a one-off (one machine, one commit),
  not tracked over time. Roadmap doesn't schedule this until after the
  verification-first phase.
