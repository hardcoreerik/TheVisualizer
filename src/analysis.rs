use rustfft::{Fft, FftPlanner, num_complex::Complex32};
use std::sync::Arc;

pub const FFT_SIZE: usize = 2_048;
pub const WAVEFORM_POINTS: usize = 256;
pub const SPECTRUM_BANDS: usize = 64;

#[derive(Clone)]
pub struct Features {
    pub waveform: Vec<f32>,
    pub spectrum: Vec<f32>,
    pub rms: f32,
    pub peak: f32,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

impl Default for Features {
    fn default() -> Self {
        Self {
            waveform: vec![0.0; WAVEFORM_POINTS],
            spectrum: vec![0.0; SPECTRUM_BANDS],
            rms: 0.0,
            peak: 0.0,
            low: 0.0,
            mid: 0.0,
            high: 0.0,
        }
    }
}

pub struct Analyzer {
    fft: Arc<dyn Fft<f32>>,
    input: Vec<Complex32>,
}

impl Analyzer {
    pub fn new() -> Self {
        let mut planner = FftPlanner::new();
        Self {
            fft: planner.plan_fft_forward(FFT_SIZE),
            input: vec![Complex32::ZERO; FFT_SIZE],
        }
    }

    pub fn analyze(&mut self, samples: &[f32], sample_rate: u32) -> Features {
        if samples.is_empty() {
            return Features::default();
        }

        let recent = &samples[samples.len().saturating_sub(FFT_SIZE)..];
        let offset = FFT_SIZE - recent.len();
        self.input.fill(Complex32::ZERO);

        for (index, sample) in recent.iter().copied().enumerate() {
            let position = offset + index;
            let window =
                0.5 - 0.5 * (std::f32::consts::TAU * position as f32 / (FFT_SIZE - 1) as f32).cos();
            self.input[position].re = sample * window;
        }

        let rms =
            (recent.iter().map(|sample| sample * sample).sum::<f32>() / recent.len() as f32).sqrt();
        let peak = recent
            .iter()
            .fold(0.0_f32, |maximum, sample| maximum.max(sample.abs()));

        self.fft.process(&mut self.input);

        let nyquist_bins = FFT_SIZE / 2;
        let hz_per_bin = sample_rate as f32 / FFT_SIZE as f32;
        let mut spectrum = vec![0.0; SPECTRUM_BANDS];
        for (band, value) in spectrum.iter_mut().enumerate() {
            let start_ratio = band as f32 / SPECTRUM_BANDS as f32;
            let end_ratio = (band + 1) as f32 / SPECTRUM_BANDS as f32;
            let start = ((start_ratio.powf(2.2) * nyquist_bins as f32) as usize).max(1);
            let end = ((end_ratio.powf(2.2) * nyquist_bins as f32) as usize)
                .max(start + 1)
                .min(nyquist_bins);
            let magnitude = self.input[start..end]
                .iter()
                .map(|bin| bin.norm())
                .sum::<f32>()
                / (end - start) as f32
                / FFT_SIZE as f32;
            *value = (magnitude * 96.0).ln_1p() / 97.0_f32.ln();
        }

        let band_energy = |minimum_hz: f32, maximum_hz: f32| {
            let start = (minimum_hz / hz_per_bin).round() as usize;
            let end = ((maximum_hz / hz_per_bin).round() as usize)
                .max(start + 1)
                .min(nyquist_bins);
            if start >= end {
                return 0.0;
            }
            let magnitude = self.input[start..end]
                .iter()
                .map(|bin| bin.norm())
                .sum::<f32>()
                / (end - start) as f32
                / FFT_SIZE as f32;
            ((magnitude * 96.0).ln_1p() / 97.0_f32.ln()).clamp(0.0, 1.0)
        };

        let waveform = (0..WAVEFORM_POINTS)
            .map(|point| {
                let index = point * recent.len() / WAVEFORM_POINTS;
                recent[index.min(recent.len() - 1)]
            })
            .collect();

        Features {
            waveform,
            spectrum,
            rms,
            peak,
            low: band_energy(20.0, 250.0),
            mid: band_energy(250.0, 2_000.0),
            high: band_energy(2_000.0, 12_000.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn silence_is_stable_and_sine_has_energy() {
        let mut analyzer = Analyzer::new();
        let silence = analyzer.analyze(&vec![0.0; FFT_SIZE], 48_000);
        assert_eq!(silence.rms, 0.0);
        assert!(silence.spectrum.iter().all(|value| *value == 0.0));

        let sine: Vec<f32> = (0..FFT_SIZE)
            .map(|index| (std::f32::consts::TAU * 440.0 * index as f32 / 48_000.0).sin() * 0.5)
            .collect();
        let tone = analyzer.analyze(&sine, 48_000);
        assert!(tone.rms > 0.3);
        assert!(tone.mid > tone.low);
        assert!(tone.mid > tone.high);
    }
}
