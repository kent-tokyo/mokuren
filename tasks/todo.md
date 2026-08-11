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
- ~~Soprano-rest support in `Melody` (roadmap phase 4)~~ — done
  2026-08-11: `Melody`/`Composer::harmonize` unchanged (still a plain,
  rest-free `Vec<Note>`); new `MelodyLine` type (`src/melody.rs`, holding
  `Note`/`Rest` events) with `phrases()` splitting at each rest into
  independent contiguous note runs, harmonized separately. Design
  grounded in real music21 data before implementing (most of the 75
  rest-excluded chorales have only 1-2 short rests — consistent with a
  breath-mark phrase boundary). `examples/chorale_benchmark.rs` moved to
  fixture format v3 (`REST` token); `tools/music21_chorale_extractor.py`
  no longer skips a rest-containing chorale. A chorale only counts as
  covered if *every* phrase harmonizes. Corpus grew 144 → 182 (+38,
  +26%); re-run coverage 94.5% (172/182), zero regressions on the
  original 144. Surfaced a third instance of the chordal-seventh/
  non-chord-tone gap (Riemenschneider 132, see "Real correctness gaps"
  below). Full detail: `tasks/baseline-v0.3.0-soprano-rest.md`.
- ~~Minor + harmonic minor (roadmap phase 3)~~ — done 2026-08-11:
  `Mode::Minor` (natural minor) plus the harmonic-minor-derived V/V7/vii°
  as an additional chromatic vocabulary layer (`NumeralSource` enum
  replacing the old `applied_to: Option<ScaleDegree>` field). Found and
  fixed a real bug before shipping: `LeadingToneResolutionRule`/
  `LeadingToneDoublingRule` weren't recognizing the *raised* leading tone
  in minor at all (`Key::functional_leading_tone()` fixes this). Corpus
  grew 182 → 348 (+166 minor chorales); coverage: major unchanged 94.5%
  (172/182, zero regressions), minor 42.8% (71/166) — a first-pass
  number, expected to be well below major's since minor has no applied
  dominants yet and no melodic minor. Deliberately deferred: `vii°7`,
  applied dominants in minor keys, melodic minor (see "Real correctness
  gaps" below). Full detail: `tasks/baseline-v0.4.0-minor-mode.md`.
- ~~Re-prioritized 2026-08-11 (user directive): minor applied
  dominants~~ — **done 2026-08-11**. `--minor-gap-report` (new CLI mode)
  classified every minor-key unreachable chromatic soprano tone from
  the v0.4.0 failures before implementing anything: 79 chorales needed
  V(7)/ii, 68 V(7)/V, 65 the melodic-minor raised 6th, 16 V(7)/IV, 1
  V(7)/vi, 0 V(7)/iii — 100% classified. V/III excluded from the
  implementation (zero evidence); melodic minor's raised 6th (ii, IV
  alternates) pulled forward from its own later phase into this same
  pass, since applied dominants alone would only have fully resolved
  16/81 chorales vs. 65/81 needing the raised 6th too.
  - Success bar (stated numerically in advance) fully met: major
    unchanged 172/182 (94.5%, zero regressions, directly diffed), 0
    hard-rule violations maintained, minor 42.8% → 64.5% (71/166 →
    107/166, +36 net, per-chorale diffed). 18 minor chorales regressed
    from the vocabulary roughly doubling again — all 18 confirmed
    beam-width-recoverable (not assumed), the same horizon-effect
    pattern applied dominants first produced for major.
  - 64.5% lands just under the "65–75% real win" bar, not the "~80% big
    hit" scale secondary dominants produced for major — a real but more
    modest first-pass result.
  - Full detail: `tasks/baseline-v0.5.0-minor-applied-dominants.md`.
- **Next**: re-taxonomize what's still failing in minor (45
  search-exhausted, 7 chordal-seventh-resolution, 6
  secondary-dominant-resolution — new, not yet investigated, minor's
  applied dominants hitting a resolution edge case major's corpus never
  exercised — 11 voice-overlap) before picking the next phase. Original
  order after minor applied dominants: adaptive/search-budget research,
  then cadential-6/4 lookahead — not yet confirmed still the right next
  step given the new secondary-dominant-resolution category.
- **Requested but not yet done**: a width-vs-coverage/runtime curve
  (32/64/128/256/512) across the *full* 182-chorale corpus (not just the
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
  a real chordal seventh. A third instance (Riemenschneider 132) surfaced
  once soprano-rest support widened the corpus (2026-08-11) — it was
  previously excluded entirely, so this wasn't a new occurrence, just a
  newly-visible one; three independent chorales hitting the same gap is
  evidence it's a recurring pattern, not a one-off. Not yet scoped as a
  roadmap phase; a genuinely different kind of model extension (soprano
  notes that don't constrain the chord) from secondary dominants (more
  chords available).
- Secondary dominants are narrower than full applied-chord theory: no
  applied leading-tone chords (vii°/x), no chained tonicization
  (encountered directly: Riemenschneider 234 needed two different
  applied dominants back-to-back with no clean intermediate resolution
  — currently only recoverable by a wider beam finding an alternate,
  non-chromatic path, not by the rule itself permitting the chain),
  strict (no inner-voice-exception) resolution. Roadmap phase 2
  follow-up, not currently scheduled.
- ~~Minor mode isn't just "add a Mode variant"~~ — resolved 2026-08-11,
  see above; ~~"minor has no applied dominants yet"~~ and ~~"no melodic
  minor"~~ below are also resolved (2026-08-11, see the minor applied
  dominants entry above) — kept as a pointer since real gaps remain:
  - Melodic minor's raised 6th is only offered via alternate ii/IV, not
    a general convention — no other chord uses it, and descending
    motion (which reverts to natural minor) isn't distinguished from
    ascending. See README's "Current limitations" item 7.
  - **`vii°7`** (the fully diminished seventh on the raised leading
    tone) isn't in the vocabulary — its chordal seventh sits on the
    *lowered* 6th, the same scale degree `ChordalSeventhResolutionRule`
    already produces failures on, so it was deliberately left out
    rather than risk an ambiguous new failure.
  - New, surfaced by the v0.5.0 re-run: 6 minor chorales fail with
    `rule conflict (secondary dominant resolution)` — minor's new
    applied dominants hitting a resolution edge case major's corpus
    never exercised. Not bisected yet.
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
