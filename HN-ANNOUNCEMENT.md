# Show HN: Every AI pipeline stage should have its own dial for model vs code

---

## The Post

Here's a pattern we keep seeing: AI pipelines call a model on every input, every time, regardless of complexity. A simple sender-domain check costs the same $0.03 API call as a nuanced intent classification. The model re-parses what a regex already decided. The pipeline treats every stage as equally expensive and every input as equally complex.

This is wasteful in a way that compounds. At scale, you're burning millions re-deriving conclusions that code could have reached in microseconds.

The fix is architectural. Give each processing stage its own dial: how much model capacity does this stage need? Most stages sit at zero — pure code, regex, arithmetic, lookups — and resolve inputs for near-zero cost. When code isn't confident, the dial turns up and the model handles the hard part. When a stage resolves confidently, downstream stages don't run at all.

The key is what flows between stages. Each one writes its conclusions into a structured packet — a tile — that carries forward. So when the model does wake up, it doesn't re-parse headers or re-count keywords. It reads the tile chain and handles only the delta: the part that code couldn't resolve. The model handles 10% of the reasoning. The tiles carry the other 90%.

We tested this with real model APIs. Email classification: 94% of inputs resolved without invoking the model. Same accuracy, 16.7× lower latency. But email is the least interesting application. The pattern generalizes to any pipeline where most inputs are decidable by simple rules and only a minority need deep reasoning:

- **Sensor monitoring** (~90% code-resolved) — thresholds and rolling statistics handle normal operation; the model handles distribution shift
- **Document triage** (~75%) — metadata and header classification route most documents; the model handles ambiguous content
- **Intent routing** (~70%) — exact-match handles common intents; the model handles novel utterances
- **Content moderation** (~60%) — blocklists and regex catch the obvious; the model handles context-dependent cases

At 100K inputs/hour with a $0.03 model call, that's the difference between $3,000/hour and $720/hour. The savings are proportional to your code-resolution rate — which turns out to be surprisingly high in most pipelines.

This isn't model routing or model cascades. Those systems pick one model per input: "which model should handle this?" We're asking a different question: "at which pipeline stage, for this input, does code stop being sufficient?" That's per-stage, per-input, with accumulated context and early termination. It's the difference between routing an input to a model and routing a *decision* to a model.

What we haven't solved: our benchmark is 50 emails — a proof of concept, not production validation. Auto-tuning the dial across a multi-stage chain is an unsolved optimization problem. Cascading failures could spike costs. The production adapters (batching, retries, rate limiting) don't exist yet. We're building in the open.

Implementation: 310 tests, zero dependencies, pure Python. The benchmark script runs against real model APIs with your own keys.

Repo: https://github.com/SuperInstance/signal-chain
Landing page: https://superinstance.github.io/signal-chain/
Proof of concept: https://github.com/SuperInstance/spreader-tool
Paper: https://github.com/SuperInstance/signal-chain/blob/master/papers/SIGNAL-CHAIN.md

Genuine question: confidence-based escalation exists in production systems (Google's serving optimization, Stripe's fraud pipeline). What we haven't seen is the per-stage dial with accumulated structured context + early pipeline termination as a composable, reusable pattern. Are people doing this under different names? We'd like to compare notes.
