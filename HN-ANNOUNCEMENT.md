# Show HN: The Signal Chain — Per-stage model routing for AI pipelines

---

## The Post

Most inputs to an AI pipeline don't need a model. We tested this: 50 emails through a spam filter, only 3 needed a model call. The other 47 were resolved by regex and keyword counting. Same accuracy. 16.7× faster. 94% fewer API calls. Tested against real APIs (Groq Llama 3.3 70B and DeepInfra Seed-2.0-mini), not mocks.

The architecture is simple. Each processing stage has a dial — α ∈ [0, 1]. At zero, pure code. At one, full model. In between, code tries first and escalates to the model only when it isn't confident. When a stage resolves confidently, downstream stages don't run at all. Every stage writes its conclusions into a tile — a structured, content-addressed packet — so the model reads what upstream code already decided instead of re-deriving everything from scratch.

The model handles only the delta. The tiles carry the other 90%.

This works because most pipelines have a property that the default architecture ignores: the vast majority of inputs are decidable by simple rules, and only a minority need deep reasoning. Email spam (~76% code-resolved). Intent routing (~70%). Content moderation (~60%). Sensor monitoring (~90%). Document triage (~75%). The pattern is the same across all of them.

The reason small models work per-room is that a model scoped to one decision with full tile context is solving a narrow problem. For binary drift detection, a standard dense layer needs 16,384 parameters. SplineLinear reparameterizes using Eisenstein lattice basis functions — 820 control points, same accuracy, 20× compression, sub-millisecond on CPU including an ESP32-S3.

What we haven't solved: 50 emails is a proof of concept, not production validation. Auto-tuning α across a multi-stage chain is an unsolved optimization problem. Cascading failures could spike costs. Tiles bloat on long chains. Stale seeds produce wrong answers when distributions shift. The production adapters (batching, retries, rate limiting) don't exist yet.

Implementation: 310 tests, zero dependencies, pure Python. The benchmark script runs against real model APIs with your own keys.

Repo: https://github.com/SuperInstance/signal-chain
Landing page (full walkthrough): https://superinstance.github.io/signal-chain/
Proof of concept: https://github.com/SuperInstance/spreader-tool
Paper: https://github.com/SuperInstance/signal-chain/blob/master/papers/SIGNAL-CHAIN.md

Genuine question: Is the per-stage α parameter actually novel, or are people already doing this under different names? Model cascades (FrugalGPT, model routing) are the closest relatives we found, but they ask "which model for this input" — we're asking "which model, at which stage, at what confidence threshold, with what accumulated context." If you're building something similar, we'd genuinely like to know.
