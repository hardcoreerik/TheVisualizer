use std::path::Path;

use eframe::egui::{self, Color32, Pos2, Rect, TextureHandle, TextureOptions, Vec2};

use crate::{render::PRESET_PARAMETER_FLOATS, studio::load_motion_frames};

pub const EVENT_HORIZON_ID: &str = "thevisualizer.event-horizon";
pub const PLATE_MIX: usize = 23;
pub const CROSSING_THRESHOLD: usize = 30;
const LIVE_ENERGY: usize = 31;
const LIVE_COMET: usize = 32;
const LIVE_GRAVITY: usize = 33;
const LIVE_CROSSING: usize = 34;
const MOTION_FPS: f32 = 12.0;

const MOTIONS: [&str; 3] = [
    "event-horizon-orbit-motion.webp",
    "event-horizon-events-motion.webp",
    "event-horizon-crossing-motion.webp",
];

#[derive(Clone, Copy, Default, PartialEq, Eq)]
enum MotionKind {
    #[default]
    Orbit,
    Events,
    Crossing,
}

impl MotionKind {
    fn index(self) -> usize {
        self as usize
    }
}

struct MotionClip {
    frames: Vec<egui::ColorImage>,
    phase: f32,
    frame: usize,
}

#[derive(Default)]
pub struct EventHorizonVisual {
    clips: Option<Vec<MotionClip>>,
    texture: Option<TextureHandle>,
    active: MotionKind,
    event_hold: f32,
    pub error: Option<String>,
    energy: f32,
    comet: f32,
    gravity: f32,
    approach: f32,
    crossing: f32,
}

impl EventHorizonVisual {
    pub fn ensure_loaded(&mut self, context: &egui::Context, asset_root: &Path) {
        if self.clips.is_some() || self.error.is_some() {
            return;
        }
        let directory = asset_root.join("event-horizon");
        let loaded = MOTIONS
            .into_iter()
            .map(|name| load_motion_frames(&directory.join(name)))
            .collect::<Result<Vec<_>, _>>();
        match loaded {
            Ok(frames) => {
                let expected = frames[0][0].size;
                if frames
                    .iter()
                    .flat_map(|clip| clip.iter())
                    .any(|frame| frame.size != expected)
                {
                    self.error =
                        Some("Event Horizon motion clips must share one resolution.".to_owned());
                    return;
                }
                self.texture = Some(context.load_texture(
                    "event-horizon-motion",
                    frames[0][0].clone(),
                    TextureOptions::LINEAR,
                ));
                self.clips = Some(
                    frames
                        .into_iter()
                        .map(|frames| MotionClip {
                            frames,
                            phase: 0.0,
                            frame: 0,
                        })
                        .collect(),
                );
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub fn update(
        &mut self,
        context: &egui::Context,
        delta: f32,
        audio: [f32; 6],
        controls: &[f32],
    ) {
        let delta = delta.clamp(0.0, 0.1);
        let [low, mid, high, rms, onset, transient] = audio;
        let target = (rms * 2.2 + low * 0.55 + mid * 0.2).clamp(0.0, 1.0);
        self.energy += (target - self.energy) * (1.0 - (-delta * 3.2).exp());
        self.comet = onset.max(self.comet * (-delta * 1.7).exp());
        self.gravity = transient.max(self.gravity * (-delta * 1.15).exp());

        let threshold = controls.get(CROSSING_THRESHOLD).copied().unwrap_or(0.72);
        if self.energy > threshold {
            self.approach = (self.approach + delta * (0.15 + self.energy * 0.22)).min(1.0);
        } else {
            self.approach = (self.approach - delta * 0.12).max(0.0);
        }
        if self.approach > 0.68 && onset > 0.52 {
            self.crossing = 1.0;
            self.approach = 0.0;
        } else {
            self.crossing *= (-delta * 0.23).exp();
            if self.crossing < 0.002 {
                self.crossing = 0.0;
            }
        }

        if self.comet.max(self.gravity) > 0.2 {
            self.event_hold = 1.25;
        } else {
            self.event_hold = (self.event_hold - delta).max(0.0);
        }
        let active = if self.crossing > 0.04 {
            MotionKind::Crossing
        } else if self.event_hold > 0.0 {
            MotionKind::Events
        } else {
            MotionKind::Orbit
        };
        let Some(clips) = self.clips.as_mut() else {
            return;
        };
        let Some(texture) = self.texture.as_mut() else {
            return;
        };
        if active != self.active {
            self.active = active;
            let clip = &mut clips[active.index()];
            clip.phase = 0.0;
            clip.frame = 0;
            texture.set(clip.frames[0].clone(), TextureOptions::LINEAR);
        }
        let clip = &mut clips[self.active.index()];
        let speed = 0.5 + rms * 0.55 + low * 0.35 + mid * 0.25 + high * 0.15;
        clip.phase += delta * MOTION_FPS * speed;
        let frame = match self.active {
            MotionKind::Orbit => ping_pong_frame(clip.phase as usize, clip.frames.len()),
            MotionKind::Events | MotionKind::Crossing => {
                (clip.phase as usize).min(clip.frames.len() - 1)
            }
        };
        if frame != clip.frame {
            clip.frame = frame;
            texture.set(clip.frames[frame].clone(), TextureOptions::LINEAR);
            context.request_repaint();
        }
    }

    pub fn apply_live_controls(&self, parameters: &mut [f32; PRESET_PARAMETER_FLOATS]) {
        parameters[LIVE_ENERGY] = self.energy;
        parameters[LIVE_COMET] = self.comet;
        parameters[LIVE_GRAVITY] = self.gravity;
        parameters[LIVE_CROSSING] = self.crossing.max(self.approach * 0.35);
    }

    pub fn reset_journey(&mut self) {
        self.energy = 0.0;
        self.comet = 0.0;
        self.gravity = 0.0;
        self.approach = 0.0;
        self.crossing = 0.0;
        self.event_hold = 0.0;
        self.active = MotionKind::Orbit;
    }

    pub fn shader_opacity(&self, requested: f32, controls: &[f32]) -> f32 {
        let plate_mix = controls.get(PLATE_MIX).copied().unwrap_or(0.0);
        requested * (1.0 - plate_mix.clamp(0.0, 1.0) * 0.32)
    }

    pub fn draw(&self, painter: &egui::Painter, rect: Rect, controls: &[f32]) {
        let Some(texture) = &self.texture else {
            return;
        };
        let mix = controls
            .get(PLATE_MIX)
            .copied()
            .unwrap_or(0.0)
            .clamp(0.0, 1.0);
        if mix <= 0.001 {
            return;
        }
        painter.image(
            texture.id(),
            rect,
            cover_uv(rect, texture.size()),
            Color32::from_white_alpha((255.0 * mix * 0.9) as u8),
        );
    }
}

fn cover_uv(rect: Rect, [width, height]: [usize; 2]) -> Rect {
    let source_aspect = width as f32 / height.max(1) as f32;
    let target_aspect = rect.width() / rect.height().max(1.0);
    let (uv_width, uv_height) = if source_aspect > target_aspect {
        (target_aspect / source_aspect, 1.0)
    } else {
        (1.0, source_aspect / target_aspect)
    };
    Rect::from_center_size(Pos2::new(0.5, 0.5), Vec2::new(uv_width, uv_height))
}

fn ping_pong_frame(frame: usize, frame_count: usize) -> usize {
    if frame_count < 2 {
        return 0;
    }
    let period = (frame_count - 1) * 2;
    let position = frame % period;
    position.min(period - position)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn musical_crossing_triggers_and_recovers() {
        let mut visual = EventHorizonVisual::default();
        let mut controls = [0.0; PRESET_PARAMETER_FLOATS];
        controls[CROSSING_THRESHOLD] = 0.2;
        let context = egui::Context::default();
        for _ in 0..160 {
            visual.update(&context, 0.05, [1.0, 0.4, 0.2, 0.8, 0.0, 0.0], &controls);
        }
        visual.update(&context, 0.05, [1.0, 0.4, 0.2, 0.8, 1.0, 0.8], &controls);
        assert_eq!(visual.crossing, 1.0);
        for _ in 0..2_000 {
            visual.update(&context, 0.05, [0.0; 6], &controls);
        }
        assert_eq!(visual.crossing, 0.0);
    }

    #[test]
    fn motion_cover_stays_inside_texture() {
        let uv = cover_uv(
            Rect::from_min_size(Pos2::ZERO, Vec2::new(3440.0, 1440.0)),
            [1024, 576],
        );
        assert!(uv.min.x >= 0.0 && uv.min.y >= 0.0);
        assert!(uv.max.x <= 1.0 && uv.max.y <= 1.0);
    }

    #[test]
    fn bundled_motion_clips_decode() {
        let mut visual = EventHorizonVisual::default();
        visual.ensure_loaded(
            &egui::Context::default(),
            Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets")),
        );
        assert!(visual.error.is_none(), "{:?}", visual.error);
        assert_eq!(visual.clips.as_ref().map(Vec::len), Some(MOTIONS.len()));
    }
}
