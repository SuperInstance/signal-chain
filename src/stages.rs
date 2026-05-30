//! Concrete signal processing stages.

use std::cell::RefCell;

use crate::{Signal, SignalStage};

// ---------------------------------------------------------------------------
// DeadbandFilter
// ---------------------------------------------------------------------------

/// Only passes signals whose value differs from the last emitted value by more
/// than `threshold`. This models the thermostat deadband — the "spectral gap"
/// insight from the Grand Pattern.
#[derive(Debug)]
pub struct DeadbandFilter {
    threshold: f64,
    last_value: RefCell<Option<f64>>,
}

impl DeadbandFilter {
    pub fn new(threshold: f64) -> Self {
        Self {
            threshold,
            last_value: RefCell::new(None),
        }
    }
}

impl SignalStage for DeadbandFilter {
    fn process_boxed(&self, input: Signal) -> Signal {
        let mut last = self.last_value.borrow_mut();
        match *last {
            Some(prev) if (input.value - prev).abs() <= self.threshold => {
                Signal {
                    confidence: 0.0,
                    ..input
                }
            }
            _ => {
                *last = Some(input.value);
                input
            }
        }
    }

    fn name(&self) -> &str {
        "DeadbandFilter"
    }

    fn box_clone(&self) -> Box<dyn SignalStage> {
        Box::new(Self {
            threshold: self.threshold,
            last_value: RefCell::new(*self.last_value.borrow()),
        })
    }
}

// ---------------------------------------------------------------------------
// MovingAverage
// ---------------------------------------------------------------------------

/// Windowed moving average of recent signal values.
#[derive(Debug)]
pub struct MovingAverage {
    window_size: usize,
    buffer: RefCell<Vec<f64>>,
}

impl MovingAverage {
    pub fn new(window_size: usize) -> Self {
        assert!(window_size > 0, "window_size must be > 0");
        Self {
            window_size,
            buffer: RefCell::new(Vec::with_capacity(window_size)),
        }
    }
}

impl SignalStage for MovingAverage {
    fn process_boxed(&self, input: Signal) -> Signal {
        let mut buf = self.buffer.borrow_mut();
        buf.push(input.value);
        if buf.len() > self.window_size {
            buf.remove(0);
        }
        let avg: f64 = buf.iter().sum::<f64>() / buf.len() as f64;
        Signal { value: avg, ..input }
    }

    fn name(&self) -> &str {
        "MovingAverage"
    }

    fn box_clone(&self) -> Box<dyn SignalStage> {
        Box::new(Self {
            window_size: self.window_size,
            buffer: RefCell::new(self.buffer.borrow().clone()),
        })
    }
}

// ---------------------------------------------------------------------------
// ExponentialSmoothing
// ---------------------------------------------------------------------------

/// Exponential moving average (EMA) with configurable smoothing factor `alpha`.
///
/// `output = alpha * input + (1 - alpha) * prev_output`
#[derive(Debug)]
pub struct ExponentialSmoothing {
    alpha: f64,
    prev: RefCell<Option<f64>>,
}

impl ExponentialSmoothing {
    pub fn new(alpha: f64) -> Self {
        assert!((0.0..=1.0).contains(&alpha), "alpha must be in [0, 1]");
        Self {
            alpha,
            prev: RefCell::new(None),
        }
    }
}

impl SignalStage for ExponentialSmoothing {
    fn process_boxed(&self, input: Signal) -> Signal {
        let mut prev = self.prev.borrow_mut();
        let smoothed = match *prev {
            Some(p) => self.alpha * input.value + (1.0 - self.alpha) * p,
            None => input.value,
        };
        *prev = Some(smoothed);
        Signal {
            value: smoothed,
            ..input
        }
    }

    fn name(&self) -> &str {
        "ExponentialSmoothing"
    }

    fn box_clone(&self) -> Box<dyn SignalStage> {
        Box::new(Self {
            alpha: self.alpha,
            prev: RefCell::new(*self.prev.borrow()),
        })
    }
}

// ---------------------------------------------------------------------------
// ConfidenceGate
// ---------------------------------------------------------------------------

/// Drops (zeroes out) signals whose confidence falls below `threshold`.
#[derive(Debug, Clone)]
pub struct ConfidenceGate {
    threshold: f64,
}

impl ConfidenceGate {
    pub fn new(threshold: f64) -> Self {
        Self { threshold }
    }
}

impl SignalStage for ConfidenceGate {
    fn process_boxed(&self, input: Signal) -> Signal {
        if input.confidence < self.threshold {
            Signal {
                value: 0.0,
                confidence: 0.0,
                ..input
            }
        } else {
            input
        }
    }

    fn name(&self) -> &str {
        "ConfidenceGate"
    }

    fn box_clone(&self) -> Box<dyn SignalStage> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// Normalizer
// ---------------------------------------------------------------------------

/// Maps signal values linearly into `[0, 1]` given known `min` and `max`.
#[derive(Debug, Clone)]
pub struct Normalizer {
    min: f64,
    max: f64,
}

impl Normalizer {
    pub fn new(min: f64, max: f64) -> Self {
        assert!(max > min, "max must be > min");
        Self { min, max }
    }
}

impl SignalStage for Normalizer {
    fn process_boxed(&self, input: Signal) -> Signal {
        let normalized = ((input.value - self.min) / (self.max - self.min)).clamp(0.0, 1.0);
        Signal {
            value: normalized,
            ..input
        }
    }

    fn name(&self) -> &str {
        "Normalizer"
    }

    fn box_clone(&self) -> Box<dyn SignalStage> {
        Box::new(self.clone())
    }
}

// ---------------------------------------------------------------------------
// Aggregator
// ---------------------------------------------------------------------------

/// Aggregation method for combining multiple signals.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregationMethod {
    Mean,
    Max,
    Weighted,
}

/// Combines multiple signals into one using the chosen method.
/// When using `Weighted`, confidence is used as the weight.
#[derive(Debug, Clone)]
pub struct Aggregator {
    method: AggregationMethod,
}

impl Aggregator {
    pub fn new(method: AggregationMethod) -> Self {
        Self { method }
    }

    /// Aggregate a slice of signals into a single signal.
    pub fn aggregate(&self, signals: &[Signal]) -> Signal {
        assert!(!signals.is_empty(), "cannot aggregate empty signal set");

        let value = match self.method {
            AggregationMethod::Mean => {
                let sum: f64 = signals.iter().map(|s| s.value).sum();
                sum / signals.len() as f64
            }
            AggregationMethod::Max => signals
                .iter()
                .map(|s| s.value)
                .fold(f64::NEG_INFINITY, f64::max),
            AggregationMethod::Weighted => {
                let total_weight: f64 = signals.iter().map(|s| s.confidence).sum();
                if total_weight == 0.0 {
                    return Signal::default();
                }
                let weighted_sum: f64 = signals.iter().map(|s| s.value * s.confidence).sum();
                weighted_sum / total_weight
            }
        };

        let confidence: f64 =
            signals.iter().map(|s| s.confidence).sum::<f64>() / signals.len() as f64;
        let timestamp = signals.iter().map(|s| s.timestamp).max().unwrap_or(0);
        let source = signals
            .iter()
            .map(|s| s.source.as_str())
            .collect::<Vec<_>>()
            .join("+");

        Signal {
            value,
            timestamp,
            confidence,
            source,
        }
    }
}

impl SignalStage for Aggregator {
    /// Single-signal passthrough (use `aggregate` for multi-signal).
    fn process_boxed(&self, input: Signal) -> Signal {
        input
    }

    fn name(&self) -> &str {
        "Aggregator"
    }

    fn box_clone(&self) -> Box<dyn SignalStage> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::approx_eq;

    // -- DeadbandFilter --
    #[test]
    fn deadband_first_always_passes() {
        let db = DeadbandFilter::new(0.5);
        let out = db.process_boxed(Signal::simple(1.0, 0));
        assert!(approx_eq(out.value, 1.0, 1e-10));
        assert!(approx_eq(out.confidence, 1.0, 1e-10));
    }

    #[test]
    fn deadband_suppresses_small_change() {
        let db = DeadbandFilter::new(1.0);
        db.process_boxed(Signal::simple(5.0, 0));
        let out = db.process_boxed(Signal::simple(5.5, 1));
        assert!(out.confidence.abs() < 1e-10);
    }

    #[test]
    fn deadband_passes_large_change() {
        let db = DeadbandFilter::new(1.0);
        db.process_boxed(Signal::simple(5.0, 0));
        let out = db.process_boxed(Signal::simple(7.0, 1));
        assert!(approx_eq(out.value, 7.0, 1e-10));
        assert!(approx_eq(out.confidence, 1.0, 1e-10));
    }

    // -- MovingAverage --
    #[test]
    fn moving_average_single() {
        let ma = MovingAverage::new(3);
        let out = ma.process_boxed(Signal::simple(6.0, 0));
        assert!(approx_eq(out.value, 6.0, 1e-10));
    }

    #[test]
    fn moving_average_window() {
        let ma = MovingAverage::new(3);
        ma.process_boxed(Signal::simple(2.0, 0));
        ma.process_boxed(Signal::simple(4.0, 1));
        let out = ma.process_boxed(Signal::simple(6.0, 2));
        assert!(approx_eq(out.value, 4.0, 1e-10));
    }

    #[test]
    fn moving_average_slides() {
        let ma = MovingAverage::new(2);
        ma.process_boxed(Signal::simple(10.0, 0));
        let out = ma.process_boxed(Signal::simple(20.0, 1));
        assert!(approx_eq(out.value, 15.0, 1e-10));
    }

    // -- ExponentialSmoothing --
    #[test]
    fn ema_first_passthrough() {
        let ema = ExponentialSmoothing::new(0.3);
        let out = ema.process_boxed(Signal::simple(10.0, 0));
        assert!(approx_eq(out.value, 10.0, 1e-10));
    }

    #[test]
    fn ema_smooths() {
        let ema = ExponentialSmoothing::new(0.5);
        ema.process_boxed(Signal::simple(0.0, 0));
        let out = ema.process_boxed(Signal::simple(10.0, 1));
        assert!(approx_eq(out.value, 5.0, 1e-10));
    }

    // -- ConfidenceGate --
    #[test]
    fn confidence_gate_passes() {
        let gate = ConfidenceGate::new(0.5);
        let s = Signal::new(3.0, 0, 0.8, "ok");
        let out = gate.process_boxed(s);
        assert!(approx_eq(out.value, 3.0, 1e-10));
    }

    #[test]
    fn confidence_gate_drops() {
        let gate = ConfidenceGate::new(0.5);
        let s = Signal::new(3.0, 0, 0.1, "bad");
        let out = gate.process_boxed(s);
        assert!(out.value.abs() < 1e-10);
        assert!(out.confidence.abs() < 1e-10);
    }

    // -- Normalizer --
    #[test]
    fn normalizer_midpoint() {
        let n = Normalizer::new(0.0, 100.0);
        let out = n.process_boxed(Signal::simple(50.0, 0));
        assert!(approx_eq(out.value, 0.5, 1e-10));
    }

    #[test]
    fn normalizer_clamps_low() {
        let n = Normalizer::new(10.0, 20.0);
        let out = n.process_boxed(Signal::simple(5.0, 0));
        assert!(out.value.abs() < 1e-10);
    }

    #[test]
    fn normalizer_clamps_high() {
        let n = Normalizer::new(10.0, 20.0);
        let out = n.process_boxed(Signal::simple(25.0, 0));
        assert!(approx_eq(out.value, 1.0, 1e-10));
    }

    // -- Aggregator --
    #[test]
    fn aggregator_mean() {
        let agg = Aggregator::new(AggregationMethod::Mean);
        let signals = vec![
            Signal::new(2.0, 0, 1.0, "a"),
            Signal::new(4.0, 1, 1.0, "b"),
            Signal::new(6.0, 2, 1.0, "c"),
        ];
        let result = agg.aggregate(&signals);
        assert!(approx_eq(result.value, 4.0, 1e-10));
    }

    #[test]
    fn aggregator_max() {
        let agg = Aggregator::new(AggregationMethod::Max);
        let signals = vec![
            Signal::new(2.0, 0, 1.0, "a"),
            Signal::new(9.0, 1, 1.0, "b"),
            Signal::new(4.0, 2, 1.0, "c"),
        ];
        let result = agg.aggregate(&signals);
        assert!(approx_eq(result.value, 9.0, 1e-10));
    }

    #[test]
    fn aggregator_weighted() {
        let agg = Aggregator::new(AggregationMethod::Weighted);
        let signals = vec![
            Signal::new(0.0, 0, 1.0, "a"),
            Signal::new(10.0, 1, 3.0, "b"),
        ];
        let result = agg.aggregate(&signals);
        assert!(approx_eq(result.value, 7.5, 1e-10));
    }
}
