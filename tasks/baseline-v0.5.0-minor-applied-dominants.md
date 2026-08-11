# mokuren v0.5.0-in-progress chorale baseline (minor applied dominants + melodic minor)

Same 348-chorale corpus as `tasks/baseline-v0.4.0-minor-mode.md`. Re-prioritized ahead of adaptive/search-budget research per an explicit user directive (2026-08-11): minor's dominant failure mode was *no candidate exists at all*, not *search missed a candidate that exists* — vocabulary has the bigger lever than beam width when candidates are missing outright.

**Vocabulary chosen from real corpus evidence, not copied from major.** Before writing any implementation, `examples/chorale_benchmark.rs --minor-gap-report` (new tooling, this pass) classified every unreachable minor-key chromatic soprano tone in the v0.4.0 failures against candidate applied-dominant targets and the melodic-minor raised 6th:

```
  79  V(7)/ii
  68  V(7)/V
  65  raised 6th (melodic minor)
  16  V(7)/IV
   1  V(7)/vi
   0  V(7)/iii
```

100% of the 81 chromatic-soprano failures were explained by one of these two categories (zero unclassified). V/III was excluded from the implementation entirely — zero evidence it's needed, unlike major's full 5-target set. Raised 6th, originally deferred as "melodic minor" to a later phase, was pulled into this same pass instead: 65/81 chorales needed it, vs. only 16/81 that applied dominants alone would have fully resolved — deferring it would have made this phase mostly a no-op.

**Mechanism**: both additions reuse the "same root, different quality" trick harmonic minor's V/V7 already established — no new root computation needed. `RomanNumeral::minor_applied_dominant_vocabulary()` (V/x, V7/x for x in {ii, IV, V, vi}) is mechanically identical to major's `applied_dominant_vocabulary`. `RomanNumeral::melodic_minor_vocabulary()` offers ii as a minor triad (not natural minor's diminished ii°) and IV as a major triad (not natural minor's minor iv) — both because the raised 6th is literally part of each chord's own stacked-thirds structure (ii's fifth; IV's third), verified by hand before implementing, the same way harmonic minor's raised-7th mechanism was.

**Results, verified per-chorale (not estimated)**:

- **Major: 172/182 (94.5%), unchanged — zero regressions.** Confirmed by diffing the exact covered-chorale ID set against the pre-this-pass baseline.
- **Minor: 71/166 (42.8%) → 107/166 (64.5%)** — +36 net (54 chorales newly covered, 18 chorales regressed). All 18 regressions are beam-width casualties confirmed recoverable (`classify_failure` directly retested each at width 64–512, not assumed) — the same horizon-effect pattern from when major's own applied-dominant vocabulary first roughly doubled candidate count (`tasks/lessons.md`). The default beam width (32) is deliberately left unchanged, same reasoning as before. At width 128, minor's coverage curve reaches 90.8% combined (348-corpus figure); isolating minor's own width-128 number is left for a future adaptive-search-budget pass, not this one.
- **Hard-rule violations: 0**, maintained across all 348 chorales.
- Combined (major+minor): 69.8% → 80.2% (243/348 → 279/348).

64.5% lands just under the user's stated "65–75% is a real win" bar, essentially at the floor — a strong first-pass result, not the ~80% "big hit" secondary dominants produced for major. Remaining minor failures (59/166): 45 search-exhausted (wider beam recovers — includes the 18 "regressions" above), 7 chordal-seventh-resolution (same known non-chord-tone gap as major), 6 secondary-dominant-resolution (new — minor's applied dominants hitting a resolution edge case not exercised by major's corpus; not investigated this pass), 11 voice-overlap (likely the same thin-vocabulary-at-a-scale-degree pattern bisected in v0.4.0, now smaller since the vocabulary that caused it is partly filled in).

Reproduce: `python3 tools/music21_chorale_extractor.py -o <dir>` then `cargo run --release --example chorale_benchmark -- <dir> --report tasks/baseline-v0.5.0-minor-applied-dominants.md`. Gap analysis: `cargo run --release --example chorale_benchmark -- <dir> --minor-gap-report`.

# Chorale benchmark report

## Provenance

- mokuren version: 0.1.0
- git commit: 4b5dc08025eb99d3263bf9f24aa025f21ba58bc6
- music21 version: 9.9.1
- corpus: 348 chorale(s) extracted, 23 skipped at extraction time
  - exclusion reasons (23 total):
    - ATB gap at a soprano onset: 10
    - unrepresentable duration: 8
    - missing part: 5
- chorales measured here: 348 (513 phrase(s) total, after splitting at rests)
- standard beam width: 32 (retry widths for failure classification: [64, 128, 256, 512])

## Coverage

- Coverage: 279/348 (80.2%)
- Search failure rate: 69/348 (19.8%)

- 64 chorale(s) had a soprano rest and were split into multiple phrases (harmonized independently, each via the same `Composer::harmonize` a rest-free chorale uses); "covered" above requires *every* phrase of a chorale to harmonize, so this is comparable to the pre-rest-support baselines, not a looser per-phrase number.

## Failure taxonomy (not lumped into one bucket)

- rule conflict: chordal seventh resolution: 7 (2.0% of all fixtures, 10.1% of failures)
- rule conflict: secondary dominant resolution: 6 (1.7% of all fixtures, 8.7% of failures)
- rule conflict: voice overlap: 11 (3.2% of all fixtures, 15.9% of failures)
- search exhausted (wider beam works): 45 (12.9% of all fixtures, 65.2% of failures)

### Beam-width coverage curve (failures only — successes at width 32 aren't retried)

- width   32: 279/348 (80.2%)
- width   64: 304/348 (87.4%)
- width  128: 316/348 (90.8%)
- width  256: 320/348 (92.0%)
- width  512: 324/348 (93.1%)

## Hard-rule violations

0 (should always be 0 by construction — a nonzero count is a bug, not a quality signal)

## Voice-leading cost

Per-chorale average (cost / position): median 7.20, p90 7.94, p95 8.09

## Runtime

Per chorale (ms, summed across phrases): median 1343.2, p90 2003.1, p95 2615.4

## Explanation completeness

- why() coverage: 96.8% of positions have at least one Reason
- why_not() success: 13335/13335 (100.0%) of positions with a valid alternative

## Cadence

(the chorale's *last phrase*'s cadence — for a multi-phrase chorale this is the piece's actual final cadence, not an average across phrase-internal cadences)

Final-cadence distribution:
- authentic: 166 (59.5%)
- deceptive: 2 (0.7%)
- half: 12 (4.3%)
- none: 32 (11.5%)
- plagal: 67 (24.0%)

Ends on a tonic-function chord (proxy for "the close is at least plausible," not full cadence-correctness verification): 237/279 (84.9%)

## Original-note match (secondary, diagnostic only — see BENCHMARK.md's non-goal)

24.0% (pooled across phrases) over 279 fixture(s) with a reference ATB

## Per-chorale

| Chorale | Result | Phrases | Voice-leading cost | Cadence | Runtime (ms) |
|---|---|---|---|---|---|
| Nun bitten wir den heiligen Geist (Riemenschneider 36) | covered | 2 | 393 | authentic | 1447.9 |
| O Ewigkeit, du Donnerwort (Riemenschneider 26) | covered | 1 | 326 | authentic | 1385.1 |
| Danket dem Herren, denn er ist sehr freundlich (Riemenschneider 228) | covered | 1 | 145 | authentic | 675.8 |
| Jesu, meiner Seelen Wonne (Riemenschneider 350) | covered | 1 | 353 | none | 1404.1 |
| Es wird schier der letzte Tag herkommen (Riemenschneider 238) | covered | 1 | 283 | plagal | 1038.0 |
| Befiehl du deine Wege (Riemenschneider 340) | covered | 1 | 269 | plagal | 1291.6 |
| Was frag’ ich nach der Welt (Riemenschneider 291) | covered | 1 | 426 | authentic | 1703.3 |
| Wo soll ich fliehen hin (Riemenschneider 281) | covered | 1 | 286 | plagal | 1319.7 |
| Nun freut euch, Gottes Kinder all’ (Riemenschneider 185) | covered | 1 | 263 | authentic | 954.0 |
| Von Gott will ich nicht lassen (Riemenschneider 332) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Wenn mein Stündlein vorhanden ist (Riemenschneider 322) | covered | 1 | 450 | authentic | 1974.3 |
| Lobt Gott, ihr Christen, allzugleich (Riemenschneider 54) | covered | 2 | 317 | plagal | 1245.6 |
| Mach’s mit mir, Gott, nach deiner Güt’ (Riemenschneider 44) | covered | 1 | 222 | authentic | 943.9 |
| Ach Gott, wie manches Herzeleid (Riemenschneider 217) | covered | 1 | 249 | authentic | 1188.8 |
| Des Heil’gen Geistes reiche Gnad’ (Riemenschneider 207) | covered | 1 | 216 | none | 953.9 |
| Schaut, ihr Sünder (Riemenschneider 171) | covered | 5 | 241 | authentic | 1014.2 |
| Ermuntre dich, mein schwacher Geist (Riemenschneider 9) | covered | 1 | 341 | authentic | 1455.7 |
| Ich hab’ mein’ Sach’ Gott heimgestellt (Riemenschneider 19) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (secondary dominant resolution)) |
| Christus, der uns selig macht (Riemenschneider 113) | covered | 1 | 496 | authentic | 1753.4 |
| Nun ruhen alle Wälder (Riemenschneider 103) | covered | 1 | 401 | plagal | 1425.4 |
| O Welt, sieh’ hier dein Leben (Riemenschneider 275) | covered | 1 | 395 | plagal | 1265.0 |
| Was mein Gott will, das g’scheh’ allzeit (Riemenschneider 265) | covered | 1 | 392 | plagal | 1275.2 |
| Ach Gott, vom Himmel sieh’ darein (Riemenschneider 253) | covered | 3 | 294 | half | 1200.6 |
| Jesu, du mein liebstes Leben (Riemenschneider 243) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 128)) |
| Gott der Vater wohn’ uns bei (Riemenschneider 135) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 512)) |
| Allein Gott in der Höh’ sei Ehr’ (Riemenschneider 125) | covered | 1 | 338 | authentic | 1256.3 |
| Wo Gott zum Haus nicht gibt sein’ Gunst (Riemenschneider 157) | covered | 1 | 290 | authentic | 1114.0 |
| Wenn ich in Angst und Not (Riemenschneider 147) | covered | 5 | 294 | authentic | 1315.2 |
| Die Nacht ist kommen (Riemenschneider 231) | covered | 6 | 244 | authentic | 964.4 |
| Ich hab’ in Gottes Herz und Sinn (Riemenschneider 349) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Herr, straf’ mich nicht in deinem Zorn (Riemenschneider 221) | covered | 1 | 246 | authentic | 1198.8 |
| Allein zu dir, Herr Jesu Christ (Riemenschneider 359) | covered | 6 | 445 | authentic | 1504.4 |
| Gelobet seist du, Jesu Christ (Riemenschneider 288) | covered | 1 | 306 | none | 1072.2 |
| Weg, mein Herz, mit den Gedanken (Riemenschneider 298) | covered | 1 | 346 | authentic | 1515.9 |
| Jesu, meine Freude (Riemenschneider 96) | covered | 1 | 330 | authentic | 1329.5 |
| Erhalt’ uns, Herr, bei deinem Wort (Riemenschneider 72) | covered | 1 | 263 | authentic | 1027.2 |
| Wer nur den lieben Gott läßt walten (Riemenschneider 62) | covered | 2 | 249 | plagal | 846.1 |
| Das alte Jahr vergangen ist (Riemenschneider 314) | covered | 1 | 452 | half | 1310.6 |
| Auf meinen lieben Gott (Riemenschneider 304) | covered | 1 | 285 | authentic | 1258.5 |
| O Welt, sieh’ hier dein Leben (Riemenschneider 366) | covered | 2 | 408 | authentic | 1399.5 |
| Heut’ ist, o Mensch, ein großer Trauertag (Riemenschneider 168) | covered | 3 | 217 | plagal | 823.4 |
| Aus tiefer Not schrei’ ich zu dir (Riemenschneider 10) | covered | 1 | 398 | half | 1266.6 |
| Das neugeborne Kindelein (Riemenschneider 178) | covered | 1 | 264 | authentic | 1144.0 |
| Wie bist du, Seele, in mir so gar betrübt (Riemenschneider 242) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 256)) |
| Jesu, nun sei gepreiset (Riemenschneider 252) | covered | 1 | 568 | authentic | 3190.0 |
| Auf, auf, mein Herz, und du, mein ganzer Sinn (Riemenschneider 124) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Du, o schönes Weltgebäude (Riemenschneider 134) | covered | 1 | 361 | authentic | 1365.8 |
| Meinen Jesum laß ich nicht (Riemenschneider 299) | covered | 1 | 436 | authentic | 1909.2 |
| Nun ruhen alle Wälder (Riemenschneider 289) | covered | 1 | 407 | plagal | 1615.6 |
| Nun bitten wir den heiligen Geist (Riemenschneider 97) | covered | 1 | 450 | authentic | 1934.7 |
| Wer nur den lieben Gott läßt walten (Riemenschneider 146) | covered | 1 | 283 | authentic | 1144.8 |
| Ach Gott, wie manches Herzeleid (Riemenschneider 156) | covered | 1 | 272 | half | 870.2 |
| Sollt’ ich meinem Gott nicht singen (Riemenschneider 220) | covered | 1 | 480 | deceptive | 2390.7 |
| Meinen Jesum laß’ ich nicht, weil (Riemenschneider 348) | covered | 1 | 326 | authentic | 1586.3 |
| Christ, der du bist der helle Tag (Riemenschneider 230) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Nun ruhen alle Wälder (Riemenschneider 63) | covered | 1 | 391 | plagal | 1328.9 |
| Herr Jesu Christ, du höchstes Gut (Riemenschneider 73) | covered | 1 | 290 | plagal | 1279.5 |
| O Gott, du frommer Gott (Riemenschneider 315) | covered | 1 | 350 | none | 1507.9 |
| Befiehl du deine Wege (Riemenschneider 367) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| Aus meines Herzens Grunde (Riemenschneider 1) | covered | 1 | 328 | authentic | 1451.4 |
| Wachet auf, ruft uns die Stimme (Riemenschneider 179) | covered | 5 | 443 | authentic | 1845.1 |
| Jesu, der du selbst so wohl (Riemenschneider 169) | covered | 1 | 405 | plagal | 1379.1 |
| Es ist das Heil uns kommen her (Riemenschneider 290) | covered | 1 | 330 | authentic | 1274.0 |
| Es spricht der Unweisen Mund (Riemenschneider 27) | covered | 1 | 339 | authentic | 1445.8 |
| Jesu, der du meine Seele (Riemenschneider 37) | covered | 1 | 366 | none | 1441.0 |
| Ich dank’ dir, lieber Herre (Riemenschneider 341) | covered | 1 | 555 | authentic | 1905.3 |
| Den Vater dort oben (Riemenschneider 239) | covered | 1 | 350 | authentic | 1528.3 |
| Wenn mein Stündlein vorhanden ist (Riemenschneider 351) | covered | 1 | 452 | authentic | 1718.7 |
| Ich danke dir, o Gott, in deinem Throne (Riemenschneider 229) | covered | 1 | 535 | plagal | 2034.6 |
| Wie schön leuchtet der Morgenstern (Riemenschneider 323) | covered | 2 | 386 | authentic | 1640.9 |
| Es woll’ uns Gott genädig sein (Riemenschneider 333) | covered | 1 | 284 | authentic | 1109.1 |
| Kommt her zu mir, spricht Gottes Sohn (Riemenschneider 45) | covered | 1 | 446 | authentic | 1572.2 |
| Wir Christenleut’ (Riemenschneider 55) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Christ lag in Todesbanden (Riemenschneider 184) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| So gibst du nun, mein Jesu, gute Nacht (Riemenschneider 206) | covered | 4 | 407 | authentic | 1620.7 |
| Es ist genug, so nimm, Herr, meinen Geist (Riemenschneider 216) | covered | 8 | 380 | none | 1259.9 |
| Gottes Sohn ist kommen (Riemenschneider 18) | covered | 1 | 361 | authentic | 1401.5 |
| Gelobet seist du, Jesu Christ (Riemenschneider 160) | covered | 1 | 292 | none | 1106.1 |
| Freuet euch, ihr Christen alle (Riemenschneider 8) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 128)) |
| Nun komm, der Heiden Heiland (Riemenschneider 170) | covered | 1 | 254 | plagal | 982.8 |
| Ermuntre dich, mein schwacher Geist (Riemenschneider 102) | covered | 1 | 369 | authentic | 1482.9 |
| Wer nur den lieben Gott läßt walten (Riemenschneider 112) | covered | 2 | 239 | plagal | 823.2 |
| Jesu, meines Herzens Freud’ (Riemenschneider 264) | covered | 1 | 402 | none | 1801.9 |
| O Ewigkeit, du Donnerwort (Riemenschneider 274) | covered | 1 | 316 | authentic | 1325.9 |
| Werde munter, mein Gemüte (Riemenschneider 95) | covered | 1 | 311 | plagal | 1372.2 |
| O Gott, du frommer Gott (Riemenschneider 85) | covered | 6 | 383 | authentic | 1298.9 |
| Die Sonn’ hat sich mit ihrem Glanz gewendet (Riemenschneider 232) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (chordal seventh resolution)) |
| Nun preiset alle Gottes Barmherzigkeit (Riemenschneider 222) | covered | 1 | 318 | plagal | 2391.4 |
| Der du bist drei in Einigkeit (Riemenschneider 154) | covered | 1 | 225 | plagal | 1988.7 |
| Wer in dem Schutz des Höchsten ist (Riemenschneider 144) | covered | 5 | 249 | authentic | 1995.2 |
| Herr Jesu Christ, dich zu uns wend’ (Riemenschneider 136) | covered | 1 | 256 | authentic | 1947.9 |
| Durch Adams Fall ist ganz verderbt (Riemenschneider 126) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (chordal seventh resolution)) |
| Liebster Jesu, wir sind hier (Riemenschneider 328) | covered | 1 | 254 | plagal | 1839.9 |
| Ein’ feste Burg ist unser Gott (Riemenschneider 250) | covered | 1 | 425 | plagal | 2911.5 |
| Jesus, meine Zuversicht (Riemenschneider 338) | covered | 1 | 349 | authentic | 2632.7 |
| Nun sich der Tag geendet hat (Riemenschneider 240) | NOT COVERED | 2 | — | — | — (phrase 1/2: search exhausted (works at width 64)) |
| Allein zu dir, Herr Jesu Christ (Riemenschneider 13) | covered | 2 | 551 | authentic | 3958.8 |
| Ach Gott, vom Himmel sieh’ darein (Riemenschneider 3) | covered | 1 | 350 | half | 2615.6 |
| Jesu, meiner Seelen Wonne (Riemenschneider 365) | covered | 1 | 304 | authentic | 2191.3 |
| Herr, wie du willst, so schick’s mit mir (Riemenschneider 317) | covered | 1 | 362 | authentic | 4857.7 |
| Christus, der uns selig macht (Riemenschneider 307) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| Ich ruf’ zu dir, Herr Jesu Christ (Riemenschneider 71) | covered | 1 | 463 | plagal | 1824.8 |
| Singen wir aus Herzensgrund (Riemenschneider 109) | covered | 1 | 452 | plagal | 2076.9 |
| Jesu Leiden, Pein und Tod (Riemenschneider 61) | covered | 1 | 377 | authentic | 2790.2 |
| Christ, unser Herr, zum Jordan kam (Riemenschneider 119) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 128)) |
| O Traurigkeit, o Herzeleid (Riemenschneider 57) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Vater unser im Himmelreich (Riemenschneider 47) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 512)) |
| Wo soll ich fliehen hin (Riemenschneider 331) | covered | 1 | 284 | plagal | 1218.0 |
| Allein Gott in der Höh’ sei Ehr’ (Riemenschneider 249) | covered | 1 | 320 | authentic | 1520.4 |
| Wir Christenleut’ (Riemenschneider 321) | covered | 1 | 248 | plagal | 1276.7 |
| Verleih’ uns Frieden gnädiglich (Riemenschneider 259) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Da der Herr Christ zu Tische saß (Riemenschneider 196) | covered | 1 | 340 | authentic | 1894.8 |
| Ach Gott, erhör’ mein Seufzen (Riemenschneider 186) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| Nimm von uns, Herr, du treuer Gott (Riemenschneider 292) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 256)) |
| Freu’ dich sehr, o meine Seele (Riemenschneider 282) | covered | 1 | 362 | authentic | 1693.4 |
| Der Herr ist mein getreuer Hirt (Riemenschneider 353) | covered | 1 | 276 | authentic | 1444.4 |
| Nun lieget alles unter dir (Riemenschneider 343) | covered | 1 | 335 | none | 1412.1 |
| Gott des Himmels und der Erden (Riemenschneider 35) | covered | 1 | 268 | authentic | 998.4 |
| Wo soll ich fliehen hin (Riemenschneider 25) | NOT COVERED | 3 | — | — | — (phrase 3/3: search exhausted (works at width 64)) |
| Lobt Gott, ihr Christen allzugleich (Riemenschneider 276) | covered | 2 | 281 | plagal | 1217.3 |
| Herr Jesu Christ, du höchstes Gut (Riemenschneider 266) | covered | 1 | 294 | plagal | 1308.1 |
| Wenn wir in höchsten Nöten sein (Riemenschneider 68) | covered | 2 | 231 | authentic | 1162.7 |
| Vater unser im Himmelreich (Riemenschneider 110) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| Herzliebster Jesu, was hast du verbrochen (Riemenschneider 78) | covered | 1 | 322 | authentic | 1104.0 |
| Durch Adams Fall ist ganz verderbt (Riemenschneider 100) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (chordal seventh resolution)) |
| Sei gegrüßet, Jesu gütig (Riemenschneider 172) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Das alte Jahr vergangen ist (Riemenschneider 162) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (secondary dominant resolution)) |
| Mitten wir im Leben sind (Riemenschneider 214) | covered | 6 | 648 | half | 2615.3 |
| Wer weiß, wie nahe mir mein Ende (Riemenschneider 204) | covered | 1 | 250 | plagal | 1041.8 |
| Komm, Gott Schöpfer, heiliger Geist (Riemenschneider 187) | covered | 1 | 278 | authentic | 1017.0 |
| Christ ist erstanden (Riemenschneider 197) | NOT COVERED | 2 | — | — | — (phrase 2/2: search exhausted (works at width 64)) |
| Vom Himmel hoch, da komm’ ich her (Riemenschneider 46) | covered | 4 | 251 | authentic | 931.4 |
| Christum wir sollen loben schon (Riemenschneider 56) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (secondary dominant resolution)) |
| Mein’ Augen schließ’ ich jetzt in Gottes Namen zu (Riemenschneider 258) | covered | 7 | 333 | authentic | 1270.3 |
| Gott sei uns gnädig und barmherzig (Riemenschneider 320) | covered | 1 | 163 | none | 607.3 |
| Sei Lob und Ehr’ dem höchsten Gut (Riemenschneider 248) | covered | 1 | 326 | authentic | 1509.4 |
| Nun danket alle Gott (Riemenschneider 330) | covered | 1 | 272 | none | 1275.5 |
| Lobt Gott, ihr Christen, allzugleich (Riemenschneider 342) | covered | 1 | 326 | authentic | 1212.9 |
| Es woll’ uns Gott genädig sein (Riemenschneider 352) | NOT COVERED | 2 | — | — | — (phrase 2/2: rule conflict (voice overlap)) |
| Valet will ich dir geben (Riemenschneider 24) | covered | 1 | 326 | authentic | 1320.6 |
| Erbarm’ dich mein, o Herre Gott (Riemenschneider 34) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Was Gott tut, das ist wohlgetan (Riemenschneider 293) | covered | 1 | 284 | authentic | 1142.6 |
| Vater unser im Himmelreich (Riemenschneider 267) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 512)) |
| Herzlich lieb hab’ ich dich, o Herr (Riemenschneider 277) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 128)) |
| Herr Christ, der ein’ge Gott’s-Sohn (Riemenschneider 101) | covered | 1 | 293 | authentic | 1194.8 |
| Heut’ triumphieret Gottes Sohn (Riemenschneider 79) | covered | 1 | 352 | none | 1565.9 |
| Herzliebster Jesu, was hast du verbrochen (Riemenschneider 111) | covered | 1 | 332 | authentic | 1343.7 |
| Komm, heiliger Geist, Herre Gott (Riemenschneider 69) | covered | 5 | 710 | authentic | 2984.7 |
| Für Freuden laßt uns springen (Riemenschneider 163) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (secondary dominant resolution)) |
| O Herzensangst, o Bangigkeit (Riemenschneider 173) | covered | 1 | 310 | authentic | 1268.7 |
| Herr Gott, dich loben wir (Riemenschneider 205) | covered | 2 | 1373 | none | 6313.6 |
| Verleih’ uns Frieden gnädiglich (Riemenschneider 215) | NOT COVERED | 2 | — | — | — (phrase 1/2: search exhausted (works at width 128)) |
| Ich dank’ dir, Gott, für all’ Wohltat (Riemenschneider 223) | covered | 3 | 397 | authentic | 1774.3 |
| Werde munter, mein Gemüte (Riemenschneider 233) | covered | 1 | 308 | none | 1138.0 |
| Warum betrübst du dich, mein Herz (Riemenschneider 145) | covered | 1 | 312 | plagal | 1373.1 |
| Hilf, Herr Jesu, laß gelingen (Riemenschneider 155) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 256)) |
| Nun bitten wir den heiligen Geist (Riemenschneider 84) | covered | 1 | 448 | authentic | 1624.9 |
| Warum betrübst du dich, mein Herz (Riemenschneider 94) | covered | 2 | 282 | plagal | 1242.5 |
| Dies sind die heil’gen zehn Gebot’ (Riemenschneider 127) | covered | 1 | 236 | half | 1256.3 |
| Wer Gott vertraut, hat wohl gebaut (Riemenschneider 137) | covered | 9 | 353 | authentic | 1485.6 |
| Was willst du dich, o meine Seele, kränken (Riemenschneider 241) | covered | 7 | 700 | none | 2873.5 |
| Wer nur den lieben Gott läßt walten (Riemenschneider 339) | covered | 1 | 237 | authentic | 975.3 |
| Ich bin ja, Herr, in deiner Macht (Riemenschneider 251) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| Sei Lob und Ehr’ dem höchsten Gut (Riemenschneider 329) | covered | 1 | 338 | authentic | 1464.5 |
| Ich dank’ dir, lieber Herre (Riemenschneider 2) | covered | 1 | 425 | authentic | 1401.9 |
| Puer natus in Bethlehem (Riemenschneider 12) | covered | 1 | 223 | authentic | 922.2 |
| Von Gott will ich nicht lassen (Riemenschneider 364) | covered | 1 | 389 | authentic | 1261.1 |
| O Mensch, bewein’ dein’ Sünde groß (Riemenschneider 306) | covered | 1 | 590 | authentic | 2328.3 |
| In dich hab’ ich gehoffet, Herr (Riemenschneider 118) | covered | 1 | 371 | authentic | 1558.3 |
| Ich freue mich in dir (Riemenschneider 60) | covered | 3 | 279 | authentic | 1163.6 |
| Valet will ich dir geben (Riemenschneider 108) | covered | 1 | 312 | authentic | 1485.4 |
| Gott sei gelobet und gebenedeiet (Riemenschneider 70) | covered | 2 | 494 | authentic | 1972.6 |
| Christ lag in Todesbanden (Riemenschneider 371) | covered | 2 | 390 | plagal | 1510.6 |
| O wie selig seid ihr doch, ihr Frommen (Riemenschneider 219) | covered | 1 | 320 | plagal | 1106.6 |
| Du Lebensfürst, Herr Jesu Christ (Riemenschneider 361) | covered | 1 | 341 | authentic | 1470.3 |
| Erschienen ist der herrliche Tag (Riemenschneider 17) | covered | 1 | 285 | plagal | 992.7 |
| Nun lob’, mein’ Seel’, den Herren (Riemenschneider 7) | covered | 1 | 559 | authentic | 2264.0 |
| Das walt’ mein Gott (Riemenschneider 75) | covered | 2 | 227 | deceptive | 1313.7 |
| Was Gott tut, das ist wohlgetan (Riemenschneider 65) | covered | 1 | 332 | authentic | 1212.0 |
| Allein Gott in der Höh’ sei Ehr’ (Riemenschneider 313) | covered | 1 | 276 | authentic | 1414.4 |
| Herr Christ, der ein’ge Gott’ssohn (Riemenschneider 303) | covered | 1 | 265 | authentic | 1156.5 |
| Nun komm, der Heiden Heiland (Riemenschneider 28) | covered | 1 | 225 | plagal | 787.7 |
| In allen meinen Taten (Riemenschneider 140) | covered | 2 | 297 | authentic | 1312.2 |
| Straf’ mich nicht in deinem Zorn (Riemenschneider 38) | covered | 1 | 281 | authentic | 983.2 |
| O Jesu, du mein Bräutigam (Riemenschneider 236) | covered | 1 | 302 | authentic | 907.2 |
| Herr Jesu Christ, du hast bereit’t (Riemenschneider 226) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (secondary dominant resolution)) |
| Verleih’ uns Frieden gnädiglich (Riemenschneider 91) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Christus, der uns selig macht (Riemenschneider 81) | covered | 1 | 490 | authentic | 1965.9 |
| Weg, mein Herz, mit den Gedanken (Riemenschneider 254) | covered | 1 | 362 | authentic | 1886.8 |
| Jesu, Jesu, du bist mein (Riemenschneider 244) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 128)) |
| Kyrie, Gott Vater in Ewigkeit (Riemenschneider 132) | NOT COVERED | 2 | — | — | — (phrase 1/2: rule conflict (chordal seventh resolution)) |
| Ist Gott mein Schild und Helfersmann (Riemenschneider 122) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Von Gott will ich nicht lassen (Riemenschneider 114) | covered | 1 | 335 | plagal | 1325.5 |
| Wer nur den lieben Gott läßt walten (Riemenschneider 104) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Ich dank’ dir, lieber Herre (Riemenschneider 272) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| AAch Gott, vom Himmel sieh’ darein (Riemenschneider 262) | covered | 1 | 347 | half | 1343.2 |
| Hilf, Herr Jesu, laß gelingen (Riemenschneider 368) | NOT COVERED | 6 | — | — | — (phrase 5/6: search exhausted (works at width 64)) |
| Christus ist erstanden, hat überwunden (Riemenschneider 200) | covered | 1 | 372 | none | 1510.1 |
| Erstanden ist der heil’ge Christ (Riemenschneider 176) | covered | 1 | 243 | authentic | 1257.9 |
| Es steh’n vor Gottes Throne (Riemenschneider 166) | covered | 1 | 343 | authentic | 1151.5 |
| Gottlob, es geht nunmehr zu Ende (Riemenschneider 192) | covered | 1 | 198 | authentic | 1042.2 |
| Wär’ Gott nicht mit uns diese Zeit (Riemenschneider 182) | covered | 1 | 349 | authentic | 1409.9 |
| Es ist das Heil uns kommen her (Riemenschneider 335) | covered | 1 | 352 | authentic | 1584.8 |
| Mit Fried’ und Freud’ ich fahr’ dahin (Riemenschneider 325) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| Das neugeborne Kindelein (Riemenschneider 53) | covered | 1 | 264 | authentic | 1097.4 |
| Nicht so traurig, nicht so sehr (Riemenschneider 149) | covered | 1 | 284 | none | 988.0 |
| Ach lieben Christen, seid getrost (Riemenschneider 31) | covered | 1 | 360 | plagal | 1380.1 |
| Als der gütige Gott (Riemenschneider 159) | covered | 4 | 182 | authentic | 788.2 |
| Herzlich tut mich verlangen (Riemenschneider 21) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 128)) |
| Warum sollt’ ich mich denn grämen (Riemenschneider 357) | covered | 1 | 376 | authentic | 1709.9 |
| Was Gott tut, das ist wohlgetan (Riemenschneider 347) | covered | 1 | 364 | authentic | 1394.5 |
| Nun lob’, mein’ Seel’, den Herren (Riemenschneider 296) | covered | 1 | 633 | none | 2961.1 |
| Befiehl du deine Wege (Riemenschneider 286) | covered | 1 | 337 | authentic | 1071.6 |
| Helft mir Gott’s Güte preisen (Riemenschneider 88) | covered | 1 | 306 | authentic | 1257.8 |
| O Haupt voll Blut und Wunden (Riemenschneider 98) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| Herzliebster Jesu, was hast du verbrochen (Riemenschneider 105) | covered | 1 | 330 | plagal | 1025.1 |
| Was mein Gott will, das g’scheh’ allezeit (Riemenschneider 115) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Jesu, meine Freude (Riemenschneider 263) | covered | 1 | 314 | authentic | 1167.6 |
| Ein’ feste Burg ist unser Gott (Riemenschneider 273) | covered | 2 | 402 | authentic | 1489.6 |
| O Mensch, bewein’ dein’ Sünde groß (Riemenschneider 201) | covered | 1 | 590 | authentic | 2313.5 |
| Jesu, der du meine Seele (Riemenschneider 369) | covered | 1 | 382 | authentic | 1442.3 |
| Weltlich’ Ehr’ und zeitlich Gut (Riemenschneider 211) | covered | 3 | 395 | half | 1649.2 |
| Du großer Schmerzensmann (Riemenschneider 167) | covered | 6 | 277 | none | 1393.9 |
| Ach bleib bei uns, Herr Jesu Christ (Riemenschneider 177) | covered | 1 | 288 | authentic | 1038.8 |
| Jesu, meine Freude (Riemenschneider 324) | covered | 1 | 300 | authentic | 1072.0 |
| Vor deinen Thron tret’ ich hiermit (Riemenschneider 334) | covered | 1 | 191 | authentic | 906.7 |
| Du Friedefürst, Herr Jesu Christ (Riemenschneider 42) | covered | 1 | 234 | authentic | 865.2 |
| Wenn mein Stündlein vorhanden ist (Riemenschneider 52) | covered | 1 | 463 | plagal | 1758.3 |
| Nun freut euch, lieben Christen, g’mein (Riemenschneider 183) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 512)) |
| Was bist du doch, o Seele, so betrübet (Riemenschneider 193) | covered | 4 | 213 | authentic | 857.0 |
| Herr, ich habe mißgehandelt (Riemenschneider 287) | covered | 1 | 257 | plagal | 857.8 |
| Jesu, der du meine Seele (Riemenschneider 297) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Helft mir Gott’s Güte preisen (Riemenschneider 99) | covered | 1 | 306 | authentic | 1298.2 |
| O Haupt voll Blut und Wunden (Riemenschneider 89) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Ein’ feste Burg ist unser Gott (Riemenschneider 20) | covered | 1 | 405 | plagal | 1461.5 |
| Der Tag der ist so freudenreich (Riemenschneider 158) | covered | 1 | 486 | authentic | 1939.5 |
| Jesus Christus, unser Heiland (Riemenschneider 30) | covered | 2 | 290 | authentic | 1137.6 |
| Uns ist ein Kindlein heut’ gebor’n (Riemenschneider 148) | covered | 1 | 272 | authentic | 1186.6 |
| Meines Lebens letzte Zeit (Riemenschneider 346) | covered | 1 | 473 | plagal | 1686.8 |
| Jesu, meine Freude (Riemenschneider 356) | covered | 1 | 313 | plagal | 1216.6 |
| Wir Christenleut’ (Riemenschneider 360) | covered | 1 | 271 | authentic | 1152.2 |
| Laß, o Herr, dein Ohr sich neigen (Riemenschneider 218) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 128)) |
| Kommt her zu mir, spricht Gottes Sohn (Riemenschneider 370) | covered | 1 | 422 | authentic | 1781.3 |
| Als vierzig Tag’ nach Ostern war (Riemenschneider 208) | covered | 1 | 338 | plagal | 1458.1 |
| Es woll’ uns Gott genädig sein (Riemenschneider 16) | NOT COVERED | 2 | — | — | — (phrase 2/2: search exhausted (works at width 128)) |
| Freu’ dich sehr, o meine Seele (Riemenschneider 64) | covered | 1 | 340 | authentic | 1409.0 |
| O Haupt voll Blut und Wunden (Riemenschneider 74) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Hilf, Gott, daß mir’s gelinge (Riemenschneider 302) | covered | 1 | 292 | plagal | 1159.0 |
| O Gott, du frommer Gott (Riemenschneider 312) | covered | 6 | 380 | authentic | 1446.6 |
| O Haupt voll Blut und Wunden (Riemenschneider 80) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Hast du denn, Jesu, dein Angesicht gänzlich verborgen (Riemenschneider 90) | covered | 3 | 295 | authentic | 1184.8 |
| Ach was soll ich Sünder machen (Riemenschneider 39) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 128)) |
| Seelenbräutigam (Riemenschneider 141) | covered | 1 | 240 | none | 986.1 |
| Freu’ dich sehr, o meine Seele (Riemenschneider 29) | covered | 1 | 343 | plagal | 1414.8 |
| Meinen Jesum laß’ ich nicht, Jesus (Riemenschneider 151) | covered | 1 | 171 | authentic | 873.8 |
| Lobet den Herren, denn er ist sehr freundlich (Riemenschneider 227) | covered | 6 | 379 | authentic | 1472.1 |
| Was betrübst du dich, mein Herze (Riemenschneider 237) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Christe, der du bist Tag und Licht (Riemenschneider 245) | covered | 1 | 254 | plagal | 978.5 |
| Was frag’ ich nach der Welt (Riemenschneider 255) | covered | 2 | 387 | authentic | 1362.2 |
| Helft mir Gott’s Güte preisen (Riemenschneider 123) | covered | 1 | 392 | none | 1611.4 |
| Jesus, meine Zuversicht (Riemenschneider 175) | covered | 1 | 199 | none | 855.9 |
| O Lamm Gottes, unschuldig (Riemenschneider 165) | covered | 1 | 322 | authentic | 1468.6 |
| O wie selig seid ihr doch, ihr Frommen (Riemenschneider 213) | covered | 2 | 306 | authentic | 1060.3 |
| O Mensch, schau’ Jesum Christum an (Riemenschneider 203) | covered | 1 | 269 | authentic | 1208.1 |
| Ein Lämmlein geht und trägt die Schuld (Riemenschneider 309) | covered | 1 | 584 | authentic | 2225.2 |
| Sanctus, Sanctus Dominus Deus Sabaoth (Riemenschneider 319) | covered | 3 | 401 | plagal | 1764.4 |
| Christ lag in Todesbanden (Riemenschneider 261) | covered | 2 | 485 | plagal | 1739.2 |
| Nun ruhen alle Wälder (Riemenschneider 117) | covered | 1 | 344 | authentic | 1470.3 |
| Herzlich lieb hab’ ich dich, o Herr (Riemenschneider 107) | covered | 1 | 796 | authentic | 3797.4 |
| Herr Jesu Christ, mein’s Lebens Licht (Riemenschneider 295) | covered | 1 | 302 | authentic | 926.0 |
| Wär’ Gott nicht mit uns diese Zeit (Riemenschneider 285) | covered | 1 | 322 | plagal | 1269.3 |
| Sei Lob und Ehr’ dem höchsten Gut (Riemenschneider 354) | covered | 1 | 326 | authentic | 1398.9 |
| Nun danket alle Gott (Riemenschneider 32) | covered | 5 | 235 | none | 930.0 |
| Schmücke dich, o liebe Seele (Riemenschneider 22) | covered | 1 | 475 | authentic | 2057.3 |
| In allen meinen Taten (Riemenschneider 50) | covered | 1 | 364 | authentic | 1756.2 |
| Alles ist an Gottes Segen (Riemenschneider 128) | covered | 1 | 378 | none | 1546.3 |
| Ach Gott und Herr (Riemenschneider 40) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (chordal seventh resolution)) |
| Jesu, meine Freude (Riemenschneider 138) | covered | 1 | 288 | plagal | 1106.9 |
| Wo Gott der Herr nicht bei uns hält (Riemenschneider 336) | covered | 1 | 356 | plagal | 1108.9 |
| Allein Gott in der Höh’ sei Ehr’ (Riemenschneider 326) | covered | 1 | 338 | authentic | 1217.2 |
| Von Gott will ich nicht lassen (Riemenschneider 191) | covered | 1 | 321 | authentic | 1210.0 |
| Gott hat das Evangelium (Riemenschneider 181) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 128)) |
| Mach’s mit mir, Gott, nach deiner Güt’ (Riemenschneider 310) | covered | 1 | 247 | plagal | 929.8 |
| Nun lob’, mein’ Seel’, den Herren (Riemenschneider 268) | covered | 1 | 642 | plagal | 2961.5 |
| Warum betrübst du dich, mein Herz (Riemenschneider 300) | covered | 1 | 301 | plagal | 1285.7 |
| Wie schön leuchtet der Morgenstern (Riemenschneider 278) | covered | 2 | 338 | authentic | 1303.7 |
| Freu’ dich sehr, o meine Seele (Riemenschneider 76) | covered | 1 | 364 | authentic | 1269.1 |
| Christ, unser Herr, zum Jordan kam (Riemenschneider 66) | covered | 1 | 509 | plagal | 1780.1 |
| O Herre Gott, dein göttlich Wort (Riemenschneider 14) | covered | 1 | 373 | authentic | 1535.1 |
| Es ist das Heil uns kommen her (Riemenschneider 4) | covered | 1 | 324 | authentic | 1149.1 |
| Es ist gewißlich an der Zeit (Riemenschneider 362) | covered | 1 | 361 | authentic | 1482.6 |
| Mit Fried’ und Freud’ ich fahr’ dahin (Riemenschneider 49) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| Liebster Jesu, wir sind hier (Riemenschneider 131) | covered | 1 | 254 | plagal | 974.6 |
| Herzliebster Jesu, was hast du verbrochen (Riemenschneider 59) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (secondary dominant resolution)) |
| Werde munter, mein Gemüte (Riemenschneider 121) | covered | 1 | 491 | plagal | 1776.8 |
| Nun laßt uns Gott, dem Herren (Riemenschneider 257) | covered | 1 | 231 | none | 1109.5 |
| Ich dank’ dir schon durch deinen Sohn (Riemenschneider 188) | covered | 1 | 267 | authentic | 1029.1 |
| Christus, der uns selig macht (Riemenschneider 198) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (voice overlap)) |
| O Jesu Christ, du höchstes Gut (Riemenschneider 92) | covered | 1 | 291 | plagal | 1092.8 |
| O großer Gott von Macht (Riemenschneider 82) | covered | 8 | 326 | half | 1401.0 |
| Heilig, heilig (Riemenschneider 235) | covered | 3 | 401 | plagal | 1761.1 |
| Gott, der du selber bist das Licht (Riemenschneider 225) | covered | 1 | 325 | plagal | 1271.1 |
| Alle Menschen müssen sterben (Riemenschneider 153) | covered | 1 | 295 | plagal | 1312.3 |
| Ach Gott und Herr, wie groß und schwer (Riemenschneider 279) | covered | 2 | 260 | none | 1145.8 |
| Ach, lieben Christen, seid getrost (Riemenschneider 301) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Jesu, der du meine Seele (Riemenschneider 269) | covered | 1 | 375 | none | 1418.7 |
| Dank sei Gott in der Höhe (Riemenschneider 311) | covered | 1 | 336 | authentic | 1445.6 |
| Freu’ dich sehr, o meine Seele (Riemenschneider 67) | covered | 1 | 340 | authentic | 1469.6 |
| In dich hab’ ich gehoffet, Herr (Riemenschneider 77) | covered | 2 | 380 | plagal | 1371.2 |
| An Wasserflüssen Babylon (Riemenschneider 5) | covered | 1 | 584 | authentic | 2342.7 |
| O Welt, sieh’ hier dein Leben (Riemenschneider 363) | covered | 1 | 405 | plagal | 1361.3 |
| Hilf, Gott, daß mir’s gelinge (Riemenschneider 199) | covered | 1 | 292 | plagal | 1192.1 |
| Herr Jesu Christ, wahr’r Mensch und Gott (Riemenschneider 189) | covered | 1 | 211 | authentic | 918.0 |
| Was mein Gott will, das g’scheh’ allzeit (Riemenschneider 120) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 64)) |
| Meine Seel erhebet den Herrn (Riemenschneider 130) | covered | 1 | 147 | none | 800.6 |
| Ach wie nichtig, ach wie flüchtig (Riemenschneider 48) | covered | 1 | 228 | none | 1250.1 |
| Singt dem Herrn ein neues Lied (Riemenschneider 246) | covered | 1 | 348 | authentic | 1358.5 |
| Jesu, deine tiefen Wunden (Riemenschneider 256) | covered | 1 | 340 | authentic | 1427.4 |
| Das walt’ Gott Vater und Gott Sohn (Riemenschneider 224) | covered | 1 | 268 | authentic | 1221.9 |
| Gott lebet noch (Riemenschneider 234) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 256)) |
| Schwing’ dich auf zu deinem Gott (Riemenschneider 142) | NOT COVERED | 1 | — | — | — (phrase 1/1: search exhausted (works at width 128)) |
| Meinen Jesum laß’ ich nicht, weil er sich für mich gegeben (Riemenschneider 152) | covered | 1 | 334 | authentic | 1347.7 |
| Jesu Leiden, Pein und Tod (Riemenschneider 83) | covered | 1 | 425 | authentic | 1687.1 |
| Wach’ auf, mein Herz, und singe (Riemenschneider 93) | covered | 1 | 231 | none | 1120.4 |
| Herr Gott, dich loben alle wir (Riemenschneider 164) | covered | 1 | 227 | authentic | 1021.9 |
| Jesus Christus, unser Heiland, der den Tod überwand (Riemenschneider 174) | covered | 4 | 261 | authentic | 959.4 |
| O wir armen Sünder (Riemenschneider 202) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (chordal seventh resolution)) |
| Herr, ich denk’ an jene Zeit (Riemenschneider 212) | covered | 1 | 343 | plagal | 1372.7 |
| Es ist gewißlich an der Zeit (Riemenschneider 260) | covered | 1 | 337 | authentic | 1390.6 |
| Herr, wie du willst, so schick’s mit mir (Riemenschneider 318) | covered | 5 | 249 | authentic | 1048.9 |
| Befiehl du deine Wege (Riemenschneider 270) | covered | 2 | 315 | authentic | 1163.5 |
| Ach Gott, wie manches Herzeleid (Riemenschneider 308) | covered | 1 | 272 | half | 875.6 |
| Jesu Leiden, Pein und Tod (Riemenschneider 106) | covered | 1 | 365 | authentic | 1575.2 |
| Nun lob’, mein’ Seel’, den Herren (Riemenschneider 116) | covered | 1 | 661 | authentic | 2525.6 |
| O Haupt voll Blut und Wunden (Riemenschneider 345) | covered | 1 | 363 | half | 1313.6 |
| Nun ruhen alle Wälder (Riemenschneider 355) | covered | 1 | 386 | authentic | 1516.1 |
| Zeuch ein zu deinen Toren (Riemenschneider 23) | covered | 1 | 306 | authentic | 1268.1 |
| Herr, ich habe mißgehandelt (Riemenschneider 33) | covered | 1 | 289 | authentic | 1015.1 |
| Herr Jesu Christ, wahr’r Mensch und Gott (Riemenschneider 284) | covered | 1 | 342 | none | 1640.1 |
| Herr Jesu Christ, du höchstes Gut (Riemenschneider 294) | covered | 1 | 286 | authentic | 1189.4 |
| Als Jesus Christus in der Nacht (Riemenschneider 180) | covered | 1 | 198 | authentic | 862.6 |
| Herr, nun laß in Friede (Riemenschneider 190) | NOT COVERED | 1 | — | — | — (phrase 1/1: rule conflict (chordal seventh resolution)) |
| Warum sollt’ ich mich denn grämen (Riemenschneider 139) | covered | 1 | 332 | authentic | 1350.7 |
| Was mein Gott will, das g’scheh’ allzeit (Riemenschneider 41) | covered | 1 | 371 | plagal | 1472.5 |
| Keinen hat Gott verlassen (Riemenschneider 129) | covered | 1 | 311 | none | 1353.8 |
| Gelobet seist du, Jesu Christ (Riemenschneider 51) | covered | 1 | 311 | plagal | 1213.4 |
| Jesu, nun sei gepreiset (Riemenschneider 327) | covered | 1 | 666 | authentic | 3038.4 |
| O Gott, du frommer Gott (Riemenschneider 337) | covered | 1 | 474 | authentic | 2123.1 |
