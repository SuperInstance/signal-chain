# Show HN: The Signal Chain – Why every room needs a dial for model vs code

---

## Version 1: The Post

Modern AI systems force a binary choice that makes no sense: run everything as deterministic code, or hand everything to a model. Both fail. The Signal Chain is an architecture where each computation stage carries a tunable parameter α ∈ [0,1] controlling how much work is done by code versus a language model. At α=0, pure algorithms at wire speed. At α=1, full agent. The interesting territory is between.

**The problem, concretely.** If you build a pipeline of rules, it works perfectly on anticipated cases and shatters on everything else. If you hand it all to an LLM, you burn compute re-discovering what your rules already know. The failure mode is the same: treating the entire pipeline as a single unit that's either "code" or "model." But real pipelines have stages, and each stage has different needs. A data-validation step doesn't need GPT-4. An escalation-routing step probably does. Setting every dial to the same position is like a guitarist cranking every knob to 10 and wondering why it sounds wrong.

**The solution: a per-stage dial.** Each stage is a "room." Each room has α. A deadband detector watches the room's KPIs — when completion rate drops, when wait times spike, when error exceeds threshold — and turns the dial up. But only for that room. The model wakes up with full context from every upstream stage (carried by "tiles" — annotated knowledge units that accumulate decisions as they flow through the chain). The model handles only the delta between what code predicted and what actually happened.

**Why small models work.** The model at stage 5 doesn't rediscover what stages 1-4 already figured out. The tiles carry that knowledge. So a 3B model handling one room's decision space outperforms a 70B model re-deriving everything from scratch. Distillation becomes per-room tone selection: identify the dominant patterns, compress, deploy the small model as default, keep the large model as fallback for when deadband opens.

**The guitar metaphor isn't decorative.** Guitar → pickups → volume pot → overdrive → delay → amp → speaker → mic. Each stage shapes the signal. Each has a dial. The deadband detector is the volume pedal — quiet when things work, loud when they don't. Seeds (validated response patterns that lock in) are practiced licks that become muscle memory. Distillation is getting 90% of the boutique tone from the $200 pedal.

**The numbers:**
- 20× parameter compression (SplineLinear, task-specific, narrow eval — honest caveat)
- 48/48 task × hardware combinations deploy successfully
- <1ms inference across all CPU targets
- 241 tests, zero dependencies, pure Python stdlib
- 100% accuracy on drift-detection for 5/6 hardware targets
- 655+ tests across the full ecosystem (7 repos, 3 languages)

**What a brutal beta review found (6/10 overall).** We asked a zero-context reviewer to tear it apart. They were right about several things:

- "Self-optimization" doesn't optimize — it generates reports. Monitoring ≠ optimization. Fair.
- The frozen-dataclass state machines don't use `dataclasses.replace()`. Baffling miss. Maintenance bomb.
- Seeds auto-lock without human review. "Validation" is a single threshold check. The `weights_ref` field points to a string literal. Biggest gap between architecture and implementation.
- "Deadband" is a misnomer. What's implemented is threshold-based anomaly detection with hysteresis. Technically correct, though we argue the term works as metaphor.
- There's no AI in the current implementation. KPIs go in, alerts come out, snapshots get saved. The architecture has slots for models, but they're empty.

We're publishing the full review unedited: https://github.com/SuperInstance/signal-chain (linked from repo).

**What we think matters regardless of execution gaps:** the *parameter* is missing from every agentic framework we've seen. LangChain, AutoGPT, CrewAI — they treat each step as a full model invocation. No per-step dial, no dynamic code-vs-model adjustment. The Sheridan scale (1978) defines levels of automation from "human does everything" to "computer does everything." But it's applied per-system. We're saying: apply it per-stage.

**Repo:** https://github.com/SuperInstance/signal-chain
**Full survey paper (35KB):** https://github.com/SuperInstance/signal-chain/blob/master/papers/SIGNAL-CHAIN-SURVEY.md
**Proof of concept:** https://github.com/SuperInstance/spreader-tool
**Live demo (tile drift visualization):** https://superinstance.github.io/cocapn-ai-web/

Genuine question for HN: Is the per-room α parameter actually novel, or are people already doing this under different names? Model cascades (FrugalGPT) are the closest relative we found, but they ask "which model?" — we're asking "which model, at which stage, at what responsibility ratio?" If you're building something similar, we'd love to know.

---

## Version 2: The First Comment (author follow-up)

(Post as first comment immediately after submission)

---

Author here. Some context on how this emerged and where we are honestly.

**Origin story.** This didn't start as a thesis. It started as a cost problem. We were running agent "rooms" (shared computation spaces in the PLATO architecture) and each room was burning full model calls on every input. Most inputs were routine — 90% could be handled by a threshold check and a lookup table. The insight was embarrassingly simple: what if each room had a dial, and we only turned it up when the easy stuff stopped working?

**Concrete example.** Drift-detection room. α=0.2 (SplineLinear micro-model, 20× compressed). Input: time-series data from fleet sensors. The algorithm handles stable readings. When drift exceeds threshold for sustained duration (hysteresis prevents flickering — anyone who's been paged at 3 AM for a transient spike knows why this matters), deadband opens, dial turns up, larger model gets the full tile context: what every upstream room decided, at what confidence, where the divergence started. Result: 100% accuracy on 5/6 hardware targets, <1ms inference, running on CPU. The 6th target (a specific ARM variant) hits 94% — we think it's a quantization boundary issue, still investigating.

**What the beta reviewer got right that we're fixing:**

1. `dataclasses.replace()` — shipping this week. Embarrassing to miss.
2. Seed auto-locking — adding a `CANDIDATE → HUMAN_REVIEW → LOCKED` gate. Seeds shouldn't self-approve.
3. The "self-optimization" naming — renaming to what it actually is: a development dashboard with KPI tracking.
4. Mutable `DeadbandState` — making it frozen with proper transitions like everything else.

**What we're building next:**

- Actual model invocation in the re-entry loop (the biggest gap — architecture has the slots, implementation doesn't fill them yet)
- Multi-room dial optimization (it's a multi-armed bandit problem complicated by room interactions — changing α at stage 3 affects what stage 5 sees)
- Chain topology beyond linear (real pipelines branch and merge; tiles extend to DAGs but deadband detection gets complex with multiple inputs)
- Publishing the full beta review to the repo so anyone can read it

**What I'd most like criticism on:** Is the deadband-as-volume-pedal metaphor actually clarifying, or is it obfuscating a straightforward concept (anomaly detection + hysteresis + state snapshot) behind a guitar analogy? The beta reviewer thought the latter. I think the metaphor reveals structure that "anomaly detection" hides — specifically, that the *response* to the anomaly is a graduated dial turn, not a binary switch. But I'm biased.

Also: the Plinko model (models as shaped peg fields determining tile routing probabilities) — is that actually a useful way to think about weight matrices, or am I just making a metaphor do too much work?
