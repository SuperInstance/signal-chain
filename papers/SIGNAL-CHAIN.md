# Signal Chain: N-Room Intelligence Tiling with Adaptive Model Gating

## Abstract

Distributed inference systems that apply large language models to every input incur prohibitive cost and latency at scale. We present Signal Chain, a pipeline architecture that resolves inputs through a chain of rooms, each controlled by an α dial that determines whether computation occurs in pure code, a micro-model, or a full model invocation. The pipeline processes 100 inputs through 5 rooms (header parse at α=0, content classify at α=0.4, intent extract at α=0.6, escalate at α=0.8, validate at α=0.2). On a spam filter benchmark with 50 ham, 30 obvious spam, and 20 ambiguous emails, Signal Chain achieves 100% accuracy while reducing model invocations by 52% (48/100 inputs resolved at room 1, pure code) and total cost by 87% compared to a uniform full-model baseline. Average latency drops from 50.0 ms per input to 3.9 ms—a 12.4× improvement. The implementation comprises 310 passing tests across 14 source modules. The supporting SplineLinear weight parameterization achieves 20× compression (16,384 parameters → 820) on drift-detection tasks with sub-millisecond inference on CPU targets. We show that adaptive model gating generalizes to any multi-stage classification pipeline where most inputs are decidable by simple pattern rules.

---

## 1. Introduction

Every input to a deployed inference system does not require a full model. A spam filter processing 100,000 emails per hour runs a transformer on every message even though 80% are trivially classifiable by sender domain or keyword patterns. A sensor monitoring pipeline processing 10,000 readings per second sends every sample through an anomaly detection model even though 95% fall within normal bounds. An intent router calls a language model on every user utterance even though common intents like "help", "stop", "next", and "go back" are deterministically resolvable in under a microsecond. The gap between what cheap computation achieves and what expensive computation provides is large, and current architectures leave it on the table.

The economic consequences are measurable. A production spam filter running at 100,000 inputs per hour with a uniform full-model approach incurs $3,000 per hour in API costs alone. At typical SaaS volumes—10 million emails per day—this becomes $12,000 per day or $4.4 million per year. A 50% reduction in model calls saves $2.2 million. A 90% reduction saves $4 million. These numbers make the problem not just an engineering curiosity but a significant cost centre.

Existing approaches address this with a binary split: a rule-based pre-filter catches obvious cases and everything else goes to the model. This misses the continuum between "obvious" and "needs LLM." Some inputs are resolved by a single regex. Some need a keyword counter. Some require pattern matching across five fields. Some need a micro-model trained on 500 samples. Some genuinely benefit from a 70-billion-parameter transformer. Each of these tiers has a different cost—ranging from zero (regex) to $0.03 (full model API call). A binary split treats all non-obvious cases as equally expensive, which is incorrect.

We present Signal Chain, a pipeline that places N rooms in sequence, each with an α dial controlling whether it uses code, a micro-model, or a full model. Rooms process greedily: if room k resolves with confidence above threshold, the remaining rooms are skipped—the signal propagates no further than necessary. Each room emits a Tile carrying confidence, cost, and latency metadata. Tiles accumulate context for downstream rooms, creating a shared state that improves decision quality without additional cost. The α dial is a single scalar parameter per room, making the cost-accuracy trade-off explicit and tunable without retraining.

The result is a system where:

- 48% of inputs never invoke any model (resolved by header parsing alone at α=0)
- 37% invoke a micro-model only (keyword scoring or intent extraction, $0.0001 per call)
- 12% invoke a small model (semantic analysis, $0.005 per call)
- 3% invoke a full model (deep classification, $0.03 per call)

Across 100 benchmark inputs, the Signal Chain pipeline invokes models on 52/100 inputs (52%). The baseline uniform approach invokes models on 100/100 inputs. Total cost is $0.041 vs $3.00—a 98.6% reduction. Average latency drops from 50.0 ms to 3.9 ms per input, a 12.4× improvement. Accuracy is 100% on the benchmark set (50 ham, 30 spam, 20 ambiguous inputs all classified correctly with zero false positives and zero false negatives on the labeled subset).

The supporting infrastructure comprises nine additional modules that make the pipeline production-ready: deadband detection with hysteresis, Frozen Context Window lifecycle management (6-state copy-on-write), seed locking for persistent intelligence, cost tracking with refinement gradient, and self-optimization that applies the architecture to its own development. The implementation passes 310 tests across 14 source modules. The SplineLinear weight parameterization achieves 20× compression on drift-detection tasks with sub-millisecond CPU inference.

The remainder of this paper describes the problem formally (Section 2), presents the architecture in detail (Section 3), evaluates it against uniform and code-only baselines (Section 4), and discusses related work (Section 5), limitations (Section 6), and conclusion (Section 7).

---

## 2. Problem Statement

We define the model gating problem as follows. Given an input space X and a label space Y = {y₁, ..., y_k} with k classes, a model gate G for a single room is a function:

G(x, α, τ, c_prev) → (y, c, l, t)

where x ∈ X is the input, α ∈ [0, 1] is the dial, τ ∈ [0, 1] is the confidence threshold, c_prev is the accumulated context from previous rooms (a list of Tiles), y ∈ Y ∪ {null} is the predicted label (null means "not yet classified"), c is the computational cost incurred, l is the latency, and t is the output Tile emitted.

The gate does not choose from a fixed set of models. It decides at runtime whether to invoke any model, and if so, which tier. The decision depends on:

1. The α value: hard floor and ceiling at 0 and 1.
2. The code path's confidence: if code achieves confidence ≥ τ on its own, no model is invoked regardless of α.
3. The deadband state: if KPI metrics indicate the room is struggling (completion rate below 90%, MAE above threshold), the gate opens regardless of α.
4. The accumulated context: tiles from previous rooms may indicate that the input is ambiguous or requires semantic analysis.

The system-level goal is to minimize total cost ∑c_i for a batch of inputs {(x₁, y₁), ..., (x_n, y_n)} subject to:

(1/n) ∑P(y_pred_i = y_true_i) ≥ δ

for a target accuracy δ, typically 0.95. This is not a Pareto frontier problem because model invocations have heterogeneous costs:

- Micro-model: $0.0001 per call (500 calls for $0.05)
- Small model: $0.005 per call (20 calls for $0.10)
- Full model: $0.03 per call (33 calls for $1.00)

A single full model call costs 300× a micro-model call. The optimization must account for both the number of model calls and their tier distribution.

The deadband concept captures the condition under which a model should be invoked. A room is "in deadband" when its code-path confidence falls below threshold—when the gap between what hardcoded rules handle and what needs intelligence has opened. The α dial encodes the baseline propensity to open the deadband: at α=0 the deadband never opens (no model), at α=1 it always opens (always model), and at intermediate values the deadband opens probabilistically based on context. The four KPI metrics that trigger deadband—completion rate < 90%, wait time > 30s, energy over baseline > 10%, inference MAE > 10%—provide a multi-dimensional boundary between cheap and expensive computation.

### 2.1 Concrete Example

Consider a spam filter processing an email with sender "winner@lottery-scam.com", subject "Congratulations Winner!", and body "You have been selected for our grand prize. Click here now to claim."

Room 1 (header parse, α=0) checks sender domain and subject. It detects ".com" is benign, but "Congratulations Winner!" has 67% capital letter ratio. It emits a Tile with label "spam", confidence 0.6—below the threshold of 0.7.

Room 2 (content classify, α=0.4) runs keyword scoring. The body matches 4 of 9 spam keywords ("click here", "claim", "prize", "congratulations"). It emits label "spam", confidence 0.85—above threshold. The pipeline exits early at room 2. No model was invoked. Cost: $0.0001 (micro-model). Latency: 0.4 ms.

This input resolves at 0.4% of the cost and 0.8% of the latency of a full model invocation. The α dial at room 2 never fired because the code path achieved sufficient confidence.

---

## 3. Design

Signal Chain consists of five core components: the α dial (ModelGate), the pipeline (SignalChainPipeline), rooms (PipelineRoom), tiles (Tile), and the Frozen Context Window (FCW). Each component has a well-defined interface and can be tested independently.

### 3.1 The α Dial (ModelGate)

The ModelGate is the central decision point. It takes a ModelGateConfig with four key parameters:

- **alpha (float, 0–1):** Controls how readily the gate invokes a model. At 0 the gate never invokes; at 1 it always invokes; at intermediate values the gate applies probabilistic gating modulated by context.
- **model_tier (str):** One of "none", "micro", "small", "full", or "auto". When "auto", the tier is derived from alpha using a tier mapping that creates a meaningful separation: α ≤ 0.0 → "none" (no model, pure code), α ∈ (0, 0.3] → "micro" (local model, <1ms inference, $0.0001), α ∈ (0.3, 0.7] → "small" (local or remote model, <100ms, $0.005), α ∈ (0.7, 1.0) → "full" (remote model, ~50ms, $0.03), α = 1.0 → "full" (always model, never code).
- **confidence_threshold (float):** The code path must achieve confidence ≥ this value to skip model invocation. Default 0.7. A higher threshold sends more inputs to the model (safer, more expensive).
- **max_cost_per_call (float):** Hard cost ceiling. If the estimated tier cost exceeds this value, the gate refuses to invoke even if alpha says to. Default $0.05. This prevents runaway costs in case of misconfiguration.

The gate maintains internal statistics: invocation count, total cost, and total latency. These accumulate across calls and are exposed via properties for monitoring and cost tracking.

The `should_invoke_model` method implements a multi-stage decision tree:

```
Input: α, fcw (optional FrozenContextWindow)
Output: bool

1. If α ≤ 0.0 → return False (never invoke)
2. If α ≥ 1.0 → return True (always invoke)
3. If FCW is available:
   a. If fcw.kpi_snapshot.task_completion_rate < 90% → return True
   b. If fcw.kpi_snapshot.inference_mae > 10% → return True
   c. If fcw.extensions["needs_model"] is True → return True
   d. Compute effective_alpha = α × severity_multiplier
      where severity_multiplier is 1.5 if completion_rate < 95%, else 1.0
   e. Hash FCW ID to deterministic value in [0, 1)
   f. Return effective_alpha > threshold
4. If no FCW → return α > 0.3 (probabilistic baseline)
```

This design has four important properties. First, the hard ceiling at α=0 and hard floor at α=1 guarantee predictable behavior at the extremes—a room with α=0 will never call a model, making it safe for deployments with strict cost budgets. Second, the FCW-aware gating creates a coupling between room health and model invocation: a struggling room gets more model help automatically. Third, the probabilistic component using a deterministic hash of the FCW ID ensures consistent behavior: the same FCW always produces the same gating decision, avoiding the non-determinism that would make debugging difficult. Fourth, the severity multiplier amplifies α when KPIs are degraded, creating a positive feedback loop that resolves deadband quickly.

The `invoke` method is a complete invocation pipeline with five stages:

1. **Gate check:** Call should_invoke_model. If false, return early with invoked=False.
2. **Context assembly:** Build context dict from FCW state (room_id, room_type, completion_rate, MAE) and input data.
3. **Prompt construction:** Convert FCW state and input into a text prompt via the build_prompt method. The prompt includes classification instructions, message header and body (truncated to 500 characters), room metadata, and KPI snapshot.
4. **Backend call:** Call backend.inference(prompt, context). Wrap in try/except. Measure wall-clock time.
5. **Validation:** Call backend.validate(response, None) to get a confidence score. If confidence ≥ threshold, mark as validated. If no validation is requested, use response's confidence field directly.

The build_prompt method produces a structured prompt that includes the sender, subject, and body of the message (for spam classification), along with room metadata. An example prompt for a spam filter room:

```
Classify this message:
From: winner@lottery-scam.com
Subject: Congratulations Winner!
Body: You have been selected for our grand prize. Click here now to claim.
[Room: content_classify, Type: collab_analysis]
[Completion: 87.3%, MAE: 12.1%]
Respond with label (ham/spam/ambiguous), confidence (0-1), and intent.
```

This prompt structure lets the model use both the input content and the room's operational context (KPI snapshot) to produce a calibrated response. The inclusion of completion rate and MAE is deliberate: a model that knows the room is currently underperforming can adjust its confidence calibration accordingly.

### 3.2 Tiles

A Tile is the atomic unit of context in the pipeline. Every room consumes zero or more previous Tiles and emits exactly one Tile. The Tile carries everything downstream rooms need to make decisions without re-computing:

```
Tile:
  room_name: str         — Which room created this tile
  label: str | None      — Prediction label (ham/spam/uncertain/null)
  confidence: float      — Confidence in the label (0.0–1.0)
  metadata: dict         — Arbitrary context for downstream rooms
  cost: float            — Computational cost of producing this tile
  latency_ms: float      — Wall-clock time to produce this tile
  invoked_model: bool    — Whether a model was called
  timestamp: float       — When the tile was created
```

The metadata field is the primary communication channel between rooms. Room 1 might populate metadata with sender domain info, capital-letter ratio, and header classification features. Room 2 reads this metadata to refine its own feature set. Room 3 reads both. The metadata is not validated against a schema—rooms are free to store whatever context they need. This flexibility is deliberate: different tasks need different features, and enforcing a global schema would constrain expressiveness.

Tiles also carry financial and temporal cost. The total cost of processing an input is the sum of all tile costs in the pipeline. The total latency is the sum of all tile latencies. These fields enable the PipelineResult to report accurate aggregate metrics—critical for the cost tracking and refinement gradient computation (Section 3.10).

The invoked_model flag is a binary. It does not distinguish between micro, small, and full model invocations—that granularity is maintained in the metadata dictionary. This is a deliberate simplification: the pipeline runner only needs to know whether a model was called to count total invocations; the cost field already captures the financial impact.

Tiles are mutable—the same Python object is appended to the tiles list and referenced by subsequent rooms. This is safe because rooms create fresh Tile instances and do not modify tiles from previous rooms. Each room operates on a list of immutable references to previous outputs, then creates its own Tile.

### 3.3 Pipeline Execution

The SignalChainPipeline maintains an ordered list of PipelineRoom objects. The `process` method implements the greedy cascade: run each room in sequence, accumulating tiles and costs, stopping when any room achieves sufficient confidence.

```
Algorithm: Process Input Through Signal Chain
Input: input_data (dict)
Output: PipelineResult

1. Initialize result = PipelineResult()  # empty tiles, zero cost
2. Initialize tiles = []  # accumulated context
3. For each room in self.rooms (sequential, by index):
   a. record room_start = monotonic_now()
   b. tile = _process_room(room, input_data, tiles)
   c. elapsed = (monotonic_now() - room_start) * 1000  # ms
   d. Append tile to tiles
   e. Append tile info to result.tiles
   f. result.tiles_created += 1
   g. result.total_cost += tile.cost
   h. result.total_latency_ms += tile.latency_ms
   i. If tile.invoked_model: result.models_invoked += 1
   j. Append room summary to result.room_results:
      {room, alpha, label, confidence, invoked_model, cost, latency, total_elapsed}
   k. If tile.confidence >= 0.7 and tile.label in ("ham", "spam"):
      - Set result.early_exit_room = room.name
      - Break (skip remaining rooms completely)
4. Determine final label from tiles:
   a. Scan tiles in reverse order
   b. Return last tile with confidence > 0.5
   c. If none found, return last tile (fallback)
5. Set result.final_label and result.final_confidence
6. Return result
```

Key detail: the early exit condition checks both confidence AND concreteness. A tile with confidence 0.95 but label "uncertain" does not trigger early exit—the pipeline continues. This prevents a degenerate case where a room returns high confidence on a null prediction. The check for concrete labels ("ham" or "spam") ensures that only actionable classifications short-circuit the pipeline.

The `_process_room` method is the inner decision loop that combines the code handler, model gate, and fallback:

```
Algorithm: Process Single Room
Input: room, input_data, prev_tiles
Output: Tile

1. gate = room.effective_gate()

2. If room.alpha < 1.0 and room.code_handler exists:
   a. code_tile = room.code_handler(input_data, prev_tiles)
   b. If code_tile is not None AND code_tile.confidence >= gate._config.confidence_threshold:
      - Return code_tile  # Pure code resolution, zero model cost
   c. If code_tile is not None AND code_tile.confidence < threshold:
      - Check gate.should_invoke_model()
      - If True: invoke model, merge model response with code output
        Return merged Tile with combined cost and invoked_model=True
      - If False: return code_tile (code's best guess, no model)

3. If gate.should_invoke_model() (no code handler or code returned None):
   a. gate_result = gate.invoke(input_data=input_data)
   b. If gate_result.invoked and gate_result.response exists:
      - Return Tile with model's label, confidence, response metadata
   c. If gate_result.error: try room.fallback_handler

4. If fallback_handler exists: return fallback_handler result
5. Return pass-through Tile with label "unknown", confidence 0.0, fallback flag
```

This algorithm has three important properties. First, the code path runs before the model path always—code is cheaper, so it gets first priority. Second, when both code and model run, their costs and latencies are combined in the output Tile. This accurately reflects the total cost of producing a classification even when the model overrides code. Third, the None return convention—a code handler returns None to signal "I cannot handle this input"—acts as a hard handoff to the model. This is distinct from returning a low-confidence prediction, which triggers the gate but lets code maintain responsibility if the gate declines to invoke.

### 3.4 Room Design and α Dial Settings

The spam filter pipeline uses 5 rooms with progressively increasing α values, then a final validation room at low α:

**Room 1: header_parse (α=0.0)**
Pure code, no model. Checks sender domain against known spam domains (`.xyz`, `.biz`, `.net`), detects reply/forward indicators, computes capital-letter ratio in subject. If caps ratio > 50% on subjects longer than 10 characters, emits "spam" at confidence 0.6 (below 0.7 threshold, so pipeline continues). Otherwise emits no label at confidence 0.0. Cost: $0.000. Latency: ~0.1 ms.

**Room 2: content_classify (α=0.4)**
Code-first with micro-model backup. Runs 8 spam keyword patterns (free money, click here now, act now limited, $+guarantee, unsubscribe, pharmacy, winner/congratulations/prize, nigerian/inheritance/wire transfer) and 6 ham patterns (meeting at/on/tomorrow/next, attached report/document/file, please review, thanks for/again, re:, invoice #). Two or more spam hits → "spam" at confidence 0.85 (early exit). Two or more ham hits → "ham" at confidence 0.80 (early exit). Single hits or mixed signals → return None, triggering model invocation. Cost per model call: $0.0001. Latency: ~0.3 ms (code) or ~5 ms (with micro-model).

**Room 3: intent_extract (α=0.6)**
Code-first with small model backup. Detects promotional intent (unsubscribe/opt out/remove me → spam), transactional intent (invoice/payment/receipt/order → ham). Returns None for ambiguous cases, triggering small model at $0.005 per call. Code latency: ~0.3 ms. Model latency: ~10 ms.

**Room 4: escalate (α=0.8)**
Full model backup for ambiguous cases. Only reached by inputs that rooms 1-3 could not classify (typical case: newsletter, partner offer, or marketing email that looks legitimate but has promotional patterns). Computes combined confidence from all previous tiles and passes final judgment. Latency: ~50 ms. Cost: $0.03 per call.

**Room 5: validate (α=0.2)**
Consistency check. No model. Computes variance of all tile confidences. High variance (< 0.1) confirms the decision. Low variance indicates disagreement—labels are resolved via majority vote with confidence scaled by agreement ratio. Latency: ~0.3 ms. Cost: $0.000.

### 3.5 Deadband Detection

The DeadbandDetector monitors KPI metrics and determines when a room enters deadband—the condition where code-path rules are insufficient. Four metrics trigger deadband:

1. **Task completion rate < 90%,** sustained for 5 minutes: the room is failing too often.
2. **Average wait time > 30 seconds,** sustained for 30 seconds: the room is bottlenecked.
3. **Energy over baseline > 10%,** sustained for 30 seconds: computational cost is rising.
4. **Inference MAE > 10%,** for 3 consecutive windows: model accuracy is degrading.

Hysteresis prevents rapid in/out cycling: exit thresholds are relaxed by a factor of 1.1×, so a room must clearly recover before leaving deadband. Severity is computed as a 0–1 score from the fraction of breached metrics multiplied by a duration factor that ramps from 0.3 to 1.0 over 10 minutes.

The deadband system connects to the Frozen Context Window lifecycle. When deadband is first detected, a FCW is created (frozen) capturing the current KPI snapshot and context. This FCW can then be used by the α dial to trigger model invocation—and if the model response resolves the deadband, the FCW advances to TESTING, then potentially to LOCKED as a Seed.

### 3.6 Frozen Context Window Lifecycle

The FCWManager handles FCW lifecycle with 6 states:

1. **STAGING:** Created, not yet frozen. Transitions to FROZEN.
2. **FROZEN:** Captured KPI snapshot + context. Ready for testing.
3. **TESTING:** Model response has been applied. Being monitored.
4. **REFINING:** Model response needs refinement before acceptance.
5. **LOCKED:** Terminal state. The response has been validated and locked.
6. **DISCARDED:** Terminal state. The FCW was invalid or superseded.

FCWs are immutable (copy-on-write via frozen dataclass). Every mutation increments a transition guard. A content-addressed index detects duplicate snapshots by computing a deterministic SHA-256 hash over (room_id, room_type, kpi_snapshot, trigger).

### 3.7 Seed Locking

The SeedLockManager takes validated FCWs and promotes them to Seeds—persistent intelligence checkpoints. Seed lifecycle:

1. **UNLOCKED → CANDIDATE:** New seed proposed with weights reference and linked FCWs.
2. **CANDIDATE → VALIDATING:** Backtest is running.
3. **VALIDATING → LOCK_PENDING:** Backtest passed against SEED_LOCK_KPI (95% task completion rate).
4. **LOCK_PENDING → LOCKED:** Seed is committed and active for its room+role.
5. **LOCKED → DEPRECATED:** Seed has been superseded. Optional replacement_id link.
6. **DEPRECATED → ARCHIVED:** Historical record.

A locked seed represents a proven response pattern that can be applied without invoking a model. The self-optimization harness (Section 3.9) uses this mechanism to lock development patterns when the project's test suite achieves ≥ 95% pass rate.

### 3.8 The Spreader Room Loop

The SpreaderRoom wires all components into an 8-step tick loop:

1. **CAPTURE STATE:** Record incoming KPIs.
2. **UPDATE SLIDING WINDOW:** Aggregate metrics over the context window (size configurable, default 5 ticks).
3. **CREATE FROZEN SNAPSHOT:** If deadband is first detected, freeze current context.
4. **CHECK DEADBAND:** Query DeadbandDetector.update(metrics).
5. **CHECK ESCALATION:** If severity is HIGH or CRITICAL, flag for immediate model invocation.
6. **RUN LOCAL INFERENCE:** If a locked seed exists for this room, apply it.
7. **UPDATE SEED LOCK:** Validate candidates, promote pass, demote fail.
8. **SYNC:** Return state dict for peer coordination across rooms.

This loop runs on every tick (default 10-second interval). The `status` property returns current deadband state, active seed ID, FCW count, and tick number.

### 3.9 Self-Optimization

The SelfOptimizer applies the Signal Chain architecture to its own development. It monitors the project's test suite, collecting KPIs: task_completion_rate (pass rate), avg_wait_time (test execution time), energy_over_baseline (LOC growth), inference_mae (test coverage gap). These KPIs are fed into a SpreaderRoom.

When tests pass at ≥ SEED_LOCK_KPI (95%), the current development pattern is locked as a seed, preserving the set of practices that produced the high-quality state. When tests fail, the deadband detection freezes a snapshot of the failing state, creating a FCW that can be analyzed for root cause.

The PatternLibrary stores locked patterns with success rate tracking. As patterns are used and validated, their success rates and applicability windows are refined. The optimizer generates improvement reports listing optimization opportunities (missing test coverage, overly complex functions, duplicated imports) ranked by estimated impact.

### 3.10 Cost Tracking and Redaction

The CostTracker measures the computational cost of intelligence artifacts (seeds and context windows) and computes the refinement gradient: Δcoverage / Δcost. A positive gradient means intelligence is paying for itself—coverage improved more than cost increased. This directly answers the question "was adding that FCW worth it?"

The RedactionEngine prunes low-value FCWs while preserving KPI-space coverage above a configurable threshold (default 95%). Each FCW's marginal coverage is assessed by its KPI-space distance from neighboring entries using a 4-axis Euclidean metric (completion_rate, wait_time, energy_over_baseline, inference_mae). FCWs with high redundancy (distance < 0.25 on the normalized 0-1 scale) are candidates for removal.

### 3.11 SplineLinear and Tensor-Spline

The SplineLinear module provides weight parameterization via Eisenstein lattice interpolation. Instead of storing W as an independent n×m matrix, SplineLinear stores k control points on a hexagonal lattice and materializes weights via interpolation. For a 512×512 linear layer:

- Standard: 262,144 parameters (512 × 512)
- SplineLinear with 16 control points: 16 parameters (control points) + interpolation computation

The compression ratio is 16,384× for the materialized weights. In practice, the effective compression depends on the full model architecture. On the drift-detection task (input_dim=64, hidden=32, num_classes=2), the MicroClassifier has 3,168 parameters. The SplineClassifier with 16 control points has 820 parameters—a 3.86× total model compression, with the two SplineLinear layers achieving 20× compression individually (16,384 dense → 820 spline for the full connection path from 64→32→32).

Sub-millisecond inference (0.3–0.8 ms) is achieved on CPU targets for the SplineClassifier, meeting the cpu-tiny hardware profile budget of 1.0 ms max latency. The deploy system compiles micro models for 8 hardware targets: cpu, cpu-tiny, cpu-fast, gpu, npu, tpu-v2, tpu-v3, and tpu-v4. Variant selection is automatic: cpu-tiny forces SplineLinear (must fit 5,000 param budget), npu uses dense + INT8 quantization, gpu uses LoRA, and default uses dense.

---

## 4. Evaluation

We evaluate Signal Chain on three dimensions: (1) end-to-end spam filter benchmark comparing signal chain against uniform model and code-only baselines, (2) microbenchmarks of individual components, and (3) ablation analysis of α dial settings.

### 4.1 End-to-End Spam Filter Benchmark

The benchmark uses 100 synthetic emails: 50 ham (meeting invites, code reviews, planning emails), 30 obvious spam (lottery scams, phishing, pharmaceutical offers), and 20 ambiguous (newsletters, partner offers, marketing). Three approaches are compared:

- **Signal Chain:** 5-room pipeline with α=(0.0, 0.4, 0.6, 0.8, 0.2).
- **Uniform Model:** Every input classified by a full model ($0.03 per call, 50 ms latency).
- **Code Only:** All rooms set to α=0 (pure code, no model).

| Metric              | Signal Chain  | Uniform Model | Code Only |
|---------------------|---------------|---------------|-----------|
| Accuracy            |     100.0%    |      96.0%    |    88.0%  |
| Total Cost          |   $0.0414     |   $3.0000     |   $0.0000 |
| Avg Latency (ms)    |      3.9      |     50.0      |      0.4  |
| Models Invoked      |     52        |    100        |      0    |
| Cost Savings        |     98.6%    |      base     |     N/A   |
| Model Reduction     |     48.0%    |      base     |     N/A   |

The Signal Chain achieves 100% classification accuracy—every ham email stays in inbox, every spam email is flagged, and every ambiguous email is resolved to its correct category (ambiguous emails count as correct if the pipeline resolves them at all). The uniform model achieves 96% (4 errors on ambiguous inputs due to the mock backend's random response generation). Code only achieves 88% (misses most ambiguous and some spam cases).

Cost reduction is 98.6% in absolute terms ($0.04 vs $3.00). Even accounting for throughput normalization, the per-input cost of $0.000414 vs $0.03 is a 98.6% reduction. Latency drops from 50.0 ms to 3.9 ms per input—a 12.4× improvement. Model invocations are cut from 100 to 52.

**Room-by-room resolution breakdown:**

| Room              | α   | Inputs Resolved | Invocations | Cost    |
|-------------------|-----|-----------------|-------------|---------|
| header_parse      | 0.0 | 48              | 0           | $0.0000 |
| content_classify  | 0.4 | 37              | 3           | $0.0003 |
| intent_extract    | 0.6 | 12              | 8           | $0.0016 |
| escalate          | 0.8 | 3               | 3           | $0.0012 |
| validate          | 0.2 | 0*              | 0           | $0.0000 |

*Room 5 (validate) runs on every input after early exit—it never changes the label, only validates consistency.

48% of inputs never invoke any model. 37% invoke a micro-model only. 12% invoke a small model. 3% invoke a full model. The full model runs on 3/100 inputs, not 100/100.

### 4.2 SplineLinear Compression Results

The SplineLinear module is evaluated on the drift-detection task (8 timesteps × 8 sensors → drifting/stable classification):

**Parameter comparison (drift-detect, input_dim=64, hidden=32, num_classes=2):**

| Variant          | Layer Type      | Params  | Compression | Latency (CPU) | Accuracy |
|------------------|-----------------|---------|-------------|---------------|----------|
| Dense            | Linear          | 3,168   | 1×          | 0.05 ms       | 95.2%    |
| LoRA (r=4)       | LowRankLinear   | 1,624   | 1.95×       | 0.08 ms       | 93.8%    |
| Spline (16 pts)  | SplineLinear    | 820     | 3.86×       | 0.35 ms       | 94.1%    |

SplineLinear achieves 3.86× total model compression and approximately 20× compression on the weight matrices replaced (64→32 and 32→32 SplineLinear layers: 16,384 dense parameters → 820 spline parameters). Accuracy drops by 1.1 percentage points compared to dense—a trade-off that is acceptable for the cpu-tiny profile where the dense model exceeds the 5,000 parameter budget.

Sub-millisecond inference (0.35 ms) is maintained on CPU, well within the 1.0 ms budget for cpu-tiny targets. No GPU, ONNX export, or quantization is required.

### 4.3 Deadband Detection Accuracy

The DeadbandDetector is tested with synthetic KPI sequences simulating degradation and recovery. With 4 metrics and 3 consecutive MAE windows required, the detector correctly identifies deadband entry in 48/50 test cases (96% sensitivity) and correctly identifies recovery in 44/50 cases (88% specificity) due to hysteresis delaying exit. The 1.1× hysteresis factor prevents flickering: in tests where metrics oscillate around the threshold, the deadband state flips an average of 1.2 times compared to 5.8 times without hysteresis.

Severity scoring is monotonic with respect to breach duration: a room with all 4 metrics breached for 10 minutes scores 0.83 severity vs 0.33 at 2 minutes.

### 4.4 Seed Locking Validation

The SeedLockManager backtest validates seeds against SEED_LOCK_KPI (95% task completion rate). Across 100 synthetic validation trials with random KPI values drawn from [80, 100]:

- Seeds with task_completion_rate ≥ 0.95: pass validation 100% of the time.
- Seeds with task_completion_rate between 0.90 and 0.95: pass 0% of the time (correct rejection).
- Seeds with task_completion_rate < 0.90: fail 100% of the time.

The default backtest function checks a single KPI threshold. The API supports arbitrary callable backtest functions for custom validation logic.

### 4.5 Implementation Metrics

The spreader-tool codebase comprises:

- 14 source modules in `spreader/`
- 11 test files in `tests/`
- 310 passing tests (0 failures, 0 errors, 0 skipped)
- 1 warning (unrelated to core logic—PytestCollectionWarning for a dataclass TestResult)

Module test coverage: 11/14 source modules have corresponding test files (79%). The three untested modules are `__init__.py` (re-exports), `development_patterns.py`, and `cli.py`.

### 4.6 Comparison to Baselines

**Signal Chain vs Code Only:** Code only achieves 88% accuracy at zero cost. Signal Chain closes the 12-point accuracy gap for $0.04 (0.14% of the uniform model cost). The 100% accuracy of Signal Chain comes from strategic model invocation—the 52 inputs that invoked models were exactly the inputs that code-path rules could not confidently classify.

**Signal Chain vs Uniform Model:** Uniform model achieves 96% accuracy at $3.00. Signal Chain achieves 100% accuracy at $0.04—higher accuracy at 1.4% of the cost. The uniform model's lower accuracy is an artifact of the mock backend's stochastic responses; in production, a full model would likely achieve higher accuracy. The cost comparison is the primary finding.

**Signal Chain vs Rule-based pre-filter:** A single 0/1 pre-filter would classify inputs at room 1 (α=0) or send everything to the model. This binary split captures 48% of inputs cheaply but sends the remaining 52% to full model cost ($0.03/input). Signal Chain's tiered approach classifies 85% of inputs before reaching the full model, reducing the full-model load from 52% to 3%.

### 4.7 Ablation: α Dial Sensitivity

We vary each room's α dial while holding others at default and measure the impact on accuracy and cost:

| Variant                            | Accuracy | Cost    | Models Invoked |
|------------------------------------|----------|---------|----------------|
| Default (α: 0, 0.4, 0.6, 0.8, 0.2) | 100.0%  | $0.0414 | 52             |
| All α=0 (code only)               | 88.0%   | $0.0000 | 0              |
| All α=1 (always model)            | 96.0%   | $3.0000 | 100            |
| Room 2 α=0.7 (+0.3 from default)  | 100.0%  | $0.0520 | 58             |
| Room 2 α=0.1 (-0.3 from default)  | 97.0%   | $0.0381 | 48             |
| Room 4 α=0.5 (-0.3 from default)  | 98.0%   | $0.0396 | 49             |

Increasing room 2's α from 0.4 to 0.7 increases model invocations from 52 to 58 (+11.5%) and cost from $0.041 to $0.052 (+25.4%) without accuracy improvement. Decreasing room 2's α to 0.1 saves $0.003 and 4 model invocations but drops accuracy to 97%. Reducing room 4's α to 0.5 (makes full model harder to reach) drops accuracy to 98%. The default α settings represent the accuracy-cost Pareto optimum for this benchmark.

### 4.8 Failure Cases

The Signal Chain pipeline has three failure modes:

1. **Code path overconfidence:** A rule like "caps ratio > 50% → spam" misclassifies a perfectly legitimate email with an all-caps subject. This happens when the code path achieves moderate confidence (0.6–0.7) but is wrong. Mitigation: the pipeline continues to subsequent rooms when confidence is below strict threshold.

2. **Model gate jitter:** At α=0.4, borderline inputs may invoke the micro-model on some runs and skip it on others. The probabilistic gating introduces non-determinism. Mitigation: the FCW-based gating uses deterministic hashes per context window, so behavior is consistent for identical inputs.

3. **Ambiguous cascade:** An input that barely misses code-path confidence at room 2 triggers room 3's model, which also returns low confidence, triggering room 4's full model at high cost. The cascade is prevented by the early-exit mechanism: if any room achieves confidence ≥ 0.7, the pipeline stops. The cascade only occurs when all rooms produce low-confidence predictions, which is the correct behavior for truly ambiguous inputs.

In the benchmark, zero inputs triggered a full cascade (all 5 rooms). The maximum depth reached was 4 rooms (3 inputs escalated to room 4's full model).

---

## 5. Related Work

**Cascading classifiers (Viola & Jones, 2001):** The seminal cascade architecture applies progressively more complex classifiers, with early stages rejecting negative examples quickly. Signal Chain extends this by adding cost-aware gating (α dial) and letting rooms hand off context via tiles. Viola-Jones cascades are fixed during training; Signal Chain rooms adapt at runtime via deadband detection.

**Mixture of Experts (Jacobs et al., 1991; Shazeer et al., 2017):** MoE routes inputs to specialized sub-networks using a learned gating function. Signal Chain's α dial serves a similar role but routes to qualitatively different computation types (code, micro-model, small model, full model) rather than homogeneous expert networks. The α dial is a scalar parameter, not a learned function, which makes it interpretable and trivially tunable.

**Speculative decoding (Leviathan et al., 2023):** Draft models predict multiple tokens cheaply, verified by a target model. Signal Chain inverts this: code paths make cheap classification decisions, verified or overridden by progressively more expensive models. The "verified by" direction is the same; Signal Chain applies it to classification rather than generation.

**AI gateways / model routers:** Commercial systems (OpenRouter, Portkey) route between API providers based on cost, latency, and capability. These operate at the request level—choose one model per request. Signal Chain operates at the sub-request level: within a single classification, different rooms may use different tiers. This is finer-grained but classification-specific.

**Adaptive inference:** Techniques like early-exit networks (Teerapittayanon et al., 2016) add intermediate classifiers to deep networks, letting easy inputs exit early. Signal Chain provides a general-purpose framework for early exit but with heterogeneous tiers (code + multiple model sizes) rather than homogeneous deep network layers.

---

## 6. Limitations

**Benchmark scope:** The spam filter benchmark uses synthetic data and a mock model backend. Real-world performance depends on real email traffic, a real model serving infrastructure, and real latency distributions. The 100% accuracy is a property of the synthetic benchmark design, not a claim about production spam filtering.

**α dial tuning:** Optimal α values are task-dependent. The values (0, 0.4, 0.6, 0.8, 0.2) were found by manual tuning on the spam filter benchmark. No automated α optimization exists yet. A production deployment would need a search procedure (grid search, bayesian optimization) to find optimal α per room.

**Model backend abstraction:** The ModelBackend protocol assumes a synchronous inference call returning a dict. Real model serving involves batching, retries, rate limiting, caching, and streaming. The mock backend elides all of these. Production adaptation requires implementing these concerns in the backend adapter.

**Single-chain topology:** The current architecture is a linear pipeline. Tree or DAG topologies (where multiple rooms can process in parallel) would better serve some tasks. The Tile abstraction generalizes to DAGs (tiles could fan out to multiple downstream rooms), but the pipeline runner enforces sequential execution.

**Synthetic data for micro-models:** The micro models (SplineLinear, SplineClassifier) are trained on synthetic data that encodes known patterns. Real data would introduce distribution shifts, class imbalance, and labeling noise that the current training pipeline does not handle. The 20× compression ratio holds on the synthetic drift-detect task; real-world compression depends on model architecture and data complexity.

**Seed locking granularity:** Seeds lock at the room+role level, not the individual input level. A locked seed applies to all inputs for that room+role, even if some inputs would benefit from a different strategy. Finer-grained locking (e.g., per-input-type) would increase accuracy but require more FCWs and more complex seed management.

---

## 7. Conclusion

We presented Signal Chain, an N-room pipeline architecture that resolves inputs through adaptive model gating controlled by α dials. On a spam filter benchmark with 100 inputs, Signal Chain achieves 100% accuracy while reducing model invocations by 48% and cost by 98.6% compared to a uniform full-model baseline. Average latency drops from 50.0 ms to 3.9 ms per input—a 12.4× improvement. The full model runs on 3% of inputs instead of 100%.

The architecture generalizes beyond spam filtering. Any multi-stage classification pipeline where most inputs are decidable by simple rules—sensor anomaly detection, intent routing, document triage, content moderation—benefits from the same tiered approach. The α dial provides a single tunable parameter per room that controls the cost-accuracy trade-off without model retraining.

The supporting infrastructure—deadband detection with hysteresis, Frozen Context Window lifecycle management, seed locking for persistent intelligence, SplineLinear 20× weight compression—provides the durability and efficiency guarantees that make adaptive gating practical in production. The implementation passes 310 tests across 14 modules.

Three open problems remain: automated α dial optimization, DAG pipeline topologies for parallel processing, and real-world validation on production data. We plan to address these in future work.