# External chorale benchmark — protocol manifest

This is the "fix the protocol before running it" document requested alongside the ROADMAP.md landscape update. It fixes *what* the benchmark measures and *what corpus it's allowed to touch*. No chorale data is vendored into this repository — see [Corpus source](#corpus-source-approach-decided-specific-source-still-open) below.

**Status**: the harness (`examples/chorale_benchmark.rs`) and the music21 extraction adapter (`tools/music21_chorale_extractor.py`) are both implemented and validated end to end against 20 real chorales extracted from a local music21 install — see [First validation run](#first-validation-run-not-the-baseline) below for what that surfaced. The full major-mode baseline (all major-mode chorales, not a 20-chorale sample) is still a deliberate next step, not done as a side effect of this validation pass.

Fixture format is v2 (duration-aware; v1 forced every note to a quarter, silently discarding real chorale rhythm — see `tasks/lessons.md`). Full spec is documented in `examples/chorale_benchmark.rs`'s module doc comment; `cargo doc --open` or read the file directly.

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

Ran `tools/music21_chorale_extractor.py --limit 20` against a local music21 install (v9.9.1) and fed the output straight to `examples/chorale_benchmark.rs`, purely to confirm the pipeline works end to end against real data before treating it as ready for the actual major-mode baseline. It's not that baseline — 20 chorales, not the full major-mode subset, and picked by iteration order, not deliberately sampled. Extracted output was not committed (per "reference, don't vendor" — see below); only what it revealed is recorded here.

**Coverage was 50% (10/20)**, and diagnosing one failure precisely (bisecting the soprano line to the shortest failing prefix, then checking `Diagnostics`) found a specific, expected cause: Riemenschneider 2 ("Ich dank' dir, lieber Herre," A major) fails to harmonize starting at its 6th soprano note, a G natural — which is not a diatonic tone in A major (only G# is; A major's diatonic scale has no plain G). No hard-rule combination is at fault and no beam width fixes it (checked up to 512) — mokuren's engine is diatonic-only by design (AGENTS.md section 5), so a chromatic tone (most likely a secondary dominant — G natural is exactly what an applied A7 resolving to IV=D major would need — or a chromatic passing/neighbor tone) has no diatonic chord that contains it, in any key.

This is the roadmap's "secondary dominants" phase (4) validated as mattering before being built, from real data rather than from reading AGENTS.md section 20 and guessing it would matter eventually. It's also a caution about magnitude: half of a 20-chorale sample failing entirely (not "harmonized poorly" — zero output) suggests chromatic content is common enough in real chorale writing that the major-mode baseline, once run properly, may show a coverage number well below 100% even before minor-mode chorales are considered. That's exactly the kind of number this benchmark exists to produce — not a reason to route around it by only testing chorales known to be fully diatonic.

## Phasing

Start with the **major-mode subset** of whatever corpus is adopted (see below) — this is exactly what v0.1 can measure *today*, with zero new theory work, and is roadmap phase 1. Minor-mode chorales get folded in once roadmap phase 2 (minor mode) lands, which also directly demonstrates that phase's benchmark impact rather than just "the code compiles and unit tests pass."

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
