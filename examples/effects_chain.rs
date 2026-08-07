// Signal Chain Examples
// Run with: cargo run --example effects_chain

use signal_chain::{SignalChain, Gain, Clipper, Delay, LowPass, SineOsc, SignalNode};

fn main() {
    // Example 1: Guitar distortion pedal
    // SineOsc → Gain(3.0) → Clipper(0.7) → output
    let mut distortion = SignalChain::new()
        .push(Gain::new(3.0))      // boost the signal
        .push(Clipper::new(0.7));  // clip the peaks

    let mut osc = SineOsc::new(220.0, 44100.0); // A3
    println!("=== Distortion Pedal ===");
    for i in 0..8 {
        let sample = osc.next();
        let distorted = distortion.process(sample);
        println!("  sample {}: in={:+.4}  out={:+.4}", i, sample, distorted);
    }

    // Example 2: Echo with feedback
    // Signal → Delay(8 samples, 0.5 feedback, 0.6 mix) → output
    let mut echo = Delay::new(8, 0.5, 0.6);
    println!("\n=== Echo (8-sample delay, 0.5 feedback) ===");
    // Send an impulse
    let impulse = [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
                   0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    for (i, &sample) in impulse.iter().enumerate() {
        let out = echo.process(sample);
        if out.abs() > 0.001 {
            println!("  tick {}: echo={:+.4}", i, out);
        }
    }

    // Example 3: Low-pass filter sweep
    // Start with a bright signal, filter it down
    let mut filter = LowPass::new(500.0, 44100.0); // 500 Hz cutoff
    let mut bright_osc = SineOsc::new(2000.0, 44100.0); // 2kHz tone
    println!("\n=== Low-Pass Filter (500Hz cutoff on 2kHz tone) ===");
    let mut sum_in = 0.0;
    let mut sum_out = 0.0;
    for _ in 0..1000 {
        let sample = bright_osc.next();
        let filtered = filter.process(sample);
        sum_in += sample.abs();
        sum_out += filtered.abs();
    }
    println!("  avg input level:  {:.4}", sum_in / 1000.0);
    println!("  avg output level: {:.4}", sum_out / 1000.0);
    println!("  attenuation:      {:.1}%", (1.0 - sum_out / sum_in) * 100.0);

    // Example 4: Full chain — the ship's foghorn
    // Low osc → heavy gain → clipper → long delay → low-pass → output
    let mut foghorn = SignalChain::new()
        .push(Gain::new(5.0))
        .push(Clipper::new(0.9))
        .push(Delay::new(4410, 0.7, 0.5))  // 100ms delay at 44.1kHz
        .push(LowPass::new(300.0, 44100.0));

    let mut horn_osc = SineOsc::new(80.0, 44100.0); // low E2
    println!("\n=== The Ship's Foghorn ===");
    for i in 0..5 {
        let sample = horn_osc.next();
        let out = foghorn.process(sample);
        println!("  sample {}: raw={:+.4}  horn={:+.4}", i, sample, out);
    }

    println!("\nAll examples complete. The signal chain works.");
}
