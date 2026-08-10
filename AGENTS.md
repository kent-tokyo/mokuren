あなたは **mokuren** の主任Rustエンジニア兼、計算音楽理論エンジニアです。

# Project: mokuren

**mokuren** は、Rustで実装する **説明可能なsymbolic composition engine** です。

単なる自動作曲ライブラリではありません。

中心となる目的は、

> **音楽理論に基づいて作曲候補を生成・探索・評価し、なぜその作曲判断を採用したのか、なぜ他の候補を採用しなかったのかを説明できること**

です。

プロダクトの基本思想は以下です。

> Generate candidates → evaluate by music theory → search → select → explain.

仮の英語taglineは以下とします。

> **mokuren — a fast, explainable symbolic composition engine for exploring music-theoretic decisions.**

README、crate metadata、設計文書では当面この定義を使用してください。

---

# 0. 最初に行うこと

実装開始前に、必ず以下を行ってください。

1. repository全体を確認する
2. 既存コード、Cargo.toml、README、docs、tests、examples、CIを確認する
3. 既存実装がある場合は、それを壊さず再利用できる部分を整理する
4. music21、SCAMP、MusPy、Abjad、Tonal、Euterpea、tunes等の設計思想を参考にする
5. ただし単なる既存ライブラリの移植にはしない
6. mokuren独自の価値を「reasoning / search / explainability」に置く
7. 実装前に `PLAN.md` または既存のtask管理文書へ具体的なフェーズ計画を書く

調査だけで終了せず、実装可能なところまで進めてください。

---

# 1. v0.1の明確なスコープ

v0.1では「自由な自動作曲全般」を実装しないでください。

最初のvertical sliceは、

> **与えられた旋律に対してCommon Practice Harmonyに基づくSATB四声体和声を探索・生成し、その判断を説明する**

ことです。

例:

```text
Input melody:
C4 C4 G4 G4 A4 A4 G4

Key:
C major

Meter:
4/4

Style:
Common Practice
```

mokurenは、

```text
I → IV → I6 → V7 → I
```

のような和声だけを返すのではなく、その判断理由も返してください。

例:

```text
Position 4: V7 selected

Why selected:
+ strong dominant function before tonic
+ supports an authentic cadence
+ soprano moves by step
+ preserves a common tone
+ no parallel fifths
+ no parallel octaves

Alternative: vi
status: valid
score difference: -0.82

Why not selected:
- weaker dominant preparation
- less effective phrase-level tension

Alternative: ii6
status: valid
score difference: -0.41

Why not selected:
- higher total voice-leading cost
```

これがmokurenの最低限の成功条件です。

---

# 2. 最重要設計原則

## 2.1 Explainability is not an afterthought

説明は後からLLMに生成させてはいけません。

探索・評価の段階で、すべての判断根拠を構造化データとして保持してください。

例えば、

```rust
Decision {
    selected: CandidateId,
    alternatives: Vec<CandidateEvaluation>,
    reasons: Vec<Reason>,
}
```

のようなモデルを持たせます。

理由はenum等として構造化してください。

例:

```rust
enum Reason {
    HarmonicFunction {
        from: HarmonicFunction,
        to: HarmonicFunction,
        score_delta: f64,
    },

    VoiceLeading {
        total_motion: u32,
        common_tones: u8,
        contrary_motion: bool,
        score_delta: f64,
    },

    CadenceSupport {
        cadence: Cadence,
        score_delta: f64,
    },

    RuleViolation {
        rule: RuleId,
        severity: Severity,
    },
}
```

自然言語説明は、この構造化reason traceから生成してください。

---

# 3. `why()` と `why_not()` を第一級APIにする

mokurenの象徴的機能として、以下を設計してください。

```rust
result.why(position)
```

および

```rust
result.why_not(position, alternative)
```

例えば、

```text
Why V7?

+ dominant-function strength: +1.20
+ cadence support: +0.90
+ smooth soprano motion: +0.35
- tenor leap penalty: -0.12

Final local score: 8.72
```

また、

```text
Why not vi?

vi was valid and ranked #2.

+ common-tone preservation: +0.40
+ smooth alto motion: +0.21
- weak dominant preparation: -0.88
- phrase-level cadence score: -0.55

Final local score: 7.90
Difference from selected V7: -0.82
```

のように、

**「なぜこれか」だけではなく「なぜあれではないか」**

まで説明できる設計にしてください。

---

# 4. Symbolic music model

最低限、以下の型を設計してください。

```text
PitchClass
Pitch
Octave
Interval
Duration
Note
Rest

Key
Mode
ScaleDegree

Chord
ChordQuality
ChordInversion
RomanNumeral
HarmonicFunction

Voice
VoiceRange

TimePosition
Beat
Measure

Melody
Part
Passage
Score
```

設計では以下を重視してください。

* stringly typed APIを避ける
* 不正状態を可能な限り表現不能にする
* Rustらしいstrong typing
* deterministic behavior
* `Clone`, `Debug`, `PartialEq` 等を適切に利用
* 必要に応じて `serde` 対応
* `f64` の意味の曖昧な乱用を避ける
* public APIを早い段階から整理する

過度なtype-level programmingは不要です。

型安全性と実用性のバランスを取ってください。

---

# 5. 音楽理論モデル

v0.1では少なくとも以下を扱ってください。

## Tonality

* Major
* Natural / harmonic / melodic minorは必要性を検討
* diatonic scale degrees
* chromatic alterations

## Intervals

* generic interval
* quality
* semitone distance
* consonance / dissonance classification

## Chords

最低限:

```text
major triad
minor triad
diminished triad

major seventh
minor seventh
dominant seventh
half-diminished seventh
diminished seventh
```

## Roman numerals

最低限:

```text
I
ii
iii
IV
V
vi
vii°

inversions:
I6
I64
V6
V7
V65
V43
V42
etc.
```

v0.1ではsecondary dominant等を無理に広げる必要はありません。

ただし将来、

```text
V/V
V7/ii
N6
Ger+6
```

などを追加可能なデータモデルにしてください。

---

# 6. Harmonic function

Roman numeralとharmonic functionを混同しないでください。

少なくとも、

```rust
enum HarmonicFunction {
    Tonic,
    Predominant,
    Dominant,
}
```

相当の概念を持たせてください。

将来、

```text
Tonic prolongation
Dominant prolongation
Applied dominant
Chromatic predominant
Modal mixture
```

などを追加可能にします。

progression evaluationでは、

```text
T → PD → D → T
```

などのfunction-level reasoningを扱えるようにしてください。

---

# 7. SATB model

v0.1の中心です。

以下の4声部を明示的に扱います。

```text
Soprano
Alto
Tenor
Bass
```

各声部に、

* default range
* preferred tessitura
* current pitch

を持たせられるようにしてください。

最低限、

```text
voice crossing
voice overlap
maximum spacing
range violation
```

を検出可能にします。

Sopranoは入力旋律として固定可能にしてください。

---

# 8. Hard constraints と Soft preferences を分離する

これは重要です。

## Hard constraints

破った候補は原則として探索から除外します。

例:

```text
Voice range violation
Voice crossing
Forbidden parallel fifth
Forbidden parallel octave
Invalid chord spelling
Missing required chord tone
```

## Soft preferences

違反しても候補は残るが、スコアを下げます。

例:

```text
Prefer stepwise motion
Prefer common tones
Prefer contrary motion
Penalize large leaps
Avoid unnecessary repeated chords
Prefer strong harmonic progression
Prefer appropriate doubling
Prefer stylistically appropriate cadence
```

public API上でも、

```rust
Constraint
Preference
```

を混ぜない設計にしてください。

---

# 9. v0.1で実装するCommon Practice rules

最低限、以下を実装・テストしてください。

### Voice leading

* parallel perfect fifth detection
* parallel octave detection
* parallel unison detection
* direct / hidden fifthsについてはv0.1で実装するか評価
* direct / hidden octavesについては同様
* voice crossing
* voice overlap
* excessive melodic leap
* spacing between upper voices
* common-tone preservation
* contrary / oblique / similar motion classification

### Harmonic rules

* chord membership
* chord inversion
* chord doubling
* leading-tone resolution
* chordal seventh resolution
* basic dominant-to-tonic behavior
* basic predominant-to-dominant behavior

すべてを絶対的な「正解」として実装しないでください。

音楽理論上、流派や文脈によって扱いが変わるものは、

```rust
RuleProfile
StyleProfile
```

等で変更可能な設計にします。

---

# 10. Rule engine

各ruleは独立して評価可能にしてください。

イメージ:

```rust
trait Rule {
    fn evaluate(
        &self,
        context: &RuleContext,
    ) -> RuleResult;
}
```

返り値は単なるboolにしないでください。

例えば、

```rust
RuleResult {
    status: Pass | Warning | Violation,
    reasons: Vec<Reason>,
    penalty: f64,
}
```

のようなrich resultにしてください。

これにより、

* testしやすい
* diagnosticsが出せる
* explainabilityにつながる
* style profileで強度を変えられる

構造にします。

---

# 11. Candidate generation

各時点で、

```text
Soprano note
Key
Previous harmony
Current voices
Phrase context
```

から可能な和音・voicing候補を生成してください。

候補生成と候補評価は明確に分離します。

```text
CandidateGenerator
CandidateEvaluator
SearchEngine
```

を同じ巨大関数にまとめないでください。

また、探索可能性を高めるため、

```text
candidate generated
candidate rejected
candidate retained
```

の統計を取得できるようにしてください。

---

# 12. Search engine

v0.1ではまず **Beam Search** を第一候補にしてください。

例:

```rust
BeamSearch::new()
    .width(128)
```

ただし将来的に、

```text
A*
Dynamic Programming
Branch and Bound
Best-first search
MCTS
Constraint programming
```

へ差し替えられるinterfaceにしてください。

概念的には、

```rust
trait SearchStrategy {
    fn search(
        &self,
        problem: &CompositionProblem,
    ) -> SearchResult;
}
```

のような差し替え可能な設計を検討してください。

---

# 13. Score model

スコアは最初から説明可能でなければなりません。

単なる、

```rust
score: f64
```

ではなく、

```rust
ScoreBreakdown {
    harmonic_function: f64,
    voice_leading: f64,
    cadence: f64,
    melodic_motion: f64,
    doubling: f64,
    style: f64,
    penalties: Vec<Penalty>,
}
```

のように分解してください。

例えば、

```text
candidate V7:

harmonic function      +2.10
voice leading          +1.40
cadence                +1.80
common tones           +0.30
large leap             -0.25

total                   5.35
```

のような情報が必ず取得できるようにします。

---

# 14. Global reasoning と local reasoning

単一和音だけを局所評価する実装にしないでください。

最低限、

```text
current chord
previous chord
next-phrase expectations
cadence position
phrase boundary
```

を考慮できる設計にしてください。

将来的には、

```text
phrase-level tension
motivic development
large-scale harmonic plan
formal structure
```

を評価できるようにします。

v0.1で全部実装する必要はありません。

ただしデータモデルは拡張可能にしてください。

---

# 15. Diagnostics

探索結果には統計情報を持たせます。

例えば、

```text
Candidates generated: 18,420
Candidates retained:   2,184
Candidates rejected:  16,236

Top rejection reasons:

parallel fifths             4,182
voice range                 3,420
voice crossing              2,109
parallel octaves            1,882
unresolved leading tone     1,204
other                       3,439
```

さらに、

```rust
result.diagnostics()
```

で取得可能にしてください。

これはdebuggingだけではなく、mokurenの重要な説明機能です。

---

# 16. Determinism

同じ入力・設定なら、原則として同じ結果を返してください。

tie-breaking ruleを明示してください。

例えば、

```text
1. total score
2. voice-leading cost
3. harmonic candidate canonical ordering
4. voicing canonical ordering
```

のようにstable orderingを設計してください。

randomizationを導入する場合もseedableにします。

---

# 17. Public API

v0.1で目指すAPIの方向性:

```rust
use mokuren::prelude::*;

let melody = Melody::parse("C4 C4 G4 G4 A4 A4 G4")?;

let result = Composer::new()
    .key(Key::C_MAJOR)
    .style(Style::CommonPractice)
    .voices(Voices::SATB)
    .search(BeamSearch::new().width(128))
    .harmonize(melody)?;

println!("{}", result.explain());

println!(
    "{}",
    result.why_not(
        Position::new(4),
        RomanNumeral::VI
    )?
);
```

最終的なAPIはrepoの実装状況に応じて改善して構いません。

重要なのは、

```text
compose
analyze
explain
why
why_not
diagnostics
```

が自然に使えることです。

---

# 18. Output

v0.1ではまず内部symbolic representationを完成させてください。

優先順位は、

```text
1. Rust structured result
2. human-readable text explanation
3. JSON serialization
4. MIDI
5. MusicXML
```

です。

MIDIやMusicXMLのためにcore architectureを遅らせないでください。

音響合成はv0.1のスコープ外です。

mokurenはDAWやsynthesizerではありません。

---

# 19. Audio synthesisは原則スコープ外

以下は当面実装しないでください。

* oscillator
* synthesizer
* DSP effects
* realtime audio engine
* DAW
* plugin host
* VST implementation

必要であれば既存Rust ecosystemと接続できる設計に留めます。

mokurenの主戦場は、

> **symbolic composition reasoning**

です。

---

# 20. 将来の方向性

v0.1完了後に以下を検討します。

## Counterpoint

* species counterpoint
* two-part counterpoint
* multi-voice counterpoint
* dissonance treatment

## Melody generation

* contour
* interval distribution
* phrase structure
* motif generation
* motif transformation

## Harmonic expansion

* secondary dominants
* modal mixture
* augmented sixth chords
* Neapolitan
* modulation
* tonicization

## Musical form

* phrase
* period
* sentence
* binary
* ternary
* rondo
* sonata-related formal structures

## Style profiles

例:

```text
Common Practice
Bach Chorale
Classical
Romantic
Jazz
Modal
Impressionist
```

ただし「○○風」を単なる固定ルール集合として雑に実装しないでください。

スタイルの根拠を明示可能にします。

---

# 21. LLM integrationはcoreから分離する

将来的にLLMとの接続は有望ですが、mokurenの作曲判断自体をLLMに依存させないでください。

理想構成:

```text
Natural-language request
        ↓
       LLM
        ↓
Structured composition constraints
        ↓
      mokuren
        ↓
Verified symbolic composition
        ↓
Structured reason trace
        ↓
       LLM
        ↓
Natural-language explanation
```

つまり、

**LLMはinterfaceとして使えるが、音楽理論reasoningのsource of truthはmokuren**

とします。

---

# 22. Testing strategy

音楽理論ライブラリなのでtestを非常に重視してください。

最低限、

## Unit tests

* intervals
* pitch arithmetic
* chord spelling
* inversions
* Roman numerals
* voice motion
* parallel fifth detection
* parallel octave detection
* voice crossing
* leading-tone resolution
* chordal seventh resolution

## Golden tests

小規模なSATB passageについて、

```text
input
expected valid/invalid
expected major violations
expected selected harmony
```

を固定します。

## Property tests

可能なら `proptest` 等を検討してください。

例:

```text
interval inversion consistency
transpose + inverse transpose
pitch-class normalization
serialization round trip
```

---

# 23. Benchmarking

v0.1からperformance計測を可能にします。

計測候補:

```text
candidate generation / sec
candidate evaluation / sec
search nodes / sec
memory use
beam width scaling
melody length scaling
```

必要に応じてCriterionを使ってください。

ただし「Rustだから速い」という主張は、測定するまでREADMEに書かないでください。

---

# 24. External validation

可能なら、公開されているchorale datasetなどを使った検証方法を検討してください。

将来的には、

```text
known chorale
↓
remove alto/tenor/bass
↓
give soprano to mokuren
↓
harmonize
↓
compare
```

のようなbenchmarkを作りたいです。

ただし、

**原曲と一致すること = 正しい**

とは考えないでください。

評価軸として、

```text
rule violation rate
voice-leading quality
cadence correctness
harmonic plausibility
solution diversity
search coverage
```

等を検討してください。

ライセンスを必ず確認してください。

---

# 25. Documentation

READMEには最低限以下を含めてください。

1. mokurenとは何か
2. 何ではないか
3. minimal example
4. explainability example
5. why_not example
6. supported theory
7. current limitations
8. roadmap

特に、

```text
Why this decision?
Why not another one?
```

をREADMEの中心デモにしてください。

---

# 26. Architectureの目安

必要ならworkspace化を検討してください。

例えば、

```text
mokuren-core
mokuren-theory
mokuren-rules
mokuren-search
mokuren-compose
mokuren-analysis
mokuren-explain
mokuren-midi
mokuren-musicxml
mokuren-python
mokuren-wasm
```

ただしv0.1から無意味にcrateを細分化しないでください。

最初はmonorepo / single crate + modulesでも構いません。

実際の依存関係が明確になった時点で分割してください。

---

# 27. Non-goals

現段階では以下を追わないでください。

* DAW機能
* 高品質audio synthesis
* VST
* realtime live coding
* deep-learning music generation
* LLMによる直接作曲
* あらゆるジャンルへの対応
* music21全機能の再実装
* MIDI editor GUI
* notation editor GUI

「何でもできる音楽ライブラリ」にはしないでください。

---

# 28. 競争優位

mokurenの競争優位は、

```text
Rust
+
strong symbolic music model
+
music-theoretic constraint engine
+
search
+
structured scoring
+
decision trace
+
why()
+
why_not()
+
deterministic diagnostics
```

です。

特に、

**`why_not()` を象徴機能として扱ってください。**

「AIがそれっぽい曲を生成した」ではなく、

> **どの候補を検討し、何が違反し、何が優れていて、なぜこの判断に至ったかを追跡可能**

であることがmokurenの存在理由です。

---

# 29. 実装優先順位

以下の順で進めてください。

## Phase 1 — Foundations

* repository audit
* architecture
* Pitch / Interval / Key
* Chord
* RomanNumeral
* Voice / SATB
* tests

## Phase 2 — Rule engine

* hard constraints
* soft preferences
* structured RuleResult
* score breakdown
* basic voice-leading rules

## Phase 3 — Candidate generation

* harmonization candidates
* valid voicings
* candidate pruning
* diagnostics

## Phase 4 — Search

* Beam Search
* deterministic tie-breaking
* configurable beam width
* search diagnostics

## Phase 5 — Explainability

* DecisionTrace
* structured Reason
* `explain()`
* `why()`
* `why_not()`
* alternative ranking

## Phase 6 — End-to-end SATB demo

以下を実際に動作させてください。

```text
melody
→ SATB harmonization
→ progression
→ score
→ decision trace
→ explanation
→ why_not
```

## Phase 7 — Validation

* golden examples
* regression suite
* benchmark
* README demo
* limitations documentation

---

# 30. Quality requirements

各フェーズで、

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

相当をgreenにしてください。

既存CIがある場合はそれに従ってください。

unsafeは原則使用しないでください。

panic可能性のあるpublic APIを避け、適切なerror typeを設計してください。

public APIにはrustdocを書いてください。

---

# 31. 実装判断の原則

迷った場合は、以下の優先順位で判断してください。

```text
1. theoretical correctness
2. explainability
3. deterministic behavior
4. API clarity
5. testability
6. extensibility
7. performance
8. feature breadth
```

performanceのために説明可能性を壊さないでください。

feature数を増やすために音楽理論上の意味を曖昧にしないでください。

---

# 32. 報告方法

細かい作業ごとに確認を求めず、自律的に進めてください。

ただし、重要な仮定や設計変更は記録してください。

各マイルストーンでは、

```text
Implemented
Tests
Measured results
Known limitations
Design decisions
Next step
```

を簡潔に残してください。

「実装したつもり」ではなく、

* test
* benchmark
* concrete example
* generated diagnostic

の証拠を可能な限り提示してください。

---

# 最終目標

mokurenが最終的に目指すのは、

> **音楽を生成するブラックボックス**

ではありません。

目指すのは、

> **音楽理論上の作曲判断を探索し、その判断過程を検証・比較・説明できるreasoning engine**

です。

ユーザーが最終的に、

```text
Why did you choose V7 here?
```

だけでなく、

```text
Why didn't you choose vi?
What would change if I allowed parallel fifths?
What is the cheapest way to make this cadence stronger?
Which rule is preventing this harmonization?
What alternatives were almost selected?
```

と問い、それにmokuren自身の計算結果を根拠として答えられる状態を目指してください。

まずrepositoryを精査し、v0.1のarchitectureとPhase 1を開始してください。
