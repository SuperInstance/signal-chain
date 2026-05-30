//! # signal-chain
//!
//! A functional signal processing pipeline inspired by the Grand Pattern's
//! signal processing chain — transforming raw sensor data into actionable
//! room signals.

mod chain;
mod stages;

pub use chain::{ChainBuilder, SignalChain};
pub use stages::{
    AggregationMethod, Aggregator, ConfidenceGate, DeadbandFilter, ExponentialSmoothing,
    MovingAverage, Normalizer,
};

/// A single processing stage in a signal chain.
///
/// Each stage takes a boxed signal and returns a transformed boxed signal.
pub trait SignalStage: std::fmt::Debug + Send {
    /// Process a signal, returning the transformed signal.
    fn process_boxed(&self, input: Signal) -> Signal;

    /// Human-readable name of this stage.
    fn name(&self) -> &str;

    /// Clone into a boxed trait object.
    fn box_clone(&self) -> Box<dyn SignalStage>;
}

/// A signal carrying a sensor value with metadata.
///
/// Signals flow through a [`SignalChain`] of [`SignalStage`] processors,
/// each transforming the signal in some way (filtering, smoothing, normalizing, etc.).
#[derive(Debug, Clone)]
pub struct Signal {
    /// The signal value.
    pub value: f64,
    /// Timestamp (e.g., Unix millis or monotonic counter).
    pub timestamp: u64,
    /// Confidence in this signal, typically in [0, 1].
    pub confidence: f64,
    /// Source identifier (sensor name, stage label, etc.).
    pub source: String,
}

impl Signal {
    /// Create a new signal with all fields.
    pub fn new(value: f64, timestamp: u64, confidence: f64, source: impl Into<String>) -> Self {
        Self {
            value,
            timestamp,
            confidence,
            source: source.into(),
        }
    }

    /// Convenience constructor for a basic signal with full confidence.
    pub fn simple(value: f64, timestamp: u64) -> Self {
        Self {
            value,
            timestamp,
            confidence: 1.0,
            source: String::new(),
        }
    }

    /// Compute "signal energy" — the squared magnitude of the value
    /// weighted by confidence. Used for conservation checks.
    pub fn energy(&self) -> f64 {
        (self.value * self.value) * self.confidence
    }
}

impl Default for Signal {
    fn default() -> Self {
        Self {
            value: 0.0,
            timestamp: 0,
            confidence: 1.0,
            source: String::new(),
        }
    }
}

/// Approximate equality helper for f64 comparisons in tests.
pub fn approx_eq(a: f64, b: f64, epsilon: f64) -> bool {
    (a - b).abs() < epsilon
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_new() {
        let s = Signal::new(3.14, 100, 0.95, "temp-sensor");
        assert!((s.value - 3.14).abs() < 1e-10);
        assert_eq!(s.timestamp, 100);
        assert!((s.confidence - 0.95).abs() < 1e-10);
        assert_eq!(s.source, "temp-sensor");
    }

    #[test]
    fn signal_simple() {
        let s = Signal::simple(42.0, 50);
        assert!((s.value - 42.0).abs() < 1e-10);
        assert_eq!(s.timestamp, 50);
        assert!((s.confidence - 1.0).abs() < 1e-10);
        assert!(s.source.is_empty());
    }

    #[test]
    fn signal_default() {
        let s = Signal::default();
        assert!((s.value).abs() < 1e-10);
        assert_eq!(s.timestamp, 0);
        assert!((s.confidence - 1.0).abs() < 1e-10);
    }

    #[test]
    fn signal_energy() {
        let s = Signal::new(2.0, 0, 0.5, "");
        assert!((s.energy() - 2.0).abs() < 1e-10); // 2^2 * 0.5 = 2.0
    }

    #[test]
    fn approx_eq_works() {
        assert!(approx_eq(1.0, 1.0 + 1e-9, 1e-6));
        assert!(!approx_eq(1.0, 1.1, 1e-6));
    }
}
