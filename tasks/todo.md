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
  succeed at width 32 don't anymore, all 4 individually confirmed to
  still succeed at a wider beam — not a new structural failure.
- ~~Bisect all 6 chorales still failing as `Other` at width 512~~ — done
  2026-08-11. First fixed a real bug in the bisection tool itself
  (truncating a melody for diagnosis made the truncation point look
  artificially final, wrongly triggering `SecondaryDominantResolutionRule`'s
  final-position rejection — fixed via `replay_to_failure` in
  `examples/chorale_benchmark.rs`, which replays the *full* melody's
  search up to the real failure point instead). With that fixed:
  - **3/6** (Riemenschneider 102, 173, 327): same root cause — an
    applied dominant's chromatic tone held/repeated across two notes
    before resolving had nowhere to go on the second occurrence, since
    `SecondaryDominantResolutionRule` required resolution at the *very
    next* position unconditionally. Fixed: prolonging the same applied
    dominant across a repeat no longer counts as unresolved. This raised
    coverage from 91.7% to **94.4% (136/144)**.
  - **2/6** (Riemenschneider 40, 202): NOT fixed — a real, larger gap.
    The soprano is forced into a formal chordal-seventh role requiring
    step-down resolution, but the real melody leaps a third — almost
    certainly a non-chord (passing) tone, which mokuren has no model for
    at all. See "Real correctness gaps" below; not attempted this pass.
  - **1/6** (Riemenschneider 234): turned out to be beam-width-recoverable
    (not structural) once the harness's own retry ladder was widened to
    512.
  - Full table and per-chorale detail: `tasks/baseline-v0.2.0-secondary-dominants.md`.
  Regression-checked again at 94.4%: only 2 chorales (135, 230) differ
  from v0.1.0, both confirmed beam-width-recoverable.
- **Next, per the user's approved order (2026-08-11)**: ① secondary
  dominants + bisection — done, above. ② soprano-rest support in
  `Melody` (roadmap phase 4) — next, since 20.2% of the *full* corpus
  is excluded before harmonization is even attempted, a bigger and more
  local win for the verification base than minor mode. ③ minor +
  harmonic minor (roadmap phase 3). ④ adaptive/search-budget research —
  see the width-curve item below. ⑤ cadential-6/4 lookahead (roadmap
  phase 6, still deprioritized — 0 baseline failures traced to it).
- **Requested but not yet done**: a width-vs-coverage/runtime curve
  (32/64/128/256/512) across the *full* 144-chorale corpus (not just the
  failure subset the existing beam-width curve already covers), to
  inform whether an adaptive-retry search strategy (`harmonize(32)` →
  retry wider only on `NoValidHarmonization`) is worth the engineering
  cost before building it. Also requested: refine `FailureCategory`
  into something like `StructuralFailure` / `SearchBudgetFailure` /
  `UnsupportedVocabulary` / `InputRepresentationFailure` — a clearer
  taxonomy now that `SearchExhausted` vs genuine rule conflicts are
  reliably distinguished (post bisection-tool fix). Neither is urgent;
  do alongside or after soprano-rest support, not before it.

## Real correctness gaps (tracked in more detail in README "Current limitations")

- **Non-chord tones (passing/neighbor tones) aren't modeled at all** —
  every soprano note must be a full chord tone; there's no way for a
  quick, unaccented note to sit *outside* the current harmony. Newly
  surfaced by bisecting Riemenschneider 40 and 202 (2026-08-11): both
  have a soprano note that's the formal chordal seventh of an applied
  dominant seventh, which `ChordalSeventhResolutionRule` requires to
  resolve down by step — but the real melody leaps a third instead,
  consistent with the note being a decorative passing tone rather than
  a real chordal seventh. Not yet scoped as a roadmap phase; a genuinely
  different kind of model extension (soprano notes that don't constrain
  the chord) from secondary dominants (more chords available).
- Secondary dominants are narrower than full applied-chord theory: no
  applied leading-tone chords (vii°/x), no chained tonicization
  (encountered directly: Riemenschneider 234 needed two different
  applied dominants back-to-back with no clean intermediate resolution
  — currently only recoverable by a wider beam finding an alternate,
  non-chromatic path, not by the rule itself permitting the chain),
  strict (no inner-voice-exception) resolution. Roadmap phase 2
  follow-up, not currently scheduled.
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
