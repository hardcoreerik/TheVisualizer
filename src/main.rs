mod analysis;
mod audio;
mod particle_forge;
mod plugin;
mod preset;
mod render;
mod scene;
mod studio;
mod visual_director;

use std::{
    collections::VecDeque,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use analysis::{Analyzer, FFT_SIZE, Features};
use audio::{AudioCapture, AudioDevice, SampleBuffer, SharedSamples, SourceKind};
use eframe::egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use eframe::egui_wgpu::RenderState;
use particle_forge::{
    ForgeForceKind, ForgeFrame, ForgeQuality, ForgeSceneSource, ForgeTopology,
    ParticleForgeRenderer, ParticleForgeState,
};
use plugin::{LoadedPlugin, PluginPackage};
use preset::Preset;
use render::{
    GpuPresetRenderer, PRESET_HISTORY_ROWS, PRESET_PARAMETER_FLOATS, PRESET_SCENE_FLOATS,
    PresetFrame,
};
use scene::{SavedScene, SceneSnapshot, SceneZone};
use studio::{
    MAX_STUDIO_LAYERS, StudioBand, StudioBlend, StudioLayer, StudioLayerKind, StudioState,
};
use visual_director::{Aspect, CaptureProfile, OutputIntent, VisualDirector};

const VISUAL_NAMES: [&str; 4] = ["NEON SCOPE", "PARTICLE FORGE", "GPU PRESET", "STUDIO"];
const VISUAL_BUTTONS: [&str; 4] = ["1 Scope", "2 Particles", "3 Preset", "4 Studio"];
const DEFAULT_DEVICE_CHECK_INTERVAL: Duration = Duration::from_secs(1);
const VISUAL_TRAIL_FRAMES: usize = 10;
const MAX_SOUND_ZONES: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq)]
enum ZoneBand {
    Full,
    Low,
    Mid,
    High,
}

impl ZoneBand {
    fn label(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Low => "Low",
            Self::Mid => "Mid",
            Self::High => "High",
        }
    }

    fn energy(self, features: &Features) -> f32 {
        match self {
            Self::Full => features.rms * 3.0,
            Self::Low => features.low,
            Self::Mid => features.mid,
            Self::High => features.high,
        }
        .clamp(0.0, 1.0)
    }

    fn code(self) -> u8 {
        match self {
            Self::Full => 0,
            Self::Low => 1,
            Self::Mid => 2,
            Self::High => 3,
        }
    }

    fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Low,
            2 => Self::Mid,
            3 => Self::High,
            _ => Self::Full,
        }
    }

    fn next(self) -> Self {
        match self {
            Self::Full => Self::Low,
            Self::Low => Self::Mid,
            Self::Mid => Self::High,
            Self::High => Self::Full,
        }
    }
}

#[derive(Clone)]
struct SoundZone {
    position: Vec2,
    radius: f32,
    strength: f32,
    band: ZoneBand,
    pinned: bool,
}

impl SoundZone {
    fn new(position: Vec2) -> Self {
        Self {
            position,
            radius: 0.16,
            strength: 1.0,
            band: ZoneBand::Full,
            pinned: false,
        }
    }
}

struct InteractionState {
    zones: Vec<SoundZone>,
    selected: Option<usize>,
    drag_origin: Option<Vec2>,
    camera_drag_origin: Option<Vec2>,
    secondary_drag_origin: Option<Vec2>,
    camera_yaw: f32,
    camera_pitch: f32,
    camera_zoom: f32,
    show_handles: bool,
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            zones: vec![SoundZone::new(Vec2::new(0.5, 0.5))],
            selected: Some(0),
            drag_origin: None,
            camera_drag_origin: None,
            secondary_drag_origin: None,
            camera_yaw: 0.0,
            camera_pitch: 0.05,
            camera_zoom: 1.0,
            show_handles: true,
        }
    }
}

impl InteractionState {
    fn cityscape() -> Self {
        Self {
            zones: vec![
                SoundZone {
                    position: Vec2::new(0.3, 0.48),
                    radius: 0.18,
                    strength: 1.35,
                    band: ZoneBand::Low,
                    pinned: true,
                },
                SoundZone {
                    position: Vec2::new(0.5, 0.76),
                    radius: 0.2,
                    strength: 1.2,
                    band: ZoneBand::Mid,
                    pinned: true,
                },
                SoundZone {
                    position: Vec2::new(0.62, 0.32),
                    radius: 0.16,
                    strength: 1.25,
                    band: ZoneBand::High,
                    pinned: true,
                },
                SoundZone {
                    position: Vec2::new(0.78, 0.2),
                    radius: 0.22,
                    strength: 1.0,
                    band: ZoneBand::Full,
                    pinned: true,
                },
            ],
            selected: Some(3),
            drag_origin: None,
            camera_drag_origin: None,
            secondary_drag_origin: None,
            camera_yaw: 0.0,
            camera_pitch: 0.05,
            camera_zoom: 1.0,
            show_handles: true,
        }
    }

    fn add_zone(&mut self, position: Vec2) {
        if self.zones.len() == MAX_SOUND_ZONES {
            return;
        }
        self.zones.push(SoundZone::new(position));
        self.selected = Some(self.zones.len() - 1);
    }

    fn remove_selected(&mut self) {
        let Some(index) = self.selected else {
            return;
        };
        if self.zones.get(index).is_some_and(|zone| zone.pinned) {
            return;
        }
        self.zones.remove(index);
        self.selected = (!self.zones.is_empty()).then(|| index.min(self.zones.len() - 1));
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PalettePreset {
    CyberNeon,
    Earth,
    Arctic,
    Inferno,
    Acid,
    Vaporwave,
    Monochrome,
    Custom,
}

impl PalettePreset {
    const ALL: [Self; 8] = [
        Self::CyberNeon,
        Self::Earth,
        Self::Arctic,
        Self::Inferno,
        Self::Acid,
        Self::Vaporwave,
        Self::Monochrome,
        Self::Custom,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::CyberNeon => "Cyber Neon",
            Self::Earth => "Earth Tones",
            Self::Arctic => "Arctic Glass",
            Self::Inferno => "Inferno",
            Self::Acid => "Acid",
            Self::Vaporwave => "Vaporwave",
            Self::Monochrome => "Monochrome",
            Self::Custom => "Custom",
        }
    }

    fn code(self) -> u8 {
        self as u8
    }

    fn from_code(code: u8) -> Self {
        Self::ALL[usize::from(code.min(Self::ALL.len() as u8 - 1))]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SurfaceFinish {
    Neon,
    Glossy,
    Matte,
    Metallic,
    Glass,
}

impl SurfaceFinish {
    const ALL: [Self; 5] = [
        Self::Neon,
        Self::Glossy,
        Self::Matte,
        Self::Metallic,
        Self::Glass,
    ];

    fn label(self) -> &'static str {
        match self {
            Self::Neon => "Neon",
            Self::Glossy => "Glossy",
            Self::Matte => "Matte",
            Self::Metallic => "Metallic",
            Self::Glass => "Glass",
        }
    }

    fn code(self) -> u8 {
        self as u8
    }

    fn from_code(code: u8) -> Self {
        Self::ALL[usize::from(code.min(Self::ALL.len() as u8 - 1))]
    }
}

struct ColorSystem {
    palette: PalettePreset,
    finish: SurfaceFinish,
    full: Color32,
    bass: Color32,
    mid: Color32,
    treble: Color32,
    background: Color32,
    glow: f32,
    gloss: f32,
    saturation: f32,
}

impl Default for ColorSystem {
    fn default() -> Self {
        let mut colors = Self {
            palette: PalettePreset::CyberNeon,
            finish: SurfaceFinish::Neon,
            full: Color32::WHITE,
            bass: Color32::WHITE,
            mid: Color32::WHITE,
            treble: Color32::WHITE,
            background: Color32::BLACK,
            glow: 1.0,
            gloss: 0.7,
            saturation: 1.0,
        };
        colors.apply_palette(PalettePreset::CyberNeon);
        colors
    }
}

impl ColorSystem {
    fn apply_palette(&mut self, palette: PalettePreset) {
        self.palette = palette;
        let colors = match palette {
            PalettePreset::CyberNeon => [
                Color32::from_rgb(225, 245, 255),
                Color32::from_rgb(255, 45, 105),
                Color32::from_rgb(0, 245, 190),
                Color32::from_rgb(65, 120, 255),
                Color32::from_rgb(2, 5, 18),
            ],
            PalettePreset::Earth => [
                Color32::from_rgb(235, 220, 175),
                Color32::from_rgb(155, 55, 30),
                Color32::from_rgb(90, 145, 70),
                Color32::from_rgb(215, 165, 75),
                Color32::from_rgb(20, 16, 12),
            ],
            PalettePreset::Arctic => [
                Color32::from_rgb(235, 255, 255),
                Color32::from_rgb(40, 145, 210),
                Color32::from_rgb(105, 245, 235),
                Color32::from_rgb(175, 205, 255),
                Color32::from_rgb(3, 15, 25),
            ],
            PalettePreset::Inferno => [
                Color32::from_rgb(255, 235, 175),
                Color32::from_rgb(255, 35, 15),
                Color32::from_rgb(255, 115, 15),
                Color32::from_rgb(255, 220, 75),
                Color32::from_rgb(18, 2, 1),
            ],
            PalettePreset::Acid => [
                Color32::from_rgb(235, 255, 120),
                Color32::from_rgb(180, 255, 0),
                Color32::from_rgb(15, 255, 105),
                Color32::from_rgb(210, 35, 255),
                Color32::from_rgb(5, 10, 2),
            ],
            PalettePreset::Vaporwave => [
                Color32::from_rgb(255, 225, 255),
                Color32::from_rgb(255, 75, 185),
                Color32::from_rgb(130, 95, 255),
                Color32::from_rgb(40, 225, 255),
                Color32::from_rgb(16, 5, 35),
            ],
            PalettePreset::Monochrome => [
                Color32::from_rgb(245, 245, 245),
                Color32::from_rgb(205, 205, 205),
                Color32::from_rgb(150, 150, 150),
                Color32::from_rgb(235, 235, 235),
                Color32::from_rgb(6, 6, 8),
            ],
            PalettePreset::Custom => return,
        };
        [self.full, self.bass, self.mid, self.treble, self.background] = colors;
    }

    fn band_color(&self, band: ZoneBand) -> Color32 {
        match band {
            ZoneBand::Full => self.full,
            ZoneBand::Low => self.bass,
            ZoneBand::Mid => self.mid,
            ZoneBand::High => self.treble,
        }
    }

    fn spectrum_color(&self, frequency: f32) -> Color32 {
        let (from, to, amount) = if frequency < 0.55 {
            (self.bass, self.mid, frequency / 0.55)
        } else {
            (self.mid, self.treble, (frequency - 0.55) / 0.45)
        };
        Color32::from_rgb(
            egui::lerp(from.r() as f32..=to.r() as f32, amount) as u8,
            egui::lerp(from.g() as f32..=to.g() as f32, amount) as u8,
            egui::lerp(from.b() as f32..=to.b() as f32, amount) as u8,
        )
    }
}

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
struct OnsetStats {
    started: Option<Instant>,
    count: u64,
    last: Option<Instant>,
}

impl OnsetStats {
    fn observe(&mut self, onset: f32, now: Instant) {
        self.started.get_or_insert(now);
        if onset > 0.5 {
            self.count = self.count.saturating_add(1);
            self.last = Some(now);
        }
    }

    fn per_minute(&self, now: Instant) -> f32 {
        self.started.map_or(0.0, |started| {
            self.count as f32 * 60.0
                / now
                    .saturating_duration_since(started)
                    .as_secs_f32()
                    .max(1.0)
        })
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
        }
        while self.spectra.len() > PRESET_HISTORY_ROWS {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameLimit {
    Display,
    Fps60,
    Fps30,
}

impl FrameLimit {
    fn label(self) -> &'static str {
        match self {
            Self::Display => "Display",
            Self::Fps60 => "60",
            Self::Fps30 => "30",
        }
    }

    fn interval(self) -> Option<Duration> {
        match self {
            Self::Display => None,
            Self::Fps60 => Some(Duration::from_secs_f64(1.0 / 60.0)),
            Self::Fps30 => Some(Duration::from_secs_f64(1.0 / 30.0)),
        }
    }
}

#[cfg(test)]
fn pacing_delay(limit: FrameLimit, elapsed: Duration) -> Option<Duration> {
    limit.interval()?.checked_sub(elapsed)
}

fn soft_ellipse(point: Pos2, center: Pos2, radius: Vec2) -> f32 {
    let distance =
        ((point.x - center.x) / radius.x).powi(2) + ((point.y - center.y) / radius.y).powi(2);
    (1.0 - distance).clamp(0.0, 1.0).powi(2)
}

fn living_photo_plant_mask(point: Pos2) -> f32 {
    soft_ellipse(point, Pos2::new(0.16, 0.56), Vec2::new(0.25, 0.28))
        .max(soft_ellipse(
            point,
            Pos2::new(0.89, 0.54),
            Vec2::new(0.24, 0.3),
        ))
        .max(soft_ellipse(
            point,
            Pos2::new(0.53, 0.43),
            Vec2::new(0.2, 0.18),
        ))
}

fn living_photo_glass_mask(point: Pos2) -> f32 {
    soft_ellipse(point, Pos2::new(0.69, 0.27), Vec2::new(0.3, 0.29)).max(soft_ellipse(
        point,
        Pos2::new(0.35, 0.27),
        Vec2::new(0.1, 0.28),
    ))
}

fn studio_image_position(rect: Rect, uv: Rect, source: Pos2) -> Pos2 {
    rect.min
        + Vec2::new(
            (source.x - uv.min.x) / uv.width(),
            (source.y - uv.min.y) / uv.height(),
        ) * rect.size()
}

fn hash01(value: f32) -> f32 {
    (value.sin() * 43_758.547).fract().abs()
}

fn rare_bird_progress(time: f32) -> Option<f32> {
    let phase = (time - 37.0).rem_euclid(53.0);
    (phase < 2.4).then_some(phase / 2.4)
}

fn visual_index_for_key(key: egui::Key) -> Option<usize> {
    match key {
        egui::Key::Num1 => Some(0),
        egui::Key::Num2 => Some(1),
        egui::Key::Num3 => Some(2),
        egui::Key::Num4 => Some(3),
        _ => None,
    }
}

fn main() -> eframe::Result {
    let icon =
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/thevisualizer-icon.png"))
            .expect("embedded application icon must be a valid PNG");
    let mut options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TheVisualizer")
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 500.0])
            .with_icon(icon),
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
    studio: StudioState,
    interaction: InteractionState,
    colors: ColorSystem,
    mode_parameters: [f32; PRESET_PARAMETER_FLOATS],
    latency: LatencyStats,
    onset_stats: OnsetStats,
    frame_stats: FrameStats,
    default_switch_timing: Option<DefaultSwitchTiming>,
    pending_default_switch: Option<Instant>,
    visual: usize,
    overlay: bool,
    show_text: bool,
    instrument_panel: bool,
    mode_panel: bool,
    forge_panel: bool,
    visual_director_panel: bool,
    presentation: PresentationMode,
    frame_limit: FrameLimit,
    last_paced_frame: Instant,
    gain: f32,
    started: Instant,
    presets: Vec<Preset>,
    preset_directory: PathBuf,
    preset_error: Option<String>,
    gpu_state: Option<RenderState>,
    particle_forge_renderer: Option<ParticleForgeRenderer>,
    particle_forge_error: Option<String>,
    particle_forge: ParticleForgeState,
    gpu_preset: Option<GpuPresetRenderer>,
    plugins: Vec<PluginPackage>,
    plugin_directory: PathBuf,
    selected_plugin: usize,
    loaded_plugin: Option<LoadedPlugin>,
    plugin_error: Option<String>,
    plugin_multiplier: f32,
    last_plugin_frame: Instant,
    cityscape_initialized: bool,
    visual_director: VisualDirector,
    scene_directory: PathBuf,
    scenes: Vec<SavedScene>,
    selected_scene: usize,
    scene_name: String,
    scene_notice: Option<String>,
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
        let mut preset_errors = discovery.errors;
        let gpu_state = creation.wgpu_render_state.clone();
        let (particle_forge_renderer, particle_forge_error) = gpu_state.as_ref().map_or_else(
            || {
                (
                    None,
                    Some("Particle Forge requires a wgpu render state.".to_owned()),
                )
            },
            |state| match ParticleForgeRenderer::install(state) {
                Ok(renderer) => (Some(renderer), None),
                Err(error) => (None, Some(error)),
            },
        );
        let mut gpu_preset = None;
        if let Some(state) = &gpu_state {
            for preset in &discovery.presets {
                match GpuPresetRenderer::install(state, preset) {
                    Ok(renderer) => {
                        gpu_preset = Some(renderer);
                        break;
                    }
                    Err(error) => preset_errors.push(error),
                }
            }
        }
        let preset_error = (!preset_errors.is_empty()).then(|| preset_errors.join("\n"));
        let active_preset = gpu_preset.as_ref().and_then(|renderer| {
            discovery
                .presets
                .iter()
                .find(|preset| preset.id == renderer.active_id())
        });
        let gain = active_preset.map_or(2.2, |preset| preset.response.default);
        let mode_parameters =
            active_preset.map_or([0.0; PRESET_PARAMETER_FLOATS], preset_parameter_defaults);
        let plugin_directory = plugin_directory();
        let plugin_discovery = plugin::discover(&plugin_directory);
        let plugin_error =
            (!plugin_discovery.errors.is_empty()).then(|| plugin_discovery.errors.join("\n"));
        let scene_directory = scene::default_directory();
        let scene_discovery = scene::discover(&scene_directory);
        let scene_notice =
            (!scene_discovery.errors.is_empty()).then(|| scene_discovery.errors.join("\n"));
        let started = Instant::now();
        let studio = bundled_studio(&creation.egui_ctx);

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
            studio,
            interaction: InteractionState::default(),
            colors: ColorSystem::default(),
            mode_parameters,
            latency: LatencyStats::default(),
            onset_stats: OnsetStats::default(),
            frame_stats: FrameStats::default(),
            default_switch_timing: None,
            pending_default_switch: None,
            visual: 0,
            overlay: true,
            show_text: true,
            instrument_panel: false,
            mode_panel: false,
            forge_panel: false,
            visual_director_panel: false,
            presentation: PresentationMode::Windowed,
            frame_limit: FrameLimit::Display,
            last_paced_frame: started,
            gain,
            started,
            presets: discovery.presets,
            preset_directory,
            preset_error,
            gpu_state,
            particle_forge_renderer,
            particle_forge_error,
            particle_forge: ParticleForgeState::default(),
            gpu_preset,
            plugins: plugin_discovery.packages,
            plugin_directory,
            selected_plugin: 0,
            loaded_plugin: None,
            plugin_error,
            plugin_multiplier: 1.0,
            last_plugin_frame: started,
            cityscape_initialized: false,
            visual_director: VisualDirector::with_default_history(),
            scene_directory,
            scenes: scene_discovery.scenes,
            selected_scene: 0,
            scene_name: String::new(),
            scene_notice,
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
                self.onset_stats = OnsetStats::default();
                self.analyzer.reset();
                self.features = Features::default();
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
        let now = Instant::now();
        self.onset_stats.observe(self.features.onset, now);
        self.visual_director.observe(&self.features, now);
        self.visual_history
            .update(snapshot.callback_sequence, &self.features);
        if let Some(timing) = &mut self.default_switch_timing {
            timing.observe_first_callback(
                &mut self.pending_default_switch,
                snapshot.callback_sequence,
                now,
            );
        }
        if let Some(age) = snapshot.newest_sample_age(now) {
            self.latency.observe(snapshot.callback_sequence, age);
        }
    }

    fn load_preset(&mut self, index: usize) {
        let Some(preset) = self.presets.get(index).cloned() else {
            return;
        };
        let result = if let Some(renderer) = &mut self.gpu_preset {
            renderer.load(&preset)
        } else if let Some(state) = &self.gpu_state {
            GpuPresetRenderer::install(state, &preset).map(|renderer| {
                self.gpu_preset = Some(renderer);
            })
        } else {
            Err("GPU preset renderer is unavailable".to_owned())
        };
        match result {
            Ok(()) => {
                self.gain = preset.response.default;
                self.mode_parameters = preset_parameter_defaults(&preset);
                if preset.id == "thevisualizer.cityscape" && !self.cityscape_initialized {
                    self.interaction = InteractionState::cityscape();
                    self.cityscape_initialized = true;
                }
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
        let mut load_errors = Vec::new();
        if let Some(index) = active_id
            .as_ref()
            .and_then(|id| self.presets.iter().position(|preset| &preset.id == id))
        {
            self.load_preset(index);
            if let Some(error) = self.preset_error.take() {
                load_errors.push(error);
            }
        } else if self.gpu_preset.is_none() && self.gpu_state.is_some() {
            for index in 0..self.presets.len() {
                self.load_preset(index);
                if let Some(error) = self.preset_error.take() {
                    load_errors.push(error);
                }
                if self.gpu_preset.is_some() {
                    break;
                }
            }
        }
        let load_error = (!load_errors.is_empty()).then(|| load_errors.join("\n"));
        self.preset_error = match (discovery_error, load_error) {
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

    fn active_scene_identity(&self) -> Result<(String, String), String> {
        match self.visual {
            0 => Ok(("host.neon-scope".to_owned(), "Neon Scope".to_owned())),
            1 => Ok((
                "host.particle-forge".to_owned(),
                "Particle Forge".to_owned(),
            )),
            2 => {
                let id = self
                    .gpu_preset
                    .as_ref()
                    .map(GpuPresetRenderer::active_id)
                    .ok_or_else(|| "No GPU preset is active.".to_owned())?;
                let preset = self
                    .presets
                    .iter()
                    .find(|preset| preset.id == id)
                    .ok_or_else(|| "The active preset is no longer available.".to_owned())?;
                Ok((preset.id.clone(), preset.name.clone()))
            }
            _ => Err("Studio compositions are session-only in this prototype.".to_owned()),
        }
    }

    fn capture_scene(&self, requested_name: &str) -> Result<SceneSnapshot, String> {
        let (mode_id, mode_name) = self.active_scene_identity()?;
        let name = if requested_name.trim().is_empty() {
            format!("{mode_name} view")
        } else {
            requested_name.trim().to_owned()
        };
        Ok(SceneSnapshot {
            saved_at_ms: 0,
            name,
            mode_id,
            mode_name,
            gain: self.gain.clamp(0.25, 6.0),
            palette: self.colors.palette.code(),
            finish: self.colors.finish.code(),
            colors: [
                self.colors.full.to_array(),
                self.colors.bass.to_array(),
                self.colors.mid.to_array(),
                self.colors.treble.to_array(),
                self.colors.background.to_array(),
            ],
            glow: self.colors.glow,
            gloss: self.colors.gloss,
            saturation: self.colors.saturation,
            camera_yaw: self.interaction.camera_yaw,
            camera_pitch: self.interaction.camera_pitch,
            camera_zoom: self.interaction.camera_zoom,
            show_handles: self.interaction.show_handles,
            selected_zone: self.interaction.selected,
            zones: self
                .interaction
                .zones
                .iter()
                .map(|zone| SceneZone {
                    x: zone.position.x,
                    y: zone.position.y,
                    radius: zone.radius,
                    strength: zone.strength,
                    band: zone.band.code(),
                    pinned: zone.pinned,
                })
                .collect(),
            parameters: self.mode_parameters,
        })
    }

    fn save_scene(&mut self) {
        let result = self
            .capture_scene(&self.scene_name)
            .and_then(|snapshot| scene::save(&self.scene_directory, snapshot));
        match result {
            Ok(saved) => {
                self.scene_notice = Some(format!("Saved scene · {}", saved.path.display()));
                self.scenes.insert(0, saved);
                self.selected_scene = 0;
                self.scene_name.clear();
            }
            Err(error) => self.scene_notice = Some(error),
        }
    }

    fn refresh_scenes(&mut self) {
        let discovery = scene::discover(&self.scene_directory);
        self.scenes = discovery.scenes;
        self.selected_scene = self.selected_scene.min(self.scenes.len().saturating_sub(1));
        self.scene_notice = if discovery.errors.is_empty() {
            Some(format!("Refreshed scenes · {}", self.scenes.len()))
        } else {
            Some(discovery.errors.join("\n"))
        };
    }

    fn restore_scene(&mut self, index: usize) {
        let Some(saved) = self.scenes.get(index).cloned() else {
            self.scene_notice = Some("The selected scene is no longer available.".to_owned());
            return;
        };
        let snapshot = saved.snapshot;
        match snapshot.mode_id.as_str() {
            "host.neon-scope" => self.visual = 0,
            "host.particle-forge" => self.visual = 1,
            id => {
                let Some(preset_index) = self.presets.iter().position(|preset| preset.id == id)
                else {
                    self.scene_notice = Some(format!(
                        "Scene requires missing preset `{}`.",
                        snapshot.mode_name
                    ));
                    return;
                };
                self.visual = 2;
                self.load_preset(preset_index);
                if self
                    .gpu_preset
                    .as_ref()
                    .is_none_or(|renderer| renderer.active_id() != id)
                {
                    self.scene_notice =
                        Some(self.preset_error.clone().unwrap_or_else(|| {
                            format!("Could not load `{}`.", snapshot.mode_name)
                        }));
                    return;
                }
            }
        }
        self.gain = snapshot.gain;
        self.colors.palette = PalettePreset::from_code(snapshot.palette);
        self.colors.finish = SurfaceFinish::from_code(snapshot.finish);
        [
            self.colors.full,
            self.colors.bass,
            self.colors.mid,
            self.colors.treble,
            self.colors.background,
        ] = snapshot
            .colors
            .map(|color| Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]));
        self.colors.glow = snapshot.glow;
        self.colors.gloss = snapshot.gloss;
        self.colors.saturation = snapshot.saturation;
        self.mode_parameters = snapshot.parameters;
        if self.visual == 2
            && let Some(preset) = self.gpu_preset.as_ref().and_then(|renderer| {
                self.presets
                    .iter()
                    .find(|preset| preset.id == renderer.active_id())
            })
        {
            for (value, parameter) in self.mode_parameters.iter_mut().zip(&preset.parameters) {
                *value = value.clamp(parameter.minimum, parameter.maximum);
                if parameter.id == "response" {
                    self.gain = *value;
                }
            }
        }
        self.interaction = InteractionState {
            zones: snapshot
                .zones
                .into_iter()
                .map(|zone| SoundZone {
                    position: Vec2::new(zone.x, zone.y),
                    radius: zone.radius,
                    strength: zone.strength,
                    band: ZoneBand::from_code(zone.band),
                    pinned: zone.pinned,
                })
                .collect(),
            selected: snapshot.selected_zone,
            drag_origin: None,
            camera_drag_origin: None,
            secondary_drag_origin: None,
            camera_yaw: snapshot.camera_yaw,
            camera_pitch: snapshot.camera_pitch,
            camera_zoom: snapshot.camera_zoom,
            show_handles: snapshot.show_handles,
        };
        self.scene_notice = Some(format!(
            "Restored scene · {} · {}",
            snapshot.name, snapshot.mode_name
        ));
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

    fn pace_frame(&mut self) {
        let interval = if self.visual == 1 {
            self.particle_forge
                .quality
                .frame_rate()
                .map(|rate| Duration::from_secs_f64(1.0 / f64::from(rate)))
        } else {
            self.frame_limit.interval()
        };
        if let Some(delay) =
            interval.and_then(|interval| interval.checked_sub(self.last_paced_frame.elapsed()))
        {
            std::thread::sleep(delay);
        }
        self.last_paced_frame = Instant::now();
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
        if ctx.egui_wants_keyboard_input() {
            return;
        }
        let (
            tab,
            left,
            right,
            up,
            down,
            borderless,
            f11,
            escape,
            microphone,
            system,
            delete,
            reset,
            revert,
            labels,
            instrument,
            mode_panel,
            forge,
        ) = ctx.input(|input| {
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
                input.key_pressed(egui::Key::Delete),
                input.key_pressed(egui::Key::R) && !input.modifiers.shift,
                input.key_pressed(egui::Key::R) && input.modifiers.shift,
                input.key_pressed(egui::Key::L),
                input.key_pressed(egui::Key::I),
                input.key_pressed(egui::Key::O),
                input.key_pressed(egui::Key::F),
            )
        });
        let direct_visual = ctx.input(|input| {
            [
                egui::Key::Num1,
                egui::Key::Num2,
                egui::Key::Num3,
                egui::Key::Num4,
            ]
            .into_iter()
            .find(|key| input.key_pressed(*key))
            .and_then(visual_index_for_key)
        });
        if tab {
            self.overlay = !self.overlay;
        }
        if let Some(visual) = direct_visual {
            self.visual = visual;
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
        if delete {
            self.interaction.remove_selected();
        }
        if reset {
            self.reset_interaction();
        }
        if revert {
            self.revert_all(ctx);
        }
        if labels {
            self.show_text = !self.show_text;
        }
        if instrument {
            self.instrument_panel = !self.instrument_panel;
        }
        if mode_panel {
            self.mode_panel = !self.mode_panel;
        }
        if forge && self.visual == 1 {
            self.forge_panel = !self.forge_panel;
        }
        if borderless {
            self.set_presentation(ctx, self.presentation.toggle_borderless());
        }
        if f11 {
            self.set_presentation(ctx, self.presentation.toggle_fullscreen());
        }
        if escape {
            if self.forge_panel {
                self.forge_panel = false;
            } else if self.mode_panel {
                self.mode_panel = false;
            } else if self.instrument_panel {
                self.instrument_panel = false;
            } else if self.presentation != PresentationMode::Windowed {
                self.set_presentation(ctx, PresentationMode::Windowed);
            } else {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn interact_visual(&mut self, response: &egui::Response, rect: Rect) {
        if self.visual == 1 {
            self.interact_forge(response, rect);
            return;
        }
        let pointer = response.interact_pointer_pos();
        if response.secondary_clicked()
            && let Some(pointer) = pointer
        {
            if let Some(index) = self.nearest_zone(rect, pointer) {
                self.interaction.selected = Some(index);
                self.interaction.zones[index].band = self.interaction.zones[index].band.next();
            } else if self.visual == 3 {
                if let Some(layer) = self.studio.layers.get_mut(self.studio.selected) {
                    layer.visible = !layer.visible;
                }
            } else {
                self.interaction
                    .add_zone(normalize_visual_position(rect, pointer));
            }
        }
        if response.double_clicked()
            && let Some(pointer) = pointer
            && self.nearest_zone(rect, pointer).is_none()
        {
            self.interaction
                .add_zone(normalize_visual_position(rect, pointer));
        }
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(pointer) = pointer
        {
            self.interaction.selected = self.nearest_zone(rect, pointer);
            self.interaction.drag_origin = self
                .interaction
                .selected
                .and_then(|index| self.interaction.zones.get(index))
                .map(|zone| zone.position);
            self.interaction.camera_drag_origin = self
                .interaction
                .selected
                .is_none()
                .then(|| Vec2::new(self.interaction.camera_yaw, self.interaction.camera_pitch));
        }
        if (response.dragged_by(egui::PointerButton::Primary)
            || response.drag_stopped_by(egui::PointerButton::Primary))
            && let Some(origin) = self.interaction.drag_origin
            && let Some(zone) = self
                .interaction
                .selected
                .and_then(|index| self.interaction.zones.get_mut(index))
        {
            let delta = response.drag_delta();
            zone.position = Vec2::new(
                (origin.x + delta.x / rect.width().max(1.0)).clamp(0.0, 1.0),
                (origin.y + delta.y / rect.height().max(1.0)).clamp(0.0, 1.0),
            );
        } else if (response.dragged_by(egui::PointerButton::Primary)
            || response.drag_stopped_by(egui::PointerButton::Primary))
            && let Some(origin) = self.interaction.camera_drag_origin
        {
            let delta = response.drag_delta();
            self.interaction.camera_yaw = origin.x - delta.x * 0.008;
            self.interaction.camera_pitch = (origin.y + delta.y * 0.004).clamp(-1.2, 1.2);
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            self.interaction.drag_origin = None;
            self.interaction.camera_drag_origin = None;
        }
        if response.drag_started_by(egui::PointerButton::Secondary)
            && let Some(pointer) = pointer
        {
            self.interaction.selected = self.nearest_zone(rect, pointer);
            self.interaction.secondary_drag_origin = self
                .interaction
                .selected
                .and_then(|index| self.interaction.zones.get(index))
                .map(|zone| Vec2::new(zone.radius, zone.strength))
                .or_else(|| {
                    (self.visual == 3)
                        .then(|| self.studio.layers.get(self.studio.selected))
                        .flatten()
                        .map(|layer| Vec2::new(layer.scale, layer.reactivity))
                })
                .or(Some(Vec2::new(self.gain, self.colors.glow)));
        }
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.drag_stopped_by(egui::PointerButton::Secondary)
        {
            let delta = response.drag_delta();
            if let Some(origin) = self.interaction.secondary_drag_origin {
                if let Some(zone) = self
                    .interaction
                    .selected
                    .and_then(|index| self.interaction.zones.get_mut(index))
                {
                    zone.radius =
                        (origin.x + delta.x / rect.width().max(1.0) * 0.4).clamp(0.04, 0.45);
                    zone.strength =
                        (origin.y - delta.y / rect.height().max(1.0) * 3.0).clamp(0.0, 3.0);
                } else if self.visual == 3 {
                    if let Some(layer) = self.studio.layers.get_mut(self.studio.selected) {
                        layer.scale =
                            (origin.x + delta.x / rect.width().max(1.0) * 1.65).clamp(0.35, 2.0);
                        layer.reactivity =
                            (origin.y - delta.y / rect.height().max(1.0) * 2.0).clamp(0.0, 2.0);
                    }
                } else {
                    self.set_response(origin.x + delta.x / rect.width().max(1.0) * (6.0 - 0.25));
                    self.colors.glow =
                        (origin.y - delta.y / rect.height().max(1.0) * 2.0).clamp(0.0, 2.0);
                }
            }
        }
        if response.drag_stopped_by(egui::PointerButton::Secondary) {
            self.interaction.secondary_drag_origin = None;
        }
        if response.hovered() {
            let scroll = response.ctx.input(|input| input.smooth_scroll_delta.y);
            if scroll != 0.0 {
                self.interaction.camera_zoom =
                    (self.interaction.camera_zoom * (-scroll * 0.0015).exp()).clamp(0.35, 3.0);
            }
        }
    }

    fn nearest_forge_node(&self, rect: Rect, pointer: Pos2) -> Option<usize> {
        self.particle_forge
            .nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| {
                let distance = forge_node_position(rect, node.position).distance(pointer);
                (distance < (node.radius * 42.0).max(20.0)).then_some((index, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(index, _)| index)
    }

    fn interact_forge(&mut self, response: &egui::Response, rect: Rect) {
        let pointer = response.interact_pointer_pos();
        if response.secondary_clicked()
            && let Some(pointer) = pointer
        {
            if let Some(index) = self.nearest_forge_node(rect, pointer) {
                self.particle_forge.selected = Some(index);
                self.particle_forge.nodes[index].band =
                    (self.particle_forge.nodes[index].band + 1) % 4;
            } else {
                let normalized = normalize_visual_position(rect, pointer);
                self.particle_forge.add_node([
                    (normalized.x - 0.5) * 4.0,
                    (0.5 - normalized.y) * 2.8,
                    0.0,
                ]);
            }
        }
        if response.drag_started_by(egui::PointerButton::Primary)
            && let Some(pointer) = pointer
        {
            self.particle_forge.selected = self.nearest_forge_node(rect, pointer);
            self.particle_forge.drag_origin = self
                .particle_forge
                .selected
                .and_then(|index| self.particle_forge.nodes.get(index))
                .map(|node| node.position);
            self.particle_forge.camera_drag_origin =
                self.particle_forge.selected.is_none().then_some([
                    self.particle_forge.camera_yaw,
                    self.particle_forge.camera_pitch,
                ]);
        }
        if response.dragged_by(egui::PointerButton::Primary)
            || response.drag_stopped_by(egui::PointerButton::Primary)
        {
            let delta = response.drag_delta();
            if let Some(origin) = self.particle_forge.drag_origin
                && let Some(node) = self
                    .particle_forge
                    .selected
                    .and_then(|index| self.particle_forge.nodes.get_mut(index))
            {
                node.position[0] =
                    (origin[0] + delta.x / rect.width().max(1.0) * 4.0).clamp(-3.0, 3.0);
                node.position[1] =
                    (origin[1] - delta.y / rect.height().max(1.0) * 2.8).clamp(-2.5, 2.5);
            } else if let Some(origin) = self.particle_forge.camera_drag_origin {
                self.particle_forge.camera_yaw = origin[0] - delta.x * 0.008;
                self.particle_forge.camera_pitch = (origin[1] + delta.y * 0.004).clamp(-1.2, 1.2);
            }
        }
        if response.drag_stopped_by(egui::PointerButton::Primary) {
            self.particle_forge.drag_origin = None;
            self.particle_forge.camera_drag_origin = None;
        }
        if response.drag_started_by(egui::PointerButton::Secondary)
            && let Some(pointer) = pointer
        {
            self.particle_forge.selected = self.nearest_forge_node(rect, pointer);
            self.particle_forge.secondary_drag_origin = self
                .particle_forge
                .selected
                .and_then(|index| self.particle_forge.nodes.get(index))
                .map(|node| [node.radius, node.strength]);
        }
        if response.dragged_by(egui::PointerButton::Secondary)
            || response.drag_stopped_by(egui::PointerButton::Secondary)
        {
            let delta = response.drag_delta();
            if let Some(origin) = self.particle_forge.secondary_drag_origin
                && let Some(node) = self
                    .particle_forge
                    .selected
                    .and_then(|index| self.particle_forge.nodes.get_mut(index))
            {
                node.radius = (origin[0] + delta.x / rect.width().max(1.0) * 1.5).clamp(0.08, 1.5);
                node.strength =
                    (origin[1] - delta.y / rect.height().max(1.0) * 3.0).clamp(0.0, 3.0);
            }
        }
        if response.drag_stopped_by(egui::PointerButton::Secondary) {
            self.particle_forge.secondary_drag_origin = None;
        }
        if response.hovered() {
            let (scroll, shift) = response
                .ctx
                .input(|input| (input.smooth_scroll_delta.y, input.modifiers.shift));
            if scroll != 0.0 {
                if shift {
                    if let Some(node) = self
                        .particle_forge
                        .selected
                        .and_then(|index| self.particle_forge.nodes.get_mut(index))
                    {
                        node.position[2] = (node.position[2] + scroll * 0.0025).clamp(-2.5, 2.5);
                    }
                } else {
                    self.particle_forge.camera_zoom = (self.particle_forge.camera_zoom
                        * (-scroll * 0.0015).exp())
                    .clamp(0.35, 3.0);
                }
            }
        }
    }

    fn reset_interaction(&mut self) {
        let cityscape = self.visual == 2
            && self
                .gpu_preset
                .as_ref()
                .is_some_and(|renderer| renderer.active_id() == "thevisualizer.cityscape");
        self.interaction = if cityscape {
            InteractionState::cityscape()
        } else {
            InteractionState::default()
        };
    }

    fn set_response(&mut self, value: f32) {
        let response_parameter = (self.visual == 2)
            .then(|| {
                self.gpu_preset.as_ref().and_then(|renderer| {
                    self.presets
                        .iter()
                        .find(|preset| preset.id == renderer.active_id())
                })
            })
            .flatten()
            .and_then(|preset| {
                preset
                    .parameters
                    .iter()
                    .enumerate()
                    .find(|(_, parameter)| parameter.id == "response")
            })
            .map(|(index, parameter)| (index, parameter.minimum, parameter.maximum));
        self.gain = response_parameter.map_or_else(
            || value.clamp(0.25, 6.0),
            |(_, minimum, maximum)| value.clamp(minimum, maximum),
        );
        if let Some((index, _, _)) = response_parameter {
            self.mode_parameters[index] = self.gain;
        }
    }

    fn revert_all(&mut self, ctx: &egui::Context) {
        self.reset_interaction();
        self.colors = ColorSystem::default();
        self.studio = bundled_studio(ctx);
        self.overlay = true;
        self.show_text = true;
        self.particle_forge.reset();
        if self.visual == 2
            && let Some(preset) = self.gpu_preset.as_ref().and_then(|renderer| {
                self.presets
                    .iter()
                    .find(|preset| preset.id == renderer.active_id())
            })
        {
            self.mode_parameters = preset_parameter_defaults(preset);
            self.gain = preset.response.default;
        } else {
            self.gain = 2.2;
        }
        self.scene_notice = Some("Reverted session changes to defaults.".to_owned());
    }

    fn nearest_zone(&self, rect: Rect, pointer: Pos2) -> Option<usize> {
        let scale = rect.size().min_elem();
        self.interaction
            .zones
            .iter()
            .enumerate()
            .filter_map(|(index, zone)| {
                let distance = visual_position(rect, zone.position).distance(pointer);
                (distance <= (zone.radius * scale).max(18.0)).then_some((index, distance))
            })
            .min_by(|left, right| left.1.total_cmp(&right.1))
            .map(|(index, _)| index)
    }

    fn draw_zone_handles(&self, painter: &egui::Painter, rect: Rect) {
        if !self.interaction.show_handles {
            return;
        }
        let scale = rect.size().min_elem();
        for (index, zone) in self.interaction.zones.iter().enumerate() {
            let center = visual_position(rect, zone.position);
            let energy = zone.band.energy(&self.features) * zone.strength;
            let radius = (zone.radius * scale * (1.0 + energy * 0.16)).max(24.0);
            let color = self.colors.band_color(zone.band);
            let selected = self.interaction.selected == Some(index);
            let finish_alpha = match self.colors.finish {
                SurfaceFinish::Neon => 1.0,
                SurfaceFinish::Glossy => 0.85,
                SurfaceFinish::Matte => 0.55,
                SurfaceFinish::Metallic => 0.72,
                SurfaceFinish::Glass => 0.62,
            };
            let range_color = color.gamma_multiply(
                (if selected { 0.52 } else { 0.16 } + energy * 0.32) * finish_alpha,
            );
            let tick_length = if selected { 12.0 } else { 7.0 } + energy * 5.0;
            for direction in [
                Vec2::new(1.0, 0.0),
                Vec2::new(0.0, 1.0),
                Vec2::new(-1.0, 0.0),
                Vec2::new(0.0, -1.0),
            ] {
                let edge = center + direction * radius;
                let tangent = Vec2::new(-direction.y, direction.x) * (tick_length * 0.5);
                painter.line_segment(
                    [edge - tangent, edge + tangent],
                    Stroke::new(if selected { 1.8 } else { 1.0 }, range_color),
                );
            }

            let marker_size = if selected { 7.0 } else { 5.0 } + energy * 3.0;
            let diamond = [
                center + Vec2::new(0.0, -marker_size),
                center + Vec2::new(marker_size, 0.0),
                center + Vec2::new(0.0, marker_size),
                center + Vec2::new(-marker_size, 0.0),
            ];
            painter.add(egui::Shape::convex_polygon(
                diamond.to_vec(),
                color.gamma_multiply(if zone.pinned { 0.7 } else { 0.18 }),
                Stroke::new(
                    if selected { 2.0 } else { 1.0 },
                    color.gamma_multiply(0.72 + energy * 0.25),
                ),
            ));
            if selected && self.show_text {
                painter.text(
                    center + Vec2::new(marker_size + 7.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{}  {}", index + 1, zone.band.label()),
                    egui::FontId::monospace(10.0),
                    color.gamma_multiply(0.78),
                );
            }
        }
    }

    fn draw_forge_handles(&self, painter: &egui::Painter, rect: Rect) {
        for (index, node) in self.particle_forge.nodes.iter().enumerate() {
            let center = forge_node_position(rect, node.position);
            let selected = self.particle_forge.selected == Some(index);
            let color = self.colors.band_color(ZoneBand::from_code(node.band));
            let radius = (node.radius * 42.0).max(12.0);
            painter.circle_stroke(
                center,
                radius,
                Stroke::new(
                    if selected { 2.0 } else { 1.0 },
                    color.gamma_multiply(if selected { 0.85 } else { 0.35 }),
                ),
            );
            painter.circle_filled(center, if selected { 5.0 } else { 3.0 }, color);
            if selected && self.particle_forge.gizmo {
                for (offset, axis_color) in [
                    (Vec2::new(34.0, 0.0), Color32::RED),
                    (Vec2::new(0.0, -34.0), Color32::GREEN),
                    (Vec2::new(24.0, 24.0), Color32::BLUE),
                ] {
                    painter.line_segment([center, center + offset], Stroke::new(2.0, axis_color));
                }
            }
            if selected && self.show_text {
                painter.text(
                    center + Vec2::new(radius + 6.0, 0.0),
                    egui::Align2::LEFT_CENTER,
                    format!("{} · {}", index + 1, node.kind.label()),
                    egui::FontId::monospace(10.0),
                    color,
                );
            }
        }
    }

    fn preset_scene_state(&self) -> [f32; PRESET_SCENE_FLOATS] {
        let mut state = [0.0; PRESET_SCENE_FLOATS];
        state[0] = self.interaction.zones.len() as f32;
        state[1] = self.interaction.selected.map_or(-1.0, |index| index as f32);
        state[2] = self.interaction.camera_yaw;
        state[3] = self.interaction.camera_pitch;
        state[4] = self.interaction.camera_zoom;
        for (index, zone) in self.interaction.zones.iter().enumerate() {
            let offset = 8 + index * 8;
            state[offset] = zone.position.x;
            state[offset + 1] = zone.position.y;
            state[offset + 2] = zone.radius;
            state[offset + 3] = zone.strength;
            state[offset + 4] = match zone.band {
                ZoneBand::Full => 0.0,
                ZoneBand::Low => 1.0,
                ZoneBand::Mid => 2.0,
                ZoneBand::High => 3.0,
            };
            state[offset + 5] = if zone.pinned { 1.0 } else { 0.0 };
            state[offset + 6] = zone.band.energy(&self.features);
        }
        for (index, color) in [
            self.colors.full,
            self.colors.bass,
            self.colors.mid,
            self.colors.treble,
            self.colors.background,
        ]
        .into_iter()
        .enumerate()
        {
            let offset = 72 + index * 4;
            state[offset] = f32::from(color.r()) / 255.0;
            state[offset + 1] = f32::from(color.g()) / 255.0;
            state[offset + 2] = f32::from(color.b()) / 255.0;
            state[offset + 3] = f32::from(color.a()) / 255.0;
        }
        state[92] = self.colors.glow;
        state[93] = self.colors.gloss;
        state[94] = self.colors.saturation;
        state[95] = match self.colors.finish {
            SurfaceFinish::Neon => 0.0,
            SurfaceFinish::Glossy => 1.0,
            SurfaceFinish::Matte => 2.0,
            SurfaceFinish::Metallic => 3.0,
            SurfaceFinish::Glass => 4.0,
        };
        state
    }

    fn draw_visual(&self, painter: &egui::Painter, rect: Rect) {
        for strip in 0..24 {
            let t = strip as f32 / 23.0;
            let top = egui::lerp(rect.top()..=rect.bottom(), t);
            let bottom = egui::lerp(rect.top()..=rect.bottom(), (t + 1.0 / 23.0).min(1.0));
            let background = self.colors.background;
            let color = Color32::from_rgb(
                (background.r() as f32 * (0.8 + t * 0.35) + t * 3.0).min(255.0) as u8,
                (background.g() as f32 * (0.8 + t * 0.35) + t * 4.0).min(255.0) as u8,
                (background.b() as f32 * (0.8 + t * 0.35) + t * 8.0).min(255.0) as u8,
            );
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(rect.left(), top), Pos2::new(rect.right(), bottom)),
                0.0,
                color,
            );
        }

        match self.visual {
            0 => self.draw_scope_layer(painter, rect, 1.0, StudioBlend::Normal, 1.0),
            1 => self.draw_particle_forge_gpu(painter, rect),
            2 => self.draw_gpu_preset_layer(painter, rect, 1.0),
            _ => self.draw_studio(painter, rect),
        }
    }

    fn draw_particle_forge_gpu(&self, painter: &egui::Painter, rect: Rect) {
        let Some(renderer) = &self.particle_forge_renderer else {
            self.draw_particles_layer(painter, rect, 1.0, StudioBlend::Normal, 1.0);
            return;
        };
        let mut forge = self.particle_forge.clone();
        forge.apply_modulation([
            self.features.low,
            self.features.mid,
            self.features.high,
            self.features.rms,
            self.features.onset,
            self.features.transient,
        ]);
        renderer.paint(
            painter,
            rect,
            ForgeFrame {
                time: self.started.elapsed().as_secs_f32(),
                delta: (self.frame_stats.current_ms as f32 / 1_000.0).min(0.05),
                gain: self.gain,
                low: self.features.low,
                mid: self.features.mid,
                high: self.features.high,
                rms: self.features.rms,
                onset: self.features.onset,
                transient: self.features.transient,
                state: &forge,
                colors: [
                    self.colors.bass.to_normalized_gamma_f32(),
                    self.colors.mid.to_normalized_gamma_f32(),
                    self.colors.treble.to_normalized_gamma_f32(),
                ],
            },
        );
    }

    fn draw_gpu_preset_layer(&self, painter: &egui::Painter, rect: Rect, drive: f32) {
        let Some(renderer) = &self.gpu_preset else {
            self.draw_tunnel(painter, rect);
            return;
        };
        let scene = self.preset_scene_state();
        let mut parameters = self.mode_parameters;
        let response_index = self
            .presets
            .iter()
            .find(|preset| preset.id == renderer.active_id())
            .and_then(|preset| {
                preset
                    .parameters
                    .iter()
                    .position(|parameter| parameter.id == "response")
            })
            .unwrap_or(0);
        let response = self.gain * self.plugin_multiplier * drive;
        parameters[response_index] = response;
        renderer.paint(
            painter,
            rect,
            PresetFrame {
                time: self.started.elapsed().as_secs_f32(),
                delta: (self.frame_stats.current_ms as f32 / 1_000.0).min(0.25),
                gain: response,
                waveform: &self.features.waveform,
                spectrum: &self.features.spectrum,
                low: self.features.low,
                mid: self.features.mid,
                high: self.features.high,
                rms: self.features.rms,
                peak: self.features.peak,
                onset: self.features.onset,
                transient: self.features.transient,
                spectrum_history: &self.visual_history.spectra,
                scene: &scene,
                parameters: &parameters,
            },
        );
        if self.show_text && renderer.active_id() == "thevisualizer.cascading-falls" {
            self.draw_waterfall_labels(painter, rect);
        }
    }

    fn draw_waterfall_labels(&self, painter: &egui::Painter, rect: Rect) {
        let sample_rate = self
            .capture
            .as_ref()
            .map_or(48_000.0, |capture| capture.sample_rate as f32);
        painter.text(
            rect.left_top() + Vec2::new(10.0, 8.0),
            egui::Align2::LEFT_TOP,
            "CASCADING FALLS // DUAL-SPECTRUM WATERFALL",
            egui::FontId::monospace(12.0),
            Color32::from_rgb(130, 225, 245),
        );
        painter.text(
            rect.right_top() + Vec2::new(-10.0, 8.0),
            egui::Align2::RIGHT_TOP,
            format!(
                "{:.1} kHz  ←  0 Hz  →  {:.1} kHz · TIME ↓",
                sample_rate / 2_000.0,
                sample_rate / 2_000.0
            ),
            egui::FontId::monospace(11.0),
            Color32::from_rgb(90, 150, 165),
        );
    }

    fn studio_energy(&self, band: StudioBand) -> f32 {
        match band {
            StudioBand::Full => self.features.rms * 3.0,
            StudioBand::Bass => self.features.low,
            StudioBand::Mid => self.features.mid,
            StudioBand::Treble => self.features.high,
        }
        .clamp(0.0, 1.0)
    }

    fn studio_color(color: Color32, opacity: f32, blend: StudioBlend) -> Color32 {
        let color = color.gamma_multiply(opacity.clamp(0.0, 1.0));
        match blend {
            StudioBlend::Normal => color,
            StudioBlend::Additive => {
                Color32::from_rgba_premultiplied(color.r(), color.g(), color.b(), 0)
            }
        }
    }

    fn draw_studio(&self, painter: &egui::Painter, rect: Rect) {
        for layer in &self.studio.layers {
            if !layer.visible || layer.opacity <= 0.0 {
                continue;
            }
            let energy = self.studio_energy(layer.band);
            let drive = layer.scale * (0.35 + energy * layer.reactivity);
            match layer.kind {
                StudioLayerKind::Image => self.draw_studio_image(painter, rect, layer, energy),
                StudioLayerKind::Preset => self.draw_gpu_preset_layer(painter, rect, drive),
                StudioLayerKind::Waveform => {
                    self.draw_scope_layer(painter, rect, layer.opacity, layer.blend, drive)
                }
                StudioLayerKind::Particles => {
                    self.draw_particles_layer(painter, rect, layer.opacity, layer.blend, drive)
                }
            }
        }
    }

    fn draw_studio_image(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        layer: &StudioLayer,
        energy: f32,
    ) {
        let Some(image) = &self.studio.image else {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "Drop a PNG, JPEG, or WebP image to begin",
                egui::FontId::proportional(20.0),
                Color32::from_rgb(175, 195, 205),
            );
            return;
        };
        let source_aspect = image.size[0] as f32 / image.size[1].max(1) as f32;
        let target_aspect = rect.width() / rect.height().max(1.0);
        let has_motion = image.motion.is_some();
        let zoom = (self.interaction.camera_zoom
            * layer.scale
            * (1.0
                + if has_motion {
                    0.0
                } else {
                    energy * layer.reactivity * 0.055
                }))
        .clamp(0.4, 4.0);
        let (mut uv_width, mut uv_height) = if source_aspect > target_aspect {
            (target_aspect / source_aspect, 1.0)
        } else {
            (1.0, source_aspect / target_aspect)
        };
        uv_width = (uv_width / zoom).clamp(0.05, 1.0);
        uv_height = (uv_height / zoom).clamp(0.05, 1.0);
        let center = Pos2::new(
            0.5 + self.interaction.camera_yaw.sin() * 0.08,
            0.5 + self.interaction.camera_pitch * 0.08,
        );
        let uv = Rect::from_center_size(
            Pos2::new(
                center.x.clamp(uv_width * 0.5, 1.0 - uv_width * 0.5),
                center.y.clamp(uv_height * 0.5, 1.0 - uv_height * 0.5),
            ),
            Vec2::new(uv_width, uv_height),
        );
        if let Some(motion) = &image.motion {
            painter.image(
                image.texture.id(),
                rect,
                uv,
                Color32::from_white_alpha((255.0 * layer.opacity.clamp(0.0, 1.0)) as u8),
            );
            painter.image(
                motion.texture.id(),
                rect,
                uv,
                Color32::from_white_alpha(
                    (255.0 * layer.opacity.clamp(0.0, 1.0) * motion.mix) as u8,
                ),
            );
            if image.rigged {
                self.draw_rigged_foliage(
                    painter,
                    rect,
                    uv,
                    motion.rig_texture.id(),
                    layer.opacity * motion.mix,
                );
            }
            return;
        }
        let time = self.started.elapsed().as_secs_f32();
        let motion = layer.reactivity;
        let mut mesh = egui::Mesh::with_texture(image.texture.id());
        const COLUMNS: u32 = 24;
        const ROWS: u32 = 14;
        for row in 0..=ROWS {
            for column in 0..=COLUMNS {
                let point = Vec2::new(column as f32 / COLUMNS as f32, row as f32 / ROWS as f32);
                let mut source = uv.min + point * uv.size();
                let plants = living_photo_plant_mask(source);
                let table = soft_ellipse(source, Pos2::new(0.54, 0.57), Vec2::new(0.19, 0.13));
                let frog = soft_ellipse(source, Pos2::new(0.555, 0.545), Vec2::new(0.055, 0.045));
                source.x += plants
                    * self.features.mid
                    * motion
                    * 0.0024
                    * (time * 0.75 + source.y * 11.0).sin();
                source.y += table * self.features.low * motion * 0.0008 * (time * 3.0).sin();
                source.y +=
                    frog * (0.00035 + self.features.rms * motion * 0.0011) * (time * 1.7).sin();
                let cloud = living_photo_glass_mask(source)
                    * (time * 0.07 + source.x * 8.0).sin()
                    * self.features.low
                    * motion;
                let light = (0.84 + energy * motion * 0.12 + cloud * 0.035).clamp(0.68, 1.0);
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: rect.min + point * rect.size(),
                    uv: source,
                    color: Color32::from_rgba_unmultiplied(
                        (255.0 * light) as u8,
                        (255.0 * light) as u8,
                        (255.0 * light) as u8,
                        (255.0 * layer.opacity.clamp(0.0, 1.0)) as u8,
                    ),
                });
            }
        }
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                let top_left = row * (COLUMNS + 1) + column;
                let bottom_left = top_left + COLUMNS + 1;
                mesh.indices.extend_from_slice(&[
                    top_left,
                    bottom_left,
                    top_left + 1,
                    top_left + 1,
                    bottom_left,
                    bottom_left + 1,
                ]);
            }
        }
        painter.add(egui::Shape::mesh(mesh));
        self.draw_living_photo_weather(painter, rect, uv, layer.opacity, time);
        self.draw_living_photo_wildlife(painter, rect, uv, layer.opacity, time);
    }

    fn draw_rigged_foliage(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        uv: Rect,
        texture: egui::TextureId,
        opacity: f32,
    ) {
        let time = self.started.elapsed().as_secs_f32();
        let gust = self.features.onset * 0.045;
        let rigs = [
            (
                Rect::from_min_max(Pos2::new(0.0, 0.25), Pos2::new(0.44, 1.0)),
                Pos2::new(0.18, 0.88),
                (time * 1.25).sin() * (self.features.mid * 0.04 + self.features.low * 0.025)
                    + (time * 5.8).sin() * self.features.high * 0.008
                    + gust,
                false,
                -0.012,
            ),
            (
                Rect::from_min_max(Pos2::new(0.62, 0.25), Pos2::new(1.0, 1.0)),
                Pos2::new(0.86, 0.88),
                (time * 1.1 + 1.7).sin() * (self.features.mid * 0.037 + self.features.low * 0.028)
                    + (time * 6.2).sin() * self.features.high * 0.009
                    - gust,
                false,
                0.014,
            ),
            (
                Rect::from_min_max(Pos2::new(0.25, 0.02), Pos2::new(0.68, 0.67)),
                Pos2::new(0.48, 0.08),
                (time * 1.55 + 0.7).sin() * (self.features.mid * 0.06 + self.features.low * 0.018)
                    + (time * 7.0).sin() * self.features.high * 0.015
                    + gust,
                true,
                0.006,
            ),
        ];
        for rig in rigs {
            self.draw_rig_region(painter, rect, uv, texture, rig, opacity * 0.78);
        }
    }

    fn draw_rig_region(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        uv: Rect,
        texture: egui::TextureId,
        (bounds, pivot, angle, hanging, depth): (Rect, Pos2, f32, bool, f32),
        opacity: f32,
    ) {
        const COLUMNS: u32 = 3;
        const ROWS: u32 = 5;
        let pivot_position = studio_image_position(rect, uv, pivot);
        let mut mesh = egui::Mesh::with_texture(texture);
        for row in 0..=ROWS {
            for column in 0..=COLUMNS {
                let point = Vec2::new(column as f32 / COLUMNS as f32, row as f32 / ROWS as f32);
                let source = bounds.min + point * bounds.size();
                let weight = if hanging {
                    ((source.y - pivot.y) / (bounds.max.y - pivot.y).max(0.01)).clamp(0.0, 1.0)
                } else {
                    ((pivot.y - source.y) / (pivot.y - bounds.min.y).max(0.01)).clamp(0.0, 1.0)
                }
                .powf(1.35);
                let base = studio_image_position(rect, uv, source);
                let delta = base - pivot_position;
                let bend = angle * weight;
                let (sin, cos) = bend.sin_cos();
                let parallax = Vec2::new(
                    self.interaction.camera_yaw.sin() * rect.width() * depth,
                    self.interaction.camera_pitch * rect.height() * depth * 0.35,
                ) * weight;
                mesh.vertices.push(egui::epaint::Vertex {
                    pos: pivot_position
                        + Vec2::new(delta.x * cos - delta.y * sin, delta.x * sin + delta.y * cos)
                        + parallax,
                    uv: source,
                    color: Color32::from_white_alpha((255.0 * opacity.clamp(0.0, 1.0)) as u8),
                });
            }
        }
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                let top_left = row * (COLUMNS + 1) + column;
                let bottom_left = top_left + COLUMNS + 1;
                mesh.indices.extend_from_slice(&[
                    top_left,
                    bottom_left,
                    top_left + 1,
                    top_left + 1,
                    bottom_left,
                    bottom_left + 1,
                ]);
            }
        }
        painter.add(egui::Shape::mesh(mesh));
    }

    fn draw_living_photo_weather(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        uv: Rect,
        opacity: f32,
        time: f32,
    ) {
        let rain = (0.15 + self.features.high * 0.85) * opacity;
        for index in 0..36 {
            let seed = hash01(index as f32 * 17.31);
            let source = Pos2::new(
                0.27 + hash01(index as f32 * 8.13) * 0.67,
                0.05 + (seed + time * (0.012 + self.features.high * 0.026)).fract() * 0.55,
            );
            if living_photo_glass_mask(source) < 0.35 {
                continue;
            }
            let start = studio_image_position(rect, uv, source);
            let end =
                studio_image_position(rect, uv, source + Vec2::new(-0.002, 0.025 + seed * 0.035));
            painter.line_segment(
                [start, end],
                Stroke::new(
                    0.45 + seed * 0.65,
                    Color32::from_rgba_unmultiplied(195, 215, 212, (rain * 58.0) as u8),
                ),
            );
        }
    }

    fn draw_living_photo_wildlife(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        uv: Rect,
        opacity: f32,
        time: f32,
    ) {
        let insect_alpha = opacity * (0.22 + self.features.high * 0.55);
        for index in 0..7 {
            let seed = hash01(index as f32 * 29.7);
            let center = Pos2::new(
                0.34 + seed * 0.43,
                0.35 + hash01(index as f32 * 11.2) * 0.28,
            );
            let source = center
                + Vec2::new(
                    (time * (0.45 + seed * 0.5) + seed * 9.0).sin() * 0.012,
                    (time * (0.62 + seed * 0.4) + seed * 13.0).cos() * 0.008,
                ) * (0.35 + self.features.high);
            let point = studio_image_position(rect, uv, source);
            if rect.contains(point) {
                painter.circle_filled(
                    point,
                    0.7 + self.features.high * 0.8,
                    Color32::from_rgba_unmultiplied(35, 31, 18, (insect_alpha * 150.0) as u8),
                );
            }
        }

        let Some(progress) = rare_bird_progress(time) else {
            return;
        };
        let source = Pos2::new(
            0.61 + progress * 0.22,
            0.24 - (progress * std::f32::consts::PI).sin() * 0.025,
        );
        let point = studio_image_position(rect, uv, source);
        if !rect.contains(point) {
            return;
        }
        let size = rect.width() * 0.0065;
        let flap = (time * (8.0 + self.features.high * 12.0)).sin() * size;
        let color = Color32::from_rgba_unmultiplied(
            35,
            42,
            37,
            (opacity * (42.0 + self.features.transient * 38.0)) as u8,
        );
        painter.circle_filled(point, size * 0.45, color);
        painter.line_segment(
            [point, point + Vec2::new(-size, -flap.abs())],
            Stroke::new(size * 0.35, color),
        );
        painter.line_segment(
            [point, point + Vec2::new(size, -flap.abs())],
            Stroke::new(size * 0.35, color),
        );
    }

    fn draw_scope_layer(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        opacity: f32,
        blend: StudioBlend,
        drive: f32,
    ) {
        let center = rect.center();
        for grid in 1..8 {
            let x = egui::lerp(rect.left()..=rect.right(), grid as f32 / 8.0);
            painter.line_segment(
                [Pos2::new(x, rect.top()), Pos2::new(x, rect.bottom())],
                Stroke::new(
                    1.0,
                    Self::studio_color(self.colors.mid, opacity * 0.1, blend),
                ),
            );
        }
        painter.line_segment(
            [
                Pos2::new(rect.left(), center.y),
                Pos2::new(rect.right(), center.y),
            ],
            Stroke::new(
                1.0,
                Self::studio_color(self.colors.full, opacity * 0.25, blend),
            ),
        );

        let amplitude = rect.height() * 0.32 * self.gain * drive;
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
            let color = Self::studio_color(
                self.colors.spectrum_color(age),
                opacity * (0.12 + age * 0.5),
                blend,
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
            Stroke::new(
                13.0 + self.features.onset * 8.0,
                Self::studio_color(
                    self.colors.full,
                    opacity * (0.05 + self.colors.glow * 0.04),
                    blend,
                ),
            ),
        ));
        painter.add(egui::Shape::line(
            points,
            Stroke::new(2.0, Self::studio_color(self.colors.full, opacity, blend)),
        ));
    }

    fn draw_particles_layer(
        &self,
        painter: &egui::Painter,
        rect: Rect,
        opacity: f32,
        blend: StudioBlend,
        drive: f32,
    ) {
        let center = rect.center() + Vec2::new(0.0, rect.height() * 0.035);
        let scale = rect.size().min_elem();
        let spectra = if self.visual_history.spectra.is_empty() {
            vec![self.features.spectrum.as_slice()]
        } else {
            self.visual_history
                .spectra
                .iter()
                .rev()
                .take(VISUAL_TRAIL_FRAMES)
                .rev()
                .map(Vec::as_slice)
                .collect()
        };
        self.draw_particle_forge(painter, center, scale, &spectra, opacity, blend, drive, 1);

        let current_spectrum = [self.features.spectrum.as_slice()];
        for zone in &self.interaction.zones {
            let zone_drive =
                drive * (0.45 + zone.band.energy(&self.features) * zone.strength * 1.55);
            self.draw_particle_forge(
                painter,
                visual_position(rect, zone.position),
                scale * zone.radius * 1.15,
                &current_spectrum,
                opacity * 0.9,
                blend,
                zone_drive,
                4,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_particle_forge(
        &self,
        painter: &egui::Painter,
        center: Pos2,
        scale: f32,
        spectra: &[&[f32]],
        opacity: f32,
        blend: StudioBlend,
        drive: f32,
        detail_step: usize,
    ) {
        let time = self.started.elapsed().as_secs_f32();
        let direction = if self.particle_forge.reverse {
            -1.0
        } else {
            1.0
        };
        let mut current_points = Vec::new();
        for (trail, spectrum) in spectra.iter().enumerate() {
            let age = (trail + 1) as f32 / spectra.len() as f32;
            let time_offset = (1.0 - age) * 0.22;
            for (index, energy) in spectrum.iter().enumerate().step_by(detail_step) {
                let frequency = index as f32 / (spectrum.len() - 1) as f32;
                let value = (energy * self.gain * drive).clamp(0.0, 1.0);
                let angle = std::f32::consts::TAU * frequency - std::f32::consts::FRAC_PI_2
                    + direction * (time * (0.035 + value * 0.12) - time_offset);
                let radius = scale * (0.15 + frequency.powf(0.72) * 0.27 + value * 0.31);
                let point = center + Vec2::new(angle.cos() * radius, angle.sin() * radius * 0.78);
                let color = Self::studio_color(
                    self.colors.spectrum_color(shifted_frequency(
                        frequency,
                        self.particle_forge.gradient_shift,
                    )),
                    opacity * (0.12 + age * 0.7),
                    blend,
                );
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
                    Stroke::new(
                        0.7,
                        Self::studio_color(color, opacity * (0.13 + value * 0.12), blend),
                    ),
                );
            }
        }
        let core = scale
            * (0.035 + self.features.low * self.gain * drive * 0.055 + self.features.onset * 0.012);
        painter.circle_filled(
            center,
            core * 1.8,
            Self::studio_color(
                Color32::from_rgb(90, 60, 255),
                opacity * 18.0 / 255.0,
                blend,
            ),
        );
        painter.circle_stroke(
            center,
            core,
            Stroke::new(
                1.5 + self.features.peak * 3.0,
                Self::studio_color(self.colors.bass, opacity, blend),
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
            let radius = 18.0
                + phase
                    * max_radius
                    * (1.0 + self.features.low * 0.16 + self.features.onset * 0.04);
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
                        let source = self.capture.as_ref().map(|capture| capture.source);
                        if let Some(hint) = status_hint(status, source) {
                            ui.label(
                                egui::RichText::new(hint)
                                    .small()
                                    .color(Color32::from_rgb(145, 165, 185)),
                            );
                            ui.add_space(4.0);
                        } else {
                            ui.add_space(8.0);
                        }

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
                                                    "format {} · {} · {} · {} · {}",
                                                    preset.format,
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
                            ui.separator();
                            if ui
                                .selectable_label(self.instrument_panel, "Instrument [I]")
                                .on_hover_text(
                                    "Open the live inspector for audio routing, sound zones, mode \
                                     controls, colors, materials, and camera settings.",
                                )
                                .clicked()
                            {
                                self.instrument_panel = !self.instrument_panel;
                            }
                            if ui
                                .selectable_label(self.mode_panel, "Mode [O]")
                                .on_hover_text("Open controls specific to the active visual.")
                                .clicked()
                            {
                                self.mode_panel = !self.mode_panel;
                            }
                            if self.visual == 1
                                && ui
                                    .selectable_label(self.forge_panel, "Forge [F]")
                                    .on_hover_text("Inspect the selected 3D force node.")
                                    .clicked()
                            {
                                self.forge_panel = !self.forge_panel;
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
                            if self.visual == 1
                                && let Some(renderer) = &self.particle_forge_renderer
                            {
                                ui.label(
                                    egui::RichText::new(format!(
                                        "GPU FORGE · {} · {} particles",
                                        renderer.adapter_name(),
                                        self.particle_forge.quality.particle_count()
                                    ))
                                    .small()
                                    .color(Color32::from_rgb(115, 145, 175)),
                                );
                            } else if self.visual == 1
                                && let Some(error) = &self.particle_forge_error
                            {
                                ui.colored_label(Color32::from_rgb(255, 145, 90), error);
                            } else if let Some(renderer) = &self.gpu_preset {
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
                                    "Estimated newest captured sample to feature update. Includes the \
                                     audio backend and UI handoff; excludes upstream playback buffering, \
                                     compositor/display scanout, and the separate FFT window duration.",
                                );
                            }
                            let now = Instant::now();
                            let last_onset = self.onset_stats.last.map_or_else(
                                || "none".to_owned(),
                                |last| {
                                    format!(
                                        "{:.2}s",
                                        now.saturating_duration_since(last).as_secs_f32()
                                    )
                                },
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "ONSET {} · {:.0}/min · LAST {} · FLUX {:.2}",
                                    self.onset_stats.count,
                                    self.onset_stats.per_minute(now),
                                    last_onset,
                                    self.features.transient
                                ))
                                .small()
                                .monospace()
                                .color(Color32::from_rgb(170, 135, 205)),
                            )
                            .on_hover_text(
                                "Adaptive attack events since the current capture opened. This is \
                                 diagnostic onset telemetry, not beat position, tempo, or BPM.",
                            );
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("PACE")
                                        .small()
                                        .color(Color32::from_rgb(125, 145, 175)),
                                );
                                for limit in
                                    [FrameLimit::Display, FrameLimit::Fps60, FrameLimit::Fps30]
                                {
                                    if ui
                                        .selectable_label(
                                            self.frame_limit == limit,
                                            limit.label(),
                                        )
                                        .on_hover_text(
                                            "Display follows the presentation cadence; 60 and 30 \
                                             reduce rendering work with a host-side frame limit.",
                                        )
                                        .clicked()
                                    {
                                        self.frame_limit = limit;
                                    }
                                }
                                if self.frame_stats.observations > 0 {
                                    ui.separator();
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "~{:.0} FPS · {:.1} ms",
                                            self.frame_stats.fps(),
                                            self.frame_stats.smoothed_ms
                                        ))
                                        .small()
                                        .monospace()
                                        .color(Color32::from_rgb(145, 155, 205)),
                                    )
                                    .on_hover_text(
                                        "Smoothed interval between application UI frames. This \
                                         measures application cadence, not monitor refresh, display \
                                         scanout, or playback-to-photon latency.",
                                    );
                                }
                            });
                            if let Some(timing) = &self.default_switch_timing {
                                let first_callback = timing.first_callback_ms.map_or_else(
                                    || "waiting".to_owned(),
                                    |milliseconds| format!("{milliseconds:.1} ms"),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "DEFAULT DETECT TO OPEN {:.1} ms · FIRST PACKET {}",
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
                                "<-/-> visual · Up/Down preset · I instrument · L labels · Shift+R revert · Tab · Esc",
                            )
                            .small()
                            .color(Color32::from_rgb(110, 130, 150)),
                        )
                        .on_hover_text(
                            "Left/Right cycles visuals; 1/2/3/4 selects one directly; Up/Down changes \
                             GPU presets; I opens the Instrument Panel; right-click/right-drag acts \
                             directly on the canvas; L hides visual text; Shift+R reverts session \
                             changes; Tab hides the overlay; Escape closes the panel or returns to \
                             windowed mode before exiting.",
                        );
                    });
            });
    }

    fn draw_studio_controls(&mut self, ui: &mut egui::Ui) {
        ui.separator();
        ui.label(
            egui::RichText::new(format!(
                "STUDIO LAYERS · {}/{}",
                self.studio.layers.len(),
                MAX_STUDIO_LAYERS
            ))
            .small()
            .strong()
            .color(Color32::from_rgb(90, 245, 220)),
        );
        ui.horizontal_wrapped(|ui| {
            ui.label("Add");
            for kind in StudioLayerKind::ALL {
                let unavailable = self.studio.layers.len() >= MAX_STUDIO_LAYERS
                    || (matches!(kind, StudioLayerKind::Image | StudioLayerKind::Preset)
                        && self.studio.layers.iter().any(|layer| layer.kind == kind));
                if ui
                    .add_enabled(!unavailable, egui::Button::new(kind.label()))
                    .clicked()
                {
                    self.studio.add(kind);
                }
            }
        });

        for (index, layer) in self.studio.layers.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut layer.visible, "");
                if ui
                    .selectable_label(self.studio.selected == index, layer.kind.label())
                    .clicked()
                {
                    self.studio.selected = index;
                }
                ui.label(
                    egui::RichText::new(format!(
                        "{} · {:.0}%",
                        layer.blend.label(),
                        layer.opacity * 100.0
                    ))
                    .small()
                    .color(Color32::from_rgb(125, 150, 170)),
                );
            });
        }

        let mut move_layer = 0;
        let mut remove_layer = false;
        ui.horizontal(|ui| {
            if ui.small_button("Up").clicked() {
                move_layer = -1;
            }
            if ui.small_button("Down").clicked() {
                move_layer = 1;
            }
            if ui
                .add_enabled(self.studio.layers.len() > 1, egui::Button::new("Remove"))
                .clicked()
            {
                remove_layer = true;
            }
        });
        if move_layer != 0 {
            self.studio.move_selected(move_layer);
        }
        if remove_layer {
            self.studio.remove_selected();
        }

        if let Some(layer) = self.studio.layers.get_mut(self.studio.selected) {
            if layer.kind != StudioLayerKind::Preset {
                ui.add(egui::Slider::new(&mut layer.opacity, 0.0..=1.0).text("Opacity"));
            } else {
                ui.label(
                    egui::RichText::new("Opaque base layer · choose the active preset below")
                        .small()
                        .color(Color32::from_rgb(125, 150, 170)),
                );
            }
            ui.add(egui::Slider::new(&mut layer.reactivity, 0.0..=2.0).text("Reactivity"));
            ui.add(egui::Slider::new(&mut layer.scale, 0.35..=2.0).text("Scale"));
            ui.horizontal(|ui| {
                ui.label("Audio");
                egui::ComboBox::from_id_salt("studio-band")
                    .selected_text(layer.band.label())
                    .show_ui(ui, |ui| {
                        for band in StudioBand::ALL {
                            ui.selectable_value(&mut layer.band, band, band.label());
                        }
                    });
                ui.label("Blend");
                if matches!(layer.kind, StudioLayerKind::Image | StudioLayerKind::Preset) {
                    layer.blend = StudioBlend::Normal;
                    ui.label("Normal");
                } else {
                    egui::ComboBox::from_id_salt("studio-blend")
                        .selected_text(layer.blend.label())
                        .show_ui(ui, |ui| {
                            for blend in StudioBlend::ALL {
                                ui.selectable_value(&mut layer.blend, blend, blend.label());
                            }
                        });
                }
            });
        }
        if self
            .studio
            .layers
            .get(self.studio.selected)
            .is_some_and(|layer| layer.kind == StudioLayerKind::Image)
            && let Some(image) = self.studio.image.as_mut()
        {
            ui.checkbox(&mut image.rigged, "2.5D foliage rig")
                .on_hover_text(
                    "Three anchored depth meshes bend foreground plants and hanging vines without \
                     moving the greenhouse frame, table, or frog.",
                );
        }

        if self
            .studio
            .layers
            .get(self.studio.selected)
            .is_some_and(|layer| layer.kind == StudioLayerKind::Preset)
        {
            let active_id = self.gpu_preset.as_ref().map(GpuPresetRenderer::active_id);
            let active_name = active_id
                .and_then(|id| self.presets.iter().find(|preset| preset.id == id))
                .map_or("No valid preset", |preset| preset.name.as_str());
            let mut chosen_preset = None;
            ui.horizontal(|ui| {
                ui.label("Preset");
                egui::ComboBox::from_id_salt("studio-preset")
                    .selected_text(active_name)
                    .width(245.0)
                    .show_ui(ui, |ui| {
                        for (index, preset) in self.presets.iter().enumerate() {
                            if ui
                                .selectable_label(
                                    active_id == Some(preset.id.as_str()),
                                    &preset.name,
                                )
                                .clicked()
                            {
                                chosen_preset = Some(index);
                            }
                        }
                    });
            });
            if let Some(index) = chosen_preset {
                self.load_preset(index);
            }
        }

        if let Some(image) = &self.studio.image {
            ui.label(
                egui::RichText::new(format!(
                    "Image · {} × {} · {}",
                    image.size[0],
                    image.size[1],
                    image.path.display()
                ))
                .small()
                .color(Color32::from_rgb(145, 170, 190)),
            );
            ui.label(
                egui::RichText::new(
                    "Living Photo · real motion + 2.5D rig · bass/mids bend anchored foliage · treble flutters leaves · onsets kick gusts",
                )
                .small()
                .color(Color32::from_rgb(125, 165, 150)),
            );
        }
        ui.label(
            egui::RichText::new("Drop a PNG, JPEG, or WebP anywhere on the window to replace it.")
                .small()
                .color(Color32::from_rgb(125, 150, 170)),
        );
        if let Some(notice) = &self.studio.notice {
            ui.label(
                egui::RichText::new(notice)
                    .small()
                    .color(Color32::from_rgb(145, 190, 170)),
            );
        }
    }

    fn draw_mode_controls(&mut self, ui: &mut egui::Ui) {
        let preset = self.gpu_preset.as_ref().and_then(|renderer| {
            self.presets
                .iter()
                .find(|preset| preset.id == renderer.active_id())
                .cloned()
        });
        if self.visual == 2
            && let Some(preset) = preset
        {
            if ui.button("Reset mode controls").clicked() {
                self.mode_parameters = preset_parameter_defaults(&preset);
                self.gain = preset.response.default;
            }
            let mut groups = Vec::new();
            for parameter in &preset.parameters {
                if !groups.contains(&parameter.group) {
                    groups.push(parameter.group.clone());
                }
            }
            for group in groups {
                egui::CollapsingHeader::new(&group)
                    .id_salt(("mode-parameter-group", &group))
                    .default_open(group == "Audio")
                    .show(ui, |ui| {
                        if ui.small_button("Reset group").clicked() {
                            for (index, parameter) in preset
                                .parameters
                                .iter()
                                .enumerate()
                                .filter(|(_, parameter)| parameter.group == group)
                            {
                                self.mode_parameters[index] = parameter.default;
                                if parameter.id == "response" {
                                    self.gain = parameter.default;
                                }
                            }
                        }
                        for (index, parameter) in preset
                            .parameters
                            .iter()
                            .enumerate()
                            .filter(|(_, parameter)| parameter.group == group)
                        {
                            let changed = ui
                                .add(
                                    egui::Slider::new(
                                        &mut self.mode_parameters[index],
                                        parameter.minimum..=parameter.maximum,
                                    )
                                    .text(&parameter.label)
                                    .fixed_decimals(2),
                                )
                                .changed();
                            if changed && parameter.id == "response" {
                                self.gain = self.mode_parameters[index];
                            }
                        }
                    });
            }
            return;
        }

        ui.add(
            egui::Slider::new(&mut self.gain, 0.25..=6.0)
                .text("Response")
                .fixed_decimals(2),
        );
        if self.visual == 1 {
            ui.horizontal(|ui| {
                ui.label("Quality");
                egui::ComboBox::from_id_salt("forge-quality")
                    .selected_text(self.particle_forge.quality.label())
                    .show_ui(ui, |ui| {
                        for quality in ForgeQuality::ALL {
                            ui.selectable_value(
                                &mut self.particle_forge.quality,
                                quality,
                                quality.label(),
                            );
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Source");
                egui::ComboBox::from_id_salt("forge-source")
                    .selected_text(self.particle_forge.source.label())
                    .show_ui(ui, |ui| {
                        for source in ForgeSceneSource::ALL {
                            ui.selectable_value(
                                &mut self.particle_forge.source,
                                source,
                                source.label(),
                            );
                        }
                    });
            });
            ui.horizontal(|ui| {
                ui.label("Topology");
                egui::ComboBox::from_id_salt("forge-topology")
                    .selected_text(self.particle_forge.topology.label())
                    .show_ui(ui, |ui| {
                        for topology in ForgeTopology::ALL {
                            ui.selectable_value(
                                &mut self.particle_forge.topology,
                                topology,
                                topology.label(),
                            );
                        }
                    });
            });
            ui.add(
                egui::Slider::new(&mut self.particle_forge.topology_morph, 0.0..=1.0)
                    .text("Topology morph"),
            );
            for (axis, label) in ["Spin X", "Spin Y", "Spin Z"].into_iter().enumerate() {
                ui.add(
                    egui::Slider::new(&mut self.particle_forge.spin[axis], -1.5..=1.5).text(label),
                );
            }
            ui.add(egui::Slider::new(&mut self.particle_forge.twist, 0.0..=2.0).text("Twist"));
            ui.add(
                egui::Slider::new(&mut self.particle_forge.precession, 0.0..=1.0)
                    .text("Precession"),
            );
            ui.checkbox(&mut self.particle_forge.reverse, "Reverse direction");
            ui.checkbox(&mut self.particle_forge.auto_camera, "Automatic camera");
            gradient_value_control(
                ui,
                "Gradient shift",
                &mut self.particle_forge.gradient_shift,
                0.0,
                1.0,
                self.colors.bass,
                self.colors.treble,
            );
            ui.separator();
            ui.label("Material families");
            ui.add(
                egui::Slider::new(&mut self.particle_forge.materials.energy, 0.0..=1.0)
                    .text("Energy"),
            );
            ui.add(
                egui::Slider::new(&mut self.particle_forge.materials.cyber, 0.0..=1.0)
                    .text("Cyber"),
            );
            ui.add(
                egui::Slider::new(&mut self.particle_forge.materials.cosmic, 0.0..=1.0)
                    .text("Cosmic"),
            );
        }
    }

    fn draw_mode_panel(&mut self, ctx: &egui::Context) {
        let mut open = self.mode_panel;
        egui::Window::new("MODE CONTROLS")
            .id(egui::Id::new("mode-panel"))
            .anchor(egui::Align2::RIGHT_BOTTOM, [-20.0, -20.0])
            .default_width(340.0)
            .resizable(true)
            .open(&mut open)
            .show(ctx, |ui| self.draw_mode_controls(ui));
        self.mode_panel = open;
    }

    fn draw_forge_panel(&mut self, ctx: &egui::Context) {
        let mut open = self.forge_panel;
        egui::Window::new("FORGE NODE")
            .id(egui::Id::new("forge-panel"))
            .anchor(egui::Align2::LEFT_BOTTOM, [20.0, -20.0])
            .default_width(360.0)
            .resizable(true)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for index in 0..self.particle_forge.nodes.len() {
                        if ui
                            .selectable_label(
                                self.particle_forge.selected == Some(index),
                                format!("{}", index + 1),
                            )
                            .clicked()
                        {
                            self.particle_forge.selected = Some(index);
                        }
                    }
                    if ui
                        .add_enabled(
                            self.particle_forge.nodes.len() < particle_forge::MAX_FORGE_NODES,
                            egui::Button::new("Add"),
                        )
                        .clicked()
                    {
                        self.particle_forge.add_node([0.0, 0.0, 0.0]);
                    }
                });
                let mut remove = false;
                if let Some(node) = self
                    .particle_forge
                    .selected
                    .and_then(|index| self.particle_forge.nodes.get_mut(index))
                {
                    egui::ComboBox::from_id_salt("forge-force-kind")
                        .selected_text(node.kind.label())
                        .show_ui(ui, |ui| {
                            for kind in ForgeForceKind::ALL {
                                ui.selectable_value(&mut node.kind, kind, kind.label());
                            }
                        });
                    egui::ComboBox::from_id_salt("forge-node-band")
                        .selected_text(["Full", "Low", "Mid", "High"][node.band as usize])
                        .show_ui(ui, |ui| {
                            for (band, label) in ["Full", "Low", "Mid", "High"].iter().enumerate() {
                                ui.selectable_value(&mut node.band, band as u8, *label);
                            }
                        });
                    ui.add(egui::Slider::new(&mut node.radius, 0.08..=1.5).text("Radius"));
                    ui.add(egui::Slider::new(&mut node.strength, 0.0..=3.0).text("Strength"));
                    ui.add(egui::Slider::new(&mut node.falloff, 0.25..=4.0).text("Falloff"));
                    ui.add(egui::Slider::new(&mut node.position[2], -2.5..=2.5).text("Depth Z"));
                    for (axis, label) in ["Axis X", "Axis Y", "Axis Z"].into_iter().enumerate() {
                        ui.add(
                            egui::Slider::new(&mut node.spin_axis[axis], -1.0..=1.0).text(label),
                        );
                    }
                    ui.checkbox(&mut node.pinned, "Pinned");
                    ui.checkbox(&mut self.particle_forge.gizmo, "Show XYZ gizmo");
                    remove = ui
                        .add_enabled(!node.pinned, egui::Button::new("Remove node"))
                        .clicked();
                }
                if remove && let Some(index) = self.particle_forge.selected {
                    self.particle_forge.nodes.remove(index);
                    self.particle_forge.selected = (!self.particle_forge.nodes.is_empty())
                        .then(|| index.min(self.particle_forge.nodes.len() - 1));
                }
                ui.separator();
                egui::CollapsingHeader::new(format!(
                    "Modulation routes · {}/{}",
                    self.particle_forge.routes.len(),
                    particle_forge::MAX_FORGE_ROUTES
                ))
                .show(ui, |ui| {
                    if ui
                        .add_enabled(
                            self.particle_forge.routes.len() < particle_forge::MAX_FORGE_ROUTES,
                            egui::Button::new("Add route"),
                        )
                        .clicked()
                    {
                        self.particle_forge
                            .routes
                            .push(particle_forge::ForgeModRoute {
                                source: 0,
                                target: 0,
                                amount: 0.5,
                                enabled: true,
                            });
                    }
                    let mut remove_route = None;
                    for (index, route) in self.particle_forge.routes.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut route.enabled, "");
                            ui.add(egui::DragValue::new(&mut route.source).range(0..=5));
                            ui.label("→");
                            ui.add(egui::DragValue::new(&mut route.target).range(0..=7));
                            ui.add(
                                egui::Slider::new(&mut route.amount, -2.0..=2.0).show_value(true),
                            );
                            if ui.small_button("×").clicked() {
                                remove_route = Some(index);
                            }
                        });
                    }
                    if let Some(index) = remove_route {
                        self.particle_forge.routes.remove(index);
                    }
                });
            });
        self.forge_panel = open;
    }

    fn draw_instrument_panel(&mut self, ctx: &egui::Context) {
        let mut open = self.instrument_panel;
        egui::Window::new("INSTRUMENT")
            .id(egui::Id::new("instrument-panel"))
            .anchor(egui::Align2::RIGHT_TOP, [-20.0, 20.0])
            .default_width(400.0)
            .min_width(340.0)
            .max_width(520.0)
            .resizable(true)
            .vscroll(true)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.set_max_width(480.0);
                let active_id = self.gpu_preset.as_ref().map(GpuPresetRenderer::active_id);
                let mode_name = match self.visual {
                    0 => "Neon Scope".to_owned(),
                    1 => "Particle Forge".to_owned(),
                    2 => active_id
                        .and_then(|id| self.presets.iter().find(|preset| preset.id == id))
                        .map_or_else(
                            || "No valid preset".to_owned(),
                            |preset| preset.name.clone(),
                        ),
                    _ => "Living Photograph".to_owned(),
                };
                let mut chosen_visual = None;
                let mut chosen_preset = None;
                ui.horizontal(|ui| {
                    ui.label("Mode");
                    egui::ComboBox::from_id_salt("instrument-mode")
                        .width(245.0)
                        .selected_text(&mode_name)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(self.visual == 0, "Neon Scope")
                                .clicked()
                            {
                                chosen_visual = Some(0);
                            }
                            if ui
                                .selectable_label(self.visual == 1, "Particle Forge")
                                .clicked()
                            {
                                chosen_visual = Some(1);
                            }
                            if ui
                                .selectable_label(self.visual == 3, "Studio · Living Photograph")
                                .clicked()
                            {
                                chosen_visual = Some(3);
                            }
                            ui.separator();
                            for (index, preset) in self.presets.iter().enumerate() {
                                if ui
                                    .selectable_label(
                                        self.visual == 2 && active_id == Some(preset.id.as_str()),
                                        &preset.name,
                                    )
                                    .clicked()
                                {
                                    chosen_visual = Some(2);
                                    chosen_preset = Some(index);
                                }
                            }
                        });
                });
                if let Some(visual) = chosen_visual {
                    self.visual = visual;
                }
                if let Some(index) = chosen_preset {
                    self.load_preset(index);
                }
                ui.horizontal(|ui| {
                    ui.checkbox(&mut self.show_text, "Show visual text [L]");
                    if ui.button("Mode controls [O]").clicked() {
                        self.mode_panel = true;
                    }
                    if ui.button("Revert all [Shift+R]").clicked() {
                        self.revert_all(ctx);
                    }
                });
                ui.label(
                    egui::RichText::new(if self.visual == 3 {
                        "Canvas · right-click toggles the selected layer · right-drag adjusts its scale/reactivity"
                    } else {
                        "Canvas · right-click empty adds a zone · right-click a zone cycles its band · right-drag tunes"
                    })
                    .small()
                    .color(Color32::from_rgb(110, 145, 165)),
                );

                if self.visual == 3 {
                    self.draw_studio_controls(ui);
                }

                if self.visual == 3 {
                    ui.label(
                        egui::RichText::new(
                            "Studio compositions are session-only in this prototype.",
                        )
                        .small()
                        .color(Color32::from_rgb(125, 150, 170)),
                    );
                } else {
                let mut save_scene = false;
                let mut refresh_scenes = false;
                let mut restore_selected_scene = false;
                egui::CollapsingHeader::new(format!("Saved Scenes · {}", self.scenes.len()))
                    .id_salt("instrument-saved-scenes")
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(
                                "Capture this mode's sliders, colors, zones, materials, and camera.",
                            )
                            .small()
                            .color(Color32::from_rgb(145, 170, 190)),
                        );
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.scene_name)
                                    .hint_text(format!("{mode_name} view"))
                                    .desired_width(245.0),
                            );
                            if ui.button("Save").clicked() {
                                save_scene = true;
                            }
                            if ui.small_button("Refresh").clicked() {
                                refresh_scenes = true;
                            }
                        });
                        if !self.scenes.is_empty() {
                            let selected = self
                                .scenes
                                .get(self.selected_scene)
                                .map_or("Select a saved scene", |saved| {
                                    saved.snapshot.name.as_str()
                                });
                            ui.horizontal(|ui| {
                                egui::ComboBox::from_id_salt("instrument-saved-scene")
                                    .selected_text(selected)
                                    .width(285.0)
                                    .show_ui(ui, |ui| {
                                        for (index, saved) in self.scenes.iter().enumerate() {
                                            ui.selectable_value(
                                                &mut self.selected_scene,
                                                index,
                                                format!(
                                                    "{} · {}",
                                                    saved.snapshot.name, saved.snapshot.mode_name
                                                ),
                                            );
                                        }
                                    });
                                if ui.button("Restore").clicked() {
                                    restore_selected_scene = true;
                                }
                            });
                        }
                        if let Some(notice) = &self.scene_notice {
                            ui.label(
                                egui::RichText::new(notice)
                                    .small()
                                    .color(Color32::from_rgb(145, 190, 170)),
                            );
                        }
                        ui.label(
                            egui::RichText::new(format!(
                                "Folder · {}",
                                self.scene_directory.display()
                            ))
                            .small()
                            .color(Color32::from_rgb(100, 125, 145)),
                        );
                    });
                if save_scene {
                    self.save_scene();
                }
                if refresh_scenes {
                    self.refresh_scenes();
                }
                if restore_selected_scene {
                    self.restore_scene(self.selected_scene);
                }
                }

                ui.separator();
                ui.label(
                    egui::RichText::new("LIVE AUDIO ROUTING")
                        .small()
                        .strong()
                        .color(Color32::from_rgb(90, 245, 220)),
                );
                for (label, value, color) in [
                    (
                        "Full",
                        (self.features.rms * 3.0).clamp(0.0, 1.0),
                        self.colors.full,
                    ),
                    ("Bass", self.features.low, self.colors.bass),
                    ("Mid", self.features.mid, self.colors.mid),
                    ("Treble", self.features.high, self.colors.treble),
                ] {
                    ui.horizontal(|ui| {
                        ui.add_sized([52.0, 18.0], egui::Label::new(label));
                        ui.add(
                            egui::ProgressBar::new(value.clamp(0.0, 1.0))
                                .desired_width(ui.available_width())
                                .fill(color)
                                .text(format!("{value:.2}")),
                        );
                    });
                }

                ui.separator();
                if ui.button("Visual Director (Experimental)…").clicked() {
                    self.visual_director_panel = true;
                }
                let mut director_open = self.visual_director_panel;
                if director_open {
                    egui::Window::new("VISUAL DIRECTOR · EXPERIMENTAL")
                        .id(egui::Id::new("visual-director-panel"))
                        .default_width(520.0)
                        .min_width(420.0)
                        .max_width(720.0)
                        .resizable(true)
                        .vscroll(true)
                        .open(&mut director_open)
                        .show(ctx, |ui| {
                            ui.set_max_width(680.0);
                        ui.label(
                            egui::RichText::new(
                                "Turns the current mode, routed colors, and live audio into an \
                                 inspectable scene specification. No API request is sent.",
                            )
                            .small()
                            .color(Color32::from_rgb(145, 170, 190)),
                        );
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("director-intent")
                                .selected_text(self.visual_director.intent.label())
                                .show_ui(ui, |ui| {
                                    for intent in OutputIntent::ALL {
                                        ui.selectable_value(
                                            &mut self.visual_director.intent,
                                            intent,
                                            intent.label(),
                                        );
                                    }
                                });
                            egui::ComboBox::from_id_salt("director-aspect")
                                .selected_text(self.visual_director.aspect.label())
                                .show_ui(ui, |ui| {
                                    for aspect in Aspect::ALL {
                                        ui.selectable_value(
                                            &mut self.visual_director.aspect,
                                            aspect,
                                            aspect.label(),
                                        );
                                    }
                                });
                        });
                        ui.horizontal(|ui| {
                            ui.label("Capture");
                            egui::ComboBox::from_id_salt("director-capture")
                                .selected_text(self.visual_director.capture_profile.label())
                                .show_ui(ui, |ui| {
                                    for profile in CaptureProfile::ALL {
                                        ui.selectable_value(
                                            &mut self.visual_director.capture_profile,
                                            profile,
                                            profile.label(),
                                        );
                                    }
                                });
                            ui.label(format!(
                                "{} · {}",
                                self.visual_director.intent.quality(),
                                self.visual_director
                                    .aspect
                                    .size(self.visual_director.intent)
                            ));
                        });

                        egui::Grid::new("director-controls")
                            .num_columns(2)
                            .spacing([8.0, 3.0])
                            .show(ui, |ui| {
                                for (label, value) in [
                                    (
                                        "Music influence",
                                        &mut self.visual_director.controls.music_influence,
                                    ),
                                    (
                                        "Photorealism",
                                        &mut self.visual_director.controls.photorealism,
                                    ),
                                    (
                                        "Abstraction",
                                        &mut self.visual_director.controls.abstraction,
                                    ),
                                    ("Chaos", &mut self.visual_director.controls.chaos),
                                    ("Weirdness", &mut self.visual_director.controls.weirdness),
                                    (
                                        "Human presence",
                                        &mut self.visual_director.controls.human_presence,
                                    ),
                                    (
                                        "Era freedom",
                                        &mut self.visual_director.controls.era_freedom,
                                    ),
                                    (
                                        "Color freedom",
                                        &mut self.visual_director.controls.color_freedom,
                                    ),
                                    (
                                        "Environment",
                                        &mut self
                                            .visual_director
                                            .controls
                                            .environmental_complexity,
                                    ),
                                    ("Novelty", &mut self.visual_director.controls.novelty),
                                ] {
                                    ui.label(label);
                                    ui.add(
                                        egui::Slider::new(value, 0.0..=1.0)
                                            .show_value(true)
                                            .fixed_decimals(2),
                                    );
                                    ui.end_row();
                                }
                            });

                        if ui.button("Compose new brief from live audio").clicked() {
                            let routed_colors = format!(
                                "full {}, bass {}, mids {}, treble {}, background {}",
                                color_hex(self.colors.full),
                                color_hex(self.colors.bass),
                                color_hex(self.colors.mid),
                                color_hex(self.colors.treble),
                                color_hex(self.colors.background),
                            );
                            self.visual_director.compose(
                                &mode_name,
                                &self.features,
                                self.colors.palette.label(),
                                self.colors.finish.label(),
                                &routed_colors,
                            );
                        }

                        let mut export_brief = false;
                        if let Some(brief) = &self.visual_director.brief {
                            ui.separator();
                            ui.label(
                                egui::RichText::new(&brief.concept)
                                    .strong()
                                    .color(Color32::from_rgb(235, 245, 255)),
                            );
                            ui.label(format!(
                                "DNA · {} · {} dominant · energy {:.2} · dynamics {:.2}",
                                brief.visual_dna.passage,
                                brief.visual_dna.dominant_band,
                                brief.visual_dna.energy,
                                brief.visual_dna.dynamics,
                            ));
                            egui::Grid::new("director-dna")
                                .num_columns(4)
                                .spacing([8.0, 2.0])
                                .show(ui, |ui| {
                                    for (index, (label, value)) in [
                                        ("Bass", brief.visual_dna.bass),
                                        ("Mids", brief.visual_dna.mids),
                                        ("Treble", brief.visual_dna.treble),
                                        ("Centroid", brief.visual_dna.spectral_center),
                                        ("Rolloff", brief.visual_dna.spectral_rolloff),
                                        ("Harmonic", brief.visual_dna.harmonic_density),
                                        ("Transient", brief.visual_dna.transient_intensity),
                                        ("Onsets", brief.visual_dna.onset_density),
                                        ("Movement", brief.visual_dna.movement),
                                    ]
                                    .into_iter()
                                    .enumerate()
                                    {
                                        ui.label(label);
                                        ui.label(format!("{value:.2}"));
                                        if index % 2 == 1 {
                                            ui.end_row();
                                        }
                                    }
                                    ui.end_row();
                                });
                            egui::CollapsingHeader::new("Scene specification")
                                .id_salt("director-scene-specification")
                                .show(ui, |ui| {
                                    ui.label(format!("Subject · {}", brief.subject));
                                    ui.label(format!("Environment · {}", brief.environment));
                                    ui.label(format!("Era · {}", brief.era));
                                    ui.label(format!("Event · {}", brief.event));
                                    ui.label(format!(
                                        "Capture · {}",
                                        brief.capture_profile.label()
                                    ));
                                    ui.label(format!("Scene · {}", brief.scene));
                                    ui.label(format!("Camera · {}", brief.camera));
                                    ui.label(format!("Lighting · {}", brief.lighting));
                                    ui.label(format!("Materials · {}", brief.materials));
                                    ui.label(format!("Motion · {}", brief.motion));
                                    ui.label(format!(
                                        "Imperfections · {}",
                                        brief.imperfections.join("; ")
                                    ));
                                    ui.label(format!("Novelty ID · {:016X}", brief.novelty_id));
                                });
                            egui::CollapsingHeader::new("Preflight critique")
                                .id_salt("director-preflight")
                                .show(ui, |ui| {
                                    for (label, value, color) in [
                                        (
                                            "Concept novelty",
                                            brief.preflight.novelty,
                                            Color32::from_rgb(20, 215, 180),
                                        ),
                                        (
                                            "Physical plausibility",
                                            brief.preflight.plausibility,
                                            Color32::from_rgb(90, 165, 255),
                                        ),
                                        (
                                            "AI cliché risk",
                                            brief.preflight.cliche_risk,
                                            Color32::from_rgb(255, 90, 125),
                                        ),
                                    ] {
                                        ui.add(
                                            egui::ProgressBar::new(value)
                                                .text(format!("{label} · {value:.2}"))
                                                .fill(color),
                                        );
                                    }
                                    for note in &brief.preflight.notes {
                                        ui.label(format!("· {note}"));
                                    }
                                });
                            let mut prompt = brief.prompt.clone();
                            let prompt_width = ui.available_width().min(480.0);
                            ui.add(
                                egui::TextEdit::multiline(&mut prompt)
                                    .desired_rows(9)
                                    .desired_width(prompt_width)
                                    .interactive(false),
                            );
                            ui.horizontal(|ui| {
                                if ui.button("Copy generation prompt").clicked() {
                                    ui.ctx().copy_text(brief.prompt.clone());
                                }
                                if ui.button("Export brief .md").clicked() {
                                    export_brief = true;
                                }
                            });
                        }
                        if export_brief {
                            self.visual_director.export_current_brief();
                        }
                        if let Some(notice) = &self.visual_director.export_notice {
                            ui.label(
                                egui::RichText::new(notice)
                                    .small()
                                    .color(Color32::from_rgb(145, 190, 170)),
                            );
                        }

                        if !self.visual_director.history().is_empty() {
                            egui::CollapsingHeader::new(format!(
                                "Brief history · {}",
                                self.visual_director.history().len()
                            ))
                            .id_salt("director-history")
                            .show(ui, |ui| {
                                for entry in self.visual_director.history().iter().rev().take(8) {
                                    ui.horizontal_wrapped(|ui| {
                                        ui.label(
                                            egui::RichText::new(&entry.concept)
                                                .strong()
                                                .color(Color32::from_rgb(205, 225, 240)),
                                        );
                                        if ui.small_button("Copy prompt").clicked() {
                                            ui.ctx().copy_text(entry.prompt.clone());
                                        }
                                    });
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} · {} · {} {} · {} · {:016X}",
                                            entry.mode,
                                            entry.capture,
                                            entry.intent,
                                            entry.size,
                                            entry.aspect,
                                            entry.novelty_id,
                                        ))
                                        .small()
                                        .color(Color32::from_rgb(130, 155, 175)),
                                    );
                                    ui.separator();
                                }
                                if let Some(path) = self.visual_director.history_location() {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "Stored locally · {}",
                                            path.display()
                                        ))
                                        .small(),
                                    );
                                }
                            });
                        }
                        if let Some(error) = &self.visual_director.history_error {
                            ui.colored_label(Color32::from_rgb(255, 160, 90), error);
                        }

                        let key_present = std::env::var_os("OPENAI_API_KEY").is_some();
                        ui.add_enabled(
                            false,
                            egui::Button::new(format!(
                                "Generate {} · API not connected",
                                self.visual_director.intent.label().to_ascii_lowercase()
                            )),
                        )
                        .on_hover_text(if key_present {
                            "A key is present, but this build intentionally has no unvalidated paid \
                             request path."
                        } else {
                            "OPENAI_API_KEY is not set. Brief composition remains fully local and \
                             cost-free."
                        });
                        });
                }
                self.visual_director_panel = director_open;

                egui::CollapsingHeader::new(format!(
                    "Sound Zones · {}/{}",
                    self.interaction.zones.len(),
                    MAX_SOUND_ZONES
                ))
                .id_salt("instrument-zones")
                .default_open(true)
                .show(ui, |ui| {
                    let mut select = None;
                    ui.horizontal_wrapped(|ui| {
                        for (index, zone) in self.interaction.zones.iter().enumerate() {
                            if ui
                                .selectable_label(
                                    self.interaction.selected == Some(index),
                                    format!("{} {}", index + 1, zone.band.label()),
                                )
                                .on_hover_text(format!(
                                    "{} · radius {:.2} · strength {:.2}{}",
                                    zone.band.label(),
                                    zone.radius,
                                    zone.strength,
                                    if zone.pinned { " · pinned" } else { "" }
                                ))
                                .clicked()
                            {
                                select = Some(index);
                            }
                        }
                    });
                    if let Some(index) = select {
                        self.interaction.selected = Some(index);
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                self.interaction.zones.len() < MAX_SOUND_ZONES,
                                egui::Button::new("Add zone"),
                            )
                            .clicked()
                        {
                            self.interaction.add_zone(Vec2::new(0.5, 0.5));
                        }
                        if ui.button("Reset zones").clicked() {
                            self.reset_interaction();
                        }
                    });

                    let mut remove = false;
                    if let Some(zone) = self
                        .interaction
                        .selected
                        .and_then(|index| self.interaction.zones.get_mut(index))
                    {
                        ui.separator();
                        ui.horizontal(|ui| {
                            egui::ComboBox::from_id_salt("instrument-zone-band")
                                .selected_text(format!("Band · {}", zone.band.label()))
                                .show_ui(ui, |ui| {
                                    for band in [
                                        ZoneBand::Full,
                                        ZoneBand::Low,
                                        ZoneBand::Mid,
                                        ZoneBand::High,
                                    ] {
                                        ui.selectable_value(&mut zone.band, band, band.label());
                                    }
                                });
                            ui.checkbox(&mut zone.pinned, "Pinned");
                        });
                        ui.add(
                            egui::Slider::new(&mut zone.radius, 0.04..=0.45)
                                .text("Radius")
                                .fixed_decimals(2),
                        );
                        ui.add(
                            egui::Slider::new(&mut zone.strength, 0.0..=3.0)
                                .text("Strength")
                                .fixed_decimals(2),
                        );
                        remove = ui
                            .add_enabled(!zone.pinned, egui::Button::new("Remove selected"))
                            .clicked();
                    } else {
                        ui.label("Select a zone here or directly in the visual.");
                    }
                    if remove {
                        self.interaction.remove_selected();
                    }
                });

                egui::CollapsingHeader::new("Colors & Materials")
                    .id_salt("instrument-colors")
                    .show(ui, |ui| {
                        let mut palette = self.colors.palette;
                        egui::ComboBox::from_id_salt("instrument-palette")
                            .selected_text(palette.label())
                            .show_ui(ui, |ui| {
                                for candidate in PalettePreset::ALL {
                                    ui.selectable_value(&mut palette, candidate, candidate.label());
                                }
                            });
                        if palette != self.colors.palette {
                            self.colors.apply_palette(palette);
                        }
                        egui::ComboBox::from_id_salt("instrument-finish")
                            .selected_text(self.colors.finish.label())
                            .show_ui(ui, |ui| {
                                for finish in SurfaceFinish::ALL {
                                    ui.selectable_value(
                                        &mut self.colors.finish,
                                        finish,
                                        finish.label(),
                                    );
                                }
                            });
                        let mut customized = false;
                        for (label, color) in [
                            ("Full range", &mut self.colors.full),
                            ("Bass", &mut self.colors.bass),
                            ("Mid", &mut self.colors.mid),
                            ("Treble", &mut self.colors.treble),
                            ("Background", &mut self.colors.background),
                        ] {
                            ui.horizontal(|ui| {
                                customized |= ui.color_edit_button_srgba(color).changed();
                                ui.label(label);
                            });
                        }
                        if customized {
                            self.colors.palette = PalettePreset::Custom;
                        }
                        gradient_value_control(
                            ui,
                            "Glow",
                            &mut self.colors.glow,
                            0.0,
                            2.0,
                            self.colors.background,
                            self.colors.full,
                        );
                        gradient_value_control(
                            ui,
                            "Gloss",
                            &mut self.colors.gloss,
                            0.0,
                            1.0,
                            Color32::from_rgb(45, 50, 58),
                            Color32::WHITE,
                        );
                        gradient_value_control(
                            ui,
                            "Saturation",
                            &mut self.colors.saturation,
                            0.0,
                            1.5,
                            Color32::from_gray(125),
                            self.colors.treble,
                        );
                    });

                egui::CollapsingHeader::new("Camera & Scene")
                    .id_salt("instrument-camera")
                    .show(ui, |ui| {
                        ui.add(
                            egui::Slider::new(
                                &mut self.interaction.camera_yaw,
                                -std::f32::consts::TAU..=std::f32::consts::TAU,
                            )
                            .text("Yaw")
                            .fixed_decimals(2),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.interaction.camera_pitch, -1.2..=1.2)
                                .text("Pitch")
                                .fixed_decimals(2),
                        );
                        ui.add(
                            egui::Slider::new(&mut self.interaction.camera_zoom, 0.35..=3.0)
                                .text("Zoom")
                                .fixed_decimals(2),
                        );
                        ui.checkbox(&mut self.interaction.show_handles, "Show zone handles");
                        if ui.button("Reset interaction").clicked() {
                            self.reset_interaction();
                        }
                    });

                ui.separator();
                ui.label(
                    egui::RichText::new(
                        "Left-drag zones · left-drag empty space to orbit · wheel zooms · R resets interaction",
                    )
                    .small()
                    .color(Color32::from_rgb(110, 145, 165)),
                );
            });
        self.instrument_panel = open;
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
        self.pace_frame();
        self.frame_stats.observe(Instant::now());
        self.keyboard(ctx);
        for dropped in ctx.input(|input| input.raw.dropped_files.clone()) {
            let result = dropped.path.as_deref().map_or_else(
                || Err("Only desktop file drops are supported.".to_owned()),
                |path| self.studio.load_image(ctx, path),
            );
            match result {
                Ok(()) => {
                    self.visual = 3;
                    self.instrument_panel = true;
                }
                Err(error) => self.studio.notice = Some(error),
            }
        }
        self.update_default_device();
        self.update_features();
        let forge_decay =
            (-6.0 * (self.frame_stats.current_ms as f32 / 1_000.0).clamp(0.0, 0.1)).exp();
        self.particle_forge.event_envelope = self
            .features
            .onset
            .max(self.particle_forge.event_envelope * forge_decay);
        self.studio.animate_motion(
            ctx,
            (self.frame_stats.current_ms as f32 / 1_000.0).clamp(0.0, 0.1),
            [
                self.features.low,
                self.features.mid,
                self.features.high,
                self.features.rms,
                self.features.onset,
            ],
        );
        self.update_plugin();
        let repaint = if self.visual == 1 {
            self.particle_forge
                .quality
                .frame_rate()
                .map_or(Duration::ZERO, |rate| {
                    Duration::from_secs_f64(1.0 / f64::from(rate))
                })
        } else {
            self.frame_limit
                .interval()
                .unwrap_or(Duration::from_millis(16))
        };
        ctx.request_repaint_after(repaint);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let rect = ui.max_rect();
        let response = ui.interact(
            rect,
            ui.id().with("visual-interaction"),
            egui::Sense::click_and_drag(),
        );
        self.interact_visual(&response, rect);
        self.draw_visual(&ui.painter_at(rect), rect);
        if self.visual == 1 {
            self.draw_forge_handles(&ui.painter_at(rect), rect);
        } else {
            self.draw_zone_handles(&ui.painter_at(rect), rect);
        }
        if self.overlay && self.show_text {
            self.draw_overlay(ui.ctx());
        }
        if self.instrument_panel {
            self.draw_instrument_panel(ui.ctx());
        }
        if self.mode_panel {
            self.draw_mode_panel(ui.ctx());
        }
        if self.forge_panel && self.visual == 1 {
            self.draw_forge_panel(ui.ctx());
        }
        if ui.ctx().input(|input| !input.raw.hovered_files.is_empty()) {
            ui.painter().rect_filled(
                rect.shrink(rect.width().min(rect.height()) * 0.12),
                12.0,
                Color32::from_black_alpha(205),
            );
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "DROP IMAGE INTO STUDIO",
                egui::FontId::proportional(24.0),
                Color32::from_rgb(90, 245, 220),
            );
        }
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.01, 0.015, 0.035, 1.0]
    }
}

fn normalize_visual_position(rect: Rect, position: Pos2) -> Vec2 {
    Vec2::new(
        ((position.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0),
        ((position.y - rect.top()) / rect.height().max(1.0)).clamp(0.0, 1.0),
    )
}

fn visual_position(rect: Rect, position: Vec2) -> Pos2 {
    Pos2::new(
        egui::lerp(rect.left()..=rect.right(), position.x),
        egui::lerp(rect.top()..=rect.bottom(), position.y),
    )
}

fn forge_node_position(rect: Rect, position: [f32; 3]) -> Pos2 {
    let depth_scale = (1.0 + position[2] * 0.12).clamp(0.55, 1.45);
    Pos2::new(
        rect.center().x + position[0] / 4.0 * rect.width() * depth_scale,
        rect.center().y - position[1] / 2.8 * rect.height() * depth_scale,
    )
}

fn shifted_frequency(frequency: f32, shift: f32) -> f32 {
    (frequency + shift).rem_euclid(1.0)
}

fn gradient_value_control(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    minimum: f32,
    maximum: f32,
    from: Color32,
    to: Color32,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        changed |= ui
            .add(
                egui::DragValue::new(value)
                    .range(minimum..=maximum)
                    .speed((maximum - minimum) / 100.0)
                    .fixed_decimals(2),
            )
            .changed();
    });
    let desired_size = Vec2::new(ui.available_width().max(120.0), 16.0);
    let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click_and_drag());
    let segments = 48;
    for segment in 0..segments {
        let start = segment as f32 / segments as f32;
        let end = (segment + 1) as f32 / segments as f32;
        ui.painter().rect_filled(
            Rect::from_min_max(
                Pos2::new(egui::lerp(rect.left()..=rect.right(), start), rect.top()),
                Pos2::new(egui::lerp(rect.left()..=rect.right(), end), rect.bottom()),
            ),
            0.0,
            lerp_color(from, to, (start + end) * 0.5),
        );
    }
    if (response.dragged() || response.clicked())
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let amount = ((pointer.x - rect.left()) / rect.width().max(1.0)).clamp(0.0, 1.0);
        let next = egui::lerp(minimum..=maximum, amount);
        changed |= (*value - next).abs() > f32::EPSILON;
        *value = next;
    }
    let amount = ((*value - minimum) / (maximum - minimum).max(f32::EPSILON)).clamp(0.0, 1.0);
    let x = egui::lerp(rect.left()..=rect.right(), amount);
    ui.painter().line_segment(
        [
            Pos2::new(x, rect.top() - 2.0),
            Pos2::new(x, rect.bottom() + 2.0),
        ],
        Stroke::new(2.0, Color32::WHITE),
    );
    ui.painter().rect_stroke(
        rect,
        3.0,
        Stroke::new(1.0, Color32::from_white_alpha(90)),
        egui::StrokeKind::Inside,
    );
    changed
}

fn lerp_color(from: Color32, to: Color32, amount: f32) -> Color32 {
    Color32::from_rgba_unmultiplied(
        egui::lerp(from.r() as f32..=to.r() as f32, amount) as u8,
        egui::lerp(from.g() as f32..=to.g() as f32, amount) as u8,
        egui::lerp(from.b() as f32..=to.b() as f32, amount) as u8,
        egui::lerp(from.a() as f32..=to.a() as f32, amount) as u8,
    )
}

fn color_hex(color: Color32) -> String {
    format!("#{:02X}{:02X}{:02X}", color.r(), color.g(), color.b())
}

fn preset_parameter_defaults(preset: &Preset) -> [f32; PRESET_PARAMETER_FLOATS] {
    let mut values = [0.0; PRESET_PARAMETER_FLOATS];
    for (value, parameter) in values.iter_mut().zip(&preset.parameters) {
        *value = parameter.default;
    }
    values
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

fn status_hint(status: &str, source: Option<SourceKind>) -> Option<&'static str> {
    match (status, source) {
        ("WAITING", Some(SourceKind::System)) => Some("Listening for system audio…"),
        ("WAITING", Some(SourceKind::Microphone)) => Some("Starting microphone capture…"),
        ("SILENT", Some(SourceKind::System)) => Some("No signal · play audio on this device"),
        ("SILENT", Some(SourceKind::Microphone)) => {
            Some("No signal · make sound near the microphone")
        }
        ("ERROR", _) => Some("Capture unavailable · review the message below"),
        _ => None,
    }
}

fn preset_directory() -> PathBuf {
    resource_directory("THEVISUALIZER_PRESETS", "presets")
}

fn asset_directory() -> PathBuf {
    resource_directory("THEVISUALIZER_ASSETS", "assets")
}

fn bundled_studio(ctx: &egui::Context) -> StudioState {
    let mut studio = StudioState::default();
    let image = asset_directory().join("living-photograph-greenhouse.png");
    if image.is_file()
        && let Err(error) = studio.load_image(ctx, &image)
    {
        studio.notice = Some(error);
    }
    let motion = asset_directory().join("living-photograph-greenhouse-motion.webp");
    if motion.is_file()
        && let Err(error) = studio.load_motion(ctx, &motion)
    {
        studio.notice = Some(error);
    }
    studio
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
        ColorSystem, DefaultSwitchTiming, Features, FrameLimit, FrameStats, InteractionState,
        LatencyStats, MAX_SOUND_ZONES, OnsetStats, PalettePreset, PresentationMode, SourceKind,
        VisualHistory, ZoneBand, callback_is_new, default_needs_recovery, lerp_color,
        living_photo_plant_mask, pacing_delay, rare_bird_progress, shifted_frequency, status_hint,
        visual_index_for_key,
    };
    use eframe::egui::Pos2;
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
    fn interaction_zones_are_bounded_and_palettes_route_band_colors() {
        let mut interaction = InteractionState::default();
        for _ in 0..MAX_SOUND_ZONES + 3 {
            interaction.add_zone(eframe::egui::Vec2::new(0.25, 0.75));
        }
        assert_eq!(interaction.zones.len(), MAX_SOUND_ZONES);

        interaction.selected = Some(0);
        interaction.zones[0].pinned = true;
        interaction.remove_selected();
        assert_eq!(interaction.zones.len(), MAX_SOUND_ZONES);
        interaction.zones[0].pinned = false;
        interaction.remove_selected();
        assert_eq!(interaction.zones.len(), MAX_SOUND_ZONES - 1);

        let mut colors = ColorSystem::default();
        colors.apply_palette(PalettePreset::Inferno);
        assert_eq!(colors.bass, eframe::egui::Color32::from_rgb(255, 35, 15));
        assert_eq!(colors.treble, eframe::egui::Color32::from_rgb(255, 220, 75));
    }

    #[test]
    fn zone_band_cycle_returns_to_full_range() {
        let band = ZoneBand::Full.next().next().next().next();
        assert!(band == ZoneBand::Full);
    }

    #[test]
    fn particle_gradient_shift_wraps_around() {
        assert!((shifted_frequency(0.8, 0.35) - 0.15).abs() < f32::EPSILON * 4.0);
    }

    #[test]
    fn gradient_colors_keep_their_endpoints() {
        let dark = eframe::egui::Color32::from_rgb(10, 20, 30);
        let bright = eframe::egui::Color32::from_rgb(110, 120, 130);
        assert_eq!(lerp_color(dark, bright, 0.0), dark);
        assert_eq!(lerp_color(dark, bright, 1.0), bright);
        assert_eq!(
            lerp_color(dark, bright, 0.5),
            eframe::egui::Color32::from_rgb(60, 70, 80)
        );
    }

    #[test]
    fn quiet_status_guidance_matches_the_active_source() {
        assert_eq!(
            status_hint("SILENT", Some(SourceKind::System)),
            Some("No signal · play audio on this device")
        );
        assert_eq!(
            status_hint("SILENT", Some(SourceKind::Microphone)),
            Some("No signal · make sound near the microphone")
        );
        assert_eq!(status_hint("LIVE", Some(SourceKind::System)), None);
        assert!(status_hint("ERROR", None).is_some());
    }

    #[test]
    fn frame_pacing_waits_only_for_limited_modes() {
        assert_eq!(
            pacing_delay(FrameLimit::Fps60, Duration::from_millis(10)),
            Some(Duration::from_secs_f64(1.0 / 60.0) - Duration::from_millis(10))
        );
        assert_eq!(
            pacing_delay(FrameLimit::Fps30, Duration::from_millis(40)),
            None
        );
        assert_eq!(
            pacing_delay(FrameLimit::Display, Duration::from_millis(1)),
            None
        );
    }

    #[test]
    fn number_keys_select_each_visual_directly() {
        assert_eq!(visual_index_for_key(eframe::egui::Key::Num1), Some(0));
        assert_eq!(visual_index_for_key(eframe::egui::Key::Num2), Some(1));
        assert_eq!(visual_index_for_key(eframe::egui::Key::Num3), Some(2));
        assert_eq!(visual_index_for_key(eframe::egui::Key::Num4), Some(3));
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
    fn onset_stats_count_only_events() {
        let started = Instant::now();
        let mut stats = OnsetStats::default();
        stats.observe(0.0, started);
        stats.observe(0.5, started);
        assert_eq!(stats.count, 0);
        assert!(stats.last.is_none());

        stats.observe(1.0, started);
        stats.observe(0.0, started + Duration::from_millis(10));
        assert_eq!(stats.count, 1);
        assert_eq!(stats.last, Some(started));
        assert_eq!(stats.per_minute(started + Duration::from_secs(60)), 1.0);
    }

    #[test]
    fn living_photo_masks_and_rare_bird_stay_bounded() {
        assert!(living_photo_plant_mask(Pos2::new(0.16, 0.56)) > 0.9);
        assert_eq!(living_photo_plant_mask(Pos2::new(0.5, 0.9)), 0.0);
        assert_eq!(rare_bird_progress(10.0), None);
        assert_eq!(rare_bird_progress(37.0), Some(0.0));
        assert!(rare_bird_progress(38.2).is_some());
        assert_eq!(rare_bird_progress(40.0), None);
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

        for sequence in 3..=super::PRESET_HISTORY_ROWS as u64 + 2 {
            history.update(sequence, &features);
        }
        assert_eq!(history.waveforms.len(), super::VISUAL_TRAIL_FRAMES);
        assert_eq!(history.spectra.len(), super::PRESET_HISTORY_ROWS);
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
