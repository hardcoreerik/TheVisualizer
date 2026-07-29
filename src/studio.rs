use std::{
    fs,
    path::{Path, PathBuf},
};

use eframe::egui::{self, TextureHandle, TextureOptions};
use image::{ImageReader, Limits};

pub const MAX_STUDIO_LAYERS: usize = 8;
const MAX_IMAGE_FILE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_IMAGE_EDGE: u32 = 8_192;
const MAX_IMAGE_ALLOC_BYTES: u64 = 256 * 1024 * 1024;

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
}

pub struct StudioState {
    pub layers: Vec<StudioLayer>,
    pub selected: usize,
    pub image: Option<StudioImage>,
    pub notice: Option<String>,
}

impl Default for StudioState {
    fn default() -> Self {
        Self {
            layers: StudioLayerKind::LIVING_PHOTOGRAPH
                .into_iter()
                .map(StudioLayer::living_photograph)
                .collect(),
            selected: 1,
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
        });
        self.notice = Some(format!("Loaded image · {}", path.display()));
        Ok(())
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
}
