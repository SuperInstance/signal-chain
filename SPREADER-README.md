# spreader-tool

**Intelligence tiling for PLATO rooms — frozen context windows, seed locking, deadband detection.**

Spreader watches PLATO rooms for **deadband**: the gap between what hardcoded rules handle and what needs real intelligence. When a room enters deadband, Spreader freezes reasoning snapshots, validates them, and locks proven-good checkpoints (Seeds) that deploy fleet-wide.

> **PLATO rooms** are shared state spaces where AI agents coordinate. A "room" holds KPIs, decisions, and context for a specific task or domain.

Every agent room has a blind spot — tasks too complex for rules, too frequent for full LLM calls. Spreader detects those gaps automatically and builds a library of validated responses. Think of it as a **self-improving reflex system** for your agent fleet.

- **Deadband detection** — continuous KPI monitoring with hysteresis (no flickering)
- **Frozen Context Windows** — immutable, copy-on-write snapshots of room reasoning state
- **Seed lifecycle** — staged validation pipeline from candidate to fleet-deployable
- **Self-optimization** — monitors its own test suite, locks proven development patterns
- **Zero dependencies** — pure Python dataclasses, no framework lock-in

## Install

```bash
# From source
pip install git+https://github.com/SuperInstance/spreader-tool.git

# Or clone and install editable
git clone https://github.com/SuperInstance/spreader-tool.git
cd spreader-tool
pip install -e ".[dev]"
```

## Quick Example

```python
from spreader import DeadbandDetector, KPIMetrics, DeadbandMetric
import time

# Set up a detector with default thresholds
detector = DeadbandDetector()

# Feed KPI snapshots on each tick
# Note: deadband requires sustained duration to trigger (not a single tick)
metrics = KPIMetrics(
    task_completion_rate=82.0,   # below 90% threshold
    avg_wait_time=45.0,          # above 30s threshold
    energy_over_baseline=5.0,
    inference_mae=8.0,
    timestamp=time.time(),
)
state = detector.update(metrics)

# First tick won't trigger deadband — need sustained violation
print(f"In deadband: {state.in_deadband}")      # False (duration not met)
print(f"Breached: {state.breached_metrics}")    # [COMPLETION_RATE, WAIT_TIME]

# After sustained violations (multiple ticks), deadband triggers
for _ in range(20):
    state = detector.update(metrics)
print(f"In deadband: {state.in_deadband}")      # True (duration met)
print(f"Severity: {state.severity:.2f}")         # 0.0–1.0
```

## CLI

```bash
# Show FCW and seed statistics
python -m spreader.cli stats

# List frozen context windows
python -m spreader.cli list-fcws

# Check deadband status
python -m spreader.cli deadband-status

# Freeze a context window for a room
python -m spreader.cli freeze --room my-room --trigger manual

# List seed candidates ready for locking
python -m spreader.cli seed-candidates

# Lock a validated seed
python -m spreader.cli lock-seed SEED_ID

# Prune low-value FCWs
python -m spreader.cli redact --target-reduction 0.3
```

## Self-Optimization

The spreader-tool can monitor its own development:

```python
from spreader.self_optimize import SelfOptimizer

opt = SelfOptimizer(".")
report = opt.generate_improvement_report()
print(report)  # KPIs, deadband status, optimization opportunities, locked patterns
```

## Architecture

```
┌─────────────┐     ┌──────────────────┐     ┌─────────────┐
│  KPI Metrics │────▶│ DeadbandDetector │────▶│  FCW Freeze  │
│  (per tick)  │     │  + hysteresis    │     │  (snapshot)  │
└─────────────┘     └──────────────────┘     └──────┬───────┘
                                                     │
                    ┌────────────────────────────────┘
                    ▼
            ┌──────────────┐     ┌──────────────┐     ┌──────────────┐
            │   Cost       │────▶│  Redaction    │────▶│  Seed Lock   │
            │  (tracking)  │     │  (pruning)    │     │  (fleet-wide)│
            └──────────────┘     └──────────────┘     └──────────────┘
```

**Flow:** KPI metrics stream in on every tick → DeadbandDetector checks thresholds with duration gates → when deadband is confirmed, a Frozen Context Window is created → cost is tracked → low-value FCWs are pruned → validated Seeds are locked for fleet deployment.

### Module Structure (12 modules)

| Module | Lines | What it does |
|--------|-------|-------------|
| `types.py` | 290 | FCW, Seed, KPI dataclasses, state machines, constants |
| `deadband.py` | 200 | DeadbandDetector with hysteresis and duration gates |
| `frozen_context.py` | 167 | FCW lifecycle: staging → frozen → testing → refining → locked |
| `store.py` | 198 | Content-addressed file storage with dedup |
| `seed_lock.py` | 216 | 8-state seed locking pipeline |
| `cost.py` | 93 | Intelligence cost tracking + refinement gradient |
| `redaction.py` | 190 | KPI-space distance pruning engine |
| `spreader_room.py` | 293 | 8-step intelligence tiling loop (the core orchestrator) |
| `cli.py` | 379 | `plato-spreader` CLI with 8 subcommands |
| `self_optimize.py` | 310 | Self-monitoring development harness |
| `development_patterns.py` | 230 | Locked pattern library (7 proven defaults) |

### Deadband triggers

| Metric | Threshold | Duration |
|--------|-----------|----------|
| Task completion rate | < 90% | 5 minutes sustained |
| Average wait time | > 30s | 30 seconds sustained |
| Energy over baseline | > 10% | 30 seconds sustained |
| Inference MAE | > 10% | 3 consecutive windows |

### FCW lifecycle

`STAGING → FROZEN → TESTING → REFINING → LOCKED` (or `DISCARDED` at any pre-lock stage)

### Seed lifecycle

`UNLOCKED → CANDIDATE → VALIDATING → LOCK_PENDING → LOCKED → DEPRECATED → ARCHIVED`

## Tests

```bash
python -m pytest tests/ -v    # 241 tests, <1 second
```

## Related Repos

| Repo | Purpose |
|------|---------|
| [plato-types](https://github.com/SuperInstance/plato-types) | Tile lifecycle, Lamport clocks |
| [plato-training](https://github.com/SuperInstance/plato-training) | Micro models, hardware deploy |
| [tensor-spline](https://github.com/SuperInstance/tensor-spline) | SplineLinear compression |
| [forgemaster](https://github.com/SuperInstance/forgemaster) | Fleet agent (constraint theory) |

## License

MIT
