# mokuren v0.4.0-in-progress chorale baseline (minor mode)

Same music21 install, re-extracted to include minor-mode chorales — the extractor no longer filters `key.mode != "major"`, emitting a `mode:` fixture field instead (fixture format v3, unchanged from soprano-rest support). Full per-chorale table below. Summary:

**Corpus grew from 182 to 348 chorales** (+166 minor-mode chorales; +91%) — the entire major+minor Riemenschneider corpus is now attemptable except for genuine data gaps (unrepresentable duration, missing part, ATB gap, or a mode music21 doesn't resolve to major/minor).

**Combined coverage: 69.8% (243/348)**, 0 hard-rule violations maintained. Split by mode (computed by cross-referencing each fixture's `mode:` field against the covered list, not estimated):

- **Major: 172/182 (94.5%)** — identical to the pre-minor-mode baseline (`tasks/baseline-v0.3.0-soprano-rest.md`), confirmed by direct diff: zero regressions from the `NumeralSource` refactor (`applied_to: Option<ScaleDegree>` → an enum distinguishing `Diatonic`/`AppliedDominant`/`HarmonicMinorRaisedSeventh`) that minor mode required.
- **Minor: 71/166 (42.8%)** — first-pass minor-mode coverage, well below major's, and expected to be: predicted in writing *before* this run (per an advisor review) that minor would land well below major because two real gaps are still open — no applied dominants in minor keys yet (only major has the escape hatch of tonicizing a foreign degree), and no melodic minor (the raised 6th), so any soprano note needing either has no chord at all.

**Failure taxonomy for the 105 minor failures**: 81 chromatic-soprano-unsupported (77.1% of failures — confirms the predicted gap above), 16 search-exhausted (wider beam recovers), 8 rule-conflict (3 chordal-seventh-resolution — same known non-chord-tone gap as major; 4 voice-overlap, 1 leading-tone-resolution — bisected one directly (Riemenschneider 367, B minor): a soprano F#4 following ii°6 has only 16 reachable candidates (i, III, natural v, harmonic V, harmonic V7 — every numeral whose chord contains F#), and *all* of them fail voice-overlap against the fixed previous voicing at every beam width up to 512. Root cause is the same missing-vocabulary gap as the chromatic-soprano-unsupported majority, not a distinct bug: minor's thin dominant-area vocabulary (3 numerals vs major's much larger applied-dominant-augmented set) leaves too few voicing choices at exactly the scale degree most cadences pass through. Not attempted as a fix this pass — implementing applied dominants for minor keys is the natural next increment, tracked in `tasks/todo.md`.

**Design, scoped narrower than full minor-key theory on purpose** (mirrors how secondary dominants were scoped): `Mode::Minor` is natural minor (matching the key signature); the harmonic-minor-derived V/V7/vii° are offered as an additional chromatic vocabulary layer alongside the seven natural-minor diatonic triads (`RomanNumeral::harmonic_minor_vocabulary`), not a redesign of `Key`. `vii°7` (fully diminished seventh) is deliberately not included in this first pass — its chordal seventh sits on the *lowered* 6th, and `ChordalSeventhResolutionRule` is the exact rule already producing 3 of the unfixed failures, so adding it in the same pass would make any new failure ambiguous between "minor mode is wrong" and "the seventh rule is too strict." `Key::functional_leading_tone()` is used by `LeadingToneResolutionRule`/`LeadingToneDoublingRule` so they check the pitch that's actually chromatically active (the *raised* 7th in a minor key, not natural minor's own unraised one, which doesn't function as a leading tone at all). `HarmonicFunctionProgressionRule`'s degree→function table now also considers quality at degree 7, since a major-triad VII (natural minor's subtonic) isn't the same chord as a diminished vii° and shouldn't be scored as a dominant-function arrival.

Reproduce: `python3 tools/music21_chorale_extractor.py -o <dir>` then `cargo run --release --example chorale_benchmark -- <dir> --report tasks/baseline-v0.4.0-minor-mode.md`.

# Chorale benchmark report

## Provenance

- mokuren version: 0.1.0
- git commit: 3ab54b62d9519f2083353f03d878d0fa21945f61
- music21 version: 9.9.1
- corpus: 348 chorale(s) extracted, 23 skipped at extraction time
  - exclusion reasons (23 total):
    - ATB gap at a soprano onset: 10
    - unrepresentable duration: 8
    - missing part: 5
- chorales measured here: 348 (513 phrase(s) total, after splitting at rests)
- standard beam width: 32 (retry widths for failure classification: [64, 128, 256, 512])

## Coverage

- Coverage: 243/348 (69.8%)
- Search failure rate: 105/348 (30.2%)

- 64 chorale(s) had a soprano rest and were split into multiple phrases (harmonized independently, each via the same `Composer::harmonize` a rest-free chorale uses); "covered" above requires *every* phrase of a chorale to harmonize, so this is comparable to the pre-rest-support baselines, not a looser per-phrase number.

## Failure taxonomy (not lumped into one bucket)

- chromatic soprano unsupported: 81 (23.3% of all fixtures, 77.1% of failures)
- rule conflict: chordal seventh resolution: 3 (0.9% of all fixtures, 2.9% of failures)
- rule conflict: leading-tone resolution: 1 (0.3% of all fixtures, 1.0% of failures)
- rule conflict: voice overlap: 4 (1.1% of all fixtures, 3.8% of failures)
- search exhausted (wider beam works): 16 (4.6% of all fixtures, 15.2% of failures)

### Beam-width coverage curve (failures only — successes at width 32 aren't retried)

- width   32: 243/348 (69.8%)
- width   64: 253/348 (72.7%)
- width  128: 253/348 (72.7%)
- width  256: 256/348 (73.6%)
- width  512: 259/348 (74.4%)

## Hard-rule violations

0 (should always be 0 by construction — a nonzero count is a bug, not a quality signal)

## Voice-leading cost

Per-chorale average (cost / position): median 7.20, p90 7.93, p95 8.19

## Runtime

Per chorale (ms, summed across phrases): median 2728.2, p90 4795.1, p95 7010.8

## Explanation completeness

- why() coverage: 96.9% of positions have at least one Reason
- why_not() success: 11768/11768 (100.0%) of positions with a valid alternative

## Cadence

(the chorale's *last phrase*'s cadence — for a multi-phrase chorale this is the piece's actual final cadence, not an average across phrase-internal cadences)

Final-cadence distribution:
- authentic: 162 (66.7%)
- deceptive: 2 (0.8%)
- half: 6 (2.5%)
- none: 27 (11.1%)
- plagal: 46 (18.9%)

Ends on a tonic-function chord (proxy for "the close is at least plausible," not full cadence-correctness verification): 211/243 (86.8%)

## Original-note match (secondary, diagnostic only — see BENCHMARK.md's non-goal)

24.7% (pooled across phrases) over 243 fixture(s) with a reference ATB

## Per-chorale

| Chorale | Result | Phrases | Voice-leading cost | Cadence | Runtime (ms) |
|---|---|---|---|---|---|
| Nun bitten wir den heiligen Geist (Riemenschneider 36) | covered | 2 | 393 | authentic | 2819.7 |
| O Ewigkeit, du Donnerwort (Riemenschneider 26) | covered | 1 | 326 | authentic | 2584.9 |
| Danket dem Herren, denn er ist sehr freundlich (Riemenschneider 228) | covered | 1 | 146 | authentic | 710.6 |
| Jesu, meiner Seelen Wonne (Riemenschneider 350) | covered | 1 | 353 | none | 2644.1 |
| Es wird schier der letzte Tag herkommen (Riemenschneider 238) | covered | 1 | 270 | authentic | 1068.0 |
| Befiehl du deine Wege (Riemenschneider 340) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Was frag’ ich nach der Welt (Riemenschneider 291) | covered | 1 | 426 | authentic | 3296.4 |
| Wo soll ich fliehen hin (Riemenschneider 281) | covered | 1 | 277 | plagal | 1315.2 |
| Nun freut euch, Gottes Kinder all’ (Riemenschneider 185) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Von Gott will ich nicht lassen (Riemenschneider 332) | covered | 1 | 441 | authentic | 1739.7 |
| Wenn mein Stündlein vorhanden ist (Riemenschneider 322) | covered | 1 | 450 | authentic | 3799.4 |
| Lobt Gott, ihr Christen, allzugleich (Riemenschneider 54) | covered | 2 | 317 | plagal | 2380.8 |
| Mach’s mit mir, Gott, nach deiner Güt’ (Riemenschneider 44) | covered | 1 | 222 | authentic | 1882.1 |
| Ach Gott, wie manches Herzeleid (Riemenschneider 217) | covered | 1 | 249 | authentic | 2289.1 |
| Des Heil’gen Geistes reiche Gnad’ (Riemenschneider 207) | covered | 1 | 210 | none | 968.1 |
| Schaut, ihr Sünder (Riemenschneider 171) | covered | 5 | 244 | authentic | 957.7 |
| Ermuntre dich, mein schwacher Geist (Riemenschneider 9) | covered | 1 | 341 | authentic | 2835.0 |
| Ich hab’ mein’ Sach’ Gott heimgestellt (Riemenschneider 19) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Christus, der uns selig macht (Riemenschneider 113) | covered | 1 | 480 | authentic | 1600.2 |
| Nun ruhen alle Wälder (Riemenschneider 103) | covered | 1 | 401 | plagal | 2800.0 |
| O Welt, sieh’ hier dein Leben (Riemenschneider 275) | covered | 1 | 395 | plagal | 2628.1 |
| Was mein Gott will, das g’scheh’ allzeit (Riemenschneider 265) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Ach Gott, vom Himmel sieh’ darein (Riemenschneider 253) | NOT COVERED | 3 | — | — | — (phrase 2/3: chromatic soprano unsupported) |
| Jesu, du mein liebstes Leben (Riemenschneider 243) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Gott der Vater wohn’ uns bei (Riemenschneider 135) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 512)) |
| Allein Gott in der Höh’ sei Ehr’ (Riemenschneider 125) | covered | 1 | 338 | authentic | 5192.7 |
| Wo Gott zum Haus nicht gibt sein’ Gunst (Riemenschneider 157) | covered | 1 | 290 | authentic | 7033.5 |
| Wenn ich in Angst und Not (Riemenschneider 147) | covered | 5 | 294 | authentic | 9135.0 |
| Die Nacht ist kommen (Riemenschneider 231) | covered | 6 | 244 | authentic | 7563.1 |
| Ich hab’ in Gottes Herz und Sinn (Riemenschneider 349) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Herr, straf’ mich nicht in deinem Zorn (Riemenschneider 221) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Allein zu dir, Herr Jesu Christ (Riemenschneider 359) | covered | 6 | 441 | authentic | 6232.4 |
| Gelobet seist du, Jesu Christ (Riemenschneider 288) | covered | 1 | 306 | none | 3808.4 |
| Weg, mein Herz, mit den Gedanken (Riemenschneider 298) | covered | 1 | 346 | authentic | 3690.3 |
| Jesu, meine Freude (Riemenschneider 96) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Erhalt’ uns, Herr, bei deinem Wort (Riemenschneider 72) | covered | 1 | 261 | authentic | 1414.2 |
| Wer nur den lieben Gott läßt walten (Riemenschneider 62) | covered | 2 | 215 | authentic | 1100.7 |
| Das alte Jahr vergangen ist (Riemenschneider 314) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Auf meinen lieben Gott (Riemenschneider 304) | covered | 1 | 271 | authentic | 1581.5 |
| O Welt, sieh’ hier dein Leben (Riemenschneider 366) | covered | 2 | 408 | authentic | 2884.8 |
| Heut’ ist, o Mensch, ein großer Trauertag (Riemenschneider 168) | covered | 3 | 204 | plagal | 1120.0 |
| Aus tiefer Not schrei’ ich zu dir (Riemenschneider 10) | covered | 1 | 393 | half | 1307.4 |
| Das neugeborne Kindelein (Riemenschneider 178) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Wie bist du, Seele, in mir so gar betrübt (Riemenschneider 242) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Jesu, nun sei gepreiset (Riemenschneider 252) | covered | 1 | 568 | authentic | 7488.3 |
| Auf, auf, mein Herz, und du, mein ganzer Sinn (Riemenschneider 124) | covered | 1 | 272 | authentic | 1729.5 |
| Du, o schönes Weltgebäude (Riemenschneider 134) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Meinen Jesum laß ich nicht (Riemenschneider 299) | covered | 1 | 436 | authentic | 4073.6 |
| Nun ruhen alle Wälder (Riemenschneider 289) | covered | 1 | 407 | plagal | 3428.6 |
| Nun bitten wir den heiligen Geist (Riemenschneider 97) | covered | 1 | 450 | authentic | 3514.8 |
| Wer nur den lieben Gott läßt walten (Riemenschneider 146) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Ach Gott, wie manches Herzeleid (Riemenschneider 156) | covered | 1 | 272 | half | 1697.0 |
| Sollt’ ich meinem Gott nicht singen (Riemenschneider 220) | covered | 1 | 480 | deceptive | 4486.7 |
| Meinen Jesum laß’ ich nicht, weil (Riemenschneider 348) | covered | 1 | 326 | authentic | 3086.1 |
| Christ, der du bist der helle Tag (Riemenschneider 230) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Nun ruhen alle Wälder (Riemenschneider 63) | covered | 1 | 391 | plagal | 2599.4 |
| Herr Jesu Christ, du höchstes Gut (Riemenschneider 73) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| O Gott, du frommer Gott (Riemenschneider 315) | covered | 1 | 350 | none | 2838.3 |
| Befiehl du deine Wege (Riemenschneider 367) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| Aus meines Herzens Grunde (Riemenschneider 1) | covered | 1 | 328 | authentic | 2728.2 |
| Wachet auf, ruft uns die Stimme (Riemenschneider 179) | covered | 5 | 443 | authentic | 3551.4 |
| Jesu, der du selbst so wohl (Riemenschneider 169) | covered | 1 | 405 | plagal | 3214.3 |
| Es ist das Heil uns kommen her (Riemenschneider 290) | covered | 1 | 330 | authentic | 4639.0 |
| Es spricht der Unweisen Mund (Riemenschneider 27) | covered | 1 | 339 | authentic | 7587.5 |
| Jesu, der du meine Seele (Riemenschneider 37) | covered | 1 | 358 | none | 4625.4 |
| Ich dank’ dir, lieber Herre (Riemenschneider 341) | covered | 1 | 555 | authentic | 10093.1 |
| Den Vater dort oben (Riemenschneider 239) | covered | 1 | 350 | authentic | 9439.5 |
| Wenn mein Stündlein vorhanden ist (Riemenschneider 351) | covered | 1 | 452 | authentic | 8419.0 |
| Ich danke dir, o Gott, in deinem Throne (Riemenschneider 229) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Wie schön leuchtet der Morgenstern (Riemenschneider 323) | covered | 2 | 386 | authentic | 9244.9 |
| Es woll’ uns Gott genädig sein (Riemenschneider 333) | covered | 1 | 284 | authentic | 6655.6 |
| Kommt her zu mir, spricht Gottes Sohn (Riemenschneider 45) | covered | 1 | 432 | authentic | 4785.2 |
| Wir Christenleut’ (Riemenschneider 55) | covered | 1 | 273 | plagal | 3320.2 |
| Christ lag in Todesbanden (Riemenschneider 184) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| So gibst du nun, mein Jesu, gute Nacht (Riemenschneider 206) | NOT COVERED | 4 | — | — | — (phrase 4/4: chromatic soprano unsupported) |
| Es ist genug, so nimm, Herr, meinen Geist (Riemenschneider 216) | covered | 8 | 380 | none | 6557.3 |
| Gottes Sohn ist kommen (Riemenschneider 18) | covered | 1 | 361 | authentic | 4719.8 |
| Gelobet seist du, Jesu Christ (Riemenschneider 160) | covered | 1 | 292 | none | 3097.9 |
| Freuet euch, ihr Christen alle (Riemenschneider 8) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Nun komm, der Heiden Heiland (Riemenschneider 170) | covered | 1 | 240 | authentic | 1220.0 |
| Ermuntre dich, mein schwacher Geist (Riemenschneider 102) | covered | 1 | 369 | authentic | 3649.1 |
| Wer nur den lieben Gott läßt walten (Riemenschneider 112) | covered | 2 | 205 | authentic | 1021.4 |
| Jesu, meines Herzens Freud’ (Riemenschneider 264) | covered | 1 | 402 | none | 4279.8 |
| O Ewigkeit, du Donnerwort (Riemenschneider 274) | covered | 1 | 316 | authentic | 3571.8 |
| Werde munter, mein Gemüte (Riemenschneider 95) | covered | 1 | 311 | plagal | 3387.9 |
| O Gott, du frommer Gott (Riemenschneider 85) | covered | 6 | 383 | authentic | 3289.8 |
| Die Sonn’ hat sich mit ihrem Glanz gewendet (Riemenschneider 232) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Nun preiset alle Gottes Barmherzigkeit (Riemenschneider 222) | covered | 1 | 318 | plagal | 3003.7 |
| Der du bist drei in Einigkeit (Riemenschneider 154) | covered | 1 | 225 | plagal | 2671.3 |
| Wer in dem Schutz des Höchsten ist (Riemenschneider 144) | covered | 5 | 249 | authentic | 2654.9 |
| Herr Jesu Christ, dich zu uns wend’ (Riemenschneider 136) | covered | 1 | 256 | authentic | 2465.6 |
| Durch Adams Fall ist ganz verderbt (Riemenschneider 126) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Liebster Jesu, wir sind hier (Riemenschneider 328) | covered | 1 | 254 | plagal | 2138.0 |
| Ein’ feste Burg ist unser Gott (Riemenschneider 250) | covered | 1 | 425 | plagal | 3102.1 |
| Jesus, meine Zuversicht (Riemenschneider 338) | covered | 1 | 349 | authentic | 2935.5 |
| Nun sich der Tag geendet hat (Riemenschneider 240) | NOT COVERED | 2 | — | — | — (phrase 1/2: search exhausted (works at width 64)) |
| Allein zu dir, Herr Jesu Christ (Riemenschneider 13) | NOT COVERED | 2 | — | — | — (phrase 2/2: search exhausted (works at width 64)) |
| Ach Gott, vom Himmel sieh’ darein (Riemenschneider 3) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Jesu, meiner Seelen Wonne (Riemenschneider 365) | covered | 1 | 304 | authentic | 2648.0 |
| Herr, wie du willst, so schick’s mit mir (Riemenschneider 317) | covered | 1 | 362 | authentic | 4044.2 |
| Christus, der uns selig macht (Riemenschneider 307) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Ich ruf’ zu dir, Herr Jesu Christ (Riemenschneider 71) | covered | 1 | 450 | plagal | 2671.1 |
| Singen wir aus Herzensgrund (Riemenschneider 109) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Jesu Leiden, Pein und Tod (Riemenschneider 61) | covered | 1 | 377 | authentic | 4134.4 |
| Christ, unser Herr, zum Jordan kam (Riemenschneider 119) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| O Traurigkeit, o Herzeleid (Riemenschneider 57) | covered | 1 | 221 | authentic | 970.8 |
| Vater unser im Himmelreich (Riemenschneider 47) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Wo soll ich fliehen hin (Riemenschneider 331) | covered | 1 | 286 | plagal | 1338.3 |
| Allein Gott in der Höh’ sei Ehr’ (Riemenschneider 249) | covered | 1 | 320 | authentic | 3437.8 |
| Wir Christenleut’ (Riemenschneider 321) | covered | 1 | 269 | authentic | 1332.7 |
| Verleih’ uns Frieden gnädiglich (Riemenschneider 259) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 256)) |
| Da der Herr Christ zu Tische saß (Riemenschneider 196) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Ach Gott, erhör’ mein Seufzen (Riemenschneider 186) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| Nimm von uns, Herr, du treuer Gott (Riemenschneider 292) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Freu’ dich sehr, o meine Seele (Riemenschneider 282) | covered | 1 | 362 | authentic | 3474.3 |
| Der Herr ist mein getreuer Hirt (Riemenschneider 353) | covered | 1 | 276 | authentic | 2763.0 |
| Nun lieget alles unter dir (Riemenschneider 343) | covered | 1 | 335 | none | 3203.9 |
| Gott des Himmels und der Erden (Riemenschneider 35) | covered | 1 | 268 | authentic | 2152.7 |
| Wo soll ich fliehen hin (Riemenschneider 25) | covered | 3 | 296 | plagal | 1416.0 |
| Lobt Gott, ihr Christen allzugleich (Riemenschneider 276) | covered | 2 | 281 | plagal | 2961.1 |
| Herr Jesu Christ, du höchstes Gut (Riemenschneider 266) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Wenn wir in höchsten Nöten sein (Riemenschneider 68) | covered | 2 | 231 | authentic | 2832.3 |
| Vater unser im Himmelreich (Riemenschneider 110) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Herzliebster Jesu, was hast du verbrochen (Riemenschneider 78) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Durch Adams Fall ist ganz verderbt (Riemenschneider 100) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Sei gegrüßet, Jesu gütig (Riemenschneider 172) | covered | 1 | 337 | authentic | 2331.8 |
| Das alte Jahr vergangen ist (Riemenschneider 162) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Mitten wir im Leben sind (Riemenschneider 214) | covered | 6 | 662 | half | 2548.8 |
| Wer weiß, wie nahe mir mein Ende (Riemenschneider 204) | covered | 1 | 250 | plagal | 1173.3 |
| Komm, Gott Schöpfer, heiliger Geist (Riemenschneider 187) | covered | 1 | 278 | authentic | 2398.9 |
| Christ ist erstanden (Riemenschneider 197) | NOT COVERED | 2 | — | — | — (phrase 1/2: chromatic soprano unsupported) |
| Vom Himmel hoch, da komm’ ich her (Riemenschneider 46) | covered | 4 | 251 | authentic | 1925.5 |
| Christum wir sollen loben schon (Riemenschneider 56) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Mein’ Augen schließ’ ich jetzt in Gottes Namen zu (Riemenschneider 258) | covered | 7 | 333 | authentic | 2544.4 |
| Gott sei uns gnädig und barmherzig (Riemenschneider 320) | covered | 1 | 163 | none | 702.0 |
| Sei Lob und Ehr’ dem höchsten Gut (Riemenschneider 248) | covered | 1 | 326 | authentic | 2973.8 |
| Nun danket alle Gott (Riemenschneider 330) | covered | 1 | 272 | none | 2505.4 |
| Lobt Gott, ihr Christen, allzugleich (Riemenschneider 342) | covered | 1 | 326 | authentic | 2258.7 |
| Es woll’ uns Gott genädig sein (Riemenschneider 352) | NOT COVERED | 2 | — | — | — (phrase 2/2: rule conflict (voice overlap)) |
| Valet will ich dir geben (Riemenschneider 24) | covered | 1 | 326 | authentic | 2928.7 |
| Erbarm’ dich mein, o Herre Gott (Riemenschneider 34) | covered | 1 | 451 | plagal | 1774.6 |
| Was Gott tut, das ist wohlgetan (Riemenschneider 293) | covered | 1 | 284 | authentic | 2884.9 |
| Vater unser im Himmelreich (Riemenschneider 267) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Herzlich lieb hab’ ich dich, o Herr (Riemenschneider 277) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Herr Christ, der ein’ge Gott’s-Sohn (Riemenschneider 101) | covered | 1 | 293 | authentic | 2787.7 |
| Heut’ triumphieret Gottes Sohn (Riemenschneider 79) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Herzliebster Jesu, was hast du verbrochen (Riemenschneider 111) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Komm, heiliger Geist, Herre Gott (Riemenschneider 69) | covered | 5 | 710 | authentic | 8211.3 |
| Für Freuden laßt uns springen (Riemenschneider 163) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| O Herzensangst, o Bangigkeit (Riemenschneider 173) | covered | 1 | 310 | authentic | 3095.4 |
| Herr Gott, dich loben wir (Riemenschneider 205) | covered | 2 | 1373 | none | 17019.1 |
| Verleih’ uns Frieden gnädiglich (Riemenschneider 215) | covered | 2 | 728 | authentic | 3804.5 |
| Ich dank’ dir, Gott, für all’ Wohltat (Riemenschneider 223) | covered | 3 | 397 | authentic | 3924.1 |
| Werde munter, mein Gemüte (Riemenschneider 233) | covered | 1 | 308 | none | 2981.0 |
| Warum betrübst du dich, mein Herz (Riemenschneider 145) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Hilf, Herr Jesu, laß gelingen (Riemenschneider 155) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 512)) |
| Nun bitten wir den heiligen Geist (Riemenschneider 84) | covered | 1 | 448 | authentic | 3211.6 |
| Warum betrübst du dich, mein Herz (Riemenschneider 94) | NOT COVERED | 2 | — | — | — (phrase 1/2: chromatic soprano unsupported) |
| Dies sind die heil’gen zehn Gebot’ (Riemenschneider 127) | covered | 1 | 236 | half | 2462.6 |
| Wer Gott vertraut, hat wohl gebaut (Riemenschneider 137) | covered | 9 | 353 | authentic | 2956.4 |
| Was willst du dich, o meine Seele, kränken (Riemenschneider 241) | NOT COVERED | 7 | — | — | — (phrase 1/7: chromatic soprano unsupported) |
| Wer nur den lieben Gott läßt walten (Riemenschneider 339) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Ich bin ja, Herr, in deiner Macht (Riemenschneider 251) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Sei Lob und Ehr’ dem höchsten Gut (Riemenschneider 329) | covered | 1 | 338 | authentic | 2845.9 |
| Ich dank’ dir, lieber Herre (Riemenschneider 2) | covered | 1 | 425 | authentic | 2704.9 |
| Puer natus in Bethlehem (Riemenschneider 12) | covered | 1 | 243 | authentic | 857.9 |
| Von Gott will ich nicht lassen (Riemenschneider 364) | covered | 1 | 365 | authentic | 1200.3 |
| O Mensch, bewein’ dein’ Sünde groß (Riemenschneider 306) | covered | 1 | 590 | authentic | 4739.7 |
| In dich hab’ ich gehoffet, Herr (Riemenschneider 118) | covered | 1 | 371 | authentic | 3045.0 |
| Ich freue mich in dir (Riemenschneider 60) | covered | 3 | 279 | authentic | 2331.5 |
| Valet will ich dir geben (Riemenschneider 108) | covered | 1 | 312 | authentic | 2821.7 |
| Gott sei gelobet und gebenedeiet (Riemenschneider 70) | covered | 2 | 494 | authentic | 3919.4 |
| Christ lag in Todesbanden (Riemenschneider 371) | NOT COVERED | 2 | — | — | — (phrase 1/2: chromatic soprano unsupported) |
| O wie selig seid ihr doch, ihr Frommen (Riemenschneider 219) | covered | 1 | 318 | authentic | 1049.4 |
| Du Lebensfürst, Herr Jesu Christ (Riemenschneider 361) | covered | 1 | 341 | authentic | 2895.7 |
| Erschienen ist der herrliche Tag (Riemenschneider 17) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Nun lob’, mein’ Seel’, den Herren (Riemenschneider 7) | covered | 1 | 559 | authentic | 4625.0 |
| Das walt’ mein Gott (Riemenschneider 75) | covered | 2 | 227 | deceptive | 3636.5 |
| Was Gott tut, das ist wohlgetan (Riemenschneider 65) | covered | 1 | 332 | authentic | 2702.7 |
| Allein Gott in der Höh’ sei Ehr’ (Riemenschneider 313) | covered | 1 | 276 | authentic | 2764.8 |
| Herr Christ, der ein’ge Gott’ssohn (Riemenschneider 303) | covered | 1 | 265 | authentic | 2286.2 |
| Nun komm, der Heiden Heiland (Riemenschneider 28) | covered | 1 | 215 | plagal | 764.0 |
| In allen meinen Taten (Riemenschneider 140) | covered | 2 | 297 | authentic | 2522.1 |
| Straf’ mich nicht in deinem Zorn (Riemenschneider 38) | covered | 1 | 281 | authentic | 1940.4 |
| O Jesu, du mein Bräutigam (Riemenschneider 236) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Herr Jesu Christ, du hast bereit’t (Riemenschneider 226) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (leading-tone resolution)) |
| Verleih’ uns Frieden gnädiglich (Riemenschneider 91) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 256)) |
| Christus, der uns selig macht (Riemenschneider 81) | covered | 1 | 460 | authentic | 1647.3 |
| Weg, mein Herz, mit den Gedanken (Riemenschneider 254) | covered | 1 | 362 | authentic | 3172.2 |
| Jesu, Jesu, du bist mein (Riemenschneider 244) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Kyrie, Gott Vater in Ewigkeit (Riemenschneider 132) | NOT COVERED | 2 | — | — | — (phrase 1/2: rule conflict (chordal seventh resolution)) |
| Ist Gott mein Schild und Helfersmann (Riemenschneider 122) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Von Gott will ich nicht lassen (Riemenschneider 114) | covered | 1 | 325 | authentic | 1262.9 |
| Wer nur den lieben Gott läßt walten (Riemenschneider 104) | covered | 1 | 263 | authentic | 922.2 |
| Ich dank’ dir, lieber Herre (Riemenschneider 272) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| AAch Gott, vom Himmel sieh’ darein (Riemenschneider 262) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Hilf, Herr Jesu, laß gelingen (Riemenschneider 368) | NOT COVERED | 6 | — | — | — (phrase 5/6: search exhausted (works at width 64)) |
| Christus ist erstanden, hat überwunden (Riemenschneider 200) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Erstanden ist der heil’ge Christ (Riemenschneider 176) | covered | 1 | 243 | authentic | 2434.4 |
| Es steh’n vor Gottes Throne (Riemenschneider 166) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Gottlob, es geht nunmehr zu Ende (Riemenschneider 192) | covered | 1 | 198 | authentic | 2014.7 |
| Wär’ Gott nicht mit uns diese Zeit (Riemenschneider 182) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Es ist das Heil uns kommen her (Riemenschneider 335) | covered | 1 | 352 | authentic | 3067.4 |
| Mit Fried’ und Freud’ ich fahr’ dahin (Riemenschneider 325) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Das neugeborne Kindelein (Riemenschneider 53) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Nicht so traurig, nicht so sehr (Riemenschneider 149) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Ach lieben Christen, seid getrost (Riemenschneider 31) | covered | 1 | 356 | plagal | 1437.7 |
| Als der gütige Gott (Riemenschneider 159) | covered | 4 | 182 | authentic | 1558.3 |
| Herzlich tut mich verlangen (Riemenschneider 21) | covered | 1 | 295 | authentic | 1164.6 |
| Warum sollt’ ich mich denn grämen (Riemenschneider 357) | covered | 1 | 376 | authentic | 3176.3 |
| Was Gott tut, das ist wohlgetan (Riemenschneider 347) | covered | 1 | 364 | authentic | 2826.6 |
| Nun lob’, mein’ Seel’, den Herren (Riemenschneider 296) | covered | 1 | 633 | none | 5783.7 |
| Befiehl du deine Wege (Riemenschneider 286) | covered | 1 | 333 | authentic | 1038.9 |
| Helft mir Gott’s Güte preisen (Riemenschneider 88) | covered | 1 | 299 | authentic | 1377.3 |
| O Haupt voll Blut und Wunden (Riemenschneider 98) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| Herzliebster Jesu, was hast du verbrochen (Riemenschneider 105) | covered | 1 | 330 | plagal | 1023.4 |
| Was mein Gott will, das g’scheh’ allezeit (Riemenschneider 115) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Jesu, meine Freude (Riemenschneider 263) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Ein’ feste Burg ist unser Gott (Riemenschneider 273) | covered | 2 | 402 | authentic | 2927.1 |
| O Mensch, bewein’ dein’ Sünde groß (Riemenschneider 201) | covered | 1 | 590 | authentic | 4553.9 |
| Jesu, der du meine Seele (Riemenschneider 369) | covered | 1 | 364 | authentic | 1431.5 |
| Weltlich’ Ehr’ und zeitlich Gut (Riemenschneider 211) | covered | 3 | 395 | half | 3217.7 |
| Du großer Schmerzensmann (Riemenschneider 167) | covered | 6 | 277 | none | 3003.2 |
| Ach bleib bei uns, Herr Jesu Christ (Riemenschneider 177) | covered | 1 | 288 | authentic | 2186.3 |
| Jesu, meine Freude (Riemenschneider 324) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Vor deinen Thron tret’ ich hiermit (Riemenschneider 334) | covered | 1 | 191 | authentic | 1817.3 |
| Du Friedefürst, Herr Jesu Christ (Riemenschneider 42) | covered | 1 | 234 | authentic | 1703.4 |
| Wenn mein Stündlein vorhanden ist (Riemenschneider 52) | covered | 1 | 463 | plagal | 3432.6 |
| Nun freut euch, lieben Christen, g’mein (Riemenschneider 183) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 512)) |
| Was bist du doch, o Seele, so betrübet (Riemenschneider 193) | covered | 4 | 218 | authentic | 1080.3 |
| Herr, ich habe mißgehandelt (Riemenschneider 287) | covered | 1 | 262 | plagal | 1252.7 |
| Jesu, der du meine Seele (Riemenschneider 297) | covered | 1 | 438 | authentic | 2545.3 |
| Helft mir Gott’s Güte preisen (Riemenschneider 99) | covered | 1 | 323 | authentic | 1666.3 |
| O Haupt voll Blut und Wunden (Riemenschneider 89) | covered | 1 | 367 | authentic | 1604.1 |
| Ein’ feste Burg ist unser Gott (Riemenschneider 20) | covered | 1 | 405 | plagal | 3839.7 |
| Der Tag der ist so freudenreich (Riemenschneider 158) | covered | 1 | 486 | authentic | 5077.0 |
| Jesus Christus, unser Heiland (Riemenschneider 30) | covered | 2 | 296 | authentic | 1365.9 |
| Uns ist ein Kindlein heut’ gebor’n (Riemenschneider 148) | covered | 1 | 272 | authentic | 2379.0 |
| Meines Lebens letzte Zeit (Riemenschneider 346) | covered | 1 | 455 | authentic | 1826.0 |
| Jesu, meine Freude (Riemenschneider 356) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Wir Christenleut’ (Riemenschneider 360) | covered | 1 | 285 | authentic | 1379.6 |
| Laß, o Herr, dein Ohr sich neigen (Riemenschneider 218) | covered | 1 | 353 | authentic | 1376.7 |
| Kommt her zu mir, spricht Gottes Sohn (Riemenschneider 370) | covered | 1 | 426 | authentic | 1897.3 |
| Als vierzig Tag’ nach Ostern war (Riemenschneider 208) | covered | 1 | 346 | plagal | 3046.5 |
| Es woll’ uns Gott genädig sein (Riemenschneider 16) | covered | 2 | 524 | authentic | 1743.1 |
| Freu’ dich sehr, o meine Seele (Riemenschneider 64) | covered | 1 | 340 | authentic | 2785.1 |
| O Haupt voll Blut und Wunden (Riemenschneider 74) | covered | 1 | 355 | authentic | 1230.5 |
| Hilf, Gott, daß mir’s gelinge (Riemenschneider 302) | covered | 1 | 313 | plagal | 1225.6 |
| O Gott, du frommer Gott (Riemenschneider 312) | covered | 6 | 380 | authentic | 2917.7 |
| O Haupt voll Blut und Wunden (Riemenschneider 80) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Hast du denn, Jesu, dein Angesicht gänzlich verborgen (Riemenschneider 90) | covered | 3 | 295 | authentic | 2876.9 |
| Ach was soll ich Sünder machen (Riemenschneider 39) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Seelenbräutigam (Riemenschneider 141) | covered | 1 | 240 | none | 2120.7 |
| Freu’ dich sehr, o meine Seele (Riemenschneider 29) | covered | 1 | 343 | plagal | 2832.6 |
| Meinen Jesum laß’ ich nicht, Jesus (Riemenschneider 151) | covered | 1 | 171 | authentic | 1761.6 |
| Lobet den Herren, denn er ist sehr freundlich (Riemenschneider 227) | NOT COVERED | 6 | — | — | — (phrase 2/6: chromatic soprano unsupported) |
| Was betrübst du dich, mein Herze (Riemenschneider 237) | covered | 1 | 463 | plagal | 2073.1 |
| Christe, der du bist Tag und Licht (Riemenschneider 245) | covered | 1 | 240 | authentic | 1004.4 |
| Was frag’ ich nach der Welt (Riemenschneider 255) | covered | 2 | 387 | authentic | 2756.2 |
| Helft mir Gott’s Güte preisen (Riemenschneider 123) | covered | 1 | 401 | none | 1769.3 |
| Jesus, meine Zuversicht (Riemenschneider 175) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| O Lamm Gottes, unschuldig (Riemenschneider 165) | covered | 1 | 322 | authentic | 2941.2 |
| O wie selig seid ihr doch, ihr Frommen (Riemenschneider 213) | NOT COVERED | 2 | — | — | — (phrase 1/2: chromatic soprano unsupported) |
| O Mensch, schau’ Jesum Christum an (Riemenschneider 203) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Ein Lämmlein geht und trägt die Schuld (Riemenschneider 309) | covered | 1 | 584 | authentic | 4453.3 |
| Sanctus, Sanctus Dominus Deus Sabaoth (Riemenschneider 319) | covered | 3 | 401 | plagal | 3492.5 |
| Christ lag in Todesbanden (Riemenschneider 261) | NOT COVERED | 2 | — | — | — (phrase 1/2: chromatic soprano unsupported) |
| Nun ruhen alle Wälder (Riemenschneider 117) | covered | 1 | 344 | authentic | 2955.0 |
| Herzlich lieb hab’ ich dich, o Herr (Riemenschneider 107) | covered | 1 | 796 | authentic | 8043.9 |
| Herr Jesu Christ, mein’s Lebens Licht (Riemenschneider 295) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Wär’ Gott nicht mit uns diese Zeit (Riemenschneider 285) | covered | 1 | 328 | plagal | 1406.9 |
| Sei Lob und Ehr’ dem höchsten Gut (Riemenschneider 354) | covered | 1 | 326 | authentic | 2753.5 |
| Nun danket alle Gott (Riemenschneider 32) | covered | 5 | 235 | none | 1793.8 |
| Schmücke dich, o liebe Seele (Riemenschneider 22) | covered | 1 | 475 | authentic | 4032.4 |
| In allen meinen Taten (Riemenschneider 50) | covered | 1 | 364 | authentic | 3465.9 |
| Alles ist an Gottes Segen (Riemenschneider 128) | covered | 1 | 378 | none | 3110.6 |
| Ach Gott und Herr (Riemenschneider 40) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (chordal seventh resolution)) |
| Jesu, meine Freude (Riemenschneider 138) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Wo Gott der Herr nicht bei uns hält (Riemenschneider 336) | covered | 1 | 346 | plagal | 1156.6 |
| Allein Gott in der Höh’ sei Ehr’ (Riemenschneider 326) | covered | 1 | 338 | authentic | 2417.0 |
| Von Gott will ich nicht lassen (Riemenschneider 191) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Gott hat das Evangelium (Riemenschneider 181) | covered | 1 | 388 | authentic | 1547.8 |
| Mach’s mit mir, Gott, nach deiner Güt’ (Riemenschneider 310) | covered | 1 | 247 | plagal | 1831.0 |
| Nun lob’, mein’ Seel’, den Herren (Riemenschneider 268) | covered | 1 | 642 | plagal | 6365.6 |
| Warum betrübst du dich, mein Herz (Riemenschneider 300) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Wie schön leuchtet der Morgenstern (Riemenschneider 278) | covered | 2 | 338 | authentic | 2548.3 |
| Freu’ dich sehr, o meine Seele (Riemenschneider 76) | covered | 1 | 364 | authentic | 2530.2 |
| Christ, unser Herr, zum Jordan kam (Riemenschneider 66) | covered | 1 | 520 | plagal | 1909.1 |
| O Herre Gott, dein göttlich Wort (Riemenschneider 14) | covered | 1 | 373 | authentic | 2975.1 |
| Es ist das Heil uns kommen her (Riemenschneider 4) | covered | 1 | 324 | authentic | 2231.7 |
| Es ist gewißlich an der Zeit (Riemenschneider 362) | covered | 1 | 361 | authentic | 2834.7 |
| Mit Fried’ und Freud’ ich fahr’ dahin (Riemenschneider 49) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Liebster Jesu, wir sind hier (Riemenschneider 131) | covered | 1 | 254 | plagal | 1889.4 |
| Herzliebster Jesu, was hast du verbrochen (Riemenschneider 59) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Werde munter, mein Gemüte (Riemenschneider 121) | covered | 1 | 491 | plagal | 3450.9 |
| Nun laßt uns Gott, dem Herren (Riemenschneider 257) | covered | 1 | 231 | none | 2168.1 |
| Ich dank’ dir schon durch deinen Sohn (Riemenschneider 188) | covered | 1 | 267 | authentic | 1992.7 |
| Christus, der uns selig macht (Riemenschneider 198) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| O Jesu Christ, du höchstes Gut (Riemenschneider 92) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| O großer Gott von Macht (Riemenschneider 82) | NOT COVERED | 8 | — | — | — (phrase 1/8: chromatic soprano unsupported) |
| Heilig, heilig (Riemenschneider 235) | covered | 3 | 401 | plagal | 3448.3 |
| Gott, der du selber bist das Licht (Riemenschneider 225) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Alle Menschen müssen sterben (Riemenschneider 153) | covered | 1 | 295 | plagal | 4797.6 |
| Ach Gott und Herr, wie groß und schwer (Riemenschneider 279) | covered | 2 | 260 | none | 7868.9 |
| Ach, lieben Christen, seid getrost (Riemenschneider 301) | covered | 1 | 321 | authentic | 4417.5 |
| Jesu, der du meine Seele (Riemenschneider 269) | covered | 1 | 350 | none | 5061.1 |
| Dank sei Gott in der Höhe (Riemenschneider 311) | covered | 1 | 336 | authentic | 6805.6 |
| Freu’ dich sehr, o meine Seele (Riemenschneider 67) | covered | 1 | 340 | authentic | 4705.9 |
| In dich hab’ ich gehoffet, Herr (Riemenschneider 77) | covered | 2 | 380 | plagal | 4134.8 |
| An Wasserflüssen Babylon (Riemenschneider 5) | covered | 1 | 584 | authentic | 4366.1 |
| O Welt, sieh’ hier dein Leben (Riemenschneider 363) | covered | 1 | 405 | plagal | 2605.8 |
| Hilf, Gott, daß mir’s gelinge (Riemenschneider 199) | covered | 1 | 313 | plagal | 1199.1 |
| Herr Jesu Christ, wahr’r Mensch und Gott (Riemenschneider 189) | covered | 1 | 211 | authentic | 1705.0 |
| Was mein Gott will, das g’scheh’ allzeit (Riemenschneider 120) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Meine Seel erhebet den Herrn (Riemenschneider 130) | covered | 1 | 147 | none | 1578.8 |
| Ach wie nichtig, ach wie flüchtig (Riemenschneider 48) | covered | 1 | 234 | none | 1214.7 |
| Singt dem Herrn ein neues Lied (Riemenschneider 246) | covered | 1 | 348 | authentic | 2595.4 |
| Jesu, deine tiefen Wunden (Riemenschneider 256) | covered | 1 | 340 | authentic | 2821.0 |
| Das walt’ Gott Vater und Gott Sohn (Riemenschneider 224) | covered | 1 | 268 | authentic | 2283.2 |
| Gott lebet noch (Riemenschneider 234) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 256)) |
| Schwing’ dich auf zu deinem Gott (Riemenschneider 142) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Meinen Jesum laß’ ich nicht, weil er sich für mich gegeben (Riemenschneider 152) | covered | 1 | 334 | authentic | 2570.8 |
| Jesu Leiden, Pein und Tod (Riemenschneider 83) | covered | 1 | 425 | authentic | 3248.4 |
| Wach’ auf, mein Herz, und singe (Riemenschneider 93) | covered | 1 | 231 | none | 2149.9 |
| Herr Gott, dich loben alle wir (Riemenschneider 164) | covered | 1 | 227 | authentic | 1975.0 |
| Jesus Christus, unser Heiland, der den Tod überwand (Riemenschneider 174) | NOT COVERED | 4 | — | — | — (phrase 1/4: chromatic soprano unsupported) |
| O wir armen Sünder (Riemenschneider 202) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (chordal seventh resolution)) |
| Herr, ich denk’ an jene Zeit (Riemenschneider 212) | covered | 1 | 343 | plagal | 2726.3 |
| Es ist gewißlich an der Zeit (Riemenschneider 260) | covered | 1 | 337 | authentic | 2806.7 |
| Herr, wie du willst, so schick’s mit mir (Riemenschneider 318) | covered | 5 | 249 | authentic | 2425.5 |
| Befiehl du deine Wege (Riemenschneider 270) | covered | 2 | 327 | authentic | 1122.1 |
| Ach Gott, wie manches Herzeleid (Riemenschneider 308) | covered | 1 | 272 | half | 1706.7 |
| Jesu Leiden, Pein und Tod (Riemenschneider 106) | covered | 1 | 365 | authentic | 3132.2 |
| Nun lob’, mein’ Seel’, den Herren (Riemenschneider 116) | covered | 1 | 661 | authentic | 4988.9 |
| O Haupt voll Blut und Wunden (Riemenschneider 345) | covered | 1 | 382 | plagal | 1304.5 |
| Nun ruhen alle Wälder (Riemenschneider 355) | covered | 1 | 386 | authentic | 2986.3 |
| Zeuch ein zu deinen Toren (Riemenschneider 23) | covered | 1 | 299 | authentic | 1375.7 |
| Herr, ich habe mißgehandelt (Riemenschneider 33) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Herr Jesu Christ, wahr’r Mensch und Gott (Riemenschneider 284) | covered | 1 | 342 | none | 3194.7 |
| Herr Jesu Christ, du höchstes Gut (Riemenschneider 294) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Als Jesus Christus in der Nacht (Riemenschneider 180) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Herr, nun laß in Friede (Riemenschneider 190) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Warum sollt’ ich mich denn grämen (Riemenschneider 139) | covered | 1 | 332 | authentic | 2644.1 |
| Was mein Gott will, das g’scheh’ allzeit (Riemenschneider 41) | NOT COVERED | 1 | — | — | — (phrase 1/1: chromatic soprano unsupported) |
| Keinen hat Gott verlassen (Riemenschneider 129) | covered | 1 | 311 | none | 2636.8 |
| Gelobet seist du, Jesu Christ (Riemenschneider 51) | covered | 1 | 311 | plagal | 2449.2 |
| Jesu, nun sei gepreiset (Riemenschneider 327) | covered | 1 | 666 | authentic | 5823.0 |
| O Gott, du frommer Gott (Riemenschneider 337) | covered | 1 | 474 | authentic | 4106.8 |
