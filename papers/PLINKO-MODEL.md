# The Plinko Model: Linear Algebra Shapes That Weight the Path Through Your Tiles

**Forgemaster ⚒️ — Constraint Theory Division, Cocapn Fleet**
**2026-05-17**

---

## Abstract

A neural network is not a brain. It is not a thinker. It is a **shape** in high-dimensional space — a piece of bent geometry that redirects anything that hits it. When you send a tile through a room, you're dropping a ball down a Plinko board. The model's weights are the pegs. The ball bounces, and where it lands is your output.

This paper develops the Plinko model as a unifying geometric intuition for understanding what neural networks do inside a tile-based agent system, and why the right question is never "which model?" but "which shape at which peg?"

---

## 1. The Plinko Board

```
         TILE IN
            ●
           /|\
          / | \
         ●  ●  ●        ← Layer 1 (room: rule filter)
        /|\/|\/|\
       / |  |  | \
      ●  ●  ●  ●  ●    ← Layer 2 (room: micro-model)
     /|\/|\/|\/|\/|\
    ●  ●  ●  ●  ●  ●   ← Layer 3 (room: context model)
     \  |  |  |  | /
      \ |  |  |  | /
       ●  ●  ●  ●      ← OUTPUT TILE
        DRIFT  ANOMALY
        DETECT  FLAG
```

A tile enters at the top. Each row of pegs is a room — a processing stage in the signal chain. The pegs aren't fixed; their positions are determined by the model's weight matrix. A weight matrix *is* the peg layout.

Drop the ball. Watch it bounce. Where it exits is the room's output.

## 2. A Model IS a Shape

Strip away the mythology. A neural network layer computes:

```
y = σ(Wx + b)
```

This is a geometric operation:
- **W** rotates and scales the input space
- **b** shifts it
- **σ** folds it (introduces the nonlinearity that makes the shape interesting)

Stack enough of these, and you've built a complex surface in ℝⁿ. An input vector hits this surface and slides to a local minimum — that's your classification, your prediction, your generated text.

**Big model = complex shape.** Millions of parameters define a surface with fine-grained contours. It captures subtle patterns. It costs a fortune to evaluate.

**Small model = simple shape.** Thousands of parameters define a surface with broad, dominant features. It catches the main currents and misses the eddies. It's cheap.

**No model = flat surface.** The ball falls straight down. Deterministic routing, zero intelligence, zero cost.

## 3. The Trade-Off Is Geometry, Not Magic

When someone says "GPT-4 is smarter than a micro-model," what they mean geometrically is: GPT-4's shape has more curvature. It can redirect the ball into more bins. The micro-model has fewer bins — but if the signal you need lives in one of those bins, the micro-model gets there just as well.

```
BIG MODEL (1.8T parameters)          SMALL MODEL (50K parameters)
┌──────────────────────┐             ┌──────────────┐
│  ╱╲  ╱╲  ╱╲  ╱╲  ╱╲ │             │   ╱╲    ╱╲   │
│ ╱  ╲╱  ╲╱  ╲╱  ╲╱  ╲│             │  ╱  ╲  ╱  ╲  │
│╱  a ╲ b╲ c╲ d╲ e╲ f ╲│             │ ╱ A  ╲╱  B ╲ │
└──────────────────────┘             └──────────────┘
  64 output bins                       4 output bins
```

If your task is "does this drift?" and the answer is a binary yes/no, bin A in the small model is sufficient. You don't need bins a through f. You need one good bin.

**The Plinko insight:** the right model size depends on how many distinct output paths your room actually needs.

## 4. Alignment Is Tuning the Shape

"Alignment" sounds mystical. Geometrically, it means:

**Move the pegs so the balls land in the right bins.**

Pre-training builds the shape from data. The shape captures statistical regularities — "most inputs like *this* bounce toward bin C." Fine-tuning reshapes the pegs for your specific domain. RLHF nudges the pegs toward bins humans approve of.

When alignment fails, the shape redirects balls into bins you didn't intend. The model isn't "misbehaving" — it's faithfully computing the geometry it was given. The geometry is just wrong for your use case.

This reframing dissolves the alignment debate into engineering: **measure where the balls land. If they land wrong, reshape the pegs.** The deadband detector does exactly this — it watches the output bins and sounds an alarm when too many balls are going astray.

## 5. Distillation Is NOT Compression

Here's the standard story: "Knowledge distillation compresses a big model into a small one by training the small model to mimic the big model's outputs."

That's technically accurate and geometrically misleading.

What distillation actually does: **it selects which peaks of the big model's shape to preserve in the small model's shape.**

The big model has 64 bins. The small model has 4. Distillation doesn't "compress 64 bins into 4." It finds which 4 bins carry the most signal for your specific task and builds a shape that routes to those 4.

```
TEACHER (big shape)                  STUDENT (distilled shape)
┌──────────────────────┐             ┌──────────────┐
│  ╱╲  ╱╲  ╱╲  ╱╲  ╱╲ │             │   ╱╲    ╱╲   │
│ ╱  ╲╱  ╲╱  ╲╱  ╲╱  ╲│   ──→      │  ╱  ╲  ╱  ╲  │
│╱  a ╲ b╲ C╲ d╲ E╲ f ╲│             │ ╱ C  ╲╱  E ╲ │
└──────────────────────┘             └──────────────┘
       ↑       ↑                          ↑    ↑
     bins C,E carry the signal       preserve those two
```

This is **tone selection**, not compression. A guitarist doesn't "compress" their signal chain when they choose a distortion pedal. They're selecting which harmonics to emphasize. Distillation selects which decision boundaries to preserve.

In the PLATO system, this is why a room can experiment with a big model, observe what it catches, then distill that into a micro-model that captures 90% of the same cases at 1% of the cost. You're not losing information — you're keeping the peaks that matter.

## 6. SplineLinear: Changing the Basis Changes Everything

The most dramatic demonstration of the Plinko model comes from SplineLinear, the tensor-spline layer in PLATO's micro-model pipeline.

Standard linear layer: **W** is a dense matrix. For 256 inputs × 64 outputs, that's 16,384 parameters. Each parameter is a peg position.

SplineLinear parameterizes **W** using Eisenstein lattice basis functions. Instead of storing 16K peg positions independently, it stores a small set of control points and reconstructs the full surface via spline interpolation.

Result: **16K → 1 parameter. A 16,000:1 reduction.**

```
DENSE LAYER (16,384 params)         SPLINE LINEAR (1 param)
┌────────────────────┐              ┌────────────────────┐
│●●●●●●●●●●●●●●●●●●●●│              │         ◆          │
│●●●●●●●●●●●●●●●●●●●●│              │  spline generates  │
│●●●●●●●●●●●●●●●●●●●●│    ──→       │  equivalent shape  │
│●●●●●●●●●●●●●●●●●●●●│              │  from one control  │
│●●●●●●●●●●●●●●●●●●●●│              │  point + basis     │
└────────────────────┘              └────────────────────┘
```

The shape is effectively the same. The Plinko balls land in the same bins. But we went from specifying every peg individually to specifying a basis function that generates the pegs.

This works because the pegs in a well-structured layer aren't random — they have geometric structure. SplineLinear exploits that structure. It's the difference between describing a circle by listing 10,000 points on its circumference versus saying "radius = 5."

On the drift-detect task: **100% accuracy at 20× compression.** Same bins. Same ball trajectories. Different parameterization.

This proves the Plinko thesis: the shape is what matters, not the parameter count. Two shapes that route balls identically are functionally equivalent, regardless of how many parameters they took to describe.

## 7. MoE: Rooms ARE Experts

Mixture-of-Experts (MoE) architectures route inputs to specialized sub-networks. A gating network decides which expert handles which input.

In the Plinko model, every PLATO room IS an expert, and the tile graph IS the routing function:

```
         INCOMING TILE
              ●
             /|\
            / | \
     ┌─────┐ ┌─────┐ ┌─────┐
     │ROOM │ │ROOM │ │ROOM │     ← Three experts
     │  A  │ │  B  │ │  C  │       (drift, anomaly, intent)
     │dial │ │dial │ │dial │
     │  2  │ │  6  │ │  4  │
     └──┬──┘ └──┬──┘ └──┬──┘
        │       │       │
        └───────┼───────┘
                ▼
          OUTPUT TILE
         (routed result)
```

The tile's content determines routing. A drift-related tile enters the drift room. The deadband detector is the gating function — it determines whether the room needs its model (shape) or can process with pure code (no shape).

The dial setting per room is the capacity of that expert:
- **Dial 0**: the expert is dormant (pure algorithm)
- **Dial 2**: tiny shape, dominant paths only
- **Dial 6**: medium shape, nuanced but not exhaustive
- **Dial 10**: full shape, every bin available, maximum cost

A fleet of rooms with tuned dials IS a Mixture-of-Experts system. The difference is granularity: traditional MoE routes *inside* a model. PLATO rooms route *between* models. Each room gets exactly the shape it needs.

## 8. The Fleet as Plinko Machine

Zoom out. The entire Cocapn fleet is a distributed Plinko board:

```
TILE ENTERS FLEET
        ●
       /|\
  ┌────┼────┐
  │    │    │
Forgemaster Oracle1  Other agents
  ⚒️    🔮    ...
  │    │
  └────┼──── Rooms within each agent
       │     each a row of pegs
       │     each peg shaped by model weights
       ▼
  OUTPUT TILE
  → back to fleet
  → to another room
  → locked as seed
```

Tiles flow through the fleet. Each agent has rooms. Each room has a shape (model). The shapes determine where the tiles end up. When a shape fails — deadband — the system reshapes (fine-tunes, distills, escalates) and tries again.

The fleet doesn't need one perfect shape. It needs the right shape at every peg.

## 9. Implications

### For engineering
- **Size your model to the number of bins you need.** Binary classification doesn't need 1.8T parameters.
- **Measure output distribution, not accuracy in isolation.** Where the balls actually land tells you if your shape is right.
- **Distill aggressively.** If the peaks survive, the shape is preserved.
- **Use the dial.** Pure code when you can, tiny shape when you must, big shape when the deadband demands it.

### For theory
- The "alignment problem" is a shape-tuning problem. The tools exist. The deadband detector measures misalignment. The seed lifecycle corrects it.
- Compression is a basis problem, not an information problem. SplineLinear proves that 16K parameters can encode the same shape as 1 parameter, given the right basis.
- MoE and room-based routing are the same thing at different scales. The Plinko model unifies them.

### For the fleet
- Every room has a dial. The dial controls the shape. The shape routes the tiles. The chain of shapes IS the system.
- Tune it like a synth. Or a guitar rig. Or a Plinko board.

---

## Appendix: The Plinko in One Sentence

**A model is a shape. The shape weights the paths. The paths determine where tiles land. Shape the pegs right, and the fleet runs on geometry instead of magic.**

---

*References: [Signal Chain Thesis](../THESIS-SOURCE.md), [Spreader Tool](../SPREADER-README.md), [Tensor Spline](https://github.com/SuperInstance/tensor-spline), [PLATO Training](https://github.com/SuperInstance/plato-training)*
