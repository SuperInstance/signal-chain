# The Volume Pedal: Deadband Detection as Model Engagement Control

Every guitarist knows the volume pedal. It sits in the signal chain, barely touched most of the time. When it's heel-down, the rig plays clean — the amp does its thing, the pedals do their thing, and the sound just works. But when something changes — the room gets louder, the band shifts dynamics, the part needs more presence — you lean into the pedal and the whole chain wakes up.

Deadband detection does the same thing for model pipelines. It's not the amp. It's not the guitar. It's the thing that decides *when to turn everything else on*.

## What Deadband Actually Means (And Why We Inverted It)

In control theory, deadband is dead space — the input range where nothing happens. Turn your thermostat a degree either way and the furnace doesn't care. That gap between "I noticed a change" and "I'm going to do something about it" is deadband. It exists to prevent jitter. Small fluctuations get ignored. Only real signals get through.

Here's the inversion: in a PLATO room, deadband isn't where the system ignores input. It's where the *model needs to engage* because the rules can't handle what's happening.

Think of it this way. Your room has two operating modes:

1. **Rules mode** — Fast, deterministic, cheap. Handles everything within the known envelope.
2. **Model mode** — Expensive, flexible, intelligent. Handles the gaps rules can't cover.

The deadband detector watches the boundary between these two modes. When KPIs are within thresholds, you're in rules mode — pure algorithm, no model cost. When the envelope gets breached, the detector signals that the model needs to wake up. The deadband *is the gap* between what rules handle and what needs intelligence.

The volume pedal stays heel-down until the signal chain needs more. Then it ramps.

## Four KPIs, One Envelope

The operating envelope is defined by four metrics. Each one is a boundary condition:

| KPI | Threshold | What it means |
|-----|-----------|---------------|
| Task completion rate | < 90% | Rules are failing to resolve cases |
| Average wait time | > 30s | Rules are bottlenecking or deferring too much |
| Energy over baseline | > 10% | The system is working harder than expected |
| Inference MAE | > 10% | Model predictions are drifting from reality |

Any one of these breaching its threshold is a signal that the rules-only path isn't cutting it. But — and this matters — one bad tick doesn't mean deadband.

## Duration Gates: One Bad Tick ≠ Deadband

This is the first place where naive threshold checking fails and real engineering begins.

Imagine completion rate drops to 85% for one evaluation window, then recovers to 94%. Was that deadband? No. That's noise. A single bad measurement, a GC pause, a network hiccup. If you trigger model engagement on every blip, you've built the world's most expensive noise amplifier.

Duration gates solve this. Each KPI has a minimum sustain time before it counts:

- Completion rate must be below 90% for **5 minutes sustained**
- Wait time must be above 30s for **30 seconds sustained**
- Energy over baseline for **30 seconds sustained**
- Inference MAE for **3 consecutive evaluation windows**

The model doesn't wake up because of a momentary dip. It wakes up because the system has been struggling for long enough that the struggle is real. This is the difference between "I stubbed my toe" and "my foot is broken." Both hurt. Only one needs a doctor.

## Hysteresis: Preventing the Flicker

Here's the second place where naive thresholding fails. Suppose completion rate drops to 85%, stays there for 5 minutes, and deadband triggers. The model engages. Completion recovers to 91%. Should deadband exit?

No. Not yet.

Because 91% is *barely* above the 90% threshold. If you exit deadband now, the model disengages, and completion promptly drops back to 85% (because the model was the thing holding it up). Then deadband triggers again. Model engages. Completion recovers. Exit. Drop. Trigger. Engage. **Flicker.**

This is the classic control theory oscillation problem, and the solution is hysteresis. The deadband detector uses a `hysteresis_exit_factor` of 1.1×. To exit deadband, KPIs must recover *past* the threshold by 10%. If the threshold is 90%, recovery means hitting 99%. If the wait time threshold is 30s, recovery means dropping below 27s.

This creates a dead zone between "trigger" and "recovery" that prevents the system from oscillating between model-on and model-off states. The model stays engaged until the problem is *actually solved*, not just barely ameliorated.

Anyone who's dealt with alert storms in production knows this pattern. Anyone who hasn't dealt with alert storms hasn't run production long enough.

## Severity Scoring: How Loud Should the Pedal Be?

Binary deadband (on/off) is a start, but the real power is a continuous signal. The severity score tells you *how much* model engagement you need.

```
severity = breach_fraction × duration_factor
```

**Breach fraction** measures how far past the threshold you are. If completion rate is at 70% and the threshold is 90%, the breach is 20 percentage points out of a 90-point scale — roughly 0.22. If completion is at 40%, the breach is massive. This gives you "how bad."

**Duration factor** measures how long you've been in breach. It ramps from 0.3 to 1.0 over 10 minutes. A fresh breach gets a 0.3 multiplier (don't overreact yet). A sustained 10-minute breach gets 1.0 (this is serious). This gives you "how long."

The product gives you a 0–1 severity signal:

- 0.0–0.2: Niggle. Monitor but don't escalate.
- 0.2–0.5: Growing concern. Consider light model engagement.
- 0.5–0.8: Active problem. Full model engagement with context.
- 0.8–1.0: Critical. Pull out the big model, full tile history, all hands.

This is the volume pedal's sweep. Heel-down at 0, toe-down at 1, and every position in between maps to a different level of model investment.

## A Concrete Example

Let's walk through a real scenario with numbers.

**T=0:** A room is processing tiles normally. Completion rate: 96%. Wait time: 12s. All KPIs green. Deadband detector: `in_deadband=False`, `severity=0.0`. Model cost: zero. Pure rules.

**T=5min:** A new class of tiles arrives — edge cases the rules weren't designed for. Completion rate drops to 82%. Breach detected (threshold: 90%). But duration gate is 5 minutes. `in_deadband=False`. The detector is watching.

**T=10min:** Completion still at 82%. Duration gate met. Deadband triggers. `in_deadband=True`. Severity: `breach_fraction=0.09` (8 points below 90/90), `duration_factor=0.3` (just crossed). Severity = 0.09 × 0.3 = **0.027**. Low severity, but the model is now engaged. A micro-model wakes up with tile context from the current FCW (Frozen Context Window).

**T=20min:** Completion drops further to 68%. Severity: `breach_fraction=0.24` (22 points below 90/90), `duration_factor=0.6`. Severity = **0.144**. The micro-model isn't cutting it. A medium model gets called in with broader context.

**T=30min:** Completion at 55%. Severity: `breach_fraction=0.39`, `duration_factor=0.9`. Severity = **0.351**. Now we're cooking. Full model engagement. The big model sees the entire tile history — what stages 1–4 computed, where the gap opened, what previous attempts looked like.

**T=45min:** The model proposes a fix (a new tile pattern). Validation runs. Completion recovers to 92%. But hysteresis requires 99% to exit. Deadband continues. The model keeps refining.

**T=55min:** Completion hits 99%. Hysteresis cleared. Deadband exits. The proposed fix gets validated, locked as a seed, and deployed fleet-wide. Next time this pattern appears, the rules handle it. No model needed.

Total model cost: ~50 minutes of engagement across three model tiers. Without deadband detection, you'd either run the big model 24/7 (expensive) or never catch the edge cases (broken).

## Why This Isn't Just a Circuit Breaker

If you're thinking "this sounds like a circuit breaker," you're close but not quite right. Circuit breakers are binary — open or closed. They protect downstream systems from overload by cutting the circuit. When the breaker trips, everything stops.

Deadband detection is *not* binary. The severity score is continuous. The response is proportional. And crucially, deadband detection doesn't cut the circuit — it *turns up the intelligence*. The room keeps processing tiles. The rules keep running. The model joins alongside, not instead of.

It's closer to **adaptive computation** in neural networks — investing more compute on harder inputs and less on easy ones. But where adaptive computation adjusts within a single model (early exit, sparse attention), deadband detection adjusts across the entire algorithm-to-model spectrum. It doesn't change how the model thinks. It changes *whether the model thinks at all*.

It's also distinct from **rate limiting**. Rate limiting throttles volume. Deadband detection throttles *intelligence spend*. A rate limiter says "you're asking too much." A deadband detector says "you're *needing* too much intelligence — let's understand why."

## The Dial Is the Deadband

Here's the thesis in one sentence: **deadband IS the dial.**

In the signal chain metaphor, every room has a dial controlling model vs code. Dial at 0: pure algorithm. Dial at 10: full agent. The deadband detector turns that dial automatically based on real-time KPI feedback.

The relationship is direct:

- α = 0 when KPIs are within thresholds. No model. Rules handle everything.
- α → 1 when severity is high. Full model engagement with complete tile context.
- α ramps smoothly between 0 and 1 based on severity scoring.

You don't set the dial manually. You set the thresholds — the operating envelope — and the deadband detector rides the dial for you. The fleet operator's job isn't to micromanage model calls. It's to tune the thresholds the same way a sound engineer tunes a mixing desk. Set the gain structure, then let the system run.

This is why the volume pedal metaphor is exact. The guitarist doesn't think about signal levels. They set their rig, dial in the tone, and play. The volume pedal responds to dynamics automatically. Deadband detection responds to KPI dynamics the same way.

## Implementation Reference

The spreader-tool implements this as a ~200-line `DeadbandDetector` with:

- `update(metrics: KPIMetrics) -> DeadbandState` — feed KPIs, get state
- Duration gates per metric (5min, 30s, 30s, 3 windows)
- `hysteresis_exit_factor=1.1` — 10% past threshold to recover
- Severity scoring: `breach_fraction × duration_factor` with 0.3→1.0 ramp
- Zero dependencies. Pure Python. Drop-in.

The detector produces a `DeadbandState` with `in_deadband: bool`, `severity: float`, and `breached_metrics: list`. Downstream systems (FCW freeze, seed validation, model routing) consume this state to decide what to do.

## Why It Matters

The industry's current approach to model integration is binary: either you have a model or you don't. Either GPT handles the whole request or a hardcoded pipeline does. There's no middle ground, no proportional response, no continuous signal.

Deadband detection gives you that middle ground. It makes model engagement a continuous variable controlled by real-time feedback. It turns the binary "use AI / don't use AI" decision into a volume knob with actual signal driving it.

The result: you use models where they're needed, not everywhere. You catch edge cases without burning compute on routine ones. And the system self-tunes because the feedback loop is closed — KPIs drive the detector, the detector drives model engagement, model engagement improves KPIs.

Quiet when things work. Loud when they don't. That's a volume pedal.

---

*Built as part of the spreader-tool — intelligence tiling for PLATO rooms. The deadband detector runs 241 tests in under a second. The hysteresis tests specifically verify that marginal recovery doesn't exit deadband, because someone has production scars.*
