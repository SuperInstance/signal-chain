# The Signal Chain: Why Every Room Needs a Dial

**How PLATO rooms turn the model-vs-code question into a mixing desk**

---

## Abstract

Modern AI systems face a binary choice: run everything as deterministic code, or hand everything to a model. Both fail at scale. Algorithmic pipelines shatter on novel inputs; agentic systems burn compute re-discovering what code already knows. We present the Signal Chain architecture, where each computation stage — called a "room" — carries a tunable parameter &alpha; &isin; [0,1] controlling how much work is done by code versus a language model. At &alpha;=0, pure algorithms run at wire speed. At &alpha;=1, a full agent handles everything. The interesting territory is between: a deadband detector monitors per-stage KPIs and turns the dial up only when algorithms can't cope. Frozen context windows carry accumulated knowledge forward through the chain, so downstream models inherit upstream decisions without re-deriving them. We describe the architecture, formalize the dial parameter, explore distillation as per-room model sizing, and present a proof-of-concept (spreader-tool, 241 tests, pure Python) that validates the monitoring and state-snapshot mechanisms. We are honest about what works, what's speculative, and where a brutal beta review found real gaps.

---

## 1. Introduction: The Two Failures

There are two ways to build a computation pipeline, and both of them break.

**Path one: algorithms.** Rules, thresholds, if/then branches. Fast, deterministic, testable. Works perfectly on anticipated cases, fails catastrophically on everything else. Your system is as smart as your last deploy.

**Path two: models.** Hand everything to an LLM. Handles novelty beautifully — until you get the bill. Every call is a fresh context window. The model re-discovers what your rules already know. Your system is as smart as your budget allows.

The problem isn't that either approach is wrong. It's that both are **monolithic**. They treat the entire pipeline as a single unit that's either "code" or "model." But real pipelines have stages, and each stage has different needs. A data-validation step doesn't need GPT-4. An escalation-routing step probably does. Treating them identically is like setting every knob on a mixing desk to the same position and wondering why the music sounds wrong.

What's missing is a per-stage control: how much model, how much code, *right here*, at this point in the chain.

---

## 2. The Signal Chain Metaphor

A guitarist doesn't just plug into an amp. They build a signal chain:

```
Guitar body → Strings → Pickups → Volume/tone pots →
Pedals (overdrive, delay, reverb) → Amp → Speaker →
Mic placement → DI blend → Console EQ → Compressor → Mix position
```

Every stage has a dial. Some are subtle (string gauge), some transformative (distortion pedal). The art is in the combination: which stages to engage, how far to push each, how upstream choices constrain downstream options.

The Signal Chain architecture maps this to computation pipelines. Each stage is a **room** — a shared state space where computation happens. Each room has a **dial** controlling the balance between algorithmic code and model intelligence. A fleet operator tuning a Signal Chain is doing the same thing as a guitarist dialing in a tone: adjusting per-stage parameters to get the output they want, at the cost they can afford, with the latency they can tolerate.

The metaphor isn't decorative. It's structural.

---

## 3. The Missing Parameter: Room Dial Settings

Each room carries a parameter &alpha; &isin; [0,1] — the **dial setting** — controlling the ratio of code to model inference.

| &alpha; | Behavior | Guitar equivalent |
|---------|----------|-------------------|
| 0.0 | Pure algorithm. Deterministic, zero inference cost. | Clean signal, no pedals. |
| 0.1–0.3 | Micro-model. Handles edge cases the algorithm misses. | Light overdrive — warmth without changing character. |
| 0.4–0.6 | Medium model with tile context. Complex cases, accumulated state. | Mid-gain amp — reshapes signal, guitar still audible. |
| 0.7–0.9 | Large model with full context. Novel situations, deep reasoning. | High-gain stack — model dominates, code provides structure. |
| 1.0 | Full agent call. The model does everything. | Synth — original signal is just a trigger. |

In practice, &alpha; isn't set manually per request. It's governed by a **deadband detector** that watches the room's KPIs and adjusts &alpha; based on performance.

### The Deadband Detector

In control theory, a deadband is the range where input changes produce no output. We repurpose the term (loosely — see Section 10) to describe the gap between what a room's code handles and what requires model intervention.

When KPIs are within thresholds — completion rate above 90%, wait times below 30s, inference error below 10% — the room runs at its current &alpha;, often near zero. No inference cost. When KPIs breach thresholds and *stay breached* (duration gates prevent flickering), the deadband opens and the dial turns up. A model wakes up with full context from every tile that passed through every previous stage.

The hysteresis matters: exiting deadband requires recovering *past* the original threshold (1.1x factor), not just back to it. This prevents the alert-resolve-alert death spiral. Anyone who's been on-call at 3 AM knows why.

### Why Not Just Auto-Scale Models?

The dial isn't just model size — it's the ratio of code-to-model *responsibility* at a specific stage. A room at &alpha;=0.3 might use a 7B model for gap-filling: the algorithm handles 90% of cases, the model handles the 10% that fall through. A room at &alpha;=0.8 might use the same 7B model for primary computation, with code only providing guardrails. The dial controls *responsibility*, not just *capacity*.

---

## 4. Tiles as Context Carriers

The Signal Chain's secret weapon isn't the models. It's the tiles.

A **tile** is a unit of knowledge — a decision, observation, or intermediate result — that passes through the chain. When a room processes a tile, it annotates it: what was decided, at what confidence, with what KPIs. The tile accumulates context as it moves downstream.

The model at stage 5 doesn't rediscover what stages 1–4 already figured out. The tiles *are* that knowledge. When deadband opens and a model wakes up, it sees the original input, every intermediate decision, every confidence score, and the specific point where algorithms stopped being sufficient. The model handles only the **delta** — the gap between what rules predicted and what actually happened.

That's why small models work at most stages: they're not doing the whole job, just the part code can't. Each pedal receives the output of the previous pedal, not the raw guitar signal. A delay pedal after distortion delays the *distorted* signal — it operates on what it receives. Tiles work the same way.

When a downstream room discovers a correction, it can emit a tile that flows backward through the chain, updating upstream context for future inputs. The feedback loop tightens the chain.

---

## 5. The Plinko Model

Here's an unintuitive way to think about what models do in the Signal Chain.

Imagine a Plinko board — the game-show toy where a disc drops through pegs, bouncing left or right at each one. Tiles are the discs. Rooms are the rows of pegs. The model's weights determine the **shape of the peg field** — which paths are likely, which slots tiles land in.

- **Big model** = complex peg arrangement, captures nuanced path distributions
- **Small model** = simple pegs, captures only dominant paths
- **No model (&alpha;=0)** = straight down, deterministic routing

This maps directly to linear algebra in neural networks. Weight matrices determine which regions of high-dimensional space are "attractors." A bigger model has higher-dimensional spaces with more complex attractor landscapes. A smaller model captures the major modes but misses fine structure. Fine-tuning is reshaping the pegs. Distillation is simplifying the arrangement while preserving the most important path biases.

The key insight: **you don't need the same peg complexity at every row.** Some rows just need left-or-right routing. Some need complex multi-path distributions. The Signal Chain lets you set complexity per row.

---

## 6. Self-Healing Through Re-Entry

When a room hits deadband, the system initiates a **re-entry loop**:

1. **Detect**: KPIs breach thresholds for sustained duration.
2. **Freeze**: Room state captured as an immutable Frozen Context Window (FCW) — tile state, KPIs, breach metadata.
3. **Invoke**: A model at higher &alpha; is called with the FCW. It sees the full chain state, failure point, and accumulated tile knowledge.
4. **Propose**: The model generates a new tile — a proposed fix, reclassification, or escalation.
5. **Validate**: Does the proposal improve KPIs? Break downstream stages?
6. **Lock**: If validation passes, the response becomes a **seed** — a locked checkpoint for similar future situations.

Seeds go through a staged lifecycle: `UNLOCKED → CANDIDATE → VALIDATING → LOCK_PENDING → LOCKED → DEPRECATED → ARCHIVED`. Each transition has gates. A locked seed is like a guitar lick practiced until it's muscle memory — it once required full attention (high &alpha;), now it's a reflex (pattern match against the seed library).

This is how the Signal Chain **learns**. Not gradient updates, but building a library of validated responses at the point of use. Each locked seed is crystallized intelligence — something that once required a model call and now runs as code.

---

## 7. Distillation as Tone Crafting

Every guitarist eventually asks: can I get *this* tone with a smaller rig?

A room at &alpha;=0.8 with a 70B model is expensive. But the model isn't using all 70B parameters for every decision. Most responses follow a few dominant patterns. Can we capture them in a 3B model at &alpha;=0.3?

The process: run the room at high &alpha;, log every input-output pair. Identify dominant patterns — what the model does 90% of the time. Train a smaller model on those patterns (standard knowledge distillation, scoped to one room). Validate through the room's pipeline. Deploy at lower &alpha;, with the deadband detector catching the cases the smaller model misses.

Because each room has its own dial, distillation doesn't have to be perfect. A distilled model catching 90% of cases at 5% of cost is a win — the deadband detector handles the other 10% by temporarily turning the dial up. You're making the big model the fallback, not the default. The $200 pedal that gets 90% of the boutique tone; for the one song where you need the last 10%, you swap in the expensive one.

### The 20x Compression Target

Early experiments with SplineLinear compression (tensor-spline weight quantization) suggest roughly 20x parameter reduction while maintaining >90% accuracy on room-specific tasks. Heavy caveats: task-specific, architecture-specific, narrow evaluation set. But the theoretical argument holds — a model that only handles one room's decision space should be dramatically more compressible than a general-purpose model.

This is speculative. We haven't validated at scale.

---

## 8. The Frozen Context Window

When a room hits deadband, the first thing that happens is a **freeze**. The room's state — KPIs, tile context, recent decisions, breach metadata — is captured as a Frozen Context Window (FCW).

FCWs are **immutable** (copy-on-write guarantees the debugging snapshot is exactly what the system saw), **content-addressed** (SHA-256 dedup — same room, same state, same deadband = one FCW, not two), and **lifecycle-managed** (`STAGING → FROZEN → TESTING → REFINING → LOCKED` or `DISCARDED`).

Immutability matters for debugging. Mutable state snapshots are like reading a newspaper someone keeps editing while you read it. FCWs are write-once: the FCW captures exact state, the model receives exact state, the model's response is validated against exact state, and if it becomes a seed, the seed references exact state. You can replay any decision, diff any two FCWs, trace exactly why the system made every choice.

Without pruning, FCW storage grows linearly with deadband events. The redaction engine computes KPI-space distance between FCWs: if two snapshots represent similar failure modes, the lower-value one (older, didn't lead to a seed, failure mode already addressed) is pruned.

---

## 9. Related Work

**Mixture of Experts.** MoE architectures (Shazeer et al., 2017; Fedus et al., 2022) route inputs to specialized sub-networks via gating. The Signal Chain's dial is conceptually similar but operates at a different level: MoE is intra-model (routing within a forward pass), while the Signal Chain is inter-stage (routing between computation stages with potentially different models). Signal Chain rooms are also independently configurable — closer to an ensemble of specialists than a gated mixture.

**Adaptive Computation.** Graves (2016) introduced Adaptive Computation Time; Universal Transformers and early-exit strategies extend it. Like the deadband detector, these avoid spending compute on easy inputs. But ACT adjusts per-token compute within a model; the Signal Chain adjusts per-stage compute across a pipeline.

**Model Cascades.** FrugalGPT (Chen et al., 2023) cascades through models of increasing size, stopping at sufficient confidence. This is the closest relative. The key difference: cascades operate on a single task with multiple model options. The Signal Chain operates on a pipeline with multiple stages, each handling a different aspect. A cascade asks "which model?" The Signal Chain asks "which model, at which stage?"

**Agent Frameworks.** LangChain, AutoGPT, CrewAI compose LLM calls into workflows but typically treat each step as a full model invocation. No per-step dial, no dynamic code-vs-model adjustment.

**Levels of Automation.** The Sheridan scale (1978) defines 10 levels from "human does everything" to "computer does everything." The Signal Chain's &alpha; is this scale applied per-stage, not per-system.

---

## 10. Proof of Concept: Spreader-Tool

[Spreader-tool](https://github.com/SuperInstance/spreader-tool) is a Python implementation of the Signal Chain's monitoring infrastructure: deadband detection, FCW management, and seed locking. 241 tests, under one second, zero dependencies beyond stdlib.

### What Works

A deliberately brutal beta review scored the project 6/10 overall. What earned marks:

**Frozen dataclass state machines (7/10).** `FrozenContextWindow` and `Seed` use `frozen=True` with explicit transition tables and copy-on-write. Transition guard counters detect stale references. Production-grade pattern.

**Hysteresis (7/10).** Duration gates require sustained violations; 1.1x exit factor prevents flickering. Tests specifically verify that marginal recovery does *not* exit deadband — catching the exact bug a naive implementation would ship.

**Zero dependencies.** Pure stdlib Python. Claimed and delivered.

**Behavioral tests.** Tests check properties ("severity increases monotonically," "hysteresis prevents flickering") rather than implementation details.

### What the Beta Review Found

The reviewer was right about several things. We're presenting findings unedited because the HN audience will find them anyway.

**"Self-optimization" doesn't optimize.** `SelfOptimizer` runs pytest, parses output, generates a report. It identifies opportunities but doesn't act on them. Monitoring dashboard, not optimization. Fair.

**Inflated module count.** The "real" library is 5 core modules. The other 6 are infrastructure, tooling, or — in the reviewer's words — "blog content dressed as a module."

**`dataclasses.replace()` not used.** The most embarrassing finding. The architecture is built on frozen dataclasses, but `transition_to()` hand-copies every field instead of using the stdlib function designed for exactly this. Maintenance bomb.

**Mutable `DeadbandState`.** Everything else is frozen. This isn't. "Inconsistent immutability is worse than consistent mutability."

**"Deadband" is a misnomer.** What's actually implemented is threshold-based anomaly detection with hysteresis. The reviewer's honest description: "anomaly detection with snapshot-and-validate." Technically correct, though we'd argue the term works metaphorically.

**Seeds auto-lock without review.** `_update_seed` proposes, validates, and locks in one call. "Validation" is a single threshold check. The locked seed's `weights_ref` is `"local://baseline"` — a string pointing to nothing. The biggest gap between architecture and implementation.

### Honest Assessment

Spreader-tool demonstrates that the **monitoring and state-management infrastructure** works. Deadband detection with hysteresis, immutable snapshots, content-addressed dedup, staged seed lifecycles — implemented and tested.

What it does *not* demonstrate is the **intelligence layer**: actual model invocation, dial-turning, distillation. No AI in the current implementation. KPIs go in, alerts come out, snapshots get saved. The architecture has slots for models (&alpha; parameter, `weights_ref` field, re-entry loop), but they're empty.

This is version 0 — the pedalboard without the pedals. The wiring is correct and the power supply works, but there's no distortion, no delay, no reverb yet. We think that's the right build order: monitoring and state management first, then models. But we want to be clear about what exists versus what's planned.

---

## 11. Open Questions

**Auto-tuning the dial.** Can we learn optimal &alpha; settings? This is a multi-armed bandit problem complicated by room interactions — changing &alpha; at stage 3 affects what stage 5 sees. A 10-room chain with 10 dial positions has 10^10 configurations. We suspect the right approach is hierarchical: optimize individual rooms, then adjacent pairs, then the full chain — mirroring how guitarists dial in rigs, amp first, then pedals one at a time.

**Minimum viable model size.** Given a room's task distribution, what's the smallest model that works? Some rooms have low-dimensional decision spaces (binary classification, threshold comparison) that might work with 1M parameters. Others need 7B+ even after distillation. We lack a principled framework for estimating this without trial and error.

**Chain topology.** Real pipelines have branches, merges, and feedback loops. Tiles extend naturally to DAGs, but the deadband detector gets complex with multiple inputs, and feedback loops don't guarantee convergence.

**Pathological cases.** Cascading deadband (every room triggers simultaneously, cost spikes exponentially). Tile bloat (long chains accumulate so much context that downstream models waste their context window on history). Stale seeds (input distribution shifts, locked seeds produce wrong answers, KPIs aren't sensitive enough to catch it). Adversarial inputs that exploit the gap between &alpha;=0 code and deadband thresholds. We don't have solutions for all of these.

**Multi-agent coordination.** The architecture assumes a single pipeline. PLATO rooms are shared spaces where multiple agents coordinate. How do multiple agents with different dial settings interact in the same room? Unexplored.

---

## 12. Conclusion: The Guitarist's Rig

**Every computation stage in a pipeline has a spectrum from pure algorithm to full model. The missing parameter in modern AI systems is a per-stage dial for where each stage operates on that spectrum.**

The Signal Chain gives you rooms with tunable &alpha; parameters, deadband detection that turns the dial up when algorithms can't cope, tiles that carry context so models don't re-derive upstream knowledge, frozen context windows for debugging and replay, seeds that crystallize model responses into reusable patterns, and distillation that compresses large-model behavior into room-scoped micro-models.

The guitarist doesn't play through a single pedal cranked to 10. They build a chain, set each dial for what that stage needs, and adjust as the song changes. Not "how much AI?" but "how much AI, where, right now?"

The rig isn't finished. But the signal path is clean, the connections are solid, and 241 tests say the wiring works. Time to plug in some pedals.

---

## Glossary

| Term | Definition |
|------|-----------|
| **Room** | A computation stage in the Signal Chain. Holds KPIs, state, and a dial setting. |
| **Dial (&alpha;)** | Per-room parameter &isin; [0,1] controlling code-vs-model ratio. |
| **Deadband** | The gap between what code handles and what requires model intervention. |
| **Tile** | A unit of knowledge flowing through the chain, accumulating context. |
| **FCW** | Frozen Context Window. Immutable state snapshot at deadband events. |
| **Seed** | A validated, locked response pattern. Replaces future model calls for similar inputs. |
| **Hysteresis** | Requiring recovery *past* the breach threshold to exit deadband. |

---

*May 2026. Spreader-tool is MIT-licensed at [github.com/SuperInstance/spreader-tool](https://github.com/SuperInstance/spreader-tool). The thesis emerged from building the tool, not the other way around — it's a post-hoc explanation of patterns discovered while trying to make agent rooms cheaper to operate.*
