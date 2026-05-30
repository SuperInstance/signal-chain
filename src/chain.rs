//! Signal chain and builder.

use crate::{
    AggregationMethod, Aggregator, ConfidenceGate, DeadbandFilter, ExponentialSmoothing,
    MovingAverage, Normalizer, Signal, SignalStage,
};

/// An immutable pipeline of signal processing stages.
///
/// Each call to [`process`](SignalChain::process) threads the signal through
/// every stage in order, returning the transformed signal.
///
/// Stages with internal state use interior mutability (`RefCell`) so the
/// chain itself remains a shared reference.
#[derive(Debug)]
pub struct SignalChain {
    stages: Vec<Box<dyn SignalStage>>,
}

impl SignalChain {
    /// Process a single signal through all stages.
    pub fn process(&self, signal: Signal) -> Signal {
        self.stages
            .iter()
            .fold(signal, |sig, stage| stage.process_boxed(sig))
    }

    /// Process a batch of signals sequentially through the chain.
    pub fn process_batch(&self, signals: Vec<Signal>) -> Vec<Signal> {
        signals.into_iter().map(|s| self.process(s)).collect()
    }

    /// Number of stages in the chain.
    pub fn len(&self) -> usize {
        self.stages.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// Return a conservation check: compare total energy of input vs output.
    /// Returns `(input_energy, output_energy, ratio)` where ratio should be ≈ 1.0
    /// for energy-conserving chains.
    pub fn conservation_check(&self, signals: Vec<Signal>) -> (f64, f64, f64) {
        let input_energy: f64 = signals.iter().map(|s| s.energy()).sum();
        let outputs = self.process_batch(signals);
        let output_energy: f64 = outputs.iter().map(|s| s.energy()).sum();
        let ratio = if input_energy == 0.0 {
            if output_energy == 0.0 { 1.0 } else { f64::INFINITY }
        } else {
            output_energy / input_energy
        };
        (input_energy, output_energy, ratio)
    }
}

impl Clone for SignalChain {
    fn clone(&self) -> Self {
        Self {
            stages: self.stages.iter().map(|s| s.box_clone()).collect(),
        }
    }
}

// ---------------------------------------------------------------------------
// ChainBuilder — fluent API
// ---------------------------------------------------------------------------

/// Fluent builder for constructing a [`SignalChain`].
///
/// # Example
///
/// ```
/// use signal_chain::ChainBuilder;
///
/// let chain = ChainBuilder::new()
///     .deadband(0.1)
///     .ema(0.3)
///     .normalize(0.0, 1.0)
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct ChainBuilder {
    stages: Vec<Box<dyn SignalStage>>,
}

impl ChainBuilder {
    /// Start a new empty builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a [`DeadbandFilter`] stage.
    pub fn deadband(mut self, threshold: f64) -> Self {
        self.stages.push(Box::new(DeadbandFilter::new(threshold)));
        self
    }

    /// Add a [`MovingAverage`] stage with the given window size.
    pub fn moving_average(mut self, window_size: usize) -> Self {
        self.stages.push(Box::new(MovingAverage::new(window_size)));
        self
    }

    /// Add an [`ExponentialSmoothing`] (EMA) stage with the given alpha.
    pub fn ema(mut self, alpha: f64) -> Self {
        self.stages
            .push(Box::new(ExponentialSmoothing::new(alpha)));
        self
    }

    /// Add a [`ConfidenceGate`] stage.
    pub fn confidence_gate(mut self, threshold: f64) -> Self {
        self.stages
            .push(Box::new(ConfidenceGate::new(threshold)));
        self
    }

    /// Add a [`Normalizer`] stage mapping `[min, max]` → `[0, 1]`.
    pub fn normalize(mut self, min: f64, max: f64) -> Self {
        self.stages.push(Box::new(Normalizer::new(min, max)));
        self
    }

    /// Add an [`Aggregator`] stage (useful as a named passthrough in single-signal chains).
    pub fn aggregate(mut self, method: AggregationMethod) -> Self {
        self.stages.push(Box::new(Aggregator::new(method)));
        self
    }

    /// Build the [`SignalChain`].
    pub fn build(self) -> SignalChain {
        SignalChain { stages: self.stages }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Signal;

    #[test]
    fn empty_chain_passthrough() {
        let chain = SignalChain { stages: vec![] };
        let out = chain.process(Signal::simple(42.0, 0));
        assert!(approx_eq(out.value, 42.0, 1e-10));
    }

    #[test]
    fn chain_with_normalizer() {
        let chain = ChainBuilder::new().normalize(0.0, 100.0).build();
        let out = chain.process(Signal::simple(50.0, 0));
        assert!(approx_eq(out.value, 0.5, 1e-10));
    }

    #[test]
    fn chain_builder_multi_stage() {
        let chain = ChainBuilder::new()
            .deadband(0.1)
            .ema(0.3)
            .normalize(0.0, 10.0)
            .build();
        assert_eq!(chain.len(), 3);
    }

    #[test]
    fn process_batch_works() {
        let chain = ChainBuilder::new().normalize(0.0, 10.0).build();
        let signals = vec![
            Signal::simple(2.5, 0),
            Signal::simple(5.0, 1),
            Signal::simple(10.0, 2),
        ];
        let results = chain.process_batch(signals);
        assert_eq!(results.len(), 3);
        assert!(approx_eq(results[0].value, 0.25, 1e-10));
        assert!(approx_eq(results[1].value, 0.5, 1e-10));
        assert!(approx_eq(results[2].value, 1.0, 1e-10));
    }

    #[test]
    fn conservation_smooth_chain() {
        // An EMA chain with alpha=1.0 is passthrough → energy conserved
        let chain = ChainBuilder::new().ema(1.0).build();
        let signals = vec![
            Signal::new(5.0, 0, 0.9, "a"),
            Signal::new(3.0, 1, 0.8, "b"),
        ];
        let (_, _, ratio) = chain.conservation_check(signals);
        assert!(approx_eq(ratio, 1.0, 1e-6));
    }

    #[test]
    fn conservation_with_normalizer_shifts_energy() {
        let chain = ChainBuilder::new().normalize(0.0, 100.0).build();
        let signals = vec![Signal::simple(50.0, 0)];
        let (_, _, ratio) = chain.conservation_check(signals);
        assert!(!approx_eq(ratio, 1.0, 0.01));
    }

    #[test]
    fn deadband_in_chain() {
        let chain = ChainBuilder::new().deadband(2.0).build();
        let out1 = chain.process(Signal::simple(10.0, 0));
        assert!(approx_eq(out1.value, 10.0, 1e-10));

        let out2 = chain.process(Signal::simple(10.5, 1));
        assert!(out2.confidence.abs() < 1e-10);
    }

    #[test]
    fn moving_average_in_chain() {
        let chain = ChainBuilder::new().moving_average(3).build();
        let signals = vec![
            Signal::simple(3.0, 0),
            Signal::simple(6.0, 1),
            Signal::simple(9.0, 2),
        ];
        let results = chain.process_batch(signals);
        assert!(approx_eq(results[2].value, 6.0, 1e-10));
    }

    #[test]
    fn confidence_gate_in_chain() {
        let chain = ChainBuilder::new().confidence_gate(0.5).build();
        let out = chain.process(Signal::new(10.0, 0, 0.3, "unreliable"));
        assert!(out.value.abs() < 1e-10);
    }

    #[test]
    fn chain_clone_and_process() {
        let chain = ChainBuilder::new().normalize(0.0, 100.0).build();
        let chain2 = chain.clone();
        let out1 = chain.process(Signal::simple(25.0, 0));
        let out2 = chain2.process(Signal::simple(75.0, 0));
        assert!(approx_eq(out1.value, 0.25, 1e-10));
        assert!(approx_eq(out2.value, 0.75, 1e-10));
    }

    #[test]
    fn full_pipeline() {
        let chain = ChainBuilder::new()
            .deadband(0.5)
            .ema(0.5)
            .normalize(0.0, 10.0)
            .confidence_gate(0.3)
            .build();

        // First signal: passes deadband, ema=5.0, norm=0.5, confidence=0.9 > 0.3
        let out = chain.process(Signal::new(5.0, 0, 0.9, "s1"));
        assert!(approx_eq(out.value, 0.5, 1e-10));
    }

    #[test]
    fn is_empty_and_len() {
        let empty = ChainBuilder::new().build();
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let chain = ChainBuilder::new().ema(0.5).build();
        assert!(!chain.is_empty());
        assert_eq!(chain.len(), 1);
    }
}
