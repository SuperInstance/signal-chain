# Tiles as Context Carriers

## Why the Model at Stage N Doesn't Need to Rediscover Stages 1..N-1

*May 2026 — SuperInstance Signal Chain Series*

---

## The Rebuild Problem

Every time you call an LLM in a pipeline, you pay a tax. The tax is **context reconstruction** — rebuilding everything the previous stages already figured out so the current model can do its job.

This tax isn't small. In a typical 6-stage pipeline, models spend 80%+ of their capacity re-deriving conclusions that earlier stages already computed. You're not using GPT-4 for intelligence. You're using it as a very expensive database query.

Tiles eliminate this tax. Not by caching prompts, but by making **context structural** — baked into the data flowing through the pipeline, not bolted on at each call.

## What a Tile Actually Is

A tile is a unit of knowledge flowing through a PLATO room chain. It's not a message. It's not a prompt. It's an **immutable, content-addressed record of what a room learned**.

```python
@dataclass
class TrainingTile:
    tile_id: str              # SHA-256 of content
    room: str                 # Which room produced this
    task: str                 # What task it was for
    stage: str                # Pipeline stage (ingest, process, validate, etc.)
    metrics: dict             # KPI snapshot at creation time
    lamport_clock: int        # Causal ordering across fleet
    lifecycle: TileLifecycle  # active → frozen → archived
    content: bytes            # The actual payload
    parent_ids: list[str]     # Tiles this was derived from
    created_at: datetime
```

The key fields aren't the payload — they're the **metadata**: `room`, `stage`, `lamport_clock`, `parent_ids`. These turn a blob of data into a navigable knowledge graph.

When a tile exits a room, it carries a stamp. The next room doesn't need to ask "what happened before?" — the tile *is* what happened before.

## The Pipeline Walkthrough: Drift Detection

Here's how tiles flow through a real pipeline — drift detection for fleet monitoring:

```
Stage 1: INGEST
  Input: Raw metrics stream
  Tile produced: {metrics_snapshot, anomalies: [], clock: 1}
  Dial: 0 (pure code — threshold checks, no model)

Stage 2: PATTERN MATCH
  Input: Tile from Stage 1
  Processing: DeadbandDetector checks KPIs against thresholds
  Tile produced: {metrics_snapshot, anomalies: [wait_time_breach], clock: 2}
  Dial: 0-1 (deadband algorithm, no model unless breached)

Stage 3: MICRO-MODEL CLASSIFICATION
  Input: Tile from Stage 2 (only if deadband detected)
  Processing: 2M-parameter drift-detect model classifies anomaly type
  Tile produced: {metrics, anomalies, drift_type: "gradual_shift", confidence: 0.87, clock: 3}
  Dial: 2-3 (tiny model, handles the delta between rules and reality)

Stage 4: CONTEXT ENRICHMENT
  Input: Tile from Stage 3
  Processing: Looks up historical tiles with similar drift patterns
  Tile produced: {metrics, anomalies, drift_type, history: [tile_abc, tile_def], clock: 4}
  Dial: 1 (lookup, no model)

Stage 5: ESCALATION DECISION
  Input: Tile from Stage 4
  Processing: Medium model evaluates whether drift warrants fleet-wide alert
  Tile produced: {everything_above, escalate: true, reasoning: "...", clock: 5}
  Dial: 6-7 (model handles the nuanced decision)

Stage 6: SEED LOCK
  Input: Tile from Stage 5
  Processing: Validates decision, locks as seed if proven correct
  Tile produced: {everything_above, seed_id: "seed_789", locked: true, clock: 6}
  Dial: 0 (validation code)
```

Notice what the model at Stage 5 does NOT do:
- It does NOT re-read the raw metrics (Stage 1 already parsed them)
- It does NOT re-detect anomalies (Stage 2 already found them)
- It does NOT re-classify the drift (Stage 3 already did that)
- It does NOT look up history (Stage 4 already enriched it)

The model at Stage 5 receives a tile that contains **four stages of accumulated knowledge**. Its only job is the delta: "given everything we know, should we escalate?" That's a tiny fraction of the total work.

## Content-Addressed Storage: SHA-256 as Truth

Tiles are stored by content hash, not by name or reference.

```python
tile_id = sha256(
    room + task + stage + metrics_json + content_bytes + parent_id_hashes
)
```

This has three consequences that matter:

**1. Automatic deduplication.** If two rooms independently produce identical knowledge, they produce the same tile ID. The store saves it once. No coordination needed — the hash IS the coordination.

**2. Tamper evidence.** You can't modify a tile without changing its ID. Every parent-child link in the tile graph is cryptographically verifiable. When a seed is locked, you can trace it back to every tile that contributed to the decision.

**3. Content-addressed retrieval.** You don't ask "give me tile #789." You ask "give me the tile with this content." The distinction matters: if the same drift pattern appears in a different fleet, the tile is already there. Knowledge is portable because it's addressed by what it says, not where it lives.

Compare this to prompt chaining, where each LLM call rebuilds context from scratch. There's no deduplication, no verification, no portability. You're burning tokens to reconstruct state that already exists.

## The Delta Principle

This is the core insight. Let's make it precise:

**Model cost at stage N is proportional to (1 - fraction of context already carried by tiles).**

```
cost(N) ∝ 1 - (tiles_through_stage_N-1 / total_context_needed)
```

When tiles carry 80% of the context (typical for mid-pipeline rooms), the model handles 20% of the cognitive load. That's why a 2M-parameter micro-model can replace a 200B-parameter general model at that stage — it's not doing 1/100th of the work, it's doing the 20% that rules can't handle, and the other 80% is already in the tile.

This is also why the dial works. A room at dial 0 runs pure code — no model cost, all context from upstream tiles. A room at dial 10 runs a full agent — the tiles still carry context, but the model is doing novel synthesis that no algorithm can handle. Most rooms sit at dial 2-4: tiny models that handle the deadband between what rules catch and what needs intelligence.

The economics compound across a fleet. If you have 9 agents each running 8 rooms, that's 72 rooms. If each room's model handles 20% of its cognitive load (the rest coming from tiles), your fleet's total model cost is 14.4 room-equivalents instead of 72. That's an 80% reduction — not from using cheaper models, but from **not asking models to redo work that's already done.**

## Rollback: Frozen Context as Checkpoints

Tiles enable something that prompt chaining can't: **arbitrary rollback.**

A Frozen Context Window (FCW) is an immutable snapshot of a room's tile state at a point in time. Because tiles are content-addressed and parent-linked, you can restore any FCW and get a complete, consistent view of the pipeline at that moment.

```python
# Rollback to a known-good state
fcw = store.load_fcw("fcw_2026_05_17_14_30")
room.restore(fcw)  # All tile state, all metadata, all history
```

This isn't just error recovery. It's **the foundation of the seed lifecycle.** When a room enters deadband (KPIs outside thresholds), Spreader freezes the context window. The agent proposes a fix, which produces new tiles. If the fix works, the FCW becomes a seed — a validated response to a known situation, deployable fleet-wide.

If the fix fails, you roll back to the FCW and try again. No state corruption, no cascading failures, no manual reconstruction. The tiles remember everything.

## The Anti-Pattern: Rebuilding Context

Here's what the standard approach looks like without tiles:

```
Stage 1: LLM call → "Parse these metrics"
Stage 2: LLM call → "Parse these metrics and check for anomalies"
Stage 3: LLM call → "Parse these metrics, check for anomalies, and classify drift"
Stage 4: LLM call → "Parse these metrics, check anomalies, classify drift, and find history"
Stage 5: LLM call → "Parse these metrics, check anomalies, classify drift, find history,
                      and decide whether to escalate"
```

Each call re-includes the full context of every previous stage. The prompt grows linearly, the token cost grows quadratically (longer prompts = more processing per token), and the model still makes mistakes because it's reconstructing context instead of receiving it.

The industry calls this "prompt chaining" and treats it as a best practice. It's not. It's the most expensive possible way to move information through a pipeline — wrapping it in natural language, sending it through a transformer, and hoping the model extracts the same conclusions the previous model already encoded.

Tiles cut this Gordian knot. Context doesn't live in prompts. It lives in structured data that flows between rooms. The model sees what it needs, not a reconstruction of everything that happened before.

## Why This Matters for Small Models

The delta principle has an implication that's easy to miss: **tiles make small models viable for complex tasks.**

A 2M-parameter micro-model can't reason about a fleet monitoring system. But it CAN reason about drift classification — because by the time the tile reaches it, four stages of preprocessing have already been done. The micro-model isn't doing the whole job. It's doing one specific thing, with complete context handed to it on a plate.

This is how PLATO training works in practice:
- **drift-detect** micro-model: 100% accuracy on 5/6 hardware targets
- **anomaly-flag**: 93% accuracy on NPU
- Sub-millisecond inference across all CPU targets
- SplineLinear compression: 20× smaller at same accuracy

None of these models would work without the tile pipeline. They're not general intelligence. They're **precision tools** that work because the context they need is already structured and delivered.

## The One-Liner

**A tile is a stamped envelope of knowledge. The stamp says what room processed it, when, and what it concluded. The next room doesn't open the envelope and start reading — it reads the stamp and gets to work.**

---

*Part of the Signal Chain Thesis series. See also: [The Signal Chain Thesis](../THESIS-SOURCE.md), [Spreader Tool](https://github.com/SuperInstance/spreader-tool).*
