# Lessons

Process lessons from building mokuren so far — not music theory notes (those
belong in code comments near the relevant rule) and not a changelog (that's
git log). This is for *how to work on this codebase* correctly next time.

## Hand-proofs of pitch-spelling math are unreliable — verify with proptest, don't trust the derivation

Twice in one session, a manually-reasoned "this is always safe" claim about
`spell_above`/`Accidental` turned out to be wrong, and proptest caught both:

1. `Chord::pitch_classes()` silently fell back to `Natural` when a chord
   tone needed an accidental beyond double-flat/double-sharp. Adding a
   property test for it (`Cbb` + `MinorSeventh`) found the fallback
   immediately.
2. Fixing (1), the doc comment on `Key::diatonic_pitch_class` claimed
   major-scale construction "never exceeds the representable accidental
   range for *any* tonic" — a hand-derived claim that felt airtight (spot-
   checked two tonics, both worked). A property test using *any* pitch
   class as tonic (not just practical ones) found a counterexample in
   seconds: a double-sharp tonic needs a triple-sharp for its own third.

**The pattern**: combinatorial pitch-class arithmetic (letter steps ×
semitone offsets × accidental range) has enough interacting cases that
"I checked a couple of examples and the algebra looks consistent" is not
a proof. When a change touches `spell_above`, `accidental_for_offset`, or
anything that spells a pitch from an arbitrary root, add a property test
covering the full input space *before* trusting an `expect()` or doc
comment that claims something is always safe. If proptest can't find a
counterexample in ~256 cases, that's actual evidence; hand-checking two
inputs is not.

## Hand-built "golden" voicings need pair-by-pair verification, not spot-checking

Building `tests/golden.rs`'s valid-progression case, an early draft of the
V7→I resolution had a genuine parallel fifth between tenor and bass — not
caught until checking *every* voice pair's interval class across *every*
transition by hand, because spot-checking "does the soprano/bass look
okay" missed an inner-voice pair. Same lesson as above in a different
domain: for something claimed "textbook valid," verify exhaustively
(all 6 voice pairs × all hard rules) rather than trusting that a
plausible-looking voicing is actually correct. This is *why* golden tests
earn their keep even in a small engine — writing one forces the
exhaustive check that a passing intuition skips.

## Score-weight changes need actual search runs, not just unit-level reasoning about one rule

Raising `CadenceSupportRule`'s authentic-cadence bonus (to fix a search
that avoided ending on tonic) had a side effect invisible from reading the
rule in isolation: with the melody used in testing, no diatonic
dominant-function chord contains the note the soprano needs at that
position, so an authentic cadence was *never reachable* for that melody at
all — the fix that "should" have worked kept failing until diagnosed by
actually running full beam searches at several weights and reading the
resulting progressions, not by re-deriving the expected outcome from the
rule table. Lesson: after any weight change, run the actual search across
representative inputs before trusting the change did what it was supposed
to; the rule table's *intent* and the search's *emergent behavior* are not
the same thing, especially once beam pruning is in the loop.

## Beam width interacts with where rewards live in the search, not just search quality in general

`CadenceSupportRule` only rewards the *final* position. A beam narrower
than ~28 (for the melody/rule-weights combination tested) pruned away the
eventually-best path before that reward could ever apply — the classic
horizon effect, but easy to miss because a narrow beam still returns *a*
valid, plausible-looking harmonization; it just wasn't the best one
reachable. Any future end-loaded reward (a new cadence type, a phrase-final
preference) should come with a note about whether the default beam width
is still wide enough, verified empirically (a width sweep), not assumed.

## For a niche/new package, go straight to docs.rs or the repo — general web search returns nothing

Verifying `music-comp-mt`'s actual feature set (for the ROADMAP competitor
table) via generic web search returned no results at all — it's real,
published, MIT-licensed, and on crates.io, just too small/new to be
indexed by a general search. Fetching `docs.rs/music-comp-mt` directly
(and drilling into its `harmonize`, `harmony`, and `analysis` module pages
specifically) got accurate, module-by-module detail that generic search
couldn't. Lesson: for a specific Rust crate, prefer docs.rs (or the repo's
own README) over web search from the start, rather than treating web
search's silence as "this doesn't exist" or falling back to writing
claims from memory.

## "Old music = public domain = free to vendor" is wrong; check the specific encoding

Bach died in 1750 — the compositions are unambiguously public domain. That
told us nothing about whether any *specific digital encoding* of the
chorales (music21's corpus, a Humdrum `**kern` file on GitHub, a kernScores
mirror) can be committed into mokuren's repository. Checked four candidate
sources for BENCHMARK.md and found real, differing restrictions on every
one (commercial-use prohibition, missing license files, ambiguous GitHub
license detection) — none of which would have been obvious without
checking each source's own stated terms individually. Lesson: "the
underlying work is old/public domain" and "this specific file is safe to
redistribute" are separate questions; answer the second one explicitly,
per source, before vendoring anything.

## A single synthetic test melody hid a real coverage gap that 20 real chorales found in minutes

The whole reason BENCHMARK.md exists is that mokuren's rules were tuned
against one melody. That risk turned out to be real, immediately: the
first 20 real chorales run through the finished harness had only 50%
coverage, not the ~100% every synthetic fixture and unit test had shown.
Bisecting one failure to its shortest failing prefix (binary search over
melody length, six lines of throwaway Rust) found the cause in minutes: a
non-diatonic soprano tone (a chromatic passing tone or secondary dominant
mokuren's diatonic-only engine has no chord for) — not a beam-width issue
(checked up to width 512, still failed), not a bug, just a real scope
boundary that a hand-picked stepwise test melody was never going to
exercise. Lesson: when a search-based system reports a bare "no valid
result" on real data, don't guess at the cause (wider beam? different
weights?) — bisect the input first. It's cheap (a binary search over
melody length is a few lines) and turns "something's wrong somewhere" into
an exact, minimal, explainable repro before touching any code.

## Vendoring risk doesn't end at "pick a licensed source" — the *derived* data can still leak the original

Deciding music21 as the canonical corpus source didn't finish the
vendoring question. Extracting chorales and writing `.chorale` files is
itself a redistribution act — committing those output files to mokuren's
repo would recreate the exact problem "reference, don't vendor" existed to
avoid, just one layer removed (redistributing a derived encoding instead
of the original one). The adapter script was safe to commit (it's just
code, contains no chorale data); its *output* was validated locally and
then deleted, never committed. Lesson: when the plan is "reference an
external source, don't vendor it," check every artifact a tool built for
that reference produces, not just the original source — a derived file
committed to the repo is still vendoring.

## A 20-chorale sample and a 144-chorale baseline agreed on the headline number but not on the shape of the problem

The 20-chorale validation run found ~50% coverage and one diagnosed
failure (chromatic soprano). The full 144-chorale major-mode baseline
landed on almost the same coverage (50.7%) — but only because the
dominant failure category (chromatic soprano, 88.7%) was large enough to
show up even in 20 samples. The smaller categories didn't: the full run
found 5 voice-range rule conflicts and 1 chordal-seventh-resolution
conflict that a 20-chorale sample had zero chance of surfacing at that
rate (5/144 ≈ 3.5%; expected count in a 20-chorale sample is <1). It also
answered a question the small sample couldn't: whether the cadential-6/4
rule (assumed, going in, to be a top-3 issue worth fixing before secondary
dominants) actually causes any failures at scale — it caused zero in 144
chorales, which reordered ROADMAP.md. Lesson: a small validation sample is
enough to prove "this component has a real gap, worth building the full
benchmark" (that's all it was ever used for), but is the wrong tool for
*prioritizing what to fix next* — that needs the full run, because rare-
but-real failure categories and "assumed but actually zero" categories
both require enough samples to appear at their true rate. Don't stop at
a validation sample and start reordering the roadmap from it.

## Adding a new "correct" score reward can still break an unrelated, fully-diatonic demo — check by running the search, not by reasoning about the one rule you touched

Building secondary dominants, a fully-correct-in-isolation reward
("resolving an applied dominant to its target scores like a strong
resolution") broke `tests/spine.rs`'s pinned demo — a melody with *no
chromatic notes at all*. The mechanism wasn't obvious from reading the
new rule alone: the reward was keyed to *any* correct applied-dominant
resolution, including `V/V -> V` (the applied dominant of the dominant,
resolving right back to a plain, already-reachable diatonic chord). That
gave the search a "free" way to rack up reward on a melody where every
note was already harmonizable diatonically, by gratuitously substituting
an applied dominant for a diatonic chord it merely *fit*, not one it was
*needed* for — and that was enough to make the search prefer ending the
whole piece on the dominant over the correct tonic close, at the
project's own default beam width. Widening the beam did NOT fix it
cleanly: 256 still picked the wrong ending, 512 was needed — a huge,
fragile jump from the documented default of 32, and a sign the real
problem was the reward's shape, not the beam. The actual fix was scoring
an applied dominant's *introduction* as neutral (0.0) rather than
rewarding it like an arrival at the true dominant, so it's chosen only
when its other advantages (voice leading, or being the *only* option for
a genuinely chromatic tone) justify it.

**The pattern**: this is the same lesson as "score-weight changes need
actual search runs, not just unit-level reasoning about one rule"
(above), but the trap here was sneakier — the new rule was correct on
every unit-level check (`cargo test` was green throughout), and the
regression only showed up on a completely unrelated pinned example that
had nothing to do with the new feature. Any new *reward* (not just a new
weight value) added to a shared scoring table needs its blast radius
checked against material the feature wasn't built for, not just material
it was — a width sweep (`for width in [8, 16, 32, 64, ...]`) on an
existing golden example is cheap and found this in minutes.

## An asymmetrically-generated voicing means a hard-coded range on the *fixed* voice is a sharper failure mode than the same range on a *generated* one

The "voice range" failure cluster (5/144 baseline chorales) looked, before
investigation, like it could be five different small issues. It was one:
every one of those 5 chorales had a soprano note on A5, one step above
`VoicePart::Soprano`'s old default ceiling (G5). The reason this killed
the *entire* harmonization rather than just narrowing the options: alto/
tenor/bass are always *generated* within their own range
(`pitches_in_range` filters candidate pitches before they're ever
scored), so they can never individually cause a range violation — but
soprano is *given*, taken directly from the input melody at every
position, never filtered. A hard-coded range on a value the engine
controls fails soft (fewer options); the same kind of range on a value
it doesn't control fails hard (zero options, `NoValidHarmonization`) the
moment real material exceeds it. Lesson: when auditing a hard-coded
bound, ask which side of the generate/accept boundary it sits on —
bounds on inputs the system doesn't control deserve a wider, evidence-
based margin (or explicit handling for "out of range"), not the same
tolerance as a bound on the system's own generated output.

## `comm` needs its input sorted the way *it* compares, not the way you sorted it

Diffing the v0.1.0 and v0.2.0-in-progress per-chorale coverage lists
(`comm -23 old_covered.txt new_covered.txt`, both piped through `sort -n`
first) reported 20 chorales as regressions that weren't real — `comm`
compares lines byte-wise (lexicographically) regardless of how the input
was sorted, so numerically-sorted multi-digit IDs (`sort -n`: ...,7, 22,
24, ...) don't match its own ordering assumption (lexicographically, "22"
sorts before "7"), and it silently produces wrong output instead of an
error. Caught only because the count (20 "regressions" out of 73) was
implausibly high for what the data should show — re-running with plain
`sort` (lexicographic, matching `comm`'s actual comparison) dropped it to
the real number (4, all independently confirmed beam-width-recoverable).
Lesson: `comm`/`sort -c` default to byte-wise comparison; `sort -n` and
`comm` silently disagree unless told to agree. When a diff-style tool's
output looks too large or too small to be plausible, don't rationalize
it — check whether the tool's comparison order actually matches the
sort order fed into it.
