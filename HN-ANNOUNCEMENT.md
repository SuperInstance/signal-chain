# Show HN: We cut LLM API calls by 94% by not calling the model when we didn't need to

---

## The Post

Most inputs to an AI pipeline don't need a model. We tested this on email classification with real model APIs (Groq Llama 3.3 70B and DeepInfra Seed-2.0-mini). 50 emails. Only 3 needed a model call. The other 47 were resolved by a regex and a keyword counter. Same accuracy. 16.7× lower latency. 94% fewer API calls.

The architecture: each processing stage has a dial for how much model capacity to use. At zero, pure code (regex, lookups, arithmetic). At one, full model invocation. In between, code tries first and escalates to the model when it isn't confident. When a stage resolves confidently, downstream stages don't run at all. Every stage writes its conclusions into a structured packet (a "tile") so the model reads what upstream code already decided instead of re-deriving everything.

This isn't model routing or model cascades. FrugalGPT and similar systems pick one model per input — "which model should handle this?" We're asking a different question: "at which pipeline stage, for this input, does code stop being sufficient?" That's a per-stage confidence threshold with accumulated context and early pipeline termination. The model handles only the delta. The tiles carry the other 90%.

The math compounds. At 100K inputs/hour with a $0.03 model call, uniform invocation costs $3,000/hour. If 76% resolve at the code stage — which our benchmark shows — you save $2,280/hour. The savings are proportional to your code-resolution rate.

The pattern generalizes. Email spam (~76% code-resolved). Intent routing (~70%). Content moderation (~60%). Sensor monitoring (~90%). Document triage (~75%). Any pipeline where most inputs are decidable by simple rules and only a minority need deep reasoning.

What we haven't solved: 50 emails is a proof of concept, not production validation. Real deployment needs 10K+ inputs, distribution shift testing, and p99 latency numbers. Auto-tuning the dial across a multi-stage chain is an unsolved optimization problem. Cascading failures could spike costs. Tiles bloat on long chains. The production adapters (batching, retries, rate limiting) don't exist yet. We're building in the open.

Implementation: 310 tests, zero dependencies, pure Python. The benchmark script runs against real model APIs with your own keys.

Repo: https://github.com/SuperInstance/signal-chain
Landing page (full walkthrough): https://superinstance.github.io/signal-chain/
Proof of concept: https://github.com/SuperInstance/spreader-tool
Paper: https://github.com/SuperInstance/signal-chain/blob/master/papers/SIGNAL-CHAIN.md

Genuine question: we're aware that confidence-based escalation and cascading classifiers exist in production systems (Google's serving cost optimization, Stripe's fraud pipeline). What we haven't seen is the per-stage dial with accumulated structured context + early pipeline termination as a composable pattern. Are people doing this under different names? If you've built something similar, we'd like to compare notes.
