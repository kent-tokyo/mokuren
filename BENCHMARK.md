# External chorale benchmark — protocol manifest

This is the "fix the protocol before running it" document requested alongside the ROADMAP.md landscape update. It fixes *what* the benchmark measures and *what corpus it's allowed to touch*. No chorale data is vendored into this repository — see [Corpus source](#corpus-source-approach-decided-specific-source-still-open) below.

**Status**: the harness (`examples/chorale_benchmark.rs`) and the music21 extraction adapter (`tools/music21_chorale_extractor.py`) are implemented and validated five times now — [v0.1.0 baseline](#v010-baseline-full-major-mode-subset-2026-08-10) (144 chorales), [v0.2.0-in-progress baseline](#v020-in-progress-baseline-secondary-dominants--soprano-range-fix-2026-08-11) (same 144), [v0.3.0-in-progress baseline](#v030-in-progress-baseline-soprano-rest-phrase-splitting-2026-08-11) (expanded to 182 chorales), [v0.4.0-in-progress baseline](#v040-in-progress-baseline-minor-mode-2026-08-11) (expanded to 348, major + minor), and [v0.5.0-in-progress baseline](#v050-in-progress-baseline-minor-applied-dominants--melodic-minor-2026-08-11) (same 348, minor coverage 42.8% → 64.5%) — establishing the "benchmark → failure decomposition → next feature → re-benchmark" loop this project now runs on.

Fixture format is v3 (rest-aware; v1 forced every note to a quarter, silently discarding real chorale rhythm; v2 couldn't represent a rest at all — see `tasks/lessons.md`). Full spec is documented in `examples/chorale_benchmark.rs`'s module doc comment; `cargo doc --open` or read the file directly.

## Purpose

Measure whether mokuren's reasoning generalizes to melodies it was never tuned against. README limitation #4 is explicit: every score weight was hand-adjusted against **one** melody (the AGENTS.md section-1 spine). Adding theory scope (minor mode, secondary dominants, more output formats) on top of an unvalidated weight set doesn't reduce that risk, it just gives the untested weights more surface area. This benchmark is how that risk gets retired — or doesn't, which is itself the useful outcome — and, per the first validation run below, already has: it found a real coverage gap in under an hour.

## Explicit non-goal

**Matching the original Bach setting note-for-note is not the target metric.** AGENTS.md section 24 says this directly: "原曲と一致すること = 正しい、とは考えないでください." Multiple SATB realizations can be equally valid for the same soprano line — Bach's own choice reflects one 18th-century composer's taste on one day, not the unique correct harmonization. Note-match is tracked as a *secondary, diagnostic* signal only (e.g. "how often does mokuren's #1-ranked choice coincide with Bach's," which is interesting context, not a pass/fail bar) — never the headline number.

## Metrics (no single aggregate score)

A single "accuracy" number would hide exactly the information this benchmark exists to produce. Track these separately, per chorale and aggregated:

| Metric | What it measures |
|---|---|
| **Coverage** | Fraction of chorale soprano lines mokuren can harmonize at all under current scope (major mode only until roadmap phase 2 lands) without `NoValidHarmonization`. |
| **Hard-rule violation rate** | Should be ~0 by construction (hard constraints reject candidates from the search), but track it — a nonzero rate here is a bug, not a quality signal. |
| **Cadence handling** | Per-chorale: does the final position's `Reason::CadenceSupport` classify as `Authentic`/`Plagal`/`Half`/`Deceptive`/`None`, and does that match what the actual soprano scale degree makes reachable (see the README limitation about melodies whose last note can't support an authentic cadence)? |
| **Voice-leading cost** | `EvaluatedCandidate::voice_leading_cost` summed over the winning path — lower is smoother, and this is directly comparable to music-comp-mt's "total movement" metric for the head-to-head phase. |
| **Search failure rate** | How often beam search returns `Err` (width too narrow, or genuinely no valid path) vs. succeeds — and at what beam width failures stop happening. |
| **Runtime** | Wall-clock per chorale at a fixed beam width, building on `benches/harmonize.rs`'s existing melody-length-scaling numbers with real (not synthetic-repeated) melodies. |
| **Explanation completeness** | Fraction of positions where `why()` returns at least one `Reason` (README limitation #6 already documents position 0 as thin — this quantifies exactly how thin, across a real corpus) and fraction of positions where `why_not()` resolves successfully for at least one valid alternative. |
| *(secondary)* **Original-note match** | Rate at which mokuren's selected voicing's pitch classes coincide with the original chorale's alto/tenor/bass — diagnostic only, never the headline metric, per the non-goal above. |

## Methodology

Implemented in `examples/chorale_benchmark.rs`:

1. For each chorale in the working subset: extract the soprano line (pitch + duration) and key; discard alto/tenor/bass (they're the ground truth for the secondary note-match metric only, not an input) unless supplying them for that metric.
2. Run `Composer::new().key(...).style(Style::CommonPractice).harmonize(soprano)`.
3. Record all seven metrics above from the `HarmonizationResult` (`Decision`s, timing).
4. Aggregate per-chorale results and print a report (mean voice-leading cost/runtime/explanation coverage, coverage and search-failure rates, `why_not()` success rate, a full cadence-type distribution rather than one number, and a per-chorale breakdown for spotting outliers).

Not yet implemented: reporting full distributions (percentiles, not just means) for the numeric metrics, and meter — the fixture format and harness currently carry the meter field through but no rule consumes it yet, matching v0.1's own scope (mokuren doesn't yet reason about meter/phrase position beyond "is this the final position").

## First validation run (not the baseline)

**Superseded by the full baseline below** — kept for history. A 20-chorale pipeline check (not a sampled subset, just iteration order) found 50% coverage and diagnosed one failure (Riemenschneider 2, a G natural not diatonic in A major) to a chromatic soprano tone. That prediction — "chromatic content is common enough that the real baseline may land well below 100%" — is what the full run below confirms.

## v0.1.0 baseline (full major-mode subset, 2026-08-10)

Full results, provenance, failure taxonomy, and the per-chorale table: [`tasks/baseline-v0.1.0.md`](tasks/baseline-v0.1.0.md). Summary:

- **144/371 Riemenschneider chorales were even attemptable** (38.8%) — 227 excluded before harmonization was ever tried: 143 minor mode (38.5% of the full corpus), 75 because the soprano has a rest mokuren's `Melody` can't represent (20.2% — a comparably large gap to minor mode, and a data-model limitation rather than a theory one), 9 other (missing part / unrepresentable duration / an ATB gap at a soprano onset).
- **Of the 144 attempted, 73 harmonized successfully (50.7% coverage)**, 0 hard-rule violations (the invariant holds).
- **Failures are not one bucket**: 63 chromatic-soprano (88.7% of failures — a pitch class outside the key's diatonic scale, most likely secondary dominants and/or chromatic non-chord tones in real chorale writing), 5 voice-range rule conflicts, 2 search-exhausted (a wider beam finds a path), 1 chordal-seventh-resolution conflict.
- **Beam width is confirmed not the bottleneck**: widening from 32 to 256 only recovers 2 chorales (50.7% → 52.1%).
- **Zero failures trace to the cadential-6/4 rule specifically** — the earlier phase order (6/4 lookahead *before* secondary dominants) isn't supported by this data. See ROADMAP.md's reordering.
- Voice-leading cost: median 7.20/position (p90 7.99, p95 8.06). Runtime: median 1.4s/chorale at width 32 (p90 2.1s, p95 2.3s). Explanation completeness: `why()` 97.8%, `why_not()` 100%. Cadence: 61.6% authentic, 20.5% plagal, 17.8% none; 82.2% end on a tonic-function chord.

Reproduce: `python3 tools/music21_chorale_extractor.py -o <dir>` then `cargo run --release --example chorale_benchmark -- <dir> --report tasks/baseline-v0.1.0.md`. Extracted `.chorale` files are never committed (deleted after each run here, per "reference, don't vendor" below) — only the report (titles, catalog numbers, mokuren's own computed statistics — no encoded musical content) is.

## v0.2.0-in-progress baseline (secondary dominants + soprano-range fix, 2026-08-11)

Same 144-chorale corpus, re-extracted. Full results and per-chorale table: [`tasks/baseline-v0.2.0-secondary-dominants.md`](tasks/baseline-v0.2.0-secondary-dominants.md). Summary:

- **Coverage rose from 50.7% (73/144) to 91.7% (132/144)** — +41.0pt absolute, +81% relative — from implementing applied dominants (`RomanNumeral::applied_to`, ROADMAP.md phase 2: the standard V/x, V7/x set for x in {ii, iii, IV, V, vi}) and fixing the soprano-range ceiling (ROADMAP.md phase 5: the v0.1.0 baseline's 5 "voice range" failures all traced to a soprano note on A5, one step above the old default ceiling of G5). 0 hard-rule violations, same invariant as v0.1.0.
- **Regression-checked, not just improved**: diffing per-chorale against the v0.1.0 baseline found 4 chorales that used to succeed at width 32 and don't anymore — the applied-dominant vocabulary roughly doubling candidates per position means more competition for the same fixed beam width (the horizon effect, README limitation #4, measurably worsened — see `tasks/lessons.md`). All 4 were directly verified to still succeed at a wider beam (2 recover at width 64, 2 at width 512), so this is the existing, documented beam-width trade-off, not a new correctness problem. The default width (32) was deliberately left unchanged rather than raised to cover the width-512 cases, which would make a rare case expensive for everyone.
- **Remaining 12 failures**: 6 search-exhausted (a wider beam finds a path — includes the 4 regressions above, verified, plus 2 more), 6 still `Other` (undiagnosed even at width 512) — not yet individually root-caused.
- Runtime roughly 1.8x the v0.1.0 baseline's median (1.4s → 2.6s/chorale at width 32), consistent with the vocabulary roughly doubling — matches the advisor-style estimate made before running it, not a surprise.
- Voice-leading cost, cadence distribution, and explanation completeness are all in the same range as v0.1.0 (median cost 7.33 vs 7.20; 68.9% authentic vs 61.6%; `why()` 98.0% vs 97.8%) — the new vocabulary didn't degrade the quality of what it *does* produce, only expanded what it covers.

**Bisecting the 6 `Other` chorales found two more real bugs, raising coverage to 94.4% (136/144)** — full detail: [`tasks/baseline-v0.2.0-secondary-dominants.md`](tasks/baseline-v0.2.0-secondary-dominants.md) (this file has been updated in place to the newer 94.4% measurement; the 91.7% numbers above are kept as the first-measurement historical record). Summary:

- The bisection tool itself had a bug (harmonizing a *truncated* melody made the truncation point look artificially final to the search, wrongly triggering final-position-only rules): fixed by replaying the full melody's search up to the real failure point instead of re-harmonizing a prefix (`replay_to_failure` in `examples/chorale_benchmark.rs`).
- **3 of 6** chorales (Riemenschneider 102, 173, 327) shared one real fix: `SecondaryDominantResolutionRule` required an applied dominant to resolve at the *very next* position unconditionally, so a chromatic tone held/repeated across two consecutive notes (common — a tied or reiterated note before the actual resolution) had nowhere valid to go on its second occurrence. Fixed: prolonging the *same* applied dominant across a repeat no longer counts as an unresolved dangling dominant.
- **2 of 6** (Riemenschneider 40, 202) remain unfixed — a fixed soprano note forced into a formal chordal-seventh role that must resolve down by step, but the real melody leaps a third instead. Very likely Bach using the note as a decorative non-chord tone (passing tone), which mokuren has no model for at all — a real, larger feature gap (see `tasks/todo.md`), not attempted this pass.
- **1 of 6** (Riemenschneider 234) turned out to be beam-width-recoverable (not structural) once the harness's own retry ladder was widened to 512.
- Regression check re-verified at 94.4%: only 2 chorales (Riemenschneider 135, 230) now differ from v0.1.0's coverage, both confirmed beam-width-recoverable, same conclusion as before.

## v0.3.0-in-progress baseline (soprano-rest phrase splitting, 2026-08-11)

Re-extracted corpus (same music21 install/version) with the extractor's "soprano contains a rest" exclusion removed — a rest is now written into the fixture as a `REST` event instead of causing the whole chorale to be skipped. Full results and per-chorale table: [`tasks/baseline-v0.3.0-soprano-rest.md`](tasks/baseline-v0.3.0-soprano-rest.md). Summary:

- `Melody`/`Composer::harmonize` are unchanged — still a plain, rest-free `Vec<Note>`. A new `MelodyLine` type (`src/melody.rs`) can hold `Note`/`Rest` events; its `phrases()` splits at each rest into independent contiguous note runs (a rest-free line always yields exactly one phrase, unchanged from before). The harness harmonizes each phrase independently and only counts a chorale as covered if *every* phrase harmonizes.
- **Corpus grew from 144 to 182 attemptable chorales** (+38, +26%) — the "soprano rest" exclusion bucket (75 chorales in the v0.2.0 baseline) is gone; only genuine data gaps (unrepresentable duration, missing part, ATB gap) remain besides minor mode.
- **Coverage: 94.5% (172/182)** — statistically the same rate as v0.2.0's 94.4% (136/144), but over a meaningfully larger population. **Zero regressions**: every one of the 136 previously-covered chorales is still covered (directly diffed, not assumed).
- **10 failures**, same taxonomy as v0.2.0: 7 search-exhausted (wider beam recovers all 7, at widths 64–512 — includes the same 2 chorales, 135 and 230, flagged beam-width-recoverable in the v0.2.0 regression check), 3 rule-conflict (chordal seventh resolution) — the same unfixed non-chord-tone gap (Riemenschneider 40, 202), plus a **third instance newly visible** (Riemenschneider 132, previously excluded for a rest) — direct evidence the gap is a recurring pattern, not a one-off.
- Voice-leading cost, cadence distribution (69.2% authentic vs 69.9%), and explanation completeness (`why()` 96.9% vs 98.0% — expected: new phrase-opening positions behave like position 0, README limitation #8) are all in the same range as v0.2.0. Runtime is *lower* despite the larger corpus (median ~1.4s vs 2.9s/chorale, on a different run/machine-load than the v0.2.0 measurement) — plausibly phrase-splitting (the 38 multi-phrase chorales never carry a full-length beam in one pass), but the single-phrase majority did identical work to v0.2.0, so this isn't fully attributed; not investigated further since it's a beneficial direction, not a regression.

Reproduce: `python3 tools/music21_chorale_extractor.py -o <dir>` then `cargo run --release --example chorale_benchmark -- <dir> --report tasks/baseline-v0.3.0-soprano-rest.md`.

## v0.4.0-in-progress baseline (minor mode, 2026-08-11)

Re-extracted corpus (same music21 install/version) with the extractor's `key.mode != "major"` filter removed — a `mode: major|minor` fixture field is emitted instead. Full results and per-chorale table: [`tasks/baseline-v0.4.0-minor-mode.md`](tasks/baseline-v0.4.0-minor-mode.md). Summary:

- **Corpus grew from 182 to 348 chorales** (+166 minor-mode chorales, +91%) — every Riemenschneider chorale music21 resolves to major or minor is now attemptable.
- **Major: 172/182 (94.5%)**, unchanged from the pre-minor baseline, zero regressions directly confirmed — the `NumeralSource` refactor (`RomanNumeral::applied_to: Option<ScaleDegree>` became an enum distinguishing `Diatonic`/`AppliedDominant`/`HarmonicMinorRaisedSeventh`, which minor mode's own chromatic layer needed) didn't touch major's behavior.
- **Minor: 71/166 (42.8%)** — a genuinely lower first-pass number, predicted in writing before the run (advisor review): minor mode has no applied dominants yet and no melodic minor (raised 6th), so a soprano tone needing either has no chord at all in the current vocabulary. 77% of minor's 105 failures are exactly that (`chromatic soprano unsupported`). A bisected sample of the smaller rule-conflict categories (voice-overlap, leading-tone-resolution) traced to the same root cause — too few candidate chords/voicings at the dominant scale degree, not a distinct bug.
- Design stayed narrower than full minor-key theory on purpose (same scoping secondary dominants used): `Mode::Minor` is natural minor; the harmonic-minor-derived V/V7/vii° are an additional chromatic vocabulary layer, not a `Key` redesign. `vii°7` (fully diminished seventh) deliberately excluded — its chordal seventh sits on the lowered 6th, the exact scale degree `ChordalSeventhResolutionRule` already produces failures on, which would make a new failure ambiguous between "minor is wrong" and "the seventh rule is too strict."
- 0 hard-rule violations maintained across all 348 chorales.

## v0.5.0-in-progress baseline (minor applied dominants + melodic minor, 2026-08-11)

Re-prioritized ahead of adaptive/search-budget research per an explicit user directive: minor's dominant failure mode was *no candidate exists at all*, not *search missed a candidate that exists*, so vocabulary had the bigger lever. Full results: [`tasks/baseline-v0.5.0-minor-applied-dominants.md`](tasks/baseline-v0.5.0-minor-applied-dominants.md). Summary:

- **Vocabulary chosen from real corpus evidence, not copied from major.** New `--minor-gap-report` CLI mode (`examples/chorale_benchmark.rs`) classified every unreachable minor-key chromatic soprano tone in the v0.4.0 failures: 79 chorales needed V(7)/ii, 68 needed V(7)/V, 65 needed the melodic-minor raised 6th, 16 needed V(7)/IV, 1 needed V(7)/vi, 0 needed V(7)/iii — 100% classified, zero unexplained. V/III was excluded from the implementation on that basis. Raised 6th, originally deferred as "melodic minor" to a later phase, was pulled forward into this same pass since only 16/81 chorales would have been fully resolved by applied dominants alone, vs. 65/81 needing the raised 6th too.
- **Major: 172/182 (94.5%), zero regressions** (directly diffed). **Minor: 42.8% → 64.5%** (71/166 → 107/166), +36 net. 0 hard-rule violations maintained throughout.
- 18 minor chorales that were covered before this pass are not covered now — all 18 confirmed beam-width-recoverable (the harness directly retested each at width 64–512, not assumed), the same horizon-effect pattern from when major's own applied-dominant vocabulary first landed. Default beam width (32) intentionally unchanged.
- Mechanism: both additions reuse the "same root, different quality" trick harmonic minor's V/V7 already established (verified by hand before implementing) — no new root-spelling logic needed for either.

## Phasing

Major-mode subset first (done above, roadmap phase 1) was exactly right: it needed zero new theory work and immediately produced the finding that reorders everything after it. Minor-mode chorales fold in once roadmap phase 3 lands — expect the chromatic-soprano failure *rate* among them to hold or worsen (minor keys have their own chromatic tendency tones), same reasoning that put secondary-dominant/chromatic support ahead of minor mode in the phase order.

## Corpus candidates and what's verified about each

Researched 2026-08-10 via each source's own documentation — not assumed. All fall into the same shape identified before starting this: **the underlying compositions are public domain (Bach, d. 1750); the specific digital encodings are a separate question with real, source-by-source variation.**

| Source | What it is | What's verified about licensing |
|---|---|---|
| [music21 corpus](https://github.com/cuthbertLab/music21/tree/master/music21/corpus/bach) (`music21.corpus.chorales`, contributed by Margaret Greentree) | Bach chorales bundled with the music21 Python package | music21's own docs state the corpus encodings carry licenses **separate from** music21's BSD code license — "distributed with the permission of the encoders and where permitted under United States copyright law," with some encodings restricted from commercial use. Must be checked per-work, not assumed blanket-permissive. |
| [craigsapp/bach-370-chorales](https://github.com/craigsapp/bach-370-chorales) (Humdrum `**kern`, GitHub) | 370 four-part chorales, from the Breitkopf & Härtel 4th edition (c. 1875, ed. Alfred Dörffel) — that edition has 371 chorales total, one of which isn't four-part, hence 370 here | GitHub reports this repo's license as "Other (NOASSERTION)" — detected license-like text that doesn't match a standard SPDX license. The README itself states no explicit terms. Hosted by the source's original cataloger (Craig Sapp, CCARH), so likely traceable to CCARH's terms below, but not confirmed in-repo. |
| [jthickstun/bach-371-chorales](https://github.com/jthickstun/bach-371-chorales) (Humdrum, GitHub) | Same underlying 371-chorale edition, different filtering (371 vs 370 — the discrepancy is exactly the one-non-four-part-chorale difference above, not two different datasets) | No license file at all (GitHub reports `license: null`). Default copyright applies absent an explicit grant; treat as **not clearly redistributable** without contacting the maintainer. |
| [CCARH kernScores](https://kern.humdrum.org/) (original host, Center for Computer Assisted Research in the Humanities, Stanford) | The canonical source both GitHub mirrors above ultimately derive from | Explicitly stated: "available without cost to non-profit institutions, though **commercial use is prohibited**." This is a real constraint if mokuren or a downstream user of it is ever used commercially. |

**The 370/371 discrepancy some sources quote isn't two different datasets** — it's the same Breitkopf & Härtel/Dörffel edition (371 chorales), with 370 being the four-part-only subset. Whichever source is adopted, cite the edition and specify which count is meant.

## Corpus source: approach decided, specific source still open

None of the four candidates above give a clean, unambiguous "vendor this into mokuren's public repository" answer — every one either restricts commercial use, lacks an explicit license, or requires per-work verification. No chorale data has been downloaded or committed.

**Decided (2026-08-10): Reference, don't vendor.** The benchmark harness reads chorale data at run time from a source the person *running* the benchmark supplies locally (e.g. their own music21 install, or a locally cloned kernScores mirror) — mokuren's repository never ships any of it, so mokuren's own license stays clean regardless of the source's terms. The alternative (requesting explicit permission from CCARH to vendor a subset) was not chosen.

**Still open, and tracked in `tasks/todo.md` rather than blocking further work**: which specific source the harness points at by default, and the exact intermediate file format it reads (mokuren has no Humdrum `**kern` or MusicXML parser yet — that's roadmap phase 5, itself paused until this benchmark runs, so the harness needs a simpler interchange format in the meantime rather than waiting on a parser). This is an engineering/format decision, not a licensing one, so it doesn't need the same sign-off the vendor-vs-reference choice did.
