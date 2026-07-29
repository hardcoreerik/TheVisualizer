use std::{
    fs::{self, File},
    io::BufReader,
    path::{Path, PathBuf},
};

use eframe::egui::{self, TextureHandle, TextureOptions};
use image::{AnimationDecoder, ImageReader, Limits, codecs::webp::WebPDecoder};

pub const MAX_STUDIO_LAYERS: usize = 8;
const MAX_IMAGE_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_IMAGE_EDGE: u32 = 8_192;
const MAX_IMAGE_ALLOC_BYTES: u64 = 256 * 1024 * 1024;
const MAX_MOTION_FRAMES: usize = 64;
const MAX_MOTION_ALLOC_BYTES: usize = 192 * 1024 * 1024;
const MOTION_FPS: f32 = 24.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StudioLayerKind {
    Image,
    Preset,
    Waveform,
    Particles,
}

impl StudioLayerKind {
    pub const ALL: [Self; 4] = [Self::Image, Self::Preset, Self::Waveform, Self::Particles];
    const LIVING_PHOTOGRAPH: [Self; 3] = [Self::Image, Self::Waveform, Self::Particles];

    pub fn label(self) -> &'static str {
        match self {
            Self::Image => "Image",
            Self::Preset => "GPU Preset",
            Self::Waveform => "Waveform",
            Self::Particles => "Particles",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StudioBlend {
    Normal,
    Additive,
}

impl StudioBlend {
    pub const ALL: [Self; 2] = [Self::Normal, Self::Additive];

    pub fn label(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Additive => "Add",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum StudioBand {
    Full,
    Bass,
    Mid,
    Treble,
}

impl StudioBand {
    pub const ALL: [Self; 4] = [Self::Full, Self::Bass, Self::Mid, Self::Treble];

    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Bass => "Bass",
            Self::Mid => "Mid",
            Self::Treble => "Treble",
        }
    }
}

#[derive(Clone)]
pub struct StudioLayer {
    pub kind: StudioLayerKind,
    pub visible: bool,
    pub opacity: f32,
    pub blend: StudioBlend,
    pub band: StudioBand,
    pub reactivity: f32,
    pub scale: f32,
}

impl StudioLayer {
    fn living_photograph(kind: StudioLayerKind) -> Self {
        match kind {
            StudioLayerKind::Image => Self {
                kind,
                visible: true,
                opacity: 1.0,
                blend: StudioBlend::Normal,
                band: StudioBand::Full,
                reactivity: 0.65,
                scale: 1.0,
            },
            StudioLayerKind::Preset => Self {
                kind,
                visible: true,
                opacity: 1.0,
                blend: StudioBlend::Normal,
                band: StudioBand::Full,
                reactivity: 0.8,
                scale: 1.0,
            },
            StudioLayerKind::Waveform => Self {
                kind,
                visible: true,
                opacity: 0.72,
                blend: StudioBlend::Additive,
                band: StudioBand::Mid,
                reactivity: 0.9,
                scale: 0.7,
            },
            StudioLayerKind::Particles => Self {
                kind,
                visible: true,
                opacity: 0.5,
                blend: StudioBlend::Additive,
                band: StudioBand::Treble,
                reactivity: 1.1,
                scale: 0.72,
            },
        }
    }
}

pub struct StudioImage {
    pub path: PathBuf,
    pub size: [usize; 2],
    pub texture: TextureHandle,
    pub motion: Option<StudioMotion>,
}

pub struct StudioMotion {
    frames: Vec<egui::ColorImage>,
    pub texture: TextureHandle,
    pub mix: f32,
    phase: f32,
    frame: usize,
}

pub struct StudioState {
    pub layers: Vec<StudioLayer>,
    pub selected: usize,
    pub image: Option<StudioImage>,
    pub notice: Option<String>,
}

impl Default for StudioState {
    fn default() -> Self {
        let mut layers = StudioLayerKind::LIVING_PHOTOGRAPH
            .into_iter()
            .map(StudioLayer::living_photograph)
            .collect::<Vec<_>>();
        for layer in layers.iter_mut().skip(1) {
            layer.visible = false;
        }
        Self {
            layers,
            selected: 0,
            image: None,
            notice: None,
        }
    }
}

impl StudioState {
    pub fn add(&mut self, kind: StudioLayerKind) -> bool {
        if self.layers.len() == MAX_STUDIO_LAYERS
            || (matches!(kind, StudioLayerKind::Image | StudioLayerKind::Preset)
                && self.layers.iter().any(|layer| layer.kind == kind))
        {
            return false;
        }
        if kind == StudioLayerKind::Preset {
            self.layers.insert(0, StudioLayer::living_photograph(kind));
            self.selected = 0;
        } else {
            self.layers.push(StudioLayer::living_photograph(kind));
            self.selected = self.layers.len() - 1;
        }
        true
    }

    pub fn remove_selected(&mut self) -> bool {
        if self.layers.len() <= 1 || self.selected >= self.layers.len() {
            return false;
        }
        self.layers.remove(self.selected);
        self.selected = self.selected.min(self.layers.len() - 1);
        true
    }

    pub fn move_selected(&mut self, offset: isize) -> bool {
        if self.selected >= self.layers.len() {
            return false;
        }
        if self.layers[self.selected].kind == StudioLayerKind::Preset {
            return false;
        }
        let target = self
            .selected
            .saturating_add_signed(offset)
            .max(usize::from(
                self.layers.first().map(|layer| layer.kind) == Some(StudioLayerKind::Preset),
            ))
            .min(self.layers.len() - 1);
        if target == self.selected {
            return false;
        }
        self.layers.swap(self.selected, target);
        self.selected = target;
        true
    }

    pub fn load_image(&mut self, context: &egui::Context, path: &Path) -> Result<(), String> {
        let metadata = fs::metadata(path)
            .map_err(|error| format!("Could not inspect image {}: {error}", path.display()))?;
        if metadata.len() > MAX_IMAGE_FILE_BYTES {
            return Err(format!(
                "Image exceeds the {} MiB file limit",
                MAX_IMAGE_FILE_BYTES / 1024 / 1024
            ));
        }
        let mut reader = ImageReader::open(path)
            .map_err(|error| format!("Could not open image {}: {error}", path.display()))?
            .with_guessed_format()
            .map_err(|error| format!("Could not identify image {}: {error}", path.display()))?;
        let mut limits = Limits::default();
        limits.max_image_width = Some(MAX_IMAGE_EDGE);
        limits.max_image_height = Some(MAX_IMAGE_EDGE);
        limits.max_alloc = Some(MAX_IMAGE_ALLOC_BYTES);
        reader.limits(limits);
        let decoded = reader
            .decode()
            .map_err(|error| format!("Could not decode image {}: {error}", path.display()))?;
        let size = [decoded.width() as usize, decoded.height() as usize];
        let rgba = decoded.to_rgba8();
        let image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        let texture = context.load_texture(
            format!("studio:{}", path.display()),
            image,
            TextureOptions::LINEAR,
        );
        self.image = Some(StudioImage {
            path: path.to_owned(),
            size,
            texture,
            motion: None,
        });
        self.notice = Some(format!("Loaded image · {}", path.display()));
        Ok(())
    }

    pub fn load_motion(&mut self, context: &egui::Context, path: &Path) -> Result<(), String> {
        let image = self
            .image
            .as_mut()
            .ok_or_else(|| "Load the base image before its motion clip.".to_owned())?;
        let decoder =
            WebPDecoder::new(BufReader::new(File::open(path).map_err(|error| {
                format!("Could not open motion {}: {error}", path.display())
            })?))
            .map_err(|error| format!("Could not decode motion {}: {error}", path.display()))?;
        if !decoder.has_animation() {
            return Err(format!("Motion asset is not animated: {}", path.display()));
        }

        let mut frames = Vec::new();
        let mut decoded_bytes = 0usize;
        let mut motion_size = None;
        for frame in decoder.into_frames() {
            if frames.len() == MAX_MOTION_FRAMES {
                return Err(format!("Motion asset exceeds {MAX_MOTION_FRAMES} frames"));
            }
            let rgba = frame
                .map_err(|error| format!("Could not decode motion frame: {error}"))?
                .into_buffer();
            let size = [rgba.width() as usize, rgba.height() as usize];
            if size[0] > MAX_IMAGE_EDGE as usize || size[1] > MAX_IMAGE_EDGE as usize {
                return Err("Motion frame exceeds the image dimension limit".to_owned());
            }
            if motion_size.is_some_and(|expected| expected != size) {
                return Err(format!(
                    "Motion frames do not share one resolution (found {}x{})",
                    size[0], size[1]
                ));
            }
            motion_size = Some(size);
            decoded_bytes = decoded_bytes.saturating_add(rgba.as_raw().len());
            if decoded_bytes > MAX_MOTION_ALLOC_BYTES {
                return Err("Motion asset exceeds the 192 MiB decoded limit".to_owned());
            }
            frames.push(egui::ColorImage::from_rgba_unmultiplied(
                size,
                rgba.as_raw(),
            ));
        }
        if frames.len() < 2 {
            return Err("Motion asset needs at least two frames".to_owned());
        }
        let texture = context.load_texture(
            format!("studio-motion:{}", path.display()),
            frames[0].clone(),
            TextureOptions::LINEAR,
        );
        image.motion = Some(StudioMotion {
            frames,
            texture,
            mix: 0.0,
            phase: 0.0,
            frame: 0,
        });
        Ok(())
    }

    pub fn animate_motion(
        &mut self,
        context: &egui::Context,
        delta: f32,
        [low, mid, high, rms, onset]: [f32; 5],
    ) {
        let Some(motion) = self.image.as_mut().and_then(|image| image.motion.as_mut()) else {
            return;
        };
        let target =
            (low * 0.45 + mid * 0.9 + high * 0.3 + rms * 1.8 + onset * 0.6).clamp(0.0, 1.0);
        let smoothing = if target > motion.mix { 0.22 } else { 0.045 };
        motion.mix += (target - motion.mix) * smoothing;

        let speed = 0.12 + low * 0.5 + mid * 1.7 + high * 0.8 + onset * 1.8;
        motion.phase += delta.clamp(0.0, 0.1) * MOTION_FPS * speed;
        let frame = ping_pong_frame(motion.phase as usize, motion.frames.len());
        if frame != motion.frame {
            motion.frame = frame;
            motion
                .texture
                .set(motion.frames[frame].clone(), TextureOptions::LINEAR);
            context.request_repaint();
        }
    }
}

fn ping_pong_frame(frame: usize, frame_count: usize) -> usize {
    let cycle = frame_count.saturating_sub(1) * 2;
    if cycle == 0 {
        return 0;
    }
    let frame = frame % cycle;
    if frame < frame_count {
        frame
    } else {
        cycle - frame
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn layer_stack_is_bounded_and_reorderable() {
        let mut studio = StudioState::default();
        assert_eq!(studio.layers.len(), 3);
        assert!(studio.layers[0].visible);
        assert!(studio.layers[1..].iter().all(|layer| !layer.visible));
        assert!(!studio.add(StudioLayerKind::Image));
        assert!(studio.add(StudioLayerKind::Preset));
        assert_eq!(studio.layers[0].kind, StudioLayerKind::Preset);
        assert!(!studio.move_selected(1));
        assert!(!studio.add(StudioLayerKind::Preset));
        while studio.add(StudioLayerKind::Waveform) {}
        assert_eq!(studio.layers.len(), MAX_STUDIO_LAYERS);
        studio.selected = studio.layers.len() - 1;
        assert!(studio.move_selected(-1));
        assert_eq!(studio.selected, MAX_STUDIO_LAYERS - 2);
        assert!(studio.remove_selected());
        assert_eq!(studio.layers.len(), MAX_STUDIO_LAYERS - 1);
    }

    #[test]
    fn bundled_living_photograph_decodes() {
        let mut studio = StudioState::default();
        studio
            .load_image(
                &egui::Context::default(),
                Path::new("assets/living-photograph-greenhouse.png"),
            )
            .unwrap();
        let size = studio.image.as_ref().unwrap().size;
        assert!(size[0] > size[1]);
        assert!(size[0] <= MAX_IMAGE_EDGE as usize);
        assert!(size[1] <= MAX_IMAGE_EDGE as usize);
    }

    #[test]
    fn motion_frames_ping_pong_without_a_seam() {
        assert_eq!(
            (0..9)
                .map(|frame| ping_pong_frame(frame, 4))
                .collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 2, 1, 0, 1, 2]
        );
    }

    #[test]
    fn bundled_living_photograph_motion_decodes() {
        let mut studio = StudioState::default();
        let context = egui::Context::default();
        studio
            .load_image(
                &context,
                Path::new("assets/living-photograph-greenhouse.png"),
            )
            .unwrap();
        studio
            .load_motion(
                &context,
                Path::new("assets/living-photograph-greenhouse-motion.webp"),
            )
            .unwrap();
        assert_eq!(studio.image.unwrap().motion.unwrap().frames.len(), 49);
    }
}
