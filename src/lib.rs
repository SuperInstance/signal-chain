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

    #[test]
    fn test_gain() {
        let mut gain = Gain::new(0.5);
        assert!((gain.process(1.0) - 0.5).abs() < 1e-10);
    }

    #[test]
    fn test_clipper() {
        let mut clip = Clipper::new(0.8);
        assert!((clip.process(1.0) - 0.8).abs() < 1e-10);
        assert!((clip.process(-1.0) + 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_chain() {
        let mut chain = SignalChain::new()
            .push(Gain::new(2.0))
            .push(Clipper::new(1.0));
        let out = chain.process(0.3);
        assert!((out - 0.6).abs() < 1e-10);
    }

    #[test]
    fn test_delay() {
        let mut delay = Delay::new(4, 0.5, 1.0);
        let out0 = delay.process(1.0);
        assert!((out0).abs() < 1e-10); // first sample: no delayed output yet
    }

    #[test]
    fn test_oscillator() {
        let mut osc = SineOsc::new(440.0, 44100.0);
        let val = osc.next();
        assert!(val.abs() <= 1.0);
    }
}
