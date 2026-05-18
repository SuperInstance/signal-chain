# The Signal Chain: Why Every Room Needs a Dial

**How PLATO rooms turn the model-vs-code question into a mixing desk**

---

## Abstract

Modern AI systems face a binary choice: run everything as deterministic code, or hand everything to a model. Both fail at scale. Algorithmic pipelines shatter on novel inputs; agentic systems burn compute re-discovering what code already knows. We present the Signal Chain architecture, where each computation stage — called a "room" — carries a tunable parameter &alpha; &isin; [0,1] controlling how much work is done by code versus a language model. At &alpha;=0, pure algorithms run at wire speed. At &alpha;=1, a full agent handles everything. The interesting territory is between: a deadband detector monitors per-stage KPIs and turns the dial up only when algorithms can't cope. Frozen context windows carry accumulated knowledge forward through the chain, so downstream models inherit upstream decisions without re-deriving them. We describe the architecture, formalize the dial parameter, explore distillation as per-room model sizing, and present a proof-of-concept implementation (spreader-tool, 241 tests, pure Python) that validates the deadband detection and state-snapshot mechanisms. We are honest about what works, what's speculative, and where the beta review found real gaps.

---

## 1. Introduction: The Two Failures

There are two ways to build a computation pipeline, and both of them break.

**Path one: algorithms.** You write rules, thresholds, if/then branches. The pipeline is fast, deterministic, and testable. It works perfectly on the cases you anticipated and fails catastrophically on the cases you didn't. Every novel input requires a code change. Every edge case is a ticket. Your system is as smart as your last deploy.

**Path two: models.** You hand everything to a language model. It handles novelty beautifully — until you get the bill. Every call is a fresh context window. The model re-discovers what your rules already know. It has no memory between invocations unless you bolt on retrieval, which introduces its own failure modes. Your system is as smart as your budget allows.

The problem isn't that either approach is wrong. It's that both are **monolithic**. They treat the entire pipeline as a single unit that's either "code" or "model." But real pipelines have stages, and each stage has different needs. A data-validation step doesn't need GPT-4. An escalation-routing step probably does. Treating them identically is like setting every knob on a mixing desk to the same position and wondering why the music sounds wrong.

What's missing is a per-stage control: how much model, how much code, right here, at this point in the chain.

That's the Signal Chain.

---

## 2. The Signal Chain Metaphor

A guitarist doesn't just plug into an amp. They build a signal chain:

```
Guitar body (wood, shape)
  → Strings (gauge, material)
    → Pickups (single-coil, humbucker)
      → Volume/tone pots
        → Pedals (overdrive, delay, reverb, compression)
          → Amp (tube type, wattage)
            → Speaker cabinet (cone size, open/closed back)
              → Microphone (dynamic, condenser, placement)
                → Preamp / DI blend
                  → Console EQ
                    → Compressor
                      → Mix position (pan, level)
```

Every stage has a dial. Some stages are subtle — string gauge changes the feel but not the fundamental character. Some are transformative — a distortion pedal reshapes the entire signal. The guitarist's art is in the combination: which stages to engage, how far to push each one, and how upstream choices constrain downstream options.

The Signal Chain architecture maps this directly to computation pipelines. Each stage is a **room** — a shared state space where computation happens. Each room has a **dial** that controls the balance between algorithmic code and model intelligence. The chain of rooms, with their dial settings, *is* the system.

A fleet operator tuning a Signal Chain is doing the same thing as a guitarist dialing in a tone: adjusting per-stage parameters to get the output they want, at the cost they can afford, with the latency they can tolerate.

The metaphor isn't decorative. It's structural. Every concept in the Signal Chain has a guitar-rig equivalent, and the equivalence helps explain why certain design decisions work.

---

## 3. The Missing Parameter: Room Dial Settings

### Formal Definition

Each room in the chain carries a parameter &alpha; &isin; [0,1] that we call the **dial setting**. It controls the ratio of algorithmic code to model inference at that stage.

| &alpha; | Behavior | Guitar equivalent |
|---------|----------|-------------------|
| 0.0 | Pure algorithm. Deterministic, zero inference cost. | Clean signal, no pedals. |
| 0.1-0.3 | Micro-model. Handles common edge cases the algorithm misses. | Light overdrive — adds warmth without changing character. |
| 0.4-0.6 | Medium model with tile context. Handles complex cases using accumulated state. | Mid-gain amp — reshapes the signal but you still hear the guitar. |
| 0.7-0.9 | Large model with full context window. Handles novel situations with deep reasoning. | High-gain stack — the model dominates, code provides structure. |
| 1.0 | Full agent call. The model does everything. | Synth — the original signal is just a trigger. |

In practice, &alpha; isn't set manually per request. It's governed by a **deadband detector**: a monitoring component that watches the room's KPIs and adjusts &alpha; based on how well the current setting is performing.

### The Deadband Detector as Volume Pedal

In control theory, a deadband is the range of input values where the controller produces no output — a region of intentional insensitivity. We repurpose the term (loosely — more on this in Section 10) to describe the gap between what a room's algorithmic code handles and what requires model intervention.

When KPIs are within thresholds — task completion rate above 90%, wait times below 30 seconds, inference error below 10% — the room runs at its current &alpha; setting, often near zero. The model is asleep. There's no inference cost.

When KPIs breach thresholds and stay breached (the duration gate prevents flickering), the deadband opens. The dial turns up. A model wakes up — and critically, it wakes up with full context from every tile that has passed through every previous stage in the chain.

The hysteresis is important: the threshold to *exit* deadband is stricter than the threshold to *enter* it. You need to recover *past* the original threshold, not just back to it. This prevents the alert-resolve-alert death spiral that plagues naive monitoring systems. Anyone who's been on-call at 3 AM knows why this matters.

### Why Not Just Auto-Scale Models?

You could argue this is just "use a bigger model when things are hard." But the dial isn't just model size — it's the ratio of code-to-model at a specific pipeline stage. A room at &alpha;=0.3 might use a 7B-parameter model but only for gap-filling: the algorithm handles 90% of cases, and the model handles the 10% that fall through. A room at &alpha;=0.8 might use the same 7B model but for the primary computation, with code only providing guardrails.

The dial controls *responsibility*, not just *capacity*.

---

## 4. Tiles as Context Carriers

The Signal Chain's secret weapon isn't the models. It's the tiles.

A **tile** is a unit of knowledge — a decision, observation, or intermediate result — that passes through the chain. When a room processes a tile, it annotates it: what was decided, what confidence level, what the KPIs looked like at the time. The tile accumulates context as it moves downstream.

This means the model at stage 5 doesn't have to rediscover what stages 1-4 already figured out. The tiles *are* that knowledge, carried at the point of use. When the deadband opens and a model wakes up, it sees:

- The original input
- Every intermediate decision
- Every confidence score
- The specific point where algorithms stopped being sufficient

The model handles only the **delta** — the gap between what rules predicted and what actually happened. That's why small models work at most stages: they're not doing the whole job, just the part that code can't.

This is the guitar-rig equivalent of signal flow. Each pedal receives the output of the previous pedal, not the raw guitar signal. A delay pedal after a distortion pedal delays the *distorted* signal — it doesn't need to understand distortion to do its job. It operates on what it receives. Tiles work the same way: each room operates on accumulated context, not raw input.

### Knowledge Flows Backward Too

When a downstream room discovers something — a pattern, an anomaly, a correction — it can emit a tile that flows backward through the chain. This is how the system self-corrects: if stage 5 discovers that stage 2's classification was wrong, the correction tile updates stage 2's context for future inputs.

In the guitar metaphor, this is like a mixing engineer sending notes back to the guitarist: "your tone is too muddy for this section — roll off some bass." The feedback loop tightens the chain.

---

## 5. The Plinko Model

Here's an unintuitive way to think about what models do in the Signal Chain.

Imagine a Plinko board — the game show toy where a disc drops through a field of pegs and bounces left or right at each one, landing in a slot at the bottom. The path depends on the entry point and the arrangement of pegs.

In the Signal Chain, tiles are the discs. Rooms are the rows of pegs. The model's weights determine the **shape of the peg field** — which paths are more likely, which slots tiles tend to land in.

- **Big model** = complex peg arrangement, captures nuanced path distributions
- **Small model** = simple peg arrangement, captures only dominant paths
- **No model (&alpha;=0)** = straight down, deterministic routing

"Alignment" in this framing is tuning the peg arrangement so tiles land in the right slots. Fine-tuning is reshaping the pegs. Distillation is simplifying the peg arrangement while preserving the most important path biases.

This isn't just a metaphor — it maps directly to how linear algebra works in neural networks. A model is a series of matrix multiplications that transform an input vector through a high-dimensional space. The weight matrices determine which regions of that space are "attractors." A bigger model has more matrices and higher-dimensional spaces, so it can represent more complex attractor landscapes. A smaller model has fewer, simpler attractors — it captures the major modes but misses the fine structure.

The Plinko framing makes one thing clear: **you don't need the same peg complexity at every row.** Some rows just need to route left or right (simple threshold). Some rows need to handle complex multi-path distributions (full model). The Signal Chain lets you set the complexity per row.

---

## 6. Self-Healing Through Re-Entry

When a room hits deadband, the system doesn't just flag a failure and page someone. It initiates a **re-entry loop**:

1. **Detect**: KPIs breach thresholds for a sustained duration.
2. **Freeze**: The room's current state is captured as an immutable snapshot — a Frozen Context Window (FCW). This includes the tile state, KPI values, and the specific point of failure.
3. **Invoke**: A model (at a higher &alpha;) is called with the FCW as context. The model sees everything: the chain state, the failure point, the accumulated tile knowledge.
4. **Propose**: The model generates a response — a new tile containing a proposed fix, reclassification, or escalation.
5. **Validate**: The proposed tile goes through the room's validation pipeline. Does it improve KPIs? Does it break downstream stages?
6. **Lock**: If validation passes, the response becomes a **seed** — a locked checkpoint that the room can use for similar future situations without invoking the model again.

The locked seed is like a lick a guitarist has practiced until it's muscle memory. The first time through, it required full attention (high &alpha;, big model). After locking, it's a reflex (low &alpha;, pattern match against the seed library).

### The Seed Lifecycle

Seeds go through a staged validation pipeline:

```
UNLOCKED → CANDIDATE → VALIDATING → LOCK_PENDING → LOCKED → DEPRECATED → ARCHIVED
```

Each transition has gates. A seed can't reach `LOCKED` without passing validation. A locked seed can be deprecated when a better one is found, or archived when it's no longer relevant. The lifecycle prevents the seed library from becoming a graveyard of stale patterns.

This is the mechanism by which the Signal Chain **learns**. Not in the ML sense of gradient updates, but in the operational sense of building a library of validated responses at the point of use. Each locked seed is a piece of crystallized intelligence — something that once required a model call and now runs as code.

---

## 7. Distillation as Tone Crafting

Every guitarist eventually asks: can I get *this* tone with a smaller rig?

In the Signal Chain, this is the distillation question. A room running at &alpha;=0.8 with a 70B-parameter model is expensive. But the model isn't using all 70 billion parameters for every decision. Most of its responses follow a few dominant patterns. Can we capture those patterns in a 3B model that runs at &alpha;=0.3?

The process:

1. **Record**: Run the room at high &alpha; with a large model. Log every input-output pair, every tile transformation.
2. **Analyze**: Identify the dominant patterns. What does the model actually do 90% of the time?
3. **Distill**: Train a smaller model on the dominant patterns. This is standard knowledge distillation, but scoped to a single room's behavior rather than general capability.
4. **Validate**: Run the distilled model through the room's validation pipeline. Does it handle the common cases? What's the accuracy drop?
5. **Deploy**: Set the room to use the distilled model at a lower &alpha;. The deadband detector will catch cases the smaller model misses, and those escalate to the larger model.

The key insight: because each room has its own dial, distillation doesn't have to be perfect. A distilled model that catches 90% of cases at 5% of the cost is a win — the deadband detector handles the other 10% by turning the dial up temporarily. You're not replacing the big model; you're making it the fallback instead of the default.

In guitar terms: you found a $200 pedal that gets you 90% of the $2,000 boutique pedal's tone. For the one song where you need the last 10%, you swap it in. For the other 30 songs in the set, the cheaper pedal is fine.

### The 20x Compression Target

Based on early experiments with SplineLinear compression (a tensor-spline approach to weight quantization), we've observed roughly 20x parameter reduction while maintaining >90% of the original model's accuracy on room-specific tasks. This number comes with heavy caveats — it's task-specific, architecture-specific, and measured on a narrow evaluation set. But it suggests that room-scoped distillation is more aggressive than general-purpose distillation because the room's task distribution is much narrower than the model's full capability set.

This is speculative. We haven't validated 20x compression at scale. But the theoretical argument is sound: a model that only needs to handle one room's decision space should be dramatically compressible compared to a model that handles everything.

---

## 8. The Frozen Context Window

When a room hits deadband, the first thing that happens is a **freeze**. The room's entire state — KPIs, tile context, recent decisions, breach metadata — is captured as an immutable snapshot called a Frozen Context Window (FCW).

FCWs are:

- **Immutable**: Once frozen, they can't be modified. Any state change produces a new FCW via copy-on-write. This guarantees that the debugging snapshot you're looking at is exactly what the system saw at the time of the event.
- **Content-addressed**: Each FCW gets a SHA-256 hash of its contents for deduplication. If the same room hits the same deadband with the same state twice, you get one FCW, not two.
- **Lifecycle-managed**: FCWs go through `STAGING → FROZEN → TESTING → REFINING → LOCKED` (or `DISCARDED`). This prevents unbounded accumulation.

### Why Immutability Matters

Mutable state snapshots are the debugging equivalent of reading a newspaper that someone keeps editing while you read it. By the time you've figured out what happened, the evidence has changed.

FCWs solve this by being write-once. When an incident occurs:

1. The FCW captures the exact state.
2. The model receives this exact state as context.
3. The model's response is validated against this exact state.
4. If the response becomes a seed, the seed references this exact FCW.

You can replay any decision. You can diff any two FCWs. You can trace exactly why the system made the choice it made, because the context window it operated on is frozen in time.

This is the "recording studio" part of the guitar metaphor. Every performance is tracked. You can go back and listen to the raw take, the processed take, and the final mix. You can A/B test different pedal settings against the same raw performance.

### Cost Control Through Redaction

FCWs accumulate. Without pruning, storage grows linearly with deadband events. The redaction engine handles this by computing a KPI-space distance metric between FCWs: if two snapshots are close in KPI-space (they represent similar failure modes), the lower-value one can be pruned.

"Lower-value" is defined by a combination of age, whether the FCW led to a locked seed, and whether the failure mode it represents has been addressed. FCWs that contributed to successful seeds are kept; redundant snapshots of already-solved problems are pruned.

---

## 9. Related Work

The Signal Chain doesn't exist in a vacuum. Several lines of research address similar problems with different approaches.

### Mixture of Experts (MoE)

MoE architectures (Shazeer et al., 2017; Fedus et al., 2022) route inputs to specialized sub-networks via a gating function. The Switch Transformer activates one expert per token; GShard and GLaM use top-k routing.

The Signal Chain's dial is conceptually similar to MoE gating but operates at a different level. MoE routes within a single model's forward pass — it's an intra-model optimization. The Signal Chain routes between entire computation stages, each of which might use a different model (or no model). The dial is inter-stage, not intra-layer.

Additionally, MoE experts are trained jointly. Signal Chain rooms are independently configurable and can use different model architectures, different training data, and different dial settings. This is closer to an ensemble of specialists than a gated mixture.

### Adaptive Computation

Graves (2016) introduced Adaptive Computation Time (ACT), where a network learns how many processing steps to apply per input. Universal Transformers (Dehghani et al., 2019) extended this to transformer architectures. More recently, early-exit strategies (Schuster et al., 2022) let models skip later layers for easy inputs.

The Signal Chain's deadband detector is philosophically similar: don't spend compute on inputs that don't need it. But ACT operates within a single model's architecture, while the Signal Chain operates across a pipeline of stages. The granularity is different — ACT adjusts per-token compute; the Signal Chain adjusts per-stage compute for entire batches of inputs.

### Routing Networks and Cascades

FrugalGPT (Chen et al., 2023) cascades through models of increasing size, stopping when confidence is high enough. Routing networks (Rosenbaum et al., 2018) learn to compose modules for different inputs. These are the closest relatives to the Signal Chain's architecture.

The key difference: cascades and routing networks typically operate on a single task with multiple model options. The Signal Chain operates on a pipeline with multiple stages, each handling a different aspect of the computation. A cascade asks "which model for this input?" The Signal Chain asks "which model for this input at this stage?"

### Agent Frameworks

LangChain, AutoGPT, CrewAI, and similar frameworks compose LLM calls into multi-step workflows. They address the orchestration problem — how to chain model calls together — but typically treat each step as a full model invocation. There's no per-step dial controlling the code-vs-model ratio.

The Signal Chain's contribution relative to agent frameworks is the dial itself: the idea that each orchestration step should be parameterized by how much work is algorithmic vs. model-driven, and that this parameter should be dynamically adjusted based on observed performance.

### Spectrum of Automation

The concept of a "level of automation" dial has precedent in robotics (Sheridan & Verplank, 1978) and human-AI teaming (Parasuraman et al., 2000). The Sheridan scale defines 10 levels from "human does everything" to "computer does everything." The Signal Chain's &alpha; parameter is essentially this scale, applied per-stage in a computation pipeline rather than per-system.

---

## 10. Proof of Concept: Spreader-Tool

### What It Is

[Spreader-tool](https://github.com/SuperInstance/spreader-tool) is a Python implementation of the deadband detection, FCW management, and seed locking components of the Signal Chain. It runs 241 tests in under a second, has zero dependencies beyond the Python standard library, and implements the core monitoring pipeline described in this paper.

The module structure:

| Module | Purpose |
|--------|---------|
| `types.py` | FCW, Seed, KPI dataclasses with frozen state machines |
| `deadband.py` | Threshold monitoring with hysteresis and duration gates |
| `frozen_context.py` | FCW lifecycle management |
| `seed_lock.py` | 8-state seed validation pipeline |
| `store.py` | Content-addressed file storage |
| `spreader_room.py` | Core orchestrator — the 8-step intelligence tiling loop |
| `cost.py` | Inference cost tracking |
| `redaction.py` | KPI-space distance pruning |
| `cli.py` | CLI with 8 subcommands |
| `self_optimize.py` | Development monitoring harness |
| `development_patterns.py` | Locked pattern library |

### What Works Well

The beta review (a deliberately brutal code review by a skeptical senior engineer) scored the project 6/10 overall. Here's what earned its marks:

**Frozen dataclass state machines (7/10 code quality).** The `FrozenContextWindow` and `Seed` types use `frozen=True` dataclasses with explicit transition tables. State changes produce new instances via copy-on-write. The transition guard counter detects stale references. This pattern is genuinely production-grade — it enforces state machine invariants at the type level.

**Hysteresis (7/10 test quality).** The deadband detector doesn't just check "is the metric above threshold?" It requires sustained violations (duration gates) and stricter recovery thresholds (hysteresis exit factor of 1.1x). The test suite specifically verifies that marginal recovery does *not* exit deadband — a test that catches the exact bug you'd ship in a naive implementation.

**Zero dependencies.** Pure Python `dataclasses` and standard library. No PyYAML, no Pydantic, no framework lock-in. The library can be dropped into any Python project without dependency conflicts. This is claimed in the README and actually delivered, which is refreshingly uncommon.

**Behavioral test suite.** Of the 241 tests, the deadband and state machine tests stand out. They test properties ("severity increases monotonically," "hysteresis prevents flickering") rather than implementation details. The test helpers (`good_metrics()`, `bad_completion()`) make tests readable.

### What the Beta Review Found

The reviewer was right about several things, and we're presenting their findings unedited because the HN audience will find them anyway.

**"Self-optimization" doesn't optimize.** The `SelfOptimizer` class runs pytest, parses output, computes KPIs, and generates a markdown report. It identifies opportunities but doesn't act on them. Calling this "self-optimization" is a stretch. It's a monitoring dashboard. We're keeping the name because the *architecture* supports optimization (the FCW → seed → lock pipeline is designed for it), but the current implementation is monitoring-only. Fair criticism.

**Inflated module count.** The reviewer identified that the "real" library is 5 core modules, with 6 additional modules that are either infrastructure, tooling, or — in the case of `development_patterns.py` — "blog content dressed as a module." Also fair. The pattern library is a static list of 7 hardcoded entries with keyword search. It's illustrative, not functional.

**`dataclasses.replace()` not used.** This is the most embarrassing finding. The entire architecture is built on frozen dataclasses, but `transition_to()` methods hand-copy every field instead of using `dataclasses.replace()`, which exists specifically for this purpose. Every new field added to `Seed` or `FrozenContextWindow` must be updated in the copy constructor. This is a maintenance bomb.

**Mutable `DeadbandState`.** Everything else in the codebase is frozen. `DeadbandState` isn't. The reviewer called this "inconsistent immutability is worse than consistent mutability." Correct.

**"Deadband" is a misnomer.** In control theory, deadband is the range where input changes produce no output. What spreader-tool actually implements is threshold-based anomaly detection with hysteresis — closer to an operating envelope boundary detector. The reviewer suggested "anomaly detection with snapshot-and-validate" as a more honest description. They're technically right, though we'd argue the term works metaphorically even if it's not a precise control-theory usage.

**Seeds auto-lock without human review.** When KPIs are good (≥95% completion), the `_update_seed` method proposes, validates, and locks a seed in one function call. The "validation" is a single threshold check. The locked seed contains a `weights_ref` of `"local://baseline"` — a string that doesn't point to actual model weights. This is the biggest gap between the architecture's promise and the implementation's reality.

### Honest Assessment

Spreader-tool demonstrates that the **monitoring and state-management infrastructure** of the Signal Chain works. Deadband detection with hysteresis, immutable state snapshots, content-addressed deduplication, and staged seed lifecycles are all implemented and tested.

What it does *not* demonstrate is the **intelligence layer** — the actual model invocation, the dial-turning, the distillation pipeline. There's no AI in the current implementation. KPIs go in, alerts come out, snapshots get saved. The architecture has slots for models (the &alpha; parameter, the `weights_ref` field, the re-entry loop), but those slots are currently empty.

This is version 0 of the signal chain — the monitoring infrastructure that the intelligence layer will plug into. It's the pedalboard without the pedals. The wiring is correct, the power supply works, and the signal path is clean. But there's no distortion, no delay, no reverb yet.

We think that's the right order of operations. Get the monitoring and state management right first, then add models. But we want to be clear about what currently exists versus what's planned.

---

## 11. Open Questions

### Auto-Tuning the Dial

The current design assumes a human operator sets &alpha; per room, or that the deadband detector adjusts it reactively. But can we learn optimal &alpha; settings?

This is a multi-armed bandit problem: each dial setting has a cost (inference spend) and a reward (KPI performance). The challenge is that rooms interact — changing &alpha; at stage 3 affects what stage 5 sees. The dial settings aren't independent.

Bayesian optimization over the joint dial space is the obvious approach, but the dimensionality grows linearly with chain length. A 10-room chain with 10 possible dial settings each has 10^10 configurations. Even with strong priors, exhaustive search is infeasible.

We suspect the right approach is hierarchical: optimize individual rooms first (holding others fixed), then optimize pairs of adjacent rooms, then the full chain. This mirrors how guitarists actually dial in their rigs — start with the amp, then add pedals one at a time.

### Optimal Model Size Per Room

Given a room's task distribution, what's the smallest model that achieves acceptable performance? This is the distillation question, and it depends on the task's intrinsic dimensionality.

Some rooms have low-dimensional decision spaces — binary classification, threshold comparison, simple routing. These might work with models as small as 1M parameters. Other rooms require nuanced reasoning over complex context — these might need 7B+ parameters even after distillation.

We don't yet have a principled way to estimate the minimum viable model size for a room without trial and error. A theoretical framework connecting task complexity to model capacity requirements would be valuable.

### Chain Topology

We've described linear chains (stage 1 → stage 2 → ... → stage N), but real pipelines have branches, merges, and feedback loops. How does the Signal Chain architecture extend to DAGs?

The tile-as-context-carrier model extends naturally: tiles follow edges in the DAG, accumulating context along their path. But the deadband detector becomes more complex — a room with multiple inputs needs to determine which upstream path is causing the KPI breach.

Feedback loops are harder. If stage 5 emits a correction tile that flows back to stage 2, and stage 2 re-processes and changes what stage 5 sees, you get a cycle. Convergence isn't guaranteed. The current architecture doesn't address this beyond "validate before locking," which is necessary but not sufficient.

### When Does the Chain Break?

There are pathological cases:

- **Cascading deadband**: Every room in the chain hits deadband simultaneously. Every room calls a model. Cost spikes exponentially.
- **Tile bloat**: Long chains accumulate so much context that downstream models spend most of their context window on upstream history rather than the current problem.
- **Stale seeds**: A locked seed works for months, then the input distribution shifts and it starts producing wrong answers. The deadband detector should catch this, but only if the KPIs are sensitive to the failure mode.
- **Adversarial inputs**: An input crafted to exploit the gap between the algorithm (&alpha;=0) and the deadband threshold. The system runs algorithmically, produces a wrong answer, and KPIs don't breach because the error is within normal variance.

We don't have solutions for all of these. Cascading deadband might be addressed with a chain-level budget (total inference spend across all rooms, enforced globally). Tile bloat can be mitigated with summarization at intermediate stages. Stale seeds need periodic re-validation. Adversarial robustness is an open problem everywhere, not just here.

### Multi-Agent Coordination

The Signal Chain as described assumes a single pipeline processing a single input stream. But PLATO rooms are shared state spaces where multiple agents coordinate. How do multiple agents with different dial settings interact in the same room?

This is unexplored territory. The architecture supports it in principle — each agent has its own dial, and the room's KPIs reflect the aggregate behavior — but we haven't built it or tested it.

---

## 12. Conclusion: The Guitarist's Rig

Here's the Signal Chain thesis in full:

**Every computation stage in a pipeline has a spectrum from pure algorithm to full model. The missing parameter in modern AI systems is a per-stage dial for where on that spectrum each stage operates.**

The Signal Chain architecture gives you:

- **Rooms** with individually tunable &alpha; parameters (the pedalboard)
- **Deadband detection** that turns the dial up when algorithms can't cope (the volume pedal)
- **Tiles** that carry context through the chain so models don't re-derive upstream knowledge (the signal path)
- **Frozen context windows** that capture exact state for debugging and replay (the recording studio)
- **Seeds** that crystallize model responses into reusable patterns (muscle memory)
- **Distillation** that compresses large-model behavior into room-scoped micro-models (the $200 pedal that sounds like the $2,000 one)

The guitarist doesn't play through a single pedal cranked to 10. They build a chain, set each dial for what that stage needs, and adjust as the song changes. The Signal Chain brings this approach to AI systems: not "how much AI?" but "how much AI, where, right now?"

Spreader-tool is the first piece — the monitoring infrastructure, the wiring, the power supply. The pedals themselves — the model integrations, the distillation pipeline, the auto-tuning — are next.

The rig isn't finished. But the signal path is clean, the connections are solid, and 241 tests say the wiring works.

Time to plug in some pedals.

---

## Appendix: Glossary

| Term | Definition |
|------|-----------|
| **Room** | A computation stage in the Signal Chain. Holds KPIs, state, and a dial setting. |
| **Dial (&alpha;)** | Per-room parameter &isin; [0,1] controlling code-vs-model ratio. |
| **Deadband** | The gap between what algorithmic code handles and what requires model intervention. (Loosely borrowed from control theory.) |
| **Tile** | A unit of knowledge (decision, observation, result) that flows through the chain, accumulating context. |
| **FCW** | Frozen Context Window. Immutable state snapshot captured at deadband events. |
| **Seed** | A validated, locked response pattern derived from model output. Replaces future model calls for similar inputs. |
| **Hysteresis** | Requiring recovery *past* the breach threshold to exit deadband, preventing alert flickering. |
| **Signal Chain** | The full pipeline of rooms, each with dial settings, connected by tile flow. |

---

*This paper was written in May 2026. Spreader-tool is MIT-licensed and available at [github.com/SuperInstance/spreader-tool](https://github.com/SuperInstance/spreader-tool). The thesis emerged from building the tool, not the other way around — the architecture is a post-hoc explanation of patterns we discovered while trying to make agent rooms cheaper to operate.*
