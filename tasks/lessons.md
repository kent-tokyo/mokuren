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
