use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use cpal::{
    DeviceId, FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
    SupportedStreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};

const MAX_SAMPLES: usize = 16_384;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    System,
    Microphone,
}

impl SourceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::System => "System audio",
            Self::Microphone => "Microphone",
        }
    }
}

pub struct SampleBuffer {
    samples: VecDeque<f32>,
    pub last_callback: Option<Instant>,
    callback_sequence: u64,
    newest_sample_backend_age: Option<Duration>,
}

impl Default for SampleBuffer {
    fn default() -> Self {
        Self {
            samples: VecDeque::with_capacity(MAX_SAMPLES),
            last_callback: None,
            callback_sequence: 0,
            newest_sample_backend_age: None,
        }
    }
}

pub struct SampleSnapshot {
    pub samples: Vec<f32>,
    pub callback_sequence: u64,
    callback_received: Option<Instant>,
    newest_sample_backend_age: Option<Duration>,
}

impl SampleSnapshot {
    pub fn newest_sample_age(&self, now: Instant) -> Option<Duration> {
        Some(
            self.newest_sample_backend_age?
                + now.saturating_duration_since(self.callback_received?),
        )
    }
}

impl SampleBuffer {
    fn push(&mut self, sample: f32) {
        if self.samples.len() == MAX_SAMPLES {
            self.samples.pop_front();
        }
        self.samples.push_back(sample.clamp(-1.0, 1.0));
    }

    pub fn snapshot(&self, count: usize) -> SampleSnapshot {
        SampleSnapshot {
            samples: self
                .samples
                .iter()
                .skip(self.samples.len().saturating_sub(count))
                .copied()
                .collect(),
            callback_sequence: self.callback_sequence,
            callback_received: self.last_callback,
            newest_sample_backend_age: self.newest_sample_backend_age,
        }
    }
}

pub type SharedSamples = Arc<Mutex<SampleBuffer>>;

#[derive(Clone)]
pub struct AudioDevice {
    device: cpal::Device,
    pub id: DeviceId,
    pub source: SourceKind,
    pub name: String,
    pub is_default: bool,
}

impl AudioDevice {
    pub fn default_id(source: SourceKind) -> Result<DeviceId, String> {
        let host = cpal::default_host();
        let device = match source {
            SourceKind::System => host.default_output_device(),
            SourceKind::Microphone => host.default_input_device(),
        }
        .ok_or_else(|| {
            format!(
                "No default {} device is available",
                source.label().to_lowercase()
            )
        })?;
        device.id().map_err(|error| {
            format!(
                "Could not read default {} device ID: {error}",
                source.label()
            )
        })
    }

    pub fn enumerate(source: SourceKind) -> Result<Vec<Self>, String> {
        let host = cpal::default_host();
        let default_id = Self::default_id(source).ok();
        let devices = match source {
            SourceKind::System => host
                .output_devices()
                .map(|devices| devices.collect::<Vec<_>>()),
            SourceKind::Microphone => host
                .input_devices()
                .map(|devices| devices.collect::<Vec<_>>()),
        }
        .map_err(|error| format!("Could not enumerate {} devices: {error}", source.label()))?;

        devices
            .into_iter()
            .map(|device| {
                let id = device.id().map_err(|error| {
                    format!("Could not read {} device ID: {error}", source.label())
                })?;
                let name = device_name(&device, source);
                let is_default = default_id.as_ref() == Some(&id);
                Ok(Self {
                    device,
                    id,
                    source,
                    name,
                    is_default,
                })
            })
            .collect()
    }

    pub fn label(&self) -> String {
        device_label(&self.name, self.is_default)
    }
}

pub struct AudioCapture {
    _stream: Stream,
    pub source: SourceKind,
    pub device_id: DeviceId,
    pub device_name: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub error: Arc<Mutex<Option<String>>>,
}

impl AudioCapture {
    pub fn start(source: SourceKind, samples: SharedSamples) -> Result<Self, String> {
        let host = cpal::default_host();
        let device = match source {
            SourceKind::System => host.default_output_device(),
            SourceKind::Microphone => host.default_input_device(),
        }
        .ok_or_else(|| format!("No {} device is available", source.label().to_lowercase()))?;
        Self::start_on_device(source, device, samples)
    }

    pub fn start_selected(device: &AudioDevice, samples: SharedSamples) -> Result<Self, String> {
        Self::start_on_device(device.source, device.device.clone(), samples)
    }

    fn start_on_device(
        source: SourceKind,
        device: cpal::Device,
        samples: SharedSamples,
    ) -> Result<Self, String> {
        let device_id = device
            .id()
            .map_err(|error| format!("Could not read {} device ID: {error}", source.label()))?;
        let device_name = device_name(&device, source);
        let supported = match source {
            SourceKind::System => device.default_output_config(),
            SourceKind::Microphone => device.default_input_config(),
        }
        .map_err(|error| format!("Could not read {device_name} configuration: {error}"))?;

        let sample_rate = supported.sample_rate();
        let channels = supported.channels();
        let error = Arc::new(Mutex::new(None));
        let stream = build_stream(&device, &supported, samples, Arc::clone(&error))
            .map_err(|stream_error| format!("Could not capture {device_name}: {stream_error}"))?;

        stream
            .play()
            .map_err(|play_error| format!("Could not start {device_name}: {play_error}"))?;

        Ok(Self {
            _stream: stream,
            source,
            device_id,
            device_name,
            sample_rate,
            channels,
            error,
        })
    }
}

fn device_name(device: &cpal::Device, source: SourceKind) -> String {
    device
        .description()
        .map(|description| description.name().to_owned())
        .unwrap_or_else(|_| source.label().to_owned())
}

fn device_label(name: &str, is_default: bool) -> String {
    if is_default {
        format!("{name} (default)")
    } else {
        name.to_owned()
    }
}

fn build_stream(
    device: &cpal::Device,
    supported: &SupportedStreamConfig,
    samples: SharedSamples,
    error: Arc<Mutex<Option<String>>>,
) -> Result<Stream, cpal::Error> {
    let config = supported.config();
    match supported.sample_format() {
        SampleFormat::I8 => build_typed::<i8>(device, config, samples, error),
        SampleFormat::I16 => build_typed::<i16>(device, config, samples, error),
        SampleFormat::I24 => build_typed::<cpal::I24>(device, config, samples, error),
        SampleFormat::I32 => build_typed::<i32>(device, config, samples, error),
        SampleFormat::I64 => build_typed::<i64>(device, config, samples, error),
        SampleFormat::U8 => build_typed::<u8>(device, config, samples, error),
        SampleFormat::U16 => build_typed::<u16>(device, config, samples, error),
        SampleFormat::U24 => build_typed::<cpal::U24>(device, config, samples, error),
        SampleFormat::U32 => build_typed::<u32>(device, config, samples, error),
        SampleFormat::U64 => build_typed::<u64>(device, config, samples, error),
        SampleFormat::F32 => build_typed::<f32>(device, config, samples, error),
        SampleFormat::F64 => build_typed::<f64>(device, config, samples, error),
        format => Err(cpal::Error::with_message(
            cpal::ErrorKind::UnsupportedConfig,
            format!("Unsupported sample format: {format}"),
        )),
    }
}

fn build_typed<T>(
    device: &cpal::Device,
    config: StreamConfig,
    samples: SharedSamples,
    error: Arc<Mutex<Option<String>>>,
) -> Result<Stream, cpal::Error>
where
    T: SizedSample + Sample,
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    let sample_rate = config.sample_rate;
    device.build_input_stream(
        config,
        move |data: &[T], info| {
            let timestamp = info.timestamp();
            let newest_sample_backend_age = newest_sample_backend_age(
                timestamp.callback.duration_since(timestamp.capture),
                data.len() / channels,
                sample_rate,
            );
            // ponytail: try_lock drops a callback under UI contention; use a lock-free ring only
            // if measurements show visible loss.
            if let Ok(mut buffer) = samples.try_lock() {
                for frame in data.chunks(channels) {
                    let mono = frame.iter().copied().map(f32::from_sample).sum::<f32>()
                        / frame.len() as f32;
                    buffer.push(mono);
                }
                buffer.last_callback = Some(Instant::now());
                buffer.callback_sequence = buffer.callback_sequence.wrapping_add(1);
                buffer.newest_sample_backend_age = Some(newest_sample_backend_age);
            }
        },
        move |stream_error| {
            if let Ok(mut slot) = error.try_lock() {
                *slot = Some(stream_error.to_string());
            }
        },
        None,
    )
}

fn newest_sample_backend_age(
    capture_to_callback: Duration,
    frames: usize,
    sample_rate: u32,
) -> Duration {
    let packet_span =
        Duration::from_secs_f64(frames.saturating_sub(1) as f64 / f64::from(sample_rate.max(1)));
    capture_to_callback.saturating_sub(packet_span)
}

#[cfg(test)]
mod tests {
    use super::{device_label, newest_sample_backend_age};
    use std::time::Duration;

    #[test]
    fn device_labels_identify_only_the_default() {
        assert_eq!(device_label("Speakers", true), "Speakers (default)");
        assert_eq!(device_label("Display audio", false), "Display audio");
    }

    #[test]
    fn newest_sample_age_removes_the_packet_span() {
        assert_eq!(
            newest_sample_backend_age(Duration::from_millis(30), 481, 48_000),
            Duration::from_millis(20)
        );
        assert_eq!(
            newest_sample_backend_age(Duration::from_millis(5), 481, 48_000),
            Duration::ZERO
        );
    }
}
