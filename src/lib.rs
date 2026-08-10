//! Signal Chain — composable DSP processing pipeline.
//!
//! Build audio/signal processing chains by connecting nodes
//! (oscillators, filters, effects) in series or parallel.

/// A single audio sample (f64 for precision).
pub type Sample = f64;

/// A processing node in the signal chain.
pub trait SignalNode: Send {
    /// Process one input sample, producing one output sample.
    fn process(&mut self, input: Sample) -> Sample;

    /// Reset internal state (e.g. phase, delay buffers).
    fn reset(&mut self) {}
}

/// A simple gain node.
pub struct Gain {
    pub amount: f64,
}

impl Gain {
    pub fn new(amount: f64) -> Self {
        Self { amount }
    }
}

impl SignalNode for Gain {
    fn process(&mut self, input: Sample) -> Sample {
        input * self.amount
    }
}

/// A biquad low-pass filter.
pub struct LowPass {
    cutoff: f64,
    sample_rate: f64,
    x1: Sample,
    x2: Sample,
    y1: Sample,
    y2: Sample,
}

impl LowPass {
    pub fn new(cutoff: f64, sample_rate: f64) -> Self {
        Self {
            cutoff,
            sample_rate,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    pub fn set_cutoff(&mut self, cutoff: f64) {
        self.cutoff = cutoff;
    }

    fn coefficients(&self) -> (f64, f64, f64, f64, f64) {
        let omega = 2.0 * std::f64::consts::PI * self.cutoff / self.sample_rate;
        let alpha = omega.sin() / (2.0 * 0.707);
        let b0 = (1.0 - omega.cos()) / 2.0;
        let b1 = 1.0 - omega.cos();
        let b2 = (1.0 - omega.cos()) / 2.0;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * omega.cos();
        let a2 = 1.0 - alpha;
        (b0 / a0, b1 / a0, b2 / a0, a1 / a0, a2 / a0)
    }
}

impl SignalNode for LowPass {
    fn process(&mut self, input: Sample) -> Sample {
        let (b0, b1, b2, a1, a2) = self.coefficients();
        let output = b0 * input + b1 * self.x1 + b2 * self.x2 - a1 * self.y1 - a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = input;
        self.y2 = self.y1;
        self.y1 = output;
        output
    }

    fn reset(&mut self) {
        self.x1 = 0.0;
        self.x2 = 0.0;
        self.y1 = 0.0;
        self.y2 = 0.0;
    }
}

/// A simple delay line.
pub struct Delay {
    buffer: Vec<Sample>,
    index: usize,
    feedback: f64,
    mix: f64,
}

impl Delay {
    pub fn new(delay_samples: usize, feedback: f64, mix: f64) -> Self {
        Self {
            buffer: vec![0.0; delay_samples],
            index: 0,
            feedback,
            mix,
        }
    }
}

impl SignalNode for Delay {
    fn process(&mut self, input: Sample) -> Sample {
        let delayed = self.buffer[self.index];
        self.buffer[self.index] = input + delayed * self.feedback;
        self.index = (self.index + 1) % self.buffer.len();
        input * (1.0 - self.mix) + delayed * self.mix
    }

    fn reset(&mut self) {
        self.buffer.fill(0.0);
        self.index = 0;
    }
}

/// A clipper/distortion node.
pub struct Clipper {
    pub threshold: f64,
}

impl Clipper {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }
}

impl SignalNode for Clipper {
    fn process(&mut self, input: Sample) -> Sample {
        input.clamp(-self.threshold, self.threshold)
    }
}

/// A signal chain that processes samples through a series of nodes.
pub struct SignalChain {
    nodes: Vec<Box<dyn SignalNode>>,
}

impl SignalChain {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn push<N: SignalNode + 'static>(mut self, node: N) -> Self {
        self.nodes.push(Box::new(node));
        self
    }

    pub fn process(&mut self, input: Sample) -> Sample {
        let mut sample = input;
        for node in &mut self.nodes {
            sample = node.process(sample);
        }
        sample
    }

    pub fn process_buffer(&mut self, buffer: &mut [Sample]) {
        for sample in buffer.iter_mut() {
            *sample = self.process(*sample);
        }
    }

    pub fn reset(&mut self) {
        for node in &mut self.nodes {
            node.reset();
        }
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// A simple sine oscillator for signal generation.
pub struct SineOsc {
    phase: f64,
    freq: f64,
    sample_rate: f64,
}

impl SineOsc {
    pub fn new(freq: f64, sample_rate: f64) -> Self {
        Self {
            phase: 0.0,
            freq,
            sample_rate,
        }
    }

    pub fn next(&mut self) -> Sample {
        let val = (2.0 * std::f64::consts::PI * self.phase).sin();
        self.phase += self.freq / self.sample_rate;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        val
    }
}

impl Default for SignalChain {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // === Gain ===

    #[test]
    fn test_gain() {
        let mut gain = Gain::new(0.5);
        assert!((gain.process(1.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn gain_zero_silences() {
        let mut gain = Gain::new(0.0);
        assert!((gain.process(1.0)).abs() < 1e-10);
    }

    #[test]
    fn gain_negative_inverts_phase() {
        let mut gain = Gain::new(-1.0);
        assert!((gain.process(0.7) + 0.7).abs() < 1e-10);
    }

    #[test]
    fn gain_processes_zero() {
        let mut gain = Gain::new(10.0);
        assert!((gain.process(0.0)).abs() < 1e-10);
    }

    #[test]
    fn gain_double_precision_preserved() {
        let mut gain = Gain::new(0.123456789);
        let out = gain.process(0.987654321);
        assert!((out - 0.123456789 * 0.987654321).abs() < 1e-15);
    }

    // === Clipper ===

    #[test]
    fn test_clipper() {
        let mut clip = Clipper::new(0.8);
        assert!((clip.process(1.0) - 0.8).abs() < 1e-10);
        assert!((clip.process(-1.0) + 0.8).abs() < 1e-10);
    }

    #[test]
    fn clipper_passes_below_threshold() {
        let mut clip = Clipper::new(0.8);
        assert!((clip.process(0.5) - 0.5).abs() < 1e-10);
        assert!((clip.process(-0.3) + 0.3).abs() < 1e-10);
    }

    #[test]
    fn clipper_at_exact_threshold_passes() {
        let mut clip = Clipper::new(0.8);
        assert!((clip.process(0.8) - 0.8).abs() < 1e-10);
        assert!((clip.process(-0.8) + 0.8).abs() < 1e-10);
    }

    #[test]
    fn clipper_zero_threshold_silences() {
        let mut clip = Clipper::new(0.0);
        assert!((clip.process(1.0)).abs() < 1e-10);
    }

    // === LowPass ===

    #[test]
    fn lowpass_dc_gain_is_unity() {
        // At DC, a lowpass filter should pass the signal unchanged
        let mut lp = LowPass::new(1000.0, 44100.0);
        let mut last = 0.0;
        // Feed a constant signal and check it converges to unity
        for _ in 0..1000 {
            last = lp.process(1.0);
        }
        assert!((last - 1.0).abs() < 0.01, "DC gain should converge to 1.0, got {}", last);
    }

    #[test]
    fn lowpass_attenuates_high_frequency() {
        // A high-frequency signal should be attenuated more than low
        let mut lp_low = LowPass::new(100.0, 44100.0);
        let mut lp_high = LowPass::new(10000.0, 44100.0);
        let freq = 15000.0_f64; // near Nyquist
        let mut max_low = 0.0_f64;
        let mut max_high = 0.0_f64;
        for i in 0..44100 {
            let s = (2.0 * std::f64::consts::PI * freq * i as f64 / 44100.0).sin();
            max_low = max_low.max(lp_low.process(s).abs());
            max_high = max_high.max(lp_high.process(s).abs());
        }
        assert!(max_low < max_high, "low cutoff should attenuate more: {} vs {}", max_low, max_high);
    }

    #[test]
    fn lowpass_set_cutoff_changes_response() {
        let mut lp = LowPass::new(5000.0, 44100.0);
        // Feed a mid-range signal
        let freq = 2000.0_f64;
        let s = (2.0 * std::f64::consts::PI * freq * 100.0 / 44100.0).sin();
        let out1 = lp.process(s);
        lp.set_cutoff(100.0);
        let out2 = lp.process(s);
        // Different cutoffs should produce different responses
        assert!((out1 - out2).abs() > 1e-6, "changing cutoff should change output");
    }

    #[test]
    fn lowpass_reset_clears_state() {
        let mut lp = LowPass::new(1000.0, 44100.0);
        // Process some samples to build state
        for i in 0..100 {
            lp.process((i as f64 * 0.01).sin());
        }
        lp.reset();
        // After reset, first sample of DC should behave like a fresh filter
        let mut lp2 = LowPass::new(1000.0, 44100.0);
        let out1 = lp.process(0.5);
        let out2 = lp2.process(0.5);
        assert!((out1 - out2).abs() < 1e-10, "reset should restore fresh state");
    }

    #[test]
    fn lowpass_output_stable_for_bounded_input() {
        // A stable biquad should not blow up — output stays bounded
        let mut lp = LowPass::new(1000.0, 44100.0);
        let mut max_output = 0.0_f64;
        for i in 0..10000 {
            let s = ((i as f64 * 0.1).sin() * 0.9).max(-1.0).min(1.0);
            let out = lp.process(s).abs();
            if out > max_output { max_output = out; }
        }
        // Allow slight ringing/overshoot typical of biquad near resonance
        assert!(max_output < 2.0, "filter unstable: max output {}", max_output);
    }

    // === Delay ===

    #[test]
    fn test_delay() {
        let mut delay = Delay::new(4, 0.5, 1.0);
        let out0 = delay.process(1.0);
        assert!((out0).abs() < 1e-10); // first sample: no delayed output yet
    }

    #[test]
    fn delay_produces_echo_after_n_samples() {
        let mut delay = Delay::new(3, 0.0, 1.0); // no feedback, full mix
        delay.process(1.0); // sample 0
        delay.process(0.0); // sample 1
        delay.process(0.0); // sample 2
        let out = delay.process(0.0); // sample 3: should echo input from 3 samples ago
        assert!((out - 1.0).abs() < 1e-10, "echo should be 1.0, got {}", out);
    }

    #[test]
    fn delay_feedback_builds_up() {
        let mut delay = Delay::new(1, 0.9, 1.0);
        delay.process(1.0); // buffer[0] = 1.0, output = 0 (mix of input and empty delay)
        // With delay_samples=1, the next call reads buffer[0]=1.0
        let out1 = delay.process(0.0);
        assert!(out1.abs() > 0.5, "feedback should produce output from previous, got {}", out1);
    }

    #[test]
    fn delay_mix_zero_is_passthrough() {
        let mut delay = Delay::new(4, 0.5, 0.0); // mix=0 → dry signal only
        for i in 0..20 {
            let s = (i as f64 * 0.1).sin();
            let out = delay.process(s);
            assert!((out - s).abs() < 1e-10, "mix=0 should pass input unchanged");
        }
    }

    #[test]
    fn delay_reset_clears_buffer() {
        let mut delay = Delay::new(3, 0.5, 1.0);
        delay.process(1.0);
        delay.process(1.0);
        delay.reset();
        // After reset, first output should be dry (buffer is empty)
        let out = delay.process(0.7);
        assert!((out - 0.0).abs() < 1e-10 || out.abs() < 0.01,
            "after reset, delayed output should be zero, got {}", out);
    }

    // === SignalChain ===

    #[test]
    fn test_chain() {
        let mut chain = SignalChain::new()
            .push(Gain::new(2.0))
            .push(Clipper::new(1.0));
        let out = chain.process(0.3);
        assert!((out - 0.6).abs() < 1e-10);
    }

    #[test]
    fn chain_empty_passes_signal() {
        let mut chain = SignalChain::new();
        let out = chain.process(0.42);
        assert!((out - 0.42).abs() < 1e-10);
    }

    #[test]
    fn chain_len_counts_nodes() {
        let chain = SignalChain::new()
            .push(Gain::new(1.0))
            .push(Clipper::new(1.0))
            .push(Delay::new(10, 0.5, 0.5));
        assert_eq!(chain.len(), 3);
    }

    #[test]
    fn chain_is_empty_for_new() {
        let chain = SignalChain::new();
        assert!(chain.is_empty());
    }

    #[test]
    fn chain_is_not_empty_after_push() {
        let chain = SignalChain::new().push(Gain::new(1.0));
        assert!(!chain.is_empty());
    }

    #[test]
    fn chain_process_buffer_transforms_all_samples() {
        let mut chain = SignalChain::new().push(Gain::new(0.5));
        let mut buf = [1.0, 0.5, -0.3, 0.0];
        chain.process_buffer(&mut buf);
        assert!((buf[0] - 0.5).abs() < 1e-10);
        assert!((buf[1] - 0.25).abs() < 1e-10);
        assert!((buf[2] + 0.15).abs() < 1e-10);
        assert!((buf[3]).abs() < 1e-10);
    }

    #[test]
    fn chain_reset_resets_all_nodes() {
        let mut chain = SignalChain::new()
            .push(Delay::new(4, 0.5, 1.0))
            .push(LowPass::new(1000.0, 44100.0));
        // Build up state
        for i in 0..100 {
            chain.process((i as f64 * 0.1).sin());
        }
        chain.reset();
        // After reset, the LowPass should have cleared state
        // The Delay should have zeroed its buffer
        // A zero input should produce near-zero output
        let out = chain.process(0.0);
        assert!(out.abs() < 0.1, "after reset, zero input should give near-zero output, got {}", out);
    }

    #[test]
    fn chain_default_is_empty() {
        let chain = SignalChain::default();
        assert!(chain.is_empty());
        assert_eq!(chain.len(), 0);
    }

    #[test]
    fn chain_multi_stage_gain_composes() {
        // Two gains of 0.5 should compose to 0.25
        let mut chain = SignalChain::new()
            .push(Gain::new(0.5))
            .push(Gain::new(0.5));
        let out = chain.process(1.0);
        assert!((out - 0.25).abs() < 1e-10);
    }

    #[test]
    fn chain_order_matters() {
        // Gain then Clipper vs Clipper then Gain produce different results
        let mut chain_a = SignalChain::new()
            .push(Gain::new(2.0))   // 0.5 → 1.0
            .push(Clipper::new(0.8)); // 1.0 → 0.8
        let mut chain_b = SignalChain::new()
            .push(Clipper::new(0.8)) // 0.5 → 0.5 (under threshold)
            .push(Gain::new(2.0));   // 0.5 → 1.0
        let out_a = chain_a.process(0.5);
        let out_b = chain_b.process(0.5);
        assert!((out_a - 0.8).abs() < 1e-10);
        assert!((out_b - 1.0).abs() < 1e-10);
    }

    // === SineOsc ===

    #[test]
    fn test_oscillator() {
        let mut osc = SineOsc::new(440.0, 44100.0);
        let val = osc.next();
        assert!(val.abs() <= 1.0);
    }

    #[test]
    fn oscillator_starts_at_zero() {
        let mut osc = SineOsc::new(440.0, 44100.0);
        // Phase starts at 0 → sin(0) = 0
        let val = osc.next();
        assert!(val.abs() < 1e-10, "first sample should be sin(0) = 0, got {}", val);
    }

    #[test]
    fn oscillator_periodic() {
        // At freq=sr/N, the oscillator should repeat every N samples
        let n = 100usize;
        let mut osc = SineOsc::new(1.0, n as f64); // 1 cycle per n samples
        let mut first_cycle = Vec::new();
        let mut second_cycle = Vec::new();
        for i in 0..(2 * n) {
            let v = osc.next();
            if i < n {
                first_cycle.push(v);
            } else {
                second_cycle.push(v);
            }
        }
        for i in 0..n {
            assert!((first_cycle[i] - second_cycle[i]).abs() < 1e-10,
                "oscillator not periodic at sample {}: {} vs {}", i, first_cycle[i], second_cycle[i]);
        }
    }

    #[test]
    fn oscillator_frequency_ratio() {
        // Higher frequency should advance phase faster
        let mut low = SineOsc::new(100.0, 44100.0);
        let mut high = SineOsc::new(1000.0, 44100.0);
        // After 100 samples, high-freq oscillator should have completed more cycles
        let mut low_crossings = 0;
        let mut high_crossings = 0;
        let mut prev_low = 0.0_f64;
        let mut prev_high = 0.0_f64;
        for _ in 0..441 {
            let l = low.next();
            let h = high.next();
            if (prev_low <= 0.0) != (l <= 0.0) { low_crossings += 1; }
            if (prev_high <= 0.0) != (h <= 0.0) { high_crossings += 1; }
            prev_low = l;
            prev_high = h;
        }
        assert!(high_crossings > low_crossings,
            "high freq should cross zero more: {} vs {}", high_crossings, low_crossings);
    }

    // === SignalNode trait ===

    #[test]
    fn default_reset_is_noop() {
        // Gain doesn't override reset — should be a no-op
        let mut gain = Gain::new(0.5);
        gain.process(1.0);
        gain.reset();
        // Should still process normally
        assert!((gain.process(1.0) - 0.5).abs() < 1e-10);
    }
}
