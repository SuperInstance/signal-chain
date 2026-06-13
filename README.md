# Signal Chain

A composable **digital signal processing (DSP) pipeline** for Rust — connect oscillators, filters, effects, and modulators into processing chains for real-time audio and signal manipulation.

## Why It Matters

Audio processing, sensor data conditioning, control systems, and software-defined radio all share a common pattern: a signal flows through a series of processing stages, each transforming it. A DSP chain abstraction lets you compose these stages declaratively — `oscillator → gain → low-pass filter → delay → clipper` — without writing monolithic processing loops. This is the same architectural pattern used in Pure Data, SuperCollider, and Web Audio's AudioContext, but as a lightweight Rust library.

## How It Works

### The SignalNode Trait

Every processing stage implements a single trait:

```rust
trait SignalNode {
    fn process(&mut self, input: Sample) -> Sample;
    fn reset(&mut self) {}
}
```

This sample-by-sample interface enables maximum flexibility: the chain processes one `f64` at a time, making it suitable for both offline processing and real-time audio callbacks (where allocation is forbidden).

### The SignalChain

Nodes are chained in a `Vec<Box<dyn SignalNode>>`. Processing is a simple fold:

```
output = input
for node in chain:
    output = node.process(output)
```

### Built-in Nodes

**Gain:** `y = x × amount` — O(1) per sample

**Biquad Low-Pass Filter:** A second-order IIR filter using the Direct Form I structure:

```
y[n] = b₀x[n] + b₁x[n-1] + b₂x[n-2] - a₁y[n-1] - a₂y[n-2]
```

where coefficients are derived from cutoff frequency and sample rate:

```
ω = 2π × f_c / f_s
α = sin(ω) / (2 × Q)     // Q = 0.707 for Butterworth
b₀ = (1 - cos(ω)) / 2
b₁ = 1 - cos(ω)
b₂ = (1 - cos(ω)) / 2
a₀ = 1 + α
```

**Delay Line:** Circular buffer with feedback:

```
y[n] = (1 - mix) × x[n] + mix × buffer[read_pos]
buffer[write_pos] = x[n] + feedback × buffer[read_pos]
```

**Clipper:** Hard clipping distortion: `y = clamp(x, -threshold, +threshold)`

**SineOsc:** Phase accumulator oscillator:

```
y = sin(2π × phase)
phase += f / f_s
```

### Complexity

| Operation | Per-sample cost |
|-----------|----------------|
| Gain | 1 multiply |
| Biquad LPF | 5 multiplies + 4 additions |
| Delay | 2 additions + 1 multiply + 1 buffer read/write |
| Clipper | 2 comparisons |
| Chain of k nodes | Σ(node costs) |

## Quick Start

```rust
use signal_chain::{SignalChain, Gain, Clipper, LowPass, Delay};

fn main() {
    // Build a guitar-amp-style chain
    let mut chain = SignalChain::new()
        .push(Gain::new(2.0))          // pre-amp boost
        .push(LowPass::new(3000.0, 44100.0))  // tone control
        .push(Delay::new(4410, 0.3, 0.2))    // 100ms delay, 30% feedback, 20% mix
        .push(Clipper::new(0.8));            // distortion

    // Process a buffer of samples
    let mut buffer = vec![0.0; 1024];
    // ... fill buffer with audio ...
    chain.process_buffer(&mut buffer);
}
```

## API

### `SignalNode` Trait
- `process(input: Sample) -> Sample` — transform one sample
- `reset()` — clear internal state (delay buffers, filter memory)

### `SignalChain`
- `new()` — empty chain
- `push(node) -> Self` — builder-pattern append
- `process(sample) -> Sample` — process one sample through the chain
- `process_buffer(&mut [Sample])` — process a whole buffer in place
- `reset()` — reset all nodes

### Nodes
- `Gain { amount: f64 }`
- `LowPass::new(cutoff, sample_rate)`
- `Delay::new(delay_samples, feedback, mix)`
- `Clipper { threshold: f64 }`
- `SineOsc::new(freq, sample_rate)`

## Architecture Notes

In SuperInstance, signal chains process sensor data streams from fleet ships. A chain might smooth noisy telemetry (low-pass), detect anomalies (clipper as event detector), and apply echo analysis (delay). The γ + η = C conservation law governs how much processing budget each chain consumes. See [Architecture](https://github.com/SuperInstance/SuperInstance/blob/main/ARCHITECTURE.md).

## References

- Smith, J.O. (2010). *Introduction to Digital Filters with Audio Applications*. W3K Publishing.
- Zölzer, U. (2008). *Digital Audio Signal Processing*, 2nd ed. Wiley.
- Boulanger, R. & Lazzarini, V. (2011). *The Audio Programming Book*. MIT Press.

## License

MIT
