# signal-chain

> **Every room has a dial. The dial controls model vs code. Tune the chain like a synth.**

---

## The Thesis

Modern AI systems are monolithic — either rigid pipelines of if/then rules, or big models that throw tokens at everything. Both fail for the same reason: one dial, one size, no shape.

The Signal Chain Thesis says: **each stage in a computation pipeline should have a tunable dial for how much model vs how much code it uses.** When rules handle it, the model sleeps. When the gap opens, the model wakes up with full context from every stage before it.

Like a guitarist's signal chain: wood → strings → pickups → pedals → amp → speaker → mic → console. Each stage shapes the tone. Each stage has a dial. You don't play raw — you craft your sound through the chain.

**The missing parameter in AI systems is the per-room dial.**

---

## Papers

📄 **[The Signal Chain Survey](papers/SIGNAL-CHAIN-SURVEY.md)** — the main paper. Why every room needs a dial, what that means for building intelligent systems, and how it changes everything about model deployment. *Start here.*

Specific papers that expand each concept:

- 🎸 **[Deadband as the Volume Pedal](papers/DEADBAND-VOLUME-PEDAL.md)** — when does the model wake up? KPI thresholds, hysteresis, duration gates, and the severity ramp from 0.3 to 1.0.
- 📦 **[Tiles as Context Carriers](papers/TILES-AS-CONTEXT.md)** — why the model at stage N doesn't need to rediscover stages 1 through N-1. Knowledge stored at the point of use.
- 🎰 **[The Plinko Model](papers/PLINKO-MODEL.md)** — models as linear algebra shapes that weight paths through the tile graph. Distillation is tone selection, not compression.
- 🔄 **[Self-Healing Through Re-Entry](papers/SELF-HEALING-REENTRY.md)** — when rooms call the agent back with full tile context. Detect → freeze → fix → validate → lock.
- 🎛️ **[Distillation as Tone Crafting](papers/DISTILLATION-TONE-CRAFTING.md)** — model size per room is the new hyperparameter. 20× compression at same accuracy, 48/48 proven deployments.

---

## Proof of Concept

The thesis isn't just theory. We built it.

| Repo | What it proves | Tests |
|------|---------------|-------|
| [**spreader-tool**](https://github.com/SuperInstance/spreader-tool) | Deadband detection, frozen context windows, seed locking, self-optimization | 241 |
| [**plato-training**](https://github.com/SuperInstance/plato-training) | Micro-model training, 8 tasks × 8 hardware targets, fleet deploy | 116 |
| [**tensor-spline**](https://github.com/SuperInstance/tensor-spline) | SplineLinear compression (Eisenstein lattice weights, 20× at same accuracy) | 57 |
| [**plato-types**](https://github.com/SuperInstance/plato-types) | Tile lifecycle, Lamport clocks, content-addressed storage | 10 |
| [**plato-data**](https://github.com/SuperInstance/plato-data) | CSV/JSONL/PLATO/fleet data loading | 10 |
| [**spectral-conservation**](https://github.com/SuperInstance/spectral-conservation) | When conservation breaks (CV=0.69 under rapid cycling) — published on crates.io | 12 |
| [**constraint-theory-core**](https://github.com/SuperInstance/constraint-theory-core) | The math underneath — published on crates.io | 45+ |

---

## The Key Numbers

- **20× compression** — SplineLinear on drift-detect at same accuracy
- **48/48 proven** — all task×hardware combos deploy successfully
- **<1ms inference** — sub-millisecond across all CPU targets
- **241 tests** — spreader-tool MVP, zero dependencies
- **100% accuracy** — drift-detect on 5/6 hardware targets
- **655+ tests** — across the full PLATO ecosystem

---

## The Guitarist's Rig

```
Input tile → Rule-based filter → Micro-model (dial: 2) →
Deadband check → Medium model with context (dial: 6) →
Seed validation → Locked pattern → Output tile
```

Each room is a pedal on the board. The deadband detector is the volume pedal — quiet when things work, loud when they don't. The tiles carry the signal forward. The model shapes the tone.

**You're not stuck with one model size. You craft the sound by experimenting with each room's chain until the output quality matches your needs.**

---

## Read More

- [Ecosystem Map](https://github.com/SuperInstance/forgemaster/blob/master/ECOSYSTEM-MAP.md) — 80+ repos, 655+ tests, 6 languages
- [Getting Started](https://github.com/SuperInstance/forgemaster/blob/master/GETTING-STARTED.md) — 3-path onboarding (Math / Models / Ecosystem)
- [Assembly Guide](https://github.com/SuperInstance/forgemaster/blob/master/ASSEMBLY-GUIDE.md) — 5 self-assembly patterns, pick what you need
- [Roadmap](https://github.com/SuperInstance/forgemaster/blob/master/ROADMAP.md) — 30KB birds-eye view of where this is going
- [Narrows Demo](https://superinstance.github.io/cocapn-ai-web/) — watch boats drift in 3 precision levels (E12 vs F32 vs F64)

---

*The missing parameter was always there. We just didn't have rooms to put it in.*
