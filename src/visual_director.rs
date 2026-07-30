use std::{
    collections::VecDeque,
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use crate::analysis::Features;

const HISTORY_LIMIT: usize = 24;
const AUDIO_WINDOW: Duration = Duration::from_secs(12);
const HISTORY_HEADER: &str = "THEVISUALIZER_DIRECTOR_HISTORY\t1";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum OutputIntent {
    Preview,
    Final,
}

impl OutputIntent {
    pub const ALL: [Self; 2] = [Self::Preview, Self::Final];

    pub fn label(self) -> &'static str {
        match self {
            Self::Preview => "Preview",
            Self::Final => "Final",
        }
    }

    pub fn quality(self) -> &'static str {
        match self {
            Self::Preview => "low",
            Self::Final => "high",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Aspect {
    Square,
    Landscape,
    Portrait,
    Cinematic,
}

impl Aspect {
    pub const ALL: [Self; 4] = [
        Self::Square,
        Self::Landscape,
        Self::Portrait,
        Self::Cinematic,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Square => "Square",
            Self::Landscape => "Landscape",
            Self::Portrait => "Portrait",
            Self::Cinematic => "Cinematic",
        }
    }

    pub fn size(self, intent: OutputIntent) -> &'static str {
        match (self, intent) {
            (Self::Square, OutputIntent::Preview) => "1024x1024",
            (Self::Landscape, OutputIntent::Preview) => "1536x1024",
            (Self::Portrait, OutputIntent::Preview) => "1024x1536",
            (Self::Cinematic, OutputIntent::Preview) => "1536x864",
            (Self::Square, OutputIntent::Final) => "2048x2048",
            (Self::Landscape, OutputIntent::Final) => "2048x1152",
            (Self::Portrait, OutputIntent::Final) => "1152x2048",
            (Self::Cinematic, OutputIntent::Final) => "2560x1440",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum CaptureProfile {
    Auto,
    Documentary,
    Street,
    Concert,
    Editorial,
    Amateur,
    Smartphone,
    Disposable,
    Film35mm,
    Archival,
    SurrealPhoto,
    Industrial,
    Nature,
    Macro,
    Aerial,
    Architectural,
    Night,
}

impl CaptureProfile {
    pub const ALL: [Self; 17] = [
        Self::Auto,
        Self::Documentary,
        Self::Street,
        Self::Concert,
        Self::Editorial,
        Self::Amateur,
        Self::Smartphone,
        Self::Disposable,
        Self::Film35mm,
        Self::Archival,
        Self::SurrealPhoto,
        Self::Industrial,
        Self::Nature,
        Self::Macro,
        Self::Aerial,
        Self::Architectural,
        Self::Night,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto capture",
            Self::Documentary => "Documentary realism",
            Self::Street => "Street photography",
            Self::Concert => "Concert photography",
            Self::Editorial => "Editorial",
            Self::Amateur => "Amateur snapshot",
            Self::Smartphone => "Smartphone",
            Self::Disposable => "Disposable camera",
            Self::Film35mm => "35mm film",
            Self::Archival => "Archival photograph",
            Self::SurrealPhoto => "Surreal photorealism",
            Self::Industrial => "Industrial documentary",
            Self::Nature => "Nature documentary",
            Self::Macro => "Macro",
            Self::Aerial => "Aerial",
            Self::Architectural => "Architectural",
            Self::Night => "Night photography",
        }
    }

    fn specification(self) -> &'static str {
        match self {
            Self::Auto => "an unstaged real-camera photograph",
            Self::Documentary => "an unstaged documentary photograph with observed detail",
            Self::Street => "candid street photography captured from within the activity",
            Self::Concert => "a physically difficult available-light concert photograph",
            Self::Editorial => "an editorial location photograph without advertising polish",
            Self::Amateur => "an imperfect amateur snapshot made by someone who was actually there",
            Self::Smartphone => {
                "a believable modern smartphone photograph with computational restraint"
            }
            Self::Disposable => "a direct-flash disposable-camera photograph with limited latitude",
            Self::Film35mm => "a scanned 35mm color-negative photograph with restrained grain",
            Self::Archival => "an aged archival photograph with period-correct tonal response",
            Self::SurrealPhoto => {
                "a materially believable photograph documenting one impossible event"
            }
            Self::Industrial => {
                "an industrial documentary photograph made under working conditions"
            }
            Self::Nature => "a patient nature-documentary photograph governed by real weather",
            Self::Macro => "a true macro photograph with physically narrow depth of field",
            Self::Aerial => "an oblique aerial survey photograph with legible geographic scale",
            Self::Architectural => {
                "an architectural photograph with corrected but not sterile geometry"
            }
            Self::Night => "an available-light night photograph with real exposure limitations",
        }
    }
}

pub struct DirectorControls {
    pub music_influence: f32,
    pub photorealism: f32,
    pub abstraction: f32,
    pub chaos: f32,
    pub weirdness: f32,
    pub human_presence: f32,
    pub era_freedom: f32,
    pub color_freedom: f32,
    pub environmental_complexity: f32,
    pub novelty: f32,
}

impl Default for DirectorControls {
    fn default() -> Self {
        Self {
            music_influence: 0.85,
            photorealism: 0.65,
            abstraction: 0.35,
            chaos: 0.4,
            weirdness: 0.65,
            human_presence: 0.35,
            era_freedom: 0.7,
            color_freedom: 0.75,
            environmental_complexity: 0.7,
            novelty: 0.85,
        }
    }
}

#[derive(Clone)]
pub struct VisualDna {
    pub energy: f32,
    pub dynamics: f32,
    pub bass: f32,
    pub mids: f32,
    pub treble: f32,
    pub spectral_center: f32,
    pub spectral_rolloff: f32,
    pub harmonic_density: f32,
    pub transient_intensity: f32,
    pub onset_density: f32,
    pub movement: f32,
    pub dominant_band: &'static str,
    pub passage: &'static str,
}

#[derive(Clone)]
pub struct PreflightReview {
    pub novelty: f32,
    pub plausibility: f32,
    pub cliche_risk: f32,
    pub notes: Vec<String>,
}

#[derive(Clone)]
pub struct SceneBrief {
    pub concept: String,
    pub visual_dna: VisualDna,
    pub subject: String,
    pub environment: String,
    pub era: String,
    pub event: String,
    pub scene: String,
    pub camera: String,
    pub lighting: String,
    pub materials: String,
    pub motion: String,
    pub capture_profile: CaptureProfile,
    pub imperfections: Vec<String>,
    pub prompt: String,
    pub preflight: PreflightReview,
    pub novelty_id: u64,
}

struct AudioMoment {
    at: Instant,
    energy: f32,
    transient: f32,
}

#[derive(Clone)]
struct ConceptRecord {
    subject: usize,
    environment: usize,
    era: usize,
    event: usize,
    composition: usize,
    lighting: usize,
}

struct Candidate {
    record: ConceptRecord,
    novelty: f32,
}

#[derive(Clone)]
pub struct BriefHistoryEntry {
    pub saved_at: u64,
    pub mode: String,
    pub concept: String,
    pub capture: String,
    pub intent: String,
    pub aspect: String,
    pub size: String,
    pub prompt: String,
    pub novelty_id: u64,
    record: ConceptRecord,
}

pub struct VisualDirector {
    pub controls: DirectorControls,
    pub intent: OutputIntent,
    pub aspect: Aspect,
    pub capture_profile: CaptureProfile,
    pub brief: Option<SceneBrief>,
    audio_history: VecDeque<AudioMoment>,
    concept_history: VecDeque<ConceptRecord>,
    brief_history: VecDeque<BriefHistoryEntry>,
    history_path: Option<PathBuf>,
    pub history_error: Option<String>,
    pub export_notice: Option<String>,
    revision: u32,
}

impl Default for VisualDirector {
    fn default() -> Self {
        Self {
            controls: DirectorControls::default(),
            intent: OutputIntent::Preview,
            aspect: Aspect::Landscape,
            capture_profile: CaptureProfile::Auto,
            brief: None,
            audio_history: VecDeque::new(),
            concept_history: VecDeque::new(),
            brief_history: VecDeque::new(),
            history_path: None,
            history_error: None,
            export_notice: None,
            revision: 0,
        }
    }
}

impl VisualDirector {
    pub fn with_default_history() -> Self {
        let Some(path) = director_history_path() else {
            return Self::default();
        };
        Self::with_history_path(path)
    }

    fn with_history_path(path: PathBuf) -> Self {
        let mut director = Self {
            history_path: Some(path.clone()),
            ..Self::default()
        };
        match fs::read_to_string(&path) {
            Ok(contents) => {
                let mut skipped = 0;
                for line in contents.lines().skip(1) {
                    match BriefHistoryEntry::decode(line) {
                        Some(entry) => director.remember(entry),
                        None => skipped += 1,
                    }
                }
                if skipped > 0 {
                    director.history_error = Some(format!(
                        "Skipped {skipped} malformed Visual Director records."
                    ));
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                director.history_error = Some(format!("Could not read brief history: {error}"));
            }
        }
        director
    }

    pub fn history(&self) -> &VecDeque<BriefHistoryEntry> {
        &self.brief_history
    }

    pub fn history_location(&self) -> Option<&Path> {
        self.history_path.as_deref()
    }

    pub fn export_current_brief(&mut self) {
        self.export_notice = Some(match self.export_current_brief_inner() {
            Ok(path) => format!("Exported brief · {}", path.display()),
            Err(error) => error,
        });
    }

    pub fn observe(&mut self, features: &Features, now: Instant) {
        self.audio_history.push_back(AudioMoment {
            at: now,
            energy: (features.rms * 3.0).clamp(0.0, 1.0),
            transient: features.transient.clamp(0.0, 1.0),
        });
        while self
            .audio_history
            .front()
            .is_some_and(|moment| now.duration_since(moment.at) > AUDIO_WINDOW)
        {
            self.audio_history.pop_front();
        }
    }

    pub fn compose(
        &mut self,
        mode: &str,
        features: &Features,
        palette: &str,
        finish: &str,
        routed_colors: &str,
    ) {
        let dna = self.visual_dna(features);
        let seed = fingerprint(&format!(
            "{mode}:{}:{}:{}:{}:{}:{}:{}",
            quantize(dna.energy),
            quantize(dna.bass),
            quantize(dna.mids),
            quantize(dna.treble),
            quantize(dna.transient_intensity),
            quantize(self.controls.weirdness),
            self.revision
        ));
        self.revision = self.revision.wrapping_add(1);
        let candidate = self.choose_candidate(seed);
        let record = candidate.record.clone();
        let profile = self.resolve_profile(seed, &dna);

        let subject = SUBJECTS[record.subject];
        let environment = ENVIRONMENTS[record.environment];
        let era = ERAS[record.era];
        let event = EVENTS[record.event];
        let scale = SCALES[index(seed, 17, SCALES.len())];
        let weather = WEATHER[index(seed, 23, WEATHER.len())];
        let composition = COMPOSITIONS[record.composition];
        let light = LIGHTING[record.lighting];
        let mode_influence = mode_influence(mode, seed);
        let imperfections = select_imperfections(seed, profile, &dna);
        let concept = format!("{subject} {event} at {environment}");
        let realism = realism_specification(&self.controls);
        let population = population_specification(self.controls.human_presence);
        let scene = format!(
            "{concept}. Set it in {era}, at {scale}; {weather}. {population}. {realism}. \
             The active visual contributes only this formal influence: {mode_influence}."
        );
        let camera = format!(
            "{}; {} frame; {}. Keep the focal hierarchy legible but resist a centered \
             poster composition.",
            profile.specification(),
            self.aspect.label(),
            composition
        );
        let lighting = format!(
            "{light}. Let {} energy shape contrast and material response; spectral center {:.0}% \
             controls highlight hardness, rolloff {:.0}% controls atmospheric reach, and the \
             current passage is {}.",
            dna.dominant_band,
            dna.spectral_center * 100.0,
            dna.spectral_rolloff * 100.0,
            dna.passage
        );
        let materials = format!(
            "{finish} material language grounded in wear, residue, edge damage, dust, and imperfect \
             reflections. Start from palette {palette}; routed audio colors are {routed_colors}. \
             Color freedom {:.0}% permits departures when motivated by real light.",
            self.controls.color_freedom * 100.0
        );
        let motion = format!(
            "Translate music into physical causality, never an equalizer: low frequencies influence \
             mass and load, mids influence human or mechanical activity, highs influence small \
             airborne or reflective detail. Energy {:.0}%, transient intensity {:.0}%, onset \
             density {:.0}%, and movement {:.0}% determine how much of that action is visible.",
            dna.energy * self.controls.music_influence * 100.0,
            dna.transient_intensity * self.controls.music_influence * 100.0,
            dna.onset_density * self.controls.music_influence * 100.0,
            dna.movement * self.controls.music_influence * 100.0,
        );
        let avoidance = tailored_avoidance(profile, palette, self.controls.human_presence, mode);
        let prompt = format!(
            "Create one memorable image for TheVisualizer's {mode} mode.\n\nVISUAL STORY\n\
             {concept}. The photograph should imply what happened immediately before and what may \
             happen next.\n\nSCENE DESIGN\n{scene}\n\nCAPTURE AND COMPOSITION\n{camera}\n\n\
             LIGHTING AND WEATHER\n{lighting}\n\nMATERIALS AND COLOR\n{materials}\n\n\
             MUSIC-TO-IMAGE LOGIC\n{motion}\n\nCONTROLLED IMPERFECTIONS\n{}\n\n\
             DIRECTOR'S RESTRAINT\n{avoidance}\n\nPreserve believable scale, contact, gravity, \
             shadows, reflections, anatomy, and optical behavior. Favor specific environmental \
             storytelling over generic beauty.",
            imperfections.join("; ")
        );
        let preflight = preflight_review(
            candidate.novelty,
            profile,
            &record,
            self.controls.weirdness,
            palette,
        );
        let novelty_id = fingerprint(&format!(
            "{}:{}:{}:{}:{}:{}",
            record.subject,
            record.environment,
            record.era,
            record.event,
            record.composition,
            record.lighting
        ));
        let brief = SceneBrief {
            concept,
            visual_dna: dna,
            subject: subject.to_owned(),
            environment: environment.to_owned(),
            era: era.to_owned(),
            event: event.to_owned(),
            scene,
            camera,
            lighting,
            materials,
            motion,
            capture_profile: profile,
            imperfections: imperfections.into_iter().map(str::to_owned).collect(),
            prompt,
            preflight,
            novelty_id,
        };
        let entry = BriefHistoryEntry {
            saved_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            mode: mode.to_owned(),
            concept: brief.concept.clone(),
            capture: brief.capture_profile.label().to_owned(),
            intent: self.intent.label().to_owned(),
            aspect: self.aspect.label().to_owned(),
            size: self.aspect.size(self.intent).to_owned(),
            prompt: brief.prompt.clone(),
            novelty_id,
            record,
        };
        self.remember(entry.clone());
        self.history_error = self.persist(&entry).err();
        self.brief = Some(brief);
    }

    fn export_current_brief_inner(&self) -> Result<PathBuf, String> {
        let brief = self
            .brief
            .as_ref()
            .ok_or_else(|| "Compose a brief before exporting.".to_owned())?;
        let entry = self
            .brief_history
            .back()
            .ok_or_else(|| "No brief metadata is available to export.".to_owned())?;
        let parent = self
            .history_path
            .as_deref()
            .and_then(Path::parent)
            .ok_or_else(|| "Brief export needs a local history location.".to_owned())?;
        fs::create_dir_all(parent)
            .map_err(|error| format!("Could not create brief export folder: {error}"))?;
        let path = parent.join(format!(
            "brief-{}-{:016X}.md",
            entry.saved_at, brief.novelty_id
        ));
        let markdown = format!(
            "# {}\n\n\
             - Mode: {}\n\
             - Saved at: {} (Unix time)\n\
             - Capture: {}\n\
             - Output: {} · {} · {}\n\
             - Novelty ID: `{:016X}`\n\n\
             ## Visual DNA\n\n\
             - Passage: {}\n\
             - Dominant band: {}\n\
             - Energy: {:.3}\n\
             - Dynamics: {:.3}\n\
             - Bass / mids / treble: {:.3} / {:.3} / {:.3}\n\
             - Spectral center / rolloff: {:.3} / {:.3}\n\
             - Harmonic density: {:.3}\n\
             - Transient intensity / onset density / movement: {:.3} / {:.3} / {:.3}\n\n\
             ## Scene specification\n\n\
             - Subject: {}\n\
             - Environment: {}\n\
             - Era: {}\n\
             - Event: {}\n\n\
             **Scene:** {}\n\n\
             **Camera:** {}\n\n\
             **Lighting:** {}\n\n\
             **Materials:** {}\n\n\
             **Motion:** {}\n\n\
             **Imperfections:** {}\n\n\
             ## Preflight\n\n\
             - Concept novelty: {:.3}\n\
             - Physical plausibility: {:.3}\n\
             - AI cliché risk: {:.3}\n\
             - Notes: {}\n\n\
             ## Generation prompt\n\n```text\n{}\n```\n",
            brief.concept,
            entry.mode,
            entry.saved_at,
            entry.capture,
            entry.intent,
            entry.aspect,
            entry.size,
            brief.novelty_id,
            brief.visual_dna.passage,
            brief.visual_dna.dominant_band,
            brief.visual_dna.energy,
            brief.visual_dna.dynamics,
            brief.visual_dna.bass,
            brief.visual_dna.mids,
            brief.visual_dna.treble,
            brief.visual_dna.spectral_center,
            brief.visual_dna.spectral_rolloff,
            brief.visual_dna.harmonic_density,
            brief.visual_dna.transient_intensity,
            brief.visual_dna.onset_density,
            brief.visual_dna.movement,
            brief.subject,
            brief.environment,
            brief.era,
            brief.event,
            brief.scene,
            brief.camera,
            brief.lighting,
            brief.materials,
            brief.motion,
            brief.imperfections.join("; "),
            brief.preflight.novelty,
            brief.preflight.plausibility,
            brief.preflight.cliche_risk,
            brief.preflight.notes.join("; "),
            brief.prompt,
        );
        fs::write(&path, markdown)
            .map_err(|error| format!("Could not export current brief: {error}"))?;
        Ok(path)
    }

    fn remember(&mut self, entry: BriefHistoryEntry) {
        self.concept_history.push_back(entry.record.clone());
        self.brief_history.push_back(entry);
        while self.concept_history.len() > HISTORY_LIMIT {
            self.concept_history.pop_front();
        }
        while self.brief_history.len() > HISTORY_LIMIT {
            self.brief_history.pop_front();
        }
    }

    fn persist(&self, entry: &BriefHistoryEntry) -> Result<(), String> {
        let Some(path) = &self.history_path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("Could not create brief history folder: {error}"))?;
        }
        let is_empty = fs::metadata(path).map_or(true, |metadata| metadata.len() == 0);
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .map_err(|error| format!("Could not open brief history: {error}"))?;
        if is_empty {
            writeln!(file, "{HISTORY_HEADER}")
                .map_err(|error| format!("Could not initialize brief history: {error}"))?;
        }
        writeln!(file, "{}", entry.encode())
            .map_err(|error| format!("Could not save brief history: {error}"))
    }

    fn visual_dna(&self, features: &Features) -> VisualDna {
        let energy = (features.rms * 3.0).clamp(0.0, 1.0);
        let mut minimum = energy;
        let mut maximum = energy;
        let mut movement = 0.0;
        let mut prior: Option<f32> = None;
        let mut onsets = 0;
        for moment in &self.audio_history {
            minimum = minimum.min(moment.energy);
            maximum = maximum.max(moment.energy);
            if let Some(previous) = prior {
                movement += (moment.energy - previous).abs();
            }
            prior = Some(moment.energy);
            onsets += usize::from(moment.transient > 0.28 && moment.energy > 0.04);
        }
        let samples = self.audio_history.len().max(1) as f32;
        let average = self
            .audio_history
            .iter()
            .map(|moment| moment.energy)
            .sum::<f32>()
            / samples;
        let duration = self
            .audio_history
            .front()
            .zip(self.audio_history.back())
            .map_or(0.0, |(first, last)| {
                last.at.duration_since(first.at).as_secs_f32()
            });
        let onset_density = if duration > 0.25 {
            (onsets as f32 / duration / 4.0).clamp(0.0, 1.0)
        } else {
            features.transient
        };
        let trend = self
            .audio_history
            .front()
            .zip(self.audio_history.back())
            .map_or(0.0, |(first, last)| last.energy - first.energy);
        let passage = if energy < 0.06 {
            "quiet"
        } else if energy > average + 0.22 && features.transient > 0.3 {
            "impact"
        } else if trend > 0.16 {
            "building"
        } else if trend < -0.16 {
            "receding"
        } else {
            "sustained"
        };
        let (dominant_band, _) = [
            ("bass", features.low),
            ("mids", features.mid),
            ("treble", features.high),
        ]
        .into_iter()
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .unwrap_or(("full range", 0.0));

        VisualDna {
            energy,
            dynamics: ((maximum - minimum) * 0.65
                + (features.crest_factor / 4.0).clamp(0.0, 1.0) * 0.35)
                .clamp(0.0, 1.0),
            bass: features.low.clamp(0.0, 1.0),
            mids: features.mid.clamp(0.0, 1.0),
            treble: features.high.clamp(0.0, 1.0),
            spectral_center: (features.centroid_hz / 12_000.0).clamp(0.0, 1.0),
            spectral_rolloff: (features.rolloff_hz / 16_000.0).clamp(0.0, 1.0),
            harmonic_density: (1.0 - features.spectral_flatness).clamp(0.0, 1.0),
            transient_intensity: features.transient.clamp(0.0, 1.0),
            onset_density,
            movement: (movement / samples * 6.0).clamp(0.0, 1.0),
            dominant_band,
            passage,
        }
    }

    fn choose_candidate(&self, seed: u64) -> Candidate {
        let attempts = (6.0 + self.controls.novelty * 26.0).round() as usize;
        (0..attempts)
            .map(|attempt| {
                let candidate_seed =
                    seed.wrapping_add((attempt as u64 + 1).wrapping_mul(0x9e3779b97f4a7c15));
                let record = ConceptRecord {
                    subject: index(candidate_seed, 1, SUBJECTS.len()),
                    environment: index(candidate_seed, 5, ENVIRONMENTS.len()),
                    era: index(candidate_seed, 9, ERAS.len()),
                    event: index(candidate_seed, 13, EVENTS.len()),
                    composition: index(candidate_seed, 17, COMPOSITIONS.len()),
                    lighting: index(candidate_seed, 21, LIGHTING.len()),
                };
                let similarity = self
                    .concept_history
                    .iter()
                    .map(|previous| concept_similarity(&record, previous))
                    .fold(0.0_f32, f32::max);
                Candidate {
                    record,
                    novelty: 1.0 - similarity,
                }
            })
            .max_by(|left, right| left.novelty.total_cmp(&right.novelty))
            .expect("at least one director candidate")
    }

    fn resolve_profile(&self, seed: u64, dna: &VisualDna) -> CaptureProfile {
        if self.capture_profile != CaptureProfile::Auto {
            return self.capture_profile;
        }
        let profiles = if self.controls.photorealism > 0.72 {
            &[
                CaptureProfile::Documentary,
                CaptureProfile::Street,
                CaptureProfile::Amateur,
                CaptureProfile::Film35mm,
                CaptureProfile::Industrial,
                CaptureProfile::Architectural,
                CaptureProfile::Night,
            ][..]
        } else {
            &[
                CaptureProfile::Editorial,
                CaptureProfile::Disposable,
                CaptureProfile::Archival,
                CaptureProfile::SurrealPhoto,
                CaptureProfile::Aerial,
                CaptureProfile::Macro,
            ][..]
        };
        profiles[index(
            seed ^ u64::from(quantize(dna.spectral_center)),
            29,
            profiles.len(),
        )]
    }
}

impl BriefHistoryEntry {
    fn encode(&self) -> String {
        format!(
            "v1\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            self.saved_at,
            self.record.subject,
            self.record.environment,
            self.record.era,
            self.record.event,
            self.record.composition,
            self.record.lighting,
            self.novelty_id,
            hex_encode(&self.mode),
            hex_encode(&self.concept),
            hex_encode(&self.capture),
            hex_encode(&self.intent),
            hex_encode(&self.aspect),
            hex_encode(&self.size),
            hex_encode(&self.prompt),
        )
    }

    fn decode(line: &str) -> Option<Self> {
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 16 || fields[0] != "v1" {
            return None;
        }
        Some(Self {
            saved_at: fields[1].parse().ok()?,
            record: ConceptRecord {
                subject: fields[2].parse().ok()?,
                environment: fields[3].parse().ok()?,
                era: fields[4].parse().ok()?,
                event: fields[5].parse().ok()?,
                composition: fields[6].parse().ok()?,
                lighting: fields[7].parse().ok()?,
            },
            novelty_id: fields[8].parse().ok()?,
            mode: hex_decode(fields[9])?,
            concept: hex_decode(fields[10])?,
            capture: hex_decode(fields[11])?,
            intent: hex_decode(fields[12])?,
            aspect: hex_decode(fields[13])?,
            size: hex_decode(fields[14])?,
            prompt: hex_decode(fields[15])?,
        })
    }
}

fn director_history_path() -> Option<PathBuf> {
    if let Some(path) = std::env::var_os("THEVISUALIZER_DIRECTOR_HISTORY") {
        return Some(path.into());
    }
    std::env::var_os("LOCALAPPDATA").map(|base| {
        PathBuf::from(base)
            .join("TheVisualizer")
            .join("visual-director-history.tsv")
    })
}

fn hex_encode(text: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut encoded = String::with_capacity(text.len() * 2);
    for byte in text.bytes() {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0F) as usize] as char);
    }
    encoded
}

fn hex_decode(text: &str) -> Option<String> {
    if !text.len().is_multiple_of(2) {
        return None;
    }
    let bytes = text
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16)?;
            let low = (pair[1] as char).to_digit(16)?;
            Some(((high << 4) | low) as u8)
        })
        .collect::<Option<Vec<_>>>()?;
    String::from_utf8(bytes).ok()
}

const SUBJECTS: [&str; 18] = [
    "a municipal night-shift crew",
    "an unattended agricultural machine",
    "a traveling fair dismantling its rides",
    "a flooded archive room",
    "a lone weather technician",
    "a crowded amateur orchestra",
    "an aging ferry under repair",
    "a subterranean greenhouse",
    "a motel laundry room",
    "a geological survey camp",
    "a group of exhausted stagehands",
    "a rooftop pigeon keeper",
    "an abandoned television studio",
    "a mobile kitchen after service",
    "a synchronized swimming team between rehearsals",
    "a warehouse full of damaged mannequins",
    "a snowplow at the edge of town",
    "a museum conservator working alone",
];

const ENVIRONMENTS: [&str; 18] = [
    "a drained indoor swimming pool",
    "a hydroelectric turbine hall",
    "a rain-soaked county fairground",
    "a windswept container port",
    "a half-finished underground station",
    "a remote radar installation",
    "a salt-flat service road",
    "an overgrown shopping arcade",
    "a basement music venue",
    "a volcanic monitoring platform",
    "a suburban hotel atrium",
    "an icebound marina",
    "a decommissioned observatory",
    "a brutalist civic plaza",
    "a desert truck stop",
    "a fogbound sports stadium",
    "a coastal apartment roof",
    "an industrial mushroom farm",
];

const ERAS: [&str; 10] = [
    "an uncertain present day",
    "the late 1960s",
    "the economically worn 1970s",
    "the fluorescent early 1980s",
    "the pre-digital 1990s",
    "a near future with repaired rather than replaced technology",
    "a future reconstructed from municipal archives",
    "an era where analog infrastructure never disappeared",
    "a time after a long regional blackout",
    "a season outside any obvious era",
];

const EVENTS: [&str; 18] = [
    "during an unplanned evacuation",
    "while preparing for a storm that never arrives",
    "minutes after an unexplained power failure",
    "during a mundane maintenance procedure at impossible scale",
    "as gravity briefly changes direction",
    "while hundreds of small objects begin vibrating in sympathy",
    "during a quiet labor dispute",
    "as water slowly enters from an unknown source",
    "between two phases of a public celebration",
    "while the building appears to inhale",
    "at the exact moment everyone notices the same distant sound",
    "during a failed rehearsal",
    "as ordinary shadows detach from their objects",
    "while a temporary structure becomes permanent",
    "during the last hour before demolition",
    "as a weather system forms indoors",
    "while workers uncover an older room inside the existing one",
    "during a completely ordinary task under extraordinary conditions",
];

const SCALES: [&str; 9] = [
    "intimate human scale",
    "claustrophobic room scale",
    "wide architectural scale",
    "industrial scale",
    "city-block scale",
    "landscape scale",
    "microscopic detail presented as terrain",
    "ordinary objects made monumentally legible",
    "vast space interrupted by one small human action",
];

const WEATHER: [&str; 12] = [
    "cold rain leaving real residue on every surface",
    "dry wind carrying paper and fine dust",
    "dense coastal air without decorative fog",
    "recent snow turning to gray slush",
    "harsh clear weather with unforgiving shadows",
    "humid heat visible in clothing and surfaces",
    "a distant electrical storm",
    "flat overcast daylight",
    "mixed indoor and outdoor temperatures",
    "wind after a sudden downpour",
    "still air before dawn",
    "ordinary weather made strange by the event",
];

const COMPOSITIONS: [&str; 14] = [
    "extreme wide with the event nearly missed at the edge",
    "wide environmental portrait with deep foreground obstruction",
    "medium view from behind the primary activity",
    "close view with the subject partially cropped",
    "extreme close detail that implies a much larger event",
    "top-down view with imperfect alignment",
    "low angle from working height rather than heroic ground level",
    "elevated view through a dirty window",
    "distant subject dominated by environment",
    "negative-space-heavy frame with action exiting",
    "crowded frame with one quiet visual anchor",
    "compressed perspective through several active layers",
    "off-center view interrupted by an ordinary foreground object",
    "asymmetrical frame where the apparent subject is not the story",
];

const LIGHTING: [&str; 12] = [
    "mixed practical fluorescent and weak exterior daylight",
    "one damaged sodium-vapor fixture and reflected spill",
    "cold overcast light entering through dirty glazing",
    "direct flash falling off into a genuinely dark background",
    "late daylight interrupted by work lights",
    "available stage light with clipped practicals",
    "emergency lighting mixed with normal fixtures",
    "low winter sun producing long physically consistent shadows",
    "industrial task lighting with large unlit areas",
    "smartphone auto-exposure struggling with a bright doorway",
    "moonlight plus distant municipal light pollution",
    "soft skylight reflected from wet ground",
];

const IMPERFECTIONS: [&str; 16] = [
    "slight subject motion blur",
    "one foreground obstruction",
    "mixed color temperatures",
    "mild underexposure",
    "a clipped practical light",
    "restrained high-ISO noise",
    "subtle film grain",
    "focus landing just behind the intended subject",
    "atmospheric haze justified by weather",
    "fingerprints on nearby glass",
    "worn and stained fabric",
    "chipped paint at contact points",
    "water residue and imperfect pavement",
    "ordinary clutter left in place",
    "subjects looking away from the camera",
    "slightly awkward framing from limited access",
];

fn realism_specification(controls: &DirectorControls) -> &'static str {
    if controls.photorealism > 0.72 {
        "Treat it as witnessed physical reality with correct contact, weight, texture, and optics"
    } else if controls.abstraction > 0.66 {
        "Keep materials and lighting physically credible while allowing selective spatial abstraction"
    } else {
        "Balance documentary credibility with one clearly defined impossible condition"
    }
}

fn population_specification(amount: f32) -> &'static str {
    if amount < 0.22 {
        "Show no people; reveal recent human presence through specific traces"
    } else if amount > 0.72 {
        "Show a diverse crowd with independent candid actions, partial occlusion, and no posing"
    } else {
        "Include a few incidental people absorbed in real tasks and unaware of the camera"
    }
}

fn mode_influence(mode: &str, seed: u64) -> &'static str {
    let mode = mode.to_ascii_lowercase();
    let options = if mode.contains("city") {
        &[
            "cylindrical depth relationships",
            "separate activity zones across architecture, street, roof, and sky",
            "an impossible sky that remains physically integrated with the block",
            "a neighborhood readable from several simultaneous points of attention",
        ][..]
    } else if mode.contains("scope") || mode.contains("ripple") {
        &[
            "one traveling disturbance made visible through material response",
            "layered traces of recent motion",
            "a long horizontal pressure relationship",
            "concentric causality without decorative rings",
        ][..]
    } else if mode.contains("particle") || mode.contains("gravity") {
        &[
            "small objects revealing an invisible field",
            "density gathering around several competing centers",
            "orbital motion governed by believable mass",
            "a dispersed system briefly assembling itself",
        ][..]
    } else if mode.contains("aurora") || mode.contains("solar") {
        &[
            "broad light behaving like a physical material",
            "layered atmospheric flow",
            "a radiant event observed in ordinary surroundings",
            "slow vertical movement across a wide environment",
        ][..]
    } else if mode.contains("kaleido") || mode.contains("feedback") {
        &[
            "repetition caused by real reflective structure",
            "a recursive space with traceable geometry",
            "several imperfect copies of one physical event",
            "depth folding around a stable documentary viewpoint",
        ][..]
    } else {
        &[
            "audio energy expressed through material cause and effect",
            "several frequency regions controlling distinct physical behaviors",
            "recent motion remaining visible as environmental evidence",
            "a stable place transformed by a temporary acoustic rule",
        ][..]
    };
    options[index(seed, 31, options.len())]
}

fn select_imperfections(seed: u64, profile: CaptureProfile, dna: &VisualDna) -> Vec<&'static str> {
    let count = if dna.dynamics > 0.65 { 3 } else { 2 };
    let mut selected = Vec::with_capacity(count);
    let profile_offset = profile as u64 * 7;
    for step in 0..IMPERFECTIONS.len() {
        let candidate =
            IMPERFECTIONS[index(seed ^ profile_offset, 37 + step as u32, IMPERFECTIONS.len())];
        if !selected.contains(&candidate) {
            selected.push(candidate);
            if selected.len() == count {
                break;
            }
        }
    }
    selected
}

fn tailored_avoidance(
    profile: CaptureProfile,
    palette: &str,
    human_presence: f32,
    mode: &str,
) -> String {
    let mut rules = vec![
        "Do not beautify ordinary wear or clean the location".to_owned(),
        "Do not center the main activity or turn the scene into key art".to_owned(),
    ];
    if palette.to_ascii_lowercase().contains("neon") {
        rules.push(
            "Use saturated color only where a visible practical source motivates it; no generic \
             cyberpunk grading"
                .to_owned(),
        );
    }
    if human_presence > 0.2 {
        rules.push(
            "Keep anatomy, skin texture, clothing tension, gaze, and hand-object contact natural"
                .to_owned(),
        );
    }
    if profile != CaptureProfile::SurrealPhoto {
        rules.push(
            "Avoid concept-art polish, excessive bloom, and ornamental atmosphere".to_owned(),
        );
    }
    if mode.to_ascii_lowercase().contains("kaleido") {
        rules
            .push("Every reflection must have a traceable surface and consistent light".to_owned());
    }
    rules.join("; ")
}

fn preflight_review(
    novelty: f32,
    profile: CaptureProfile,
    record: &ConceptRecord,
    weirdness: f32,
    palette: &str,
) -> PreflightReview {
    let cliche_risk = if palette.to_ascii_lowercase().contains("neon") {
        0.28
    } else {
        0.12
    };
    let plausibility = (0.92 - weirdness * 0.18).clamp(0.0, 1.0);
    let mut notes = vec![format!(
        "Concept spans subject {}, environment {}, era {}, event {}, and composition {}.",
        record.subject + 1,
        record.environment + 1,
        record.era + 1,
        record.event + 1,
        record.composition + 1
    )];
    notes.push(format!(
        "{} selected; physical constraints remain explicit.",
        profile.label()
    ));
    if cliche_risk > 0.2 {
        notes.push(
            "Palette raises neon-cliché risk; prompt requires motivated practical sources."
                .to_owned(),
        );
    }
    PreflightReview {
        novelty,
        plausibility,
        cliche_risk,
        notes,
    }
}

fn concept_similarity(left: &ConceptRecord, right: &ConceptRecord) -> f32 {
    let matches = usize::from(left.subject == right.subject) * 3
        + usize::from(left.environment == right.environment) * 3
        + usize::from(left.era == right.era)
        + usize::from(left.event == right.event) * 2
        + usize::from(left.composition == right.composition)
        + usize::from(left.lighting == right.lighting);
    matches as f32 / 11.0
}

fn index(seed: u64, rotation: u32, length: usize) -> usize {
    seed.rotate_left(rotation) as usize % length
}

fn quantize(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 15.0).round() as u8
}

fn fingerprint(text: &str) -> u64 {
    text.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energetic_bass() -> Features {
        let mut spectrum = Features::default().spectrum;
        spectrum[4] = 0.8;
        Features {
            spectrum,
            rms: 0.2,
            peak: 0.7,
            low: 0.8,
            mid: 0.3,
            high: 0.1,
            centroid_hz: 900.0,
            rolloff_hz: 3_200.0,
            crest_factor: 2.3,
            transient: 0.7,
            ..Features::default()
        }
    }

    #[test]
    fn director_builds_structured_briefs_without_repeating_immediately() {
        let features = energetic_bass();
        let mut director = VisualDirector::default();
        director.observe(&features, Instant::now());
        director.compose(
            "TheVisualCityScape",
            &features,
            "Cyber Neon",
            "Glass",
            "bass #FF0020, treble #2080FF",
        );
        let first = director.brief.clone().expect("first brief");
        assert_eq!(first.visual_dna.dominant_band, "bass");
        assert!(first.prompt.contains("SCENE DESIGN"));
        assert!(first.prompt.contains("DIRECTOR'S RESTRAINT"));
        assert!(first.imperfections.len() >= 2);
        assert!(first.preflight.cliche_risk > 0.0);

        director.compose(
            "TheVisualCityScape",
            &features,
            "Cyber Neon",
            "Glass",
            "bass #FF0020, treble #2080FF",
        );
        let second = director.brief.as_ref().expect("second brief");
        assert_ne!(second.novelty_id, first.novelty_id);
        assert_ne!(second.concept, first.concept);
    }

    #[test]
    fn rolling_audio_history_detects_a_build() {
        let started = Instant::now();
        let mut director = VisualDirector::default();
        let mut features = energetic_bass();
        features.transient = 0.0;
        for index in 0..8 {
            features.rms = 0.02 + index as f32 * 0.025;
            features.transient = if index % 2 == 0 { 0.5 } else { 0.05 };
            director.observe(&features, started + Duration::from_millis(index * 300));
        }
        let dna = director.visual_dna(&features);
        assert_eq!(dna.passage, "building");
        assert!(dna.onset_density > 0.0);
        assert!(dna.movement > 0.0);
    }

    #[test]
    fn capture_profiles_and_output_sizes_are_explicit() {
        assert_eq!(Aspect::Landscape.size(OutputIntent::Preview), "1536x1024");
        assert_eq!(Aspect::Cinematic.size(OutputIntent::Final), "2560x1440");
        assert!(CaptureProfile::ALL.len() > 12);
        assert!(
            CaptureProfile::Industrial
                .specification()
                .contains("industrial documentary")
        );
    }

    #[test]
    fn similarity_weights_concept_over_surface_treatment() {
        let base = ConceptRecord {
            subject: 1,
            environment: 2,
            era: 3,
            event: 4,
            composition: 5,
            lighting: 6,
        };
        let only_light_changes = ConceptRecord {
            lighting: 7,
            ..base.clone()
        };
        let subject_and_place_change = ConceptRecord {
            subject: 8,
            environment: 9,
            ..base.clone()
        };
        assert!(
            concept_similarity(&base, &only_light_changes)
                > concept_similarity(&base, &subject_and_place_change)
        );
    }

    #[test]
    fn brief_history_round_trips_and_seeds_future_novelty() {
        let path = std::env::temp_dir().join(format!(
            "thevisualizer-director-{}-{}.tsv",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let features = energetic_bass();
        let first_id;
        {
            let mut director = VisualDirector::with_history_path(path.clone());
            director.compose(
                "TheVisualCityScape",
                &features,
                "Cyber Neon",
                "Glass",
                "bass #FF0020,\ttreble #2080FF\n",
            );
            let entry = director.history().back().expect("saved brief");
            first_id = entry.novelty_id;
            assert!(entry.prompt.contains("bass #FF0020,\ttreble"));
            assert!(director.history_error.is_none());
            let export = director.export_current_brief_inner().expect("export brief");
            let markdown = fs::read_to_string(&export).expect("read export");
            assert!(markdown.contains("## Visual DNA"));
            assert!(markdown.contains("## Generation prompt"));
            fs::remove_file(export).expect("remove isolated export");
        }

        let mut reloaded = VisualDirector::with_history_path(path.clone());
        assert_eq!(reloaded.history().len(), 1);
        assert_eq!(reloaded.history().back().unwrap().novelty_id, first_id);
        reloaded.compose(
            "TheVisualCityScape",
            &features,
            "Cyber Neon",
            "Glass",
            "bass #FF0020, treble #2080FF",
        );
        assert_ne!(reloaded.brief.as_ref().unwrap().novelty_id, first_id);

        fs::remove_file(path).expect("remove isolated history");
    }
}
