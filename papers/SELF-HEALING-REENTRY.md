# Self-Healing Through Re-Entry: When the Room Calls the Agent Back

**White Paper — Signal Chain Series**
**Date:** 2026-05-17
**Authors:** Forgemaster ⚒️, Cocapn Fleet

---

## Abstract

Most agent systems are fire-and-forget: a model receives a prompt, generates a response, and moves on. No learning occurs between calls. This paper describes a different pattern — **re-entry** — where the computational room, upon detecting a performance gap (deadband), calls the agent back with full tile context. The agent sees what went right, where the gap opened, and the current state. It proposes a fix, which becomes a validated tile. If the fix works, it locks as a seed and deploys fleet-wide. This is not reinforcement learning. There is no reward function, no gradient, no policy update. The system learns by accumulating validated responses at the point of use.

---

## 1. The Problem: Fire-and-Forget Agents

The dominant pattern in agent systems today:

```
User → Agent → Response → Done
```

The agent has no memory of whether its response worked. It cannot observe the downstream effects. If the same problem appears tomorrow, the agent reasons from scratch — same cost, same latency, same risk of failure.

This is acceptable for one-off tasks. It breaks down at fleet scale, where:

- The same deadband patterns recur across rooms and agents
- Cost compounds: every recurrence burns the same model budget
- No institutional memory forms: agents can't share what worked
- The system never gets better at the edges

**The core issue:** the agent never sees its own wake. It generates output and goes dark.

---

## 2. The Re-Entry Loop

Re-entry inverts the control flow. Instead of the agent calling the room, **the room calls the agent back** when it detects deadband.

```
┌─────────────────────────────────────────────────────────────┐
│                      NORMAL OPERATION                       │
│                                                             │
│  Input Tile ──▶ [Rules/Algo] ──▶ Output Tile ──▶ Next Room │
│                      │                                      │
│                      ▼                                      │
│               KPI Monitor (hysteresis)                      │
│                      │                                      │
│            ┌─────────┴──────────┐                           │
│         OK │                 GAP │ (deadband)               │
│            ▼                     ▼                           │
│     Continue normal       FREEZE CONTEXT                    │
│     operation              (FCW snapshot)                   │
│                                  │                          │
│                                  ▼                          │
│                          CALL AGENT BACK                    │
│                          with full tile context:            │
│                          • What stages 1-N produced         │
│                          • Where the gap opened             │
│                          • Current KPI state                │
│                          • Previous fixes attempted         │
│                                  │                          │
│                                  ▼                          │
│                          AGENT PROPOSES FIX                 │
│                                  │                          │
│                                  ▼                          │
│                          NEW TILE (candidate)               │
│                                  │                          │
│                          ┌───────┴────────┐                 │
│                       FAIL │            OK │                 │
│                          ▼              ▼                   │
│                     Discard      LOCK AS SEED               │
│                                  │                          │
│                                  ▼                          │
│                         Fleet-wide deploy                   │
│                         (next time: rules handle it)        │
└─────────────────────────────────────────────────────────────┘
```

The room doesn't just report failure. It packages the **complete reasoning trail** — every tile that passed through every previous stage — and hands it to the agent. The agent doesn't need to rediscover context. It handles only the delta: the specific gap the rules couldn't close.

---

## 3. The Seed Lifecycle: Quality Through Staged Validation

A proposed fix doesn't go straight into production. It passes through an 8-state validation pipeline:

```
UNLOCKED → CANDIDATE → VALIDATING → LOCK_PENDING → LOCKED → DEPRECATED → ARCHIVED
                                    ↘ DISCARDED (at any pre-lock stage)
```

Each transition is **gated**:

| Stage | Gate | What It Proves |
|-------|------|----------------|
| UNLOCKED → CANDIDATE | Agent proposed a fix | There is a plausible response to this deadband |
| CANDIDATE → VALIDATING | Fix applied in test | The fix doesn't break existing behavior |
| VALIDATING → LOCK_PENDING | KPIs recovered past hysteresis | The fix actually resolved the deadband |
| LOCK_PENDING → LOCKED | Sustained good KPIs (≥95% completion) | The fix is stable, not a fluke |
| LOCKED → DEPRECATED | Better seed found | New knowledge supersedes old |
| DEPRECATED → ARCHIVED | Fleet migration complete | Old knowledge preserved but inactive |

**Key design decisions:**

1. **Hysteresis on exit:** A seed doesn't lock just because KPIs briefly recovered. The system requires sustained performance above threshold (with a 1.1x exit factor) before locking. This prevents premature locking on transient improvements.

2. **Copy-on-write immutability:** Seeds are frozen dataclasses. Transitions create new instances, never mutate. A locked seed cannot be corrupted by subsequent operations.

3. **Content-addressed storage:** Seeds are hashed and deduplicated. The same fix proposed twice produces the same seed ID. This prevents the library from bloating with duplicates.

4. **Fleet-wide propagation:** Once locked, a seed becomes available to every room in the fleet. The next time that deadband pattern appears — in any room — the rules can handle it without calling the agent.

---

## 4. Why This Is Not Reinforcement Learning

The comparison is inevitable. Let me be explicit about why this is different.

| Dimension | Reinforcement Learning | Re-Entry Healing |
|-----------|----------------------|-------------------|
| Reward function | Required (designed by humans) | None. Binary: deadband resolved or not |
| Gradient update | Backprop through policy/value network | No gradient. Accumulate validated tiles |
| State representation | Learned embedding | Explicit tile graph (human-readable) |
| Exploration | Policy noise, epsilon-greedy | Agent generates candidate fixes |
| Training cost | GPU-hours to weeks | Cost of one agent call per deadband event |
| Catastrophic forgetting | Real risk (weights overwrite) | Impossible (seeds are append-only) |
| Interpretability | Low (latent space) | High (tiles are structured data) |
| Cold start | Must train before deployment | Rules run from tick 0, agent adds incrementally |

The critical difference: **RL optimizes a function. Re-entry accumulates a library.**

In RL, the agent's weights change. Past knowledge can be overwritten. The system converges to a single policy.

In re-entry, the library only grows. Past knowledge is never overwritten — it's deprecated and archived. The system accumulates validated responses, and the rule-based layer serves them without model cost.

This is simpler. It's also more predictable. You can inspect any seed and understand exactly what problem it solves and how. You cannot do this with a weight matrix.

---

## 5. The Pattern Library as Accumulated Knowledge

Over time, the locked seeds form a **pattern library** — a fleet-wide knowledge base of validated responses to deadband events.

```
Pattern Library
├── drift-detect/
│   ├── seed-001: "Feature drift on categorical columns" (locked, 47 uses)
│   ├── seed-002: "Concept drift on time-series targets" (locked, 23 uses)
│   └── seed-003: "Feature drift on numerical columns" (deprecated, replaced by seed-005)
├── anomaly-flag/
│   ├── seed-010: "Seasonal anomaly false positives" (locked, 112 uses)
│   └── seed-011: "Distribution shift from upstream schema change" (locked, 8 uses)
├── intent-detect/
│   └── seed-020: "Ambiguous short queries" (validating, KPIs recovering)
└── escalation/
    └── seed-030: "Multi-room cascading failure" (candidate, agent proposed)
```

Each seed is a **proven response** to a specific deadband. It carries:

- The KPI snapshot that triggered it
- The tile context that the agent saw
- The fix the agent proposed
- The validation results that proved it works
- A usage count (how many times it's been served)
- A deprecation trail (if superseded)

The pattern library is the system's memory. It's not latent. It's not implicit. It's an explicit, inspectable, auditable record of what works.

---

## 6. Comparison to Existing Practices

The re-entry pattern has recognizable analogs in established systems engineering:

### Chaos Engineering (Netflix Chaos Monkey, Gremlin)

- **Similarity:** Both deliberately probe for failure modes and observe system response.
- **Difference:** Chaos engineering injects failure. Re-entry detects organic failure. Chaos engineering tests resilience. Re-entry builds resilience.

### Canary Deployments

- **Similarity:** Both validate changes against KPIs before full rollout.
- **Difference:** Canaries validate code changes. Re-entry validates knowledge changes (seeds). A locked seed is the equivalent of a canary that passed — promoted to full deployment.

### Feature Flags

- **Similarity:** Both provide a runtime toggle between behaviors.
- **Difference:** Feature flags toggle code paths. Seeds toggle knowledge paths. A room with a locked seed for a deadband pattern is like having a feature flag that auto-enables when the pattern recurs.

### Incident Runbooks

- **Similarity:** Both document known responses to known problems.
- **Difference:** Runbooks require humans. Seeds are machine-readable and auto-served. A runbook says "when X happens, do Y." A seed *does* Y when X happens, without human intervention.

The re-entry loop is best understood as **automated runbook generation with staged validation.** The agent writes the runbook entry. The seed lifecycle is the review process. Locked seeds are the approved entries.

---

## 7. How the Room Remembers: Tiles as Context Pipes

A critical design decision: the agent doesn't receive a compressed summary. It receives the **full tile chain** — every tile produced by every stage up to the deadband point.

```
Stage 1 (filter)  → Tile A: "Input classified as time-series, 3 features"
Stage 2 (stats)   → Tile B: "Rolling mean shifted 12% in last window"
Stage 3 (detect)  → Tile C: "Algorithm: no anomaly detected (threshold: 15%)"
Stage 4 (monitor) → Tile D: "DEADBAND: completion rate 78%, wait time 42s"
```

When the agent is called back, it sees Tiles A, B, C, and D. It can reason:

- The filter classified correctly (Tile A)
- The stats module detected a real shift (Tile B)
- But the algorithm missed it because the shift was below threshold (Tile C)
- This caused the deadband (Tile D)

The fix is obvious: lower the detection threshold for time-series with 3 features. The agent proposes this. The proposal becomes a candidate seed. If it resolves the deadband, it locks.

**Why tiles matter:** if the agent only saw "deadband at stage 4," it would have to guess the cause. Tiles eliminate the guesswork. They carry the reasoning forward so the agent handles only the delta — the specific gap in the chain.

---

## 8. Honest Limitations

This section draws from a beta review of the spreader-tool implementation. The pattern is sound; the implementation has gaps.

### 8.1 The Validation Gate Is Thin

Currently, seed validation checks whether KPIs recovered past the hysteresis threshold. This is a necessary condition but not sufficient. A deadband can resolve for unrelated reasons (the underlying problem stopped, not because of the fix). The system cannot distinguish "fix caused recovery" from "recovery happened to coincide with fix."

**Mitigation:** Correlation isn't causation, but at fleet scale, repeated correlation becomes evidence. Seeds that lock and then see the same deadband recur (because the fix didn't help) will generate new deadband events, producing better seeds.

### 8.2 Self-Optimization Is Monitoring, Not Optimization

The self-optimizer generates reports. It identifies gaps. It does not close them. Calling this "self-optimization" overclaims. It is a monitoring dashboard with good taste.

### 8.3 The Pattern Library Starts Empty

A new fleet has no seeds. Every deadband calls the agent. This is expensive until the library fills. The system needs a bootstrap period — either pre-seeded patterns from development or a higher agent budget during deployment.

### 8.4 Coverage Gap Is Heuristic

The system estimates knowledge coverage by comparing known deadband patterns to observed patterns. This is a rough proxy. It doesn't capture whether the seeds are *good* — only whether they exist for observed patterns.

### 8.5 No Cross-Room Learning (Yet)

A seed locked in the drift-detect room doesn't automatically apply to the anomaly-flag room, even if the underlying pattern is similar. Cross-room generalization requires either manual mapping or a second-order learning system that the current architecture doesn't include.

---

## 9. The Dial Metaphor: Where Re-Entry Fits

From the Signal Chain thesis: every room has a dial controlling model vs. code. Re-entry is what happens when the dial turns up.

```
Dial 0:  Pure rules. No model cost. Handles the known-knowns.
Dial 3:  Micro-model. Handles common edge cases the rules miss.
Dial 6:  Medium model with tile context. Handles complex deadband events.
Dial 8:  Full agent call with re-entry. Handles novel deadband events.
Dial 10: Full agent with no constraints. Expensive, rare, for the unknown-unknowns.
```

Re-entry operates at dial 6-8. The deadband detector is the mechanism that turns the dial. When rules handle everything, the dial stays at 0. When a gap opens, the dial turns up, the agent is called, and the response is validated into a seed.

**Over time, the dial should drift toward 0.** As the seed library fills, rules handle more cases. Agent calls become less frequent. The system becomes more efficient — not because the model got better, but because the library got bigger.

This is the self-healing promise: **the system doesn't need to be smarter. It needs to remember what worked.**

---

## 10. Conclusion

Self-healing through re-entry is not a new learning algorithm. It is a systems engineering pattern:

1. **Detect** the gap (deadband with hysteresis)
2. **Package** the context (frozen tiles from every upstream stage)
3. **Recall** the agent (re-entry with full context)
4. **Propose** a fix (agent generates candidate)
5. **Validate** the fix (staged seed lifecycle)
6. **Lock** what works (append-only pattern library)
7. **Serve** without model cost (rules handle known patterns)

The result is a system that gets better at the edges without getting more expensive at the core. The agent is the teacher. The seed library is the textbook. The rules are the student who no longer needs to ask.

It's not RL. It's not fine-tuning. It's **remembering at the point of use.**

---

*Part of the Signal Chain thesis. Built on spreader-tool, validated against 241 tests, reviewed by skeptical senior devs who gave it 6/10 for utility and called the marketing inflated. They're right about the marketing. The pattern is still sound.*

---

**References:**
- [Signal Chain Thesis](../THESIS-SOURCE.md) — the dial metaphor and tile-as-context-pipe model
- [Spreader-Tool README](../SPREADER-README.md) — deadband detection, FCW lifecycle, seed locking
- [Beta Critic Review](../BETA-CRITIC.md) — honest limitations, code smells, and the 6/10 assessment
