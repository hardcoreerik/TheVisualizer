mod analysis;
mod audio;
mod plugin;
mod preset;
mod render;

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use analysis::{Analyzer, FFT_SIZE, Features};
use audio::{AudioCapture, AudioDevice, SampleBuffer, SharedSamples, SourceKind};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use plugin::{LoadedPlugin, PluginPackage};
use preset::Preset;
use render::{GpuPresetRenderer, PresetFrame};

const VISUAL_NAMES: [&str; 3] = ["NEON SCOPE", "PARTICLE ARRAY", "GPU PRESET"];
const VISUAL_BUTTONS: [&str; 3] = ["Scope", "Particles", "Preset"];
const DEFAULT_DEVICE_CHECK_INTERVAL: Duration = Duration::from_secs(1);
const VISUAL_TRAIL_FRAMES: usize = 10;

#[derive(Default)]
struct LatencyStats {
    current_ms: f64,
    average_ms: f64,
    peak_ms: f64,
    observations: u64,
    last_callback_sequence: u64,
}

impl LatencyStats {
    fn observe(&mut self, callback_sequence: u64, latency: Duration) {
        if callback_sequence == 0 || callback_sequence == self.last_callback_sequence {
            return;
        }
        self.last_callback_sequence = callback_sequence;
        self.current_ms = latency.as_secs_f64() * 1_000.0;
        self.observations += 1;
        self.average_ms += (self.current_ms - self.average_ms) / self.observations as f64;
        self.peak_ms = self.peak_ms.max(self.current_ms);
    }
}

#[derive(Default)]
struct FrameStats {
    last_frame: Option<Instant>,
    current_ms: f64,
    smoothed_ms: f64,
    observations: u64,
}

impl FrameStats {
    fn observe(&mut self, now: Instant) {
        let Some(previous) = self.last_frame.replace(now) else {
            return;
        };
        self.current_ms = now.saturating_duration_since(previous).as_secs_f64() * 1_000.0;
        self.observations += 1;
        if self.observations == 1 {
            self.smoothed_ms = self.current_ms;
        } else {
            self.smoothed_ms += (self.current_ms - self.smoothed_ms) * 0.1;
        }
    }

    fn fps(&self) -> f64 {
        if self.smoothed_ms > 0.0 {
            1_000.0 / self.smoothed_ms
        } else {
            0.0
        }
    }
}

#[derive(Default)]
struct DefaultSwitchTiming {
    reopen_ms: f64,
    first_callback_ms: Option<f64>,
}

impl DefaultSwitchTiming {
    fn observe_first_callback(
        &mut self,
        pending_since: &mut Option<Instant>,
        callback_sequence: u64,
        now: Instant,
    ) {
        if callback_sequence == 0 || self.first_callback_ms.is_some() {
            return;
        }
        if let Some(started) = pending_since.take() {
            self.first_callback_ms =
                Some(now.saturating_duration_since(started).as_secs_f64() * 1_000.0);
        }
    }
}

#[derive(Default)]
struct VisualHistory {
    last_callback_sequence: u64,
    waveforms: VecDeque<Vec<f32>>,
    spectra: VecDeque<Vec<f32>>,
    smoothed_spectrum: Vec<f32>,
}

impl VisualHistory {
    fn update(&mut self, callback_sequence: u64, features: &Features) {
        if callback_sequence == 0 || callback_sequence == self.last_callback_sequence {
            return;
        }
        self.last_callback_sequence = callback_sequence;
        if self.smoothed_spectrum.len() != features.spectrum.len() {
            self.smoothed_spectrum.clone_from(&features.spectrum);
        } else {
            for (smoothed, current) in self.smoothed_spectrum.iter_mut().zip(&features.spectrum) {
                let speed = if *current > *smoothed { 0.55 } else { 0.14 };
                *smoothed += (*current - *smoothed) * speed;
            }
        }
        self.waveforms.push_back(features.waveform.clone());
        self.spectra.push_back(self.smoothed_spectrum.clone());
        while self.waveforms.len() > VISUAL_TRAIL_FRAMES {
            self.waveforms.pop_front();
            self.spectra.pop_front();
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PresentationMode {
    Windowed,
    Borderless,
    Fullscreen,
}

impl PresentationMode {
    fn label(self) -> &'static str {
        match self {
            Self::Windowed => "Windowed",
            Self::Borderless => "Borderless [B]",
            Self::Fullscreen => "Fullscreen [F11]",
        }
    }

    fn toggle_borderless(self) -> Self {
        if self == Self::Borderless {
            Self::Windowed
        } else {
            Self::Borderless
        }
    }

    fn toggle_fullscreen(self) -> Self {
        if self == Self::Fullscreen {
            Self::Windowed
        } else {
            Self::Fullscreen
        }
    }
}

fn main() -> eframe::Result {
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TheVisualizer")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 500.0]),
        ..Default::default()
    };
    if let Ok(name) = std::env::var("THEVISUALIZER_GPU")
        && let eframe::egui_wgpu::WgpuSetup::CreateNew(setup) = &mut options.wgpu_options.wgpu_setup
    {
        setup.native_adapter_selector = Some(Arc::new(move |adapters, surface| {
            let available = adapters
                .iter()
                .map(|adapter| adapter.get_info().name)
                .collect::<Vec<_>>()
                .join(", ");
            adapters
                .iter()
                .find(|adapter| {
                    adapter
                        .get_info()
                        .name
                        .to_lowercase()
                        .contains(&name.to_lowercase())
                        && surface.is_none_or(|surface| adapter.is_surface_supported(surface))
                })
                .cloned()
                .ok_or_else(|| {
                    format!("No compatible GPU matched {name:?}; available: {available}")
                })
        }));
    }

    eframe::run_native(
        "TheVisualizer",
        options,
        Box::new(|creation| Ok(Box::new(VisualizerApp::new(creation)))),
    )
}

struct VisualizerApp {
    samples: SharedSamples,
    capture: Option<AudioCapture>,
    capture_error: Option<String>,
    follow_error: Option<String>,
    follow_default: Option<SourceKind>,
    last_default_check: Instant,
    devices: Vec<AudioDevice>,
    device_error: Option<String>,
    analyzer: Analyzer,
    last_analyzed_callback_sequence: u64,
    features: Features,
    visual_history: VisualHistory,
    latency: LatencyStats,
    frame_stats: FrameStats,
    default_switch_timing: Option<DefaultSwitchTiming>,
    pending_default_switch: Option<Instant>,
    visual: usize,
    overlay: bool,
    presentation: PresentationMode,
    gain: f32,
    started: Instant,
    presets: Vec<Preset>,
    preset_directory: PathBuf,
    preset_error: Option<String>,
    gpu_preset: Option<GpuPresetRenderer>,
    plugins: Vec<PluginPackage>,
    plugin_directory: PathBuf,
    selected_plugin: usize,
    loaded_plugin: Option<LoadedPlugin>,
    plugin_error: Option<String>,
    plugin_multiplier: f32,
    last_plugin_frame: Instant,
}

impl VisualizerApp {
    fn new(creation: &eframe::CreationContext<'_>) -> Self {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = Color32::from_rgb(4, 6, 12);
        visuals.window_fill = Color32::from_rgba_unmultiplied(8, 12, 24, 235);
        visuals.selection.bg_fill = Color32::from_rgb(0, 220, 190);
        creation.egui_ctx.set_visuals(visuals);
        let preset_directory = preset_directory();
        let discovery = preset::discover(&preset_directory);
        let mut preset_error = (!discovery.errors.is_empty()).then(|| discovery.errors.join("\n"));
        let gpu_preset = discovery.presets.first().and_then(|preset| {
            match GpuPresetRenderer::install(creation, preset) {
                Ok(renderer) => renderer,
                Err(error) => {
                    preset_error = Some(error);
                    None
                }
            }
        });
        let gain = discovery
            .presets
            .first()
            .map_or(2.2, |preset| preset.response.default);
        let plugin_directory = plugin_directory();
        let plugin_discovery = plugin::discover(&plugin_directory);
        let plugin_error =
            (!plugin_discovery.errors.is_empty()).then(|| plugin_discovery.errors.join("\n"));
        let started = Instant::now();

        let samples = Arc::new(Mutex::new(SampleBuffer::default()));
        let (capture, capture_error) =
            match AudioCapture::start(SourceKind::System, Arc::clone(&samples)) {
                Ok(capture) => (Some(capture), None),
                Err(system_error) => {
                    match AudioCapture::start(SourceKind::Microphone, Arc::clone(&samples)) {
                        Ok(capture) => (
                            Some(capture),
                            Some(format!("System audio unavailable: {system_error}")),
                        ),
                        Err(microphone_error) => {
                            (None, Some(format!("{system_error}; {microphone_error}")))
                        }
                    }
                }
            };

        let follow_default = capture
            .as_ref()
            .map_or(Some(SourceKind::System), |capture| Some(capture.source));
        let mut app = Self {
            samples,
            capture,
            capture_error,
            follow_error: None,
            follow_default,
            last_default_check: Instant::now(),
            devices: Vec::new(),
            device_error: None,
            analyzer: Analyzer::new(),
            last_analyzed_callback_sequence: 0,
            features: Features::default(),
            visual_history: VisualHistory::default(),
            latency: LatencyStats::default(),
            frame_stats: FrameStats::default(),
            default_switch_timing: None,
            pending_default_switch: None,
            visual: 0,
            overlay: true,
            presentation: PresentationMode::Windowed,
            gain,
            started,
            presets: discovery.presets,
            preset_directory,
            preset_error,
            gpu_preset,
            plugins: plugin_discovery.packages,
            plugin_directory,
            selected_plugin: 0,
            loaded_plugin: None,
            plugin_error,
            plugin_multiplier: 1.0,
            last_plugin_frame: started,
        };
        app.refresh_devices();
        app
    }

    fn replace_capture(&mut self, result: Result<AudioCapture, String>) -> bool {
        match result {
            Ok(capture) => {
                if let Ok(mut samples) = self.samples.lock() {
                    *samples = SampleBuffer::default();
                }
                self.latency = LatencyStats::default();
                self.visual_history = VisualHistory::default();
                self.last_analyzed_callback_sequence = 0;
                self.capture = Some(capture);
                self.capture_error = None;
                true
            }
            Err(error) => {
                self.capture_error = Some(error);
                false
            }
        }
    }

    fn switch_default_source(&mut self, source: SourceKind) {
        self.follow_default = Some(source);
        self.follow_error = None;
        self.default_switch_timing = None;
        self.pending_default_switch = None;
        let result = AudioCapture::start(source, Arc::clone(&self.samples));
        self.replace_capture(result);
        self.refresh_devices();
    }

    fn switch_device(&mut self, device: &AudioDevice) {
        self.follow_default = None;
        self.follow_error = None;
        self.default_switch_timing = None;
        self.pending_default_switch = None;
        let result = AudioCapture::start_selected(device, Arc::clone(&self.samples));
        self.replace_capture(result);
    }

    fn refresh_devices(&mut self) {
        let mut devices = Vec::new();
        let mut errors = Vec::new();
        for source in [SourceKind::System, SourceKind::Microphone] {
            match AudioDevice::enumerate(source) {
                Ok(found) => devices.extend(found),
                Err(error) => errors.push(error),
            }
        }
        self.devices = devices;
        self.device_error = (!errors.is_empty()).then(|| errors.join("; "));
    }

    fn update_default_device(&mut self) {
        if self.last_default_check.elapsed() < DEFAULT_DEVICE_CHECK_INTERVAL {
            return;
        }
        self.last_default_check = Instant::now();
        let Some(source) = self.follow_default else {
            return;
        };
        let default_id = match AudioDevice::default_id(source) {
            Ok(id) => id,
            Err(error) => {
                self.follow_error = Some(format!("Cannot follow Windows default: {error}"));
                return;
            }
        };
        let active_id = self
            .capture
            .as_ref()
            .filter(|capture| capture.source == source)
            .map(|capture| &capture.device_id);
        let capture_failed = self.capture.as_ref().is_none_or(|capture| {
            capture
                .error
                .try_lock()
                .ok()
                .is_some_and(|error| error.is_some())
        });
        if !default_needs_recovery(true, active_id, &default_id, capture_failed) {
            self.follow_error = None;
            return;
        }

        let reason = if active_id == Some(&default_id) {
            "Audio capture stopped"
        } else {
            "Windows default changed"
        };
        let switch_started = Instant::now();
        let result = AudioCapture::start(source, Arc::clone(&self.samples))
            .map_err(|error| format!("{reason}, but recovery failed: {error}"));
        let reopen_ms = switch_started.elapsed().as_secs_f64() * 1_000.0;
        if self.replace_capture(result) {
            self.default_switch_timing = Some(DefaultSwitchTiming {
                reopen_ms,
                first_callback_ms: None,
            });
            self.pending_default_switch = Some(switch_started);
            self.follow_error = None;
            self.refresh_devices();
        } else {
            self.follow_error.clone_from(&self.capture_error);
        }
    }

    fn update_features(&mut self) {
        let sample_rate = self
            .capture
            .as_ref()
            .map_or(48_000, |capture| capture.sample_rate);
        let snapshot = match self.samples.try_lock() {
            Ok(samples) => samples.snapshot(FFT_SIZE),
            Err(_) => return,
        };
        if !callback_is_new(
            &mut self.last_analyzed_callback_sequence,
            snapshot.callback_sequence,
        ) {
            return;
        }
        self.features = self.analyzer.analyze(&snapshot.samples, sample_rate);
        self.visual_history
            .update(snapshot.callback_sequence, &self.features);
        if let Some(timing) = &mut self.default_switch_timing {
            timing.observe_first_callback(
                &mut self.pending_default_switch,
                snapshot.callback_sequence,
                Instant::now(),
            );
        }
        if let Some(age) = snapshot.newest_sample_age(Instant::now()) {
            self.latency.observe(snapshot.callback_sequence, age);
        }
    }

    fn load_preset(&mut self, index: usize) {
        let Some(preset) = self.presets.get(index).cloned() else {
            return;
        };
        let Some(renderer) = &mut self.gpu_preset else {
            self.preset_error = Some("GPU preset renderer is unavailable".to_owned());
            return;
        };
        match renderer.load(&preset) {
            Ok(()) => {
                self.gain = preset.response.default;
                self.preset_error = None;
            }
            Err(error) => self.preset_error = Some(error),
        }
    }

    fn refresh_presets(&mut self) {
        let active_id = self
            .gpu_preset
            .as_ref()
            .map(|renderer| renderer.active_id().to_owned());
        let discovery = preset::discover(&self.preset_directory);
        self.presets = discovery.presets;
        let discovery_error = (!discovery.errors.is_empty()).then(|| discovery.errors.join("\n"));
        self.preset_error = None;
        if let Some(index) = active_id
            .as_ref()
            .and_then(|id| self.presets.iter().position(|preset| &preset.id == id))
        {
            self.load_preset(index);
        }
        self.preset_error = match (discovery_error, self.preset_error.take()) {
            (Some(discovery), Some(load)) => Some(format!("{discovery}\n{load}")),
            (Some(error), None) | (None, Some(error)) => Some(error),
            (None, None) => None,
        };
    }

    fn cycle_preset(&mut self, forward: bool) {
        if self.presets.len() < 2 {
            return;
        }
        let current = self
            .gpu_preset
            .as_ref()
            .and_then(|renderer| {
                self.presets
                    .iter()
                    .position(|preset| preset.id == renderer.active_id())
            })
            .unwrap_or(0);
        let next = if forward {
            (current + 1) % self.presets.len()
        } else {
            (current + self.presets.len() - 1) % self.presets.len()
        };
        self.load_preset(next);
    }

    fn refresh_plugins(&mut self) {
        self.loaded_plugin = None;
        self.plugin_multiplier = 1.0;
        let discovery = plugin::discover(&self.plugin_directory);
        self.plugins = discovery.packages;
        self.selected_plugin = self
            .selected_plugin
            .min(self.plugins.len().saturating_sub(1));
        self.plugin_error = (!discovery.errors.is_empty()).then(|| discovery.errors.join("\n"));
    }

    fn approve_plugin(&mut self) {
        let Some(package) = self.plugins.get(self.selected_plugin) else {
            return;
        };
        match LoadedPlugin::load(package) {
            Ok(plugin) => {
                self.loaded_plugin = Some(plugin);
                self.plugin_error = None;
            }
            Err(error) => {
                self.loaded_plugin = None;
                self.plugin_multiplier = 1.0;
                self.plugin_error = Some(error);
            }
        }
    }

    fn update_plugin(&mut self) {
        let now = Instant::now();
        let delta = now.duration_since(self.last_plugin_frame).as_secs_f32();
        self.last_plugin_frame = now;
        let Some(plugin) = &self.loaded_plugin else {
            self.plugin_multiplier = 1.0;
            return;
        };
        match plugin.process(&self.features, self.started.elapsed().as_secs_f32(), delta) {
            Ok(multiplier) => self.plugin_multiplier = multiplier,
            Err(error) => {
                self.loaded_plugin = None;
                self.plugin_multiplier = 1.0;
                self.plugin_error = Some(error);
            }
        }
    }

    fn set_presentation(&mut self, ctx: &egui::Context, mode: PresentationMode) {
        if mode == self.presentation {
            return;
        }
        match mode {
            PresentationMode::Windowed => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
            }
            PresentationMode::Borderless => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
            }
            PresentationMode::Fullscreen => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(true));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
            }
        }
        self.presentation = mode;
    }

    fn status(&self) -> (&'static str, Color32) {
        if self.capture.is_none() || self.stream_error().is_some() {
            return ("ERROR", Color32::from_rgb(255, 80, 110));
        }
        let last_callback = self
            .samples
            .try_lock()
            .ok()
            .and_then(|samples| samples.last_callback);
        match last_callback {
            None => ("WAITING", Color32::from_rgb(255, 190, 70)),
            Some(last) if last.elapsed() >= Duration::from_secs(2) || self.features.rms < 0.001 => {
                ("SILENT", Color32::from_rgb(120, 145, 175))
            }
            Some(_) => ("LIVE", Color32::from_rgb(40, 255, 190)),
        }
    }

    fn stream_error(&self) -> Option<String> {
        self.follow_error
            .clone()
            .or_else(|| {
                self.capture
                    .as_ref()
                    .and_then(|capture| capture.error.try_lock().ok()?.clone())
            })
            .or_else(|| self.capture_error.clone())
    }

    fn keyboard(&mut self, ctx: &egui::Context) {
        let (tab, left, right, up, down, borderless, f11, escape, microphone, system) =
            ctx.input(|input| {
                (
                    input.key_pressed(egui::Key::Tab),
                    input.key_pressed(egui::Key::ArrowLeft),
                    input.key_pressed(egui::Key::ArrowRight),
                    input.key_pressed(egui::Key::ArrowUp),
                    input.key_pressed(egui::Key::ArrowDown),
                    input.key_pressed(egui::Key::B),
                    input.key_pressed(egui::Key::F11),
                    input.key_pressed(egui::Key::Escape),
                    input.key_pressed(egui::Key::M),
                    input.key_pressed(egui::Key::S),
                )
            });
        if tab {
            self.overlay = !self.overlay;
        }
        if left {
            self.visual = (self.visual + VISUAL_NAMES.len() - 1) % VISUAL_NAMES.len();
        }
        if right {
            self.visual = (self.visual + 1) % VISUAL_NAMES.len();
        }
        if self.visual == 2 && up {
            self.cycle_preset(false);
        }
        if self.visual == 2 && down {
            self.cycle_preset(true);
        }
        if microphone {
            self.switch_default_source(SourceKind::Microphone);
        }
        if system {
            self.switch_default_source(SourceKind::System);
        }
        if borderless {
            self.set_presentation(ctx, self.presentation.toggle_borderless());
        }
        if f11 {
            self.set_presentation(ctx, self.presentation.toggle_fullscreen());
        }
        if escape {
            if self.presentation != PresentationMode::Windowed {
                self.set_presentation(ctx, PresentationMode::Windowed);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn draw_visual(&self, painter: &egui::Painter, rect: Rect) {
        for strip in 0..24 {
            let t = strip as f32 / 23.0;
            let top = egui::lerp(rect.top()..=rect.bottom(), t);
            let bottom = egui::lerp(rect.top()..=rect.bottom(), (t + 1.0 / 23.0).min(1.0));
            let color = Color32::from_rgb(
                (3.0 + 7.0 * t) as u8,
                (5.0 + 8.0 * t) as u8,
                (14.0 + 18.0 * t) as u8,
            );
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(rect.left(), top), Pos2::new(rect.right(), bottom)),
                0.0,
                color,
            );
        }

        match self.visual {
            0 => self.draw_scope(painter, rect),
            1 => self.draw_particles(painter, rect),
            _ => {
                if let Some(renderer) = &self.gpu_preset {
                    renderer.paint(
                        painter,
                        rect,
                        PresetFrame {
                            time: self.started.elapsed().as_secs_f32(),
                            delta: (self.frame_stats.current_ms as f32 / 1_000.0).min(0.25),
                            gain: self.gain * self.plugin_multiplier,
                            waveform: &self.features.waveform,
                            spectrum: &self.features.spectrum,
                            low: self.features.low,
                            mid: self.features.mid,
                            high: self.features.high,
                            rms: self.features.rms,
                            peak: self.features.peak,
                        },
                    );
                } else {
                    self.draw_tunnel(painter, rect);
                }
            }
        }
    }

    fn draw_scope(&self, painter: &egui::Painter, rect: Rect) {
        let center = rect.center();
        for grid in 1..8 {
            let x = egui::lerp(rect.left()..=rect.right(), grid as f32 / 8.0);
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(1.0, Color32::from_rgba_unmultiplied(20, 120, 145, 25)),
            );
        }
        painter.line_segment(
            [
                Pos2::new(rect.left(), center.y),
                Pos2::new(rect.right(), center.y),
            ],
            Stroke::new(1.0, Color32::from_rgba_unmultiplied(30, 230, 210, 65)),
        );

        let amplitude = rect.height() * 0.32 * self.gain;
        let history = if self.visual_history.waveforms.is_empty() {
            vec![self.features.waveform.as_slice()]
        } else {
            self.visual_history
                .waveforms
                .iter()
                .map(Vec::as_slice)
                .collect()
        };
        for (trail, waveform) in history.iter().enumerate() {
            let age = (trail + 1) as f32 / history.len() as f32;
            let points = waveform_points(waveform, rect, center.y, amplitude);
            let color = Color32::from_rgba_unmultiplied(
                (125.0 - age * 90.0) as u8,
                (85.0 + age * 170.0) as u8,
                245,
                (12.0 + age * 105.0) as u8,
            );
            painter.add(egui::Shape::line(
                points,
                Stroke::new(0.7 + age * 1.2, color),
            ));
        }
        let points = waveform_points(
            history
                .last()
                .copied()
                .unwrap_or(self.features.waveform.as_slice()),
            rect,
            center.y,
            amplitude,
        );
        painter.add(egui::Shape::line(
            points.clone(),
            Stroke::new(13.0, Color32::from_rgba_unmultiplied(0, 255, 220, 18)),
        ));
        painter.add(egui::Shape::line(
            points,
            Stroke::new(2.0, Color32::from_rgb(215, 255, 250)),
        ));
    }

    fn draw_particles(&self, painter: &egui::Painter, rect: Rect) {
        let time = self.started.elapsed().as_secs_f32();
        let center = rect.center() + Vec2::new(0.0, rect.height() * 0.035);
        let scale = rect.size().min_elem();
        let spectra = if self.visual_history.spectra.is_empty() {
            vec![self.features.spectrum.as_slice()]
        } else {
            self.visual_history
                .spectra
                .iter()
                .map(Vec::as_slice)
                .collect()
        };
        let mut current_points = Vec::new();
        for (trail, spectrum) in spectra.iter().enumerate() {
            let age = (trail + 1) as f32 / spectra.len() as f32;
            let time_offset = (1.0 - age) * 0.22;
            for (index, energy) in spectrum.iter().enumerate() {
                let frequency = index as f32 / (spectrum.len() - 1) as f32;
                let value = (energy * self.gain).clamp(0.0, 1.0);
                let angle = std::f32::consts::TAU * frequency - std::f32::consts::FRAC_PI_2
                    + time * (0.035 + value * 0.12)
                    - time_offset;
                let radius = scale * (0.15 + frequency.powf(0.72) * 0.27 + value * 0.31);
                let point = center + Vec2::new(angle.cos() * radius, angle.sin() * radius * 0.78);
                let color = spectrum_color(frequency).gamma_multiply(0.12 + age * 0.7);
                painter.circle_filled(point, 0.7 + age * (1.0 + value * 3.8), color);
                if trail + 1 == spectra.len() {
                    current_points.push((point, color, value));
                }
            }
        }
        for index in 0..current_points.len() {
            let (point, color, value) = current_points[index];
            let next = current_points[(index + 1) % current_points.len()].0;
            painter.line_segment(
                [point, next],
                Stroke::new(0.6 + value * 1.1, color.gamma_multiply(0.32)),
            );
            if index % 4 == 0 {
                painter.line_segment(
                    [center, point],
                    Stroke::new(0.7, color.gamma_multiply(0.13 + value * 0.12)),
                );
            }
        }
        let core = scale * (0.035 + self.features.low * self.gain * 0.055);
        painter.circle_filled(
            center,
            core * 1.8,
            Color32::from_rgba_unmultiplied(90, 60, 255, 18),
        );
        painter.circle_stroke(
            center,
            core,
            Stroke::new(
                1.5 + self.features.peak * 3.0,
                Color32::from_rgb(100, 255, 225),
            ),
        );
    }

    fn draw_tunnel(&self, painter: &egui::Painter, rect: Rect) {
        let time = self.started.elapsed().as_secs_f32();
        let center = rect.center()
            + Vec2::new((time * 0.37).sin(), (time * 0.29).cos())
                * rect.size().min_elem()
                * 0.08
                * (0.4 + self.features.mid);
        let max_radius = rect.size().min_elem() * 0.72;

        for ring in (0..22).rev() {
            let phase =
                ((ring as f32 / 22.0 + time * (0.08 + self.features.low * 0.15)) % 1.0).powf(1.7);
            let radius = 18.0 + phase * max_radius * (1.0 + self.features.low * 0.16);
            let sides = 8;
            let rotation = time * 0.22 + ring as f32 * 0.11 + self.features.high * 0.6;
            let points: Vec<Pos2> = (0..=sides)
                .map(|side| {
                    let angle = std::f32::consts::TAU * side as f32 / sides as f32 + rotation;
                    center + Vec2::angled(angle) * radius
                })
                .collect();
            let alpha = (35.0 + 190.0 * phase) as u8;
            let color = if ring % 2 == 0 {
                Color32::from_rgba_unmultiplied(0, 245, 210, alpha)
            } else {
                Color32::from_rgba_unmultiplied(135, 70, 255, alpha)
            };
            painter.add(egui::Shape::line(
                points,
                Stroke::new(1.0 + phase * 3.0, color),
            ));
        }
    }

    fn draw_overlay(&mut self, ctx: &egui::Context) {
        let (status, status_color) = self.status();
        egui::Area::new(egui::Id::new("player-overlay"))
            .fixed_pos([24.0, 22.0])
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::from_rgba_unmultiplied(5, 10, 20, 225))
                    .stroke(Stroke::new(
                        1.0,
                        Color32::from_rgba_unmultiplied(70, 240, 220, 70),
                    ))
                    .corner_radius(12)
                    .inner_margin(14)
                    .show(ui, |ui| {
                        ui.set_width(430.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new("THE VISUALIZER")
                                    .strong()
                                    .size(19.0)
                                    .color(Color32::from_rgb(220, 255, 250)),
                            );
                            ui.add_space(8.0);
                            ui.label(egui::RichText::new(status).strong().color(status_color));
                        });
                        ui.label(
                            egui::RichText::new(VISUAL_NAMES[self.visual])
                                .monospace()
                                .color(Color32::from_rgb(90, 245, 220)),
                        );
                        ui.add_space(8.0);

                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(
                                    self.capture.as_ref().is_some_and(|capture| {
                                        capture.source == SourceKind::System
                                    }),
                                    "System [S]",
                                )
                                .clicked()
                            {
                                self.switch_default_source(SourceKind::System);
                            }
                            if ui
                                .selectable_label(
                                    self.capture.as_ref().is_some_and(|capture| {
                                        capture.source == SourceKind::Microphone
                                    }),
                                    "Microphone [M]",
                                )
                                .clicked()
                            {
                                self.switch_default_source(SourceKind::Microphone);
                            }
                        });
                        let active_source = self
                            .capture
                            .as_ref()
                            .map_or(SourceKind::System, |capture| capture.source);
                        let selected_id = self
                            .capture
                            .as_ref()
                            .map(|capture| capture.device_id.clone());
                        let selected_text = self.capture.as_ref().map_or_else(
                            || "No active device".to_owned(),
                            |capture| capture.device_name.clone(),
                        );
                        let mut chosen = None;
                        let mut refresh = false;
                        ui.horizontal(|ui| {
                            ui.label("Device");
                            egui::ComboBox::from_id_salt("audio-device")
                                .width(285.0)
                                .selected_text(selected_text)
                                .show_ui(ui, |ui| {
                                    for device in self
                                        .devices
                                        .iter()
                                        .filter(|device| device.source == active_source)
                                    {
                                        if ui
                                            .selectable_label(
                                                selected_id.as_ref() == Some(&device.id),
                                                device.label(),
                                            )
                                            .clicked()
                                        {
                                            chosen = Some(device.clone());
                                        }
                                    }
                                });
                            refresh = ui.button("Refresh").clicked();
                        });
                        if let Some(device) = chosen {
                            self.switch_device(&device);
                        }
                        if refresh {
                            self.refresh_devices();
                        }
                        if let Some(capture) = &self.capture {
                            let mode = if self.follow_default == Some(capture.source) {
                                "AUTO · follows Windows default"
                            } else {
                                "PINNED · manual device"
                            };
                            ui.label(
                                egui::RichText::new(mode)
                                    .small()
                                    .color(Color32::from_rgb(90, 205, 190)),
                            );
                        }
                        if let Some(error) = &self.preset_error {
                            ui.label(
                                egui::RichText::new(error)
                                    .small()
                                    .color(Color32::from_rgb(255, 190, 90)),
                            );
                        }
                        if let Some(error) = self.stream_error() {
                            ui.label(
                                egui::RichText::new(error)
                                    .small()
                                    .color(Color32::from_rgb(255, 120, 135)),
                            );
                        }
                        if let Some(error) = &self.device_error {
                            ui.label(
                                egui::RichText::new(error)
                                    .small()
                                    .color(Color32::from_rgb(255, 190, 90)),
                            );
                        }

                        if self.visual == 2 {
                            let active_id = self
                                .gpu_preset
                                .as_ref()
                                .map(|renderer| renderer.active_id().to_owned());
                            let selected_text = active_id
                                .as_ref()
                                .and_then(|id| self.presets.iter().find(|preset| &preset.id == id))
                                .map_or_else(
                                    || {
                                        active_id
                                            .clone()
                                            .unwrap_or_else(|| "No valid preset".to_owned())
                                    },
                                    |preset| preset.name.clone(),
                                );
                            let mut selected = None;
                            let mut refresh = false;
                            ui.horizontal(|ui| {
                                ui.label("Preset");
                                egui::ComboBox::from_id_salt("gpu-preset")
                                    .width(270.0)
                                    .selected_text(selected_text)
                                    .show_ui(ui, |ui| {
                                        for (index, preset) in self.presets.iter().enumerate() {
                                            if ui
                                                .selectable_label(
                                                    active_id.as_ref() == Some(&preset.id),
                                                    &preset.name,
                                                )
                                                .on_hover_text(format!(
                                                    "{} · {} · {} · {}",
                                                    preset.version,
                                                    preset.author,
                                                    preset.license,
                                                    preset.path.display()
                                                ))
                                                .clicked()
                                            {
                                                selected = Some(index);
                                            }
                                        }
                                    });
                                refresh = ui.button("Refresh").clicked();
                            });
                            if let Some(index) = selected {
                                self.load_preset(index);
                            }
                            if refresh {
                                self.refresh_presets();
                            }
                        }
                        let response = self
                            .gpu_preset
                            .as_ref()
                            .and_then(|renderer| {
                                self.presets
                                    .iter()
                                    .find(|preset| preset.id == renderer.active_id())
                            })
                            .map(|preset| preset.response.clone());
                        let range = response
                            .map(|parameter| parameter.minimum..=parameter.maximum)
                            .unwrap_or(0.5..=6.0);
                        ui.add(egui::Slider::new(&mut self.gain, range).text("response"));
                        ui.horizontal(|ui| {
                            for (index, label) in VISUAL_BUTTONS.iter().enumerate() {
                                if ui
                                    .selectable_label(self.visual == index, *label)
                                    .on_hover_text(VISUAL_NAMES[index])
                                    .clicked()
                                {
                                    self.visual = index;
                                }
                            }
                            ui.separator();
                            ui.label(format!(
                                "RMS {:.3} · PK {:.3}",
                                self.features.rms, self.features.peak
                            ));
                        });
                        ui.horizontal(|ui| {
                            ui.label("View");
                            for mode in [
                                PresentationMode::Windowed,
                                PresentationMode::Borderless,
                                PresentationMode::Fullscreen,
                            ] {
                                if ui
                                    .selectable_label(self.presentation == mode, mode.label())
                                    .clicked()
                                {
                                    self.set_presentation(ctx, mode);
                                }
                            }
                        });
                        let details_label = if self.visual == 2 && !self.plugins.is_empty() {
                            "DETAILS & EXTENSIONS"
                        } else {
                            "DETAILS"
                        };
                        egui::CollapsingHeader::new(
                            egui::RichText::new(details_label)
                                .small()
                                .color(Color32::from_rgb(110, 150, 170)),
                        )
                        .id_salt("technical-details")
                        .default_open(false)
                        .show(ui, |ui| {
                            if let Some(capture) = &self.capture {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "CAPTURE · {} · {} Hz · {} ch",
                                        capture.device_name,
                                        capture.sample_rate,
                                        capture.channels
                                    ))
                                    .small()
                                    .color(Color32::from_rgb(145, 165, 185)),
                                );
                            }
                            if let Some(renderer) = &self.gpu_preset {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "GPU · {}",
                                        renderer.adapter_name()
                                    ))
                                    .small()
                                    .color(Color32::from_rgb(115, 145, 175)),
                                );
                            }
                            if self.visual == 2 {
                                self.draw_plugin_controls(ui);
                            }
                            if self.latency.observations > 0 {
                                let fft_window_ms = FFT_SIZE as f64 * 1_000.0
                                    / self.capture.as_ref().map_or(48_000.0, |capture| {
                                        f64::from(capture.sample_rate)
                                    });
                                ui.label(
                                    egui::RichText::new(format!(
                                        "PIPE ~{:.1} ms · AVG {:.1} · PEAK {:.1} · FFT {:.1} ms",
                                        self.latency.current_ms,
                                        self.latency.average_ms,
                                        self.latency.peak_ms,
                                        fft_window_ms
                                    ))
                                    .small()
                                    .monospace()
                                    .color(Color32::from_rgb(110, 180, 175)),
                                )
                                .on_hover_text(
                                    "Estimated newest captured sample → feature update. Includes the \
                                     audio backend and UI handoff; excludes upstream playback buffering, \
                                     compositor/display scanout, and the separate FFT window duration.",
                                );
                            }
                            if self.frame_stats.observations > 0 {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "FRAME ~{:.0} FPS · {:.1} ms",
                                        self.frame_stats.fps(),
                                        self.frame_stats.smoothed_ms
                                    ))
                                    .small()
                                    .monospace()
                                    .color(Color32::from_rgb(145, 155, 205)),
                                )
                                .on_hover_text(
                                    "Smoothed interval between application UI frames. This measures \
                                     application cadence, not monitor refresh, display scanout, or \
                                     playback-to-photon latency.",
                                );
                            }
                            if let Some(timing) = &self.default_switch_timing {
                                let first_callback = timing.first_callback_ms.map_or_else(
                                    || "waiting".to_owned(),
                                    |milliseconds| format!("{milliseconds:.1} ms"),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "DEFAULT DETECT → OPEN {:.1} ms · FIRST PACKET {}",
                                        timing.reopen_ms, first_callback
                                    ))
                                    .small()
                                    .monospace()
                                    .color(Color32::from_rgb(125, 190, 235)),
                                )
                                .on_hover_text(
                                    "Measured from the one-second default-device poll detecting a new \
                                     endpoint. First packet includes any time the endpoint remains \
                                     quiet; the unknown time before detection is 0–1000 ms.",
                                );
                            }
                        });
                        ui.label(
                            egui::RichText::new(
                                "←/→ visual · ↑/↓ preset · B borderless · F11 fullscreen · Tab hide · Esc back/exit",
                            )
                            .small()
                            .color(Color32::from_rgb(110, 130, 150)),
                        );
                    });
            });
    }

    fn draw_plugin_controls(&mut self, ui: &mut egui::Ui) {
        let selected_text = self
            .plugins
            .get(self.selected_plugin)
            .map_or("No plugin package", |package| package.name.as_str());
        let mut selected = None;
        let mut refresh = false;
        ui.horizontal(|ui| {
            ui.label("Plugin");
            egui::ComboBox::from_id_salt("native-plugin")
                .width(270.0)
                .selected_text(selected_text)
                .show_ui(ui, |ui| {
                    for (index, package) in self.plugins.iter().enumerate() {
                        if ui
                            .selectable_label(self.selected_plugin == index, &package.name)
                            .clicked()
                        {
                            selected = Some(index);
                        }
                    }
                });
            refresh = ui.button("Refresh").clicked();
        });
        if let Some(index) = selected {
            self.selected_plugin = index;
        }
        if refresh {
            self.refresh_plugins();
        }
        if let Some(error) = &self.plugin_error {
            ui.label(
                egui::RichText::new(error)
                    .small()
                    .color(Color32::from_rgb(255, 190, 90)),
            );
        }

        let Some(package) = self.plugins.get(self.selected_plugin).cloned() else {
            return;
        };
        let is_loaded = self.loaded_plugin.as_ref().is_some_and(|plugin| {
            plugin.package_id() == package.id && plugin.artifact_hash() == package.artifact_hash()
        });
        if is_loaded {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "ACTIVE · response ×{:.2}",
                        self.plugin_multiplier
                    ))
                    .strong()
                    .color(Color32::from_rgb(40, 255, 190)),
                );
                if ui.button("Unload").clicked() {
                    self.loaded_plugin = None;
                    self.plugin_multiplier = 1.0;
                }
            });
        } else {
            let hash = package.hash_hex();
            ui.label(
                egui::RichText::new(format!(
                    "DISABLED · {} · {} · {}",
                    package.version, package.author, package.license
                ))
                .small()
                .color(Color32::from_rgb(145, 165, 185)),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{}\nSHA-256 {}…",
                    package.display_library_path(),
                    &hash[..16]
                ))
                .small()
                .monospace()
                .color(Color32::from_rgb(115, 145, 175)),
            )
            .on_hover_text(format!("Full SHA-256 {hash}"));
            ui.label(
                egui::RichText::new(
                    "Native code runs with your user privileges and is not sandboxed. Approval lasts for this run.",
                )
                .small()
                .color(Color32::from_rgb(255, 165, 80)),
            );
            if ui
                .add(
                    egui::Button::new("Approve & Load")
                        .fill(Color32::from_rgb(115, 65, 25))
                        .stroke(Stroke::new(1.0, Color32::from_rgb(255, 175, 85))),
                )
                .clicked()
            {
                self.approve_plugin();
            }
        }
        ui.label(
            egui::RichText::new(format!("Manifest · {}", package.manifest_path.display()))
                .small()
                .color(Color32::from_rgb(90, 115, 140)),
        );
    }
}

impl eframe::App for VisualizerApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.frame_stats.observe(Instant::now());
        self.keyboard(ctx);
        self.update_default_device();
        self.update_features();
        self.update_plugin();
        ctx.request_repaint_after(Duration::from_millis(16));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let rect = ui.max_rect();
        self.draw_visual(&ui.painter_at(rect), rect);
        if self.overlay {
            self.draw_overlay(ui.ctx());
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.01, 0.015, 0.035, 1.0]
    }
}

fn waveform_points(waveform: &[f32], rect: Rect, center_y: f32, amplitude: f32) -> Vec<Pos2> {
    let denominator = waveform.len().saturating_sub(1).max(1) as f32;
    waveform
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let x = egui::lerp(
                (rect.left() + 24.0)..=(rect.right() - 24.0),
                index as f32 / denominator,
            );
            Pos2::new(x, center_y - sample * amplitude)
        })
        .collect()
}

fn spectrum_color(frequency: f32) -> Color32 {
    let (from, to, amount) = if frequency < 0.55 {
        ([25.0, 245.0, 210.0], [135.0, 85.0, 255.0], frequency / 0.55)
    } else {
        (
            [135.0, 85.0, 255.0],
            [255.0, 105.0, 115.0],
            (frequency - 0.55) / 0.45,
        )
    };
    Color32::from_rgb(
        egui::lerp(from[0]..=to[0], amount) as u8,
        egui::lerp(from[1]..=to[1], amount) as u8,
        egui::lerp(from[2]..=to[2], amount) as u8,
    )
}

fn default_needs_recovery<T: Eq>(
    follow: bool,
    active: Option<&T>,
    current_default: &T,
    capture_failed: bool,
) -> bool {
    follow && (capture_failed || active != Some(current_default))
}

fn callback_is_new(last: &mut u64, current: u64) -> bool {
    if current == 0 || current == *last {
        false
    } else {
        *last = current;
        true
    }
}

fn preset_directory() -> PathBuf {
    resource_directory("THEVISUALIZER_PRESETS", "presets")
}

fn plugin_directory() -> PathBuf {
    resource_directory("THEVISUALIZER_PLUGINS", "plugins")
}

fn resource_directory(environment: &str, folder: &str) -> PathBuf {
    if let Some(path) = std::env::var_os(environment) {
        return path.into();
    }
    let beside_executable = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|parent| parent.join(folder)));
    if let Some(path) = beside_executable.filter(|path| path.is_dir()) {
        path
    } else {
        PathBuf::from(folder)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DefaultSwitchTiming, Features, FrameStats, LatencyStats, PresentationMode, VisualHistory,
        callback_is_new, default_needs_recovery,
    };
    use std::time::{Duration, Instant};

    #[test]
    fn default_following_respects_pins_and_retries_failure() {
        assert!(!default_needs_recovery(false, Some(&"old"), &"new", true));
        assert!(!default_needs_recovery(true, Some(&"same"), &"same", false));
        assert!(default_needs_recovery(true, Some(&"same"), &"same", true));
        assert!(default_needs_recovery(true, Some(&"old"), &"new", false));
        assert!(default_needs_recovery(true, None, &"new", false));
    }

    #[test]
    fn analysis_runs_once_per_audio_callback() {
        let mut last = 0;
        assert!(!callback_is_new(&mut last, 0));
        assert!(callback_is_new(&mut last, 1));
        assert!(!callback_is_new(&mut last, 1));
        assert!(callback_is_new(&mut last, 2));
    }

    #[test]
    fn presentation_shortcuts_return_to_windowed() {
        assert_eq!(
            PresentationMode::Windowed.toggle_borderless(),
            PresentationMode::Borderless
        );
        assert_eq!(
            PresentationMode::Borderless.toggle_borderless(),
            PresentationMode::Windowed
        );
        assert_eq!(
            PresentationMode::Windowed.toggle_fullscreen(),
            PresentationMode::Fullscreen
        );
        assert_eq!(
            PresentationMode::Fullscreen.toggle_fullscreen(),
            PresentationMode::Windowed
        );
    }

    #[test]
    fn latency_stats_count_each_callback_once() {
        let mut stats = LatencyStats::default();
        stats.observe(1, Duration::from_millis(10));
        stats.observe(1, Duration::from_millis(100));
        stats.observe(2, Duration::from_millis(30));

        assert_eq!(stats.observations, 2);
        assert_eq!(stats.current_ms, 30.0);
        assert_eq!(stats.average_ms, 20.0);
        assert_eq!(stats.peak_ms, 30.0);
    }

    #[test]
    fn frame_stats_smooth_ui_cadence() {
        let started = Instant::now();
        let mut stats = FrameStats::default();
        stats.observe(started);
        stats.observe(started + Duration::from_millis(10));
        stats.observe(started + Duration::from_millis(30));

        assert_eq!(stats.observations, 2);
        assert_eq!(stats.current_ms, 20.0);
        assert_eq!(stats.smoothed_ms, 11.0);
        assert!((stats.fps() - 90.9).abs() < 0.1);
    }

    #[test]
    fn visual_history_is_bounded_and_smoothes_falloff() {
        let mut history = VisualHistory::default();
        let mut features = Features::default();
        features.spectrum.fill(0.8);
        history.update(1, &features);
        assert_eq!(history.smoothed_spectrum[0], 0.8);

        features.spectrum.fill(0.0);
        history.update(2, &features);
        assert!(history.smoothed_spectrum[0] > 0.0);
        assert!(history.smoothed_spectrum[0] < 0.8);

        for sequence in 3..20 {
            history.update(sequence, &features);
        }
        assert_eq!(history.waveforms.len(), super::VISUAL_TRAIL_FRAMES);
        assert_eq!(history.spectra.len(), super::VISUAL_TRAIL_FRAMES);
    }

    #[test]
    fn default_switch_timing_finishes_on_first_callback() {
        let started = Instant::now();
        let mut pending = Some(started);
        let mut timing = DefaultSwitchTiming {
            reopen_ms: 4.0,
            first_callback_ms: None,
        };
        timing.observe_first_callback(&mut pending, 0, started + Duration::from_millis(20));
        assert!(timing.first_callback_ms.is_none());
        timing.observe_first_callback(&mut pending, 1, started + Duration::from_millis(25));
        assert_eq!(timing.first_callback_ms, Some(25.0));
        assert!(pending.is_none());
    }
}
