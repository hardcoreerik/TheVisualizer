pub const PERFORMANCE_MACROS: usize = 10;
pub const MAX_FAVORITE_SCENES: usize = 64;

#[derive(Clone, Debug, PartialEq)]
pub struct Deck {
    pub mode_id: String,
    pub mode_name: String,
    pub level: f32,
    pub speed: f32,
    pub hue_shift: f32,
    pub frozen: bool,
    pub muted: bool,
}

impl Deck {
    pub fn new(mode_id: impl Into<String>, mode_name: impl Into<String>) -> Self {
        Self {
            mode_id: mode_id.into(),
            mode_name: mode_name.into(),
            level: 1.0,
            speed: 1.0,
            hue_shift: 0.0,
            frozen: false,
            muted: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transition {
    Crossfade,
    Luma,
    Radial,
    Horizontal,
    Vertical,
    PixelDissolve,
    Kaleido,
    Glitch,
    FeedbackBloom,
    AudioCut,
}

impl Transition {
    pub const ALL: [Self; 10] = [
        Self::Crossfade,
        Self::Luma,
        Self::Radial,
        Self::Horizontal,
        Self::Vertical,
        Self::PixelDissolve,
        Self::Kaleido,
        Self::Glitch,
        Self::FeedbackBloom,
        Self::AudioCut,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Crossfade => "Crossfade",
            Self::Luma => "Luma",
            Self::Radial => "Radial",
            Self::Horizontal => "Horizontal wipe",
            Self::Vertical => "Vertical wipe",
            Self::PixelDissolve => "Pixel dissolve",
            Self::Kaleido => "Kaleido fold",
            Self::Glitch => "Glitch bands",
            Self::FeedbackBloom => "Feedback bloom",
            Self::AudioCut => "Audio cut",
        }
    }

    fn from_code(code: u8) -> Result<Self, String> {
        Self::ALL
            .get(code as usize)
            .copied()
            .ok_or_else(|| "invalid transition".to_owned())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MacroSource {
    Full,
    Bass,
    Mid,
    Treble,
    Transient,
    Onset,
}

impl MacroSource {
    pub const ALL: [Self; 6] = [
        Self::Full,
        Self::Bass,
        Self::Mid,
        Self::Treble,
        Self::Transient,
        Self::Onset,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Full => "Full",
            Self::Bass => "Bass",
            Self::Mid => "Mid",
            Self::Treble => "Treble",
            Self::Transient => "Transient",
            Self::Onset => "Onset",
        }
    }

    fn from_code(code: u8) -> Result<Self, String> {
        Self::ALL
            .get(code as usize)
            .copied()
            .ok_or_else(|| "invalid macro source".to_owned())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MacroControl {
    pub value: f32,
    pub source: MacroSource,
    pub audio_amount: f32,
    pub attack: f32,
    pub release: f32,
    pub curve: f32,
    pub inverted: bool,
    pub enabled: bool,
    pub resolved: f32,
}

impl Default for MacroControl {
    fn default() -> Self {
        Self {
            value: 0.5,
            source: MacroSource::Full,
            audio_amount: 0.0,
            attack: 0.08,
            release: 0.35,
            curve: 1.0,
            inverted: false,
            enabled: false,
            resolved: 0.5,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PerformanceState {
    pub decks: [Deck; 2],
    pub selected_deck: usize,
    pub crossfader: f32,
    pub transition: Transition,
    pub transition_softness: f32,
    pub transition_seconds: f32,
    pub beat_sync: bool,
    pub auto_take: bool,
    pub live_mix: bool,
    pub blackout: bool,
    pub freeze_output: bool,
    pub macros: [MacroControl; PERFORMANCE_MACROS],
    pub selected_macro: usize,
    pub scene_query: String,
    pub favorite_scenes: Vec<String>,
}

impl Default for PerformanceState {
    fn default() -> Self {
        Self {
            decks: [
                Deck::new("host.neon-scope", "Neon Scope"),
                Deck::new("host.particle-forge", "Particle Forge"),
            ],
            selected_deck: 0,
            crossfader: 0.0,
            transition: Transition::Crossfade,
            transition_softness: 0.35,
            transition_seconds: 1.5,
            beat_sync: true,
            auto_take: false,
            live_mix: false,
            blackout: false,
            freeze_output: false,
            macros: [MacroControl::default(); PERFORMANCE_MACROS],
            selected_macro: 0,
            scene_query: String::new(),
            favorite_scenes: Vec::new(),
        }
    }
}

impl PerformanceState {
    pub const MACRO_LABELS: [&'static str; PERFORMANCE_MACROS] = [
        "Energy",
        "Motion",
        "Depth",
        "Color",
        "Chaos",
        "Detail",
        "Camera",
        "FX",
        "Scale",
        "Atmosphere",
    ];

    pub fn assign(&mut self, deck: usize, mode_id: &str, mode_name: &str) {
        if let Some(target) = self.decks.get_mut(deck) {
            target.mode_id = mode_id.to_owned();
            target.mode_name = mode_name.to_owned();
        }
    }

    pub fn swap(&mut self) {
        self.decks.swap(0, 1);
        self.crossfader = 1.0 - self.crossfader;
        self.selected_deck = 1 - self.selected_deck.min(1);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn toggle_favorite(&mut self, key: &str) {
        if let Some(index) = self.favorite_scenes.iter().position(|item| item == key) {
            self.favorite_scenes.remove(index);
        } else if self.favorite_scenes.len() < MAX_FAVORITE_SCENES {
            self.favorite_scenes.push(key.to_owned());
        }
    }

    pub fn update_macros(
        &mut self,
        audio: [f32; 6],
        delta: f32,
    ) -> [Option<f32>; PERFORMANCE_MACROS] {
        let mut changed = [None; PERFORMANCE_MACROS];
        for (index, control) in self.macros.iter_mut().enumerate() {
            if !control.enabled {
                continue;
            }
            let source = audio[control.source as usize].clamp(0.0, 1.0);
            let source = if control.inverted {
                1.0 - source
            } else {
                source
            };
            let target =
                (control.value + source.powf(control.curve) * control.audio_amount).clamp(0.0, 1.0);
            let seconds = if target > control.resolved {
                control.attack
            } else {
                control.release
            };
            let amount = if seconds <= f32::EPSILON {
                1.0
            } else {
                (delta / seconds).clamp(0.0, 1.0)
            };
            control.resolved += (target - control.resolved) * amount;
            changed[index] = Some(control.resolved);
        }
        changed
    }

    pub fn transition_mix(&self, time: f32, audio: f32) -> f32 {
        let x = self.crossfader.clamp(0.0, 1.0);
        match self.transition {
            Transition::Crossfade => x,
            Transition::Luma => x * x * (3.0 - 2.0 * x),
            Transition::Radial => x.sqrt(),
            Transition::Horizontal => (x * 1.15 - 0.075).clamp(0.0, 1.0),
            Transition::Vertical => (x * 1.35 - 0.175).clamp(0.0, 1.0),
            Transition::PixelDissolve => (x * 16.0).floor() / 16.0,
            Transition::Kaleido => {
                (x + (time * 2.0).sin() * self.transition_softness * 0.025).clamp(0.0, 1.0)
            }
            Transition::Glitch => {
                (x + (time * 31.0).sin() * audio * self.transition_softness * 0.08).clamp(0.0, 1.0)
            }
            Transition::FeedbackBloom => x.powf(0.55 + self.transition_softness),
            Transition::AudioCut => {
                if x + audio * self.transition_softness * 0.25 >= 0.5 {
                    1.0
                } else {
                    0.0
                }
            }
        }
    }

    pub fn encode_scene(&self) -> String {
        let mut output = format!(
            "version=1\nstate={},{},{},{},{},{},{},{},{},{}\n",
            self.selected_deck,
            self.crossfader,
            self.transition as u8,
            self.transition_softness,
            self.transition_seconds,
            u8::from(self.beat_sync),
            u8::from(self.auto_take),
            u8::from(self.live_mix),
            u8::from(self.blackout),
            u8::from(self.freeze_output),
        );
        for deck in &self.decks {
            output.push_str(&format!(
                "deck={},{},{},{},{},{},{}\n",
                hex(&deck.mode_id),
                hex(&deck.mode_name),
                deck.level,
                deck.speed,
                deck.hue_shift,
                u8::from(deck.frozen),
                u8::from(deck.muted),
            ));
        }
        for control in &self.macros {
            output.push_str(&format!(
                "macro={},{},{},{},{},{},{},{}\n",
                control.value,
                control.source as u8,
                control.audio_amount,
                control.attack,
                control.release,
                control.curve,
                u8::from(control.inverted),
                u8::from(control.enabled),
            ));
        }
        output.push_str(&format!("query={}\n", hex(&self.scene_query)));
        for favorite in &self.favorite_scenes {
            output.push_str(&format!("favorite={}\n", hex(favorite)));
        }
        output
    }

    pub fn decode_scene(source: &str) -> Result<Self, String> {
        if source.len() > 16 * 1024 || !source.is_ascii() {
            return Err("performance state is invalid or oversized".to_owned());
        }
        let mut version = None;
        let mut state_values = None;
        let mut decks = Vec::new();
        let mut macros = Vec::new();
        let mut scene_query = None;
        let mut favorite_scenes = Vec::new();
        for line in source.lines().filter(|line| !line.is_empty()) {
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| "invalid performance state line".to_owned())?;
            match key {
                "version" => set_once(&mut version, parse(value)?)?,
                "state" => set_once(&mut state_values, value.to_owned())?,
                "deck" if decks.len() < 2 => decks.push(parse_deck(value)?),
                "macro" if macros.len() < PERFORMANCE_MACROS => {
                    macros.push(parse_macro(value)?);
                }
                "query" => set_once(&mut scene_query, unhex(value)?)?,
                "favorite" if favorite_scenes.len() < MAX_FAVORITE_SCENES => {
                    favorite_scenes.push(unhex(value)?);
                }
                "favorite" => return Err("performance favorites exceed limits".to_owned()),
                "deck" | "macro" => return Err("performance state exceeds limits".to_owned()),
                _ => return Err(format!("unknown performance state key `{key}`")),
            }
        }
        if version != Some(1) || decks.len() != 2 || macros.len() != PERFORMANCE_MACROS {
            return Err("incomplete performance state".to_owned());
        }
        if scene_query.as_ref().is_some_and(|query| query.len() > 128)
            || favorite_scenes.iter().any(|favorite| favorite.len() > 256)
        {
            return Err("performance browser text exceeds limits".to_owned());
        }
        let state_values = state_values.ok_or_else(|| "missing performance state".to_owned())?;
        let values = split::<10>(&state_values)?;
        Ok(Self {
            decks: decks.try_into().map_err(|_| "missing decks".to_owned())?,
            selected_deck: parse_range(values[0], 0usize, 1usize)?,
            crossfader: parse_f32(values[1], 0.0, 1.0)?,
            transition: Transition::from_code(parse(values[2])?)?,
            transition_softness: parse_f32(values[3], 0.0, 1.0)?,
            transition_seconds: parse_f32(values[4], 0.05, 16.0)?,
            beat_sync: parse_bool(values[5])?,
            auto_take: parse_bool(values[6])?,
            live_mix: parse_bool(values[7])?,
            blackout: parse_bool(values[8])?,
            freeze_output: parse_bool(values[9])?,
            macros: macros.try_into().map_err(|_| "missing macros".to_owned())?,
            selected_macro: 0,
            scene_query: scene_query.unwrap_or_default(),
            favorite_scenes,
        })
    }
}

fn parse_deck(value: &str) -> Result<Deck, String> {
    let values = split::<7>(value)?;
    let mode_id = unhex(values[0])?;
    let mode_name = unhex(values[1])?;
    if mode_id.is_empty() || mode_id.len() > 128 || mode_name.is_empty() || mode_name.len() > 128 {
        return Err("invalid deck identity".to_owned());
    }
    Ok(Deck {
        mode_id,
        mode_name,
        level: parse_f32(values[2], 0.0, 1.5)?,
        speed: parse_f32(values[3], 0.0, 2.0)?,
        hue_shift: parse_f32(values[4], -1.0, 1.0)?,
        frozen: parse_bool(values[5])?,
        muted: parse_bool(values[6])?,
    })
}

fn parse_macro(value: &str) -> Result<MacroControl, String> {
    let values = split::<8>(value)?;
    let value = parse_f32(values[0], 0.0, 1.0)?;
    Ok(MacroControl {
        value,
        source: MacroSource::from_code(parse(values[1])?)?,
        audio_amount: parse_f32(values[2], -1.0, 1.0)?,
        attack: parse_f32(values[3], 0.0, 2.0)?,
        release: parse_f32(values[4], 0.0, 4.0)?,
        curve: parse_f32(values[5], 0.25, 4.0)?,
        inverted: parse_bool(values[6])?,
        enabled: parse_bool(values[7])?,
        resolved: value,
    })
}

fn split<const N: usize>(value: &str) -> Result<[&str; N], String> {
    value
        .split(',')
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| format!("expected {N} values"))
}

fn parse<T: std::str::FromStr>(value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid performance value `{value}`"))
}

fn parse_range<T>(value: &str, minimum: T, maximum: T) -> Result<T, String>
where
    T: std::str::FromStr + PartialOrd,
{
    let value: T = parse(value)?;
    if (minimum..=maximum).contains(&value) {
        Ok(value)
    } else {
        Err("performance value is out of range".to_owned())
    }
}

fn parse_f32(value: &str, minimum: f32, maximum: f32) -> Result<f32, String> {
    let value: f32 = parse(value)?;
    if value.is_finite() && (minimum..=maximum).contains(&value) {
        Ok(value)
    } else {
        Err("performance value is out of range".to_owned())
    }
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err("performance flag must be 0 or 1".to_owned()),
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err("duplicate performance state value".to_owned())
    } else {
        Ok(())
    }
}

fn hex(value: &str) -> String {
    value.bytes().map(|byte| format!("{byte:02X}")).collect()
}

fn unhex(value: &str) -> Result<String, String> {
    if !value.len().is_multiple_of(2) {
        return Err("invalid encoded deck identity".to_owned());
    }
    let bytes = value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "invalid encoded deck identity".to_owned())?;
    String::from_utf8(bytes).map_err(|_| "deck identity is not UTF-8".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deck_swap_preserves_the_live_mix() {
        let mut state = PerformanceState {
            crossfader: 0.2,
            selected_deck: 0,
            ..PerformanceState::default()
        };
        state.swap();
        assert_eq!(state.crossfader, 0.8);
        assert_eq!(state.selected_deck, 1);
        assert_eq!(state.decks[0].mode_id, "host.particle-forge");
        assert_eq!(state.macros.len(), 10);
        assert_eq!(Transition::ALL.len(), 10);
        assert_eq!(
            PerformanceState::decode_scene(&state.encode_scene()).unwrap(),
            state
        );
    }
}
