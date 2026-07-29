use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_SCENE_BYTES: u64 = 64 * 1024;
const MAX_DISCOVERED_SCENES: usize = 256;
pub const SCENE_PARAMETER_COUNT: usize = 40;
pub const SCENE_ZONE_LIMIT: usize = 8;

#[derive(Clone, Debug, PartialEq)]
pub struct SceneZone {
    pub x: f32,
    pub y: f32,
    pub radius: f32,
    pub strength: f32,
    pub band: u8,
    pub pinned: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SceneSnapshot {
    pub saved_at_ms: u128,
    pub name: String,
    pub mode_id: String,
    pub mode_name: String,
    pub gain: f32,
    pub palette: u8,
    pub finish: u8,
    pub colors: [[u8; 4]; 5],
    pub glow: f32,
    pub gloss: f32,
    pub saturation: f32,
    pub color_motion: [f32; 2],
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub camera_zoom: f32,
    pub show_handles: bool,
    pub selected_zone: Option<usize>,
    pub zones: Vec<SceneZone>,
    pub parameters: [f32; SCENE_PARAMETER_COUNT],
    pub forge_state: Option<String>,
    pub performance_state: Option<String>,
}

#[derive(Clone)]
pub struct SavedScene {
    pub path: PathBuf,
    pub snapshot: SceneSnapshot,
}

pub struct SceneDiscovery {
    pub scenes: Vec<SavedScene>,
    pub errors: Vec<String>,
}

pub fn default_directory() -> PathBuf {
    if let Some(path) = std::env::var_os("THEVISUALIZER_SCENES") {
        return path.into();
    }
    std::env::var_os("LOCALAPPDATA").map_or_else(
        || PathBuf::from("scenes"),
        |base| PathBuf::from(base).join("TheVisualizer").join("scenes"),
    )
}

pub fn save(directory: &Path, mut snapshot: SceneSnapshot) -> Result<SavedScene, String> {
    snapshot.saved_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    validate(&snapshot)?;
    fs::create_dir_all(directory)
        .map_err(|error| format!("Could not create scene folder: {error}"))?;
    let path = directory.join(format!(
        "scene-{}-{:016X}.tvscene",
        snapshot.saved_at_ms,
        fingerprint(&format!("{}:{}", snapshot.mode_id, snapshot.name))
    ));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .map_err(|error| format!("Could not create scene snapshot: {error}"))?;
    file.write_all(encode(&snapshot).as_bytes())
        .map_err(|error| format!("Could not save scene snapshot: {error}"))?;
    Ok(SavedScene { path, snapshot })
}

pub fn discover(directory: &Path) -> SceneDiscovery {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return SceneDiscovery {
                scenes: Vec::new(),
                errors: Vec::new(),
            };
        }
        Err(error) => {
            return SceneDiscovery {
                scenes: Vec::new(),
                errors: vec![format!(
                    "Could not read scene folder {}: {error}",
                    directory.display()
                )],
            };
        }
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tvscene"))
        })
        .take(MAX_DISCOVERED_SCENES)
        .collect::<Vec<_>>();
    paths.sort();
    let mut scenes = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        let result = fs::metadata(&path)
            .map_err(|error| error.to_string())
            .and_then(|metadata| {
                if metadata.len() > MAX_SCENE_BYTES {
                    Err(format!("scene exceeds {MAX_SCENE_BYTES} bytes"))
                } else {
                    fs::read_to_string(&path).map_err(|error| error.to_string())
                }
            })
            .and_then(|source| decode(&source));
        match result {
            Ok(snapshot) => scenes.push(SavedScene { path, snapshot }),
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }
    scenes.sort_by_key(|scene| std::cmp::Reverse(scene.snapshot.saved_at_ms));
    SceneDiscovery { scenes, errors }
}

fn encode(snapshot: &SceneSnapshot) -> String {
    let colors = snapshot
        .colors
        .iter()
        .map(|color| {
            format!(
                "{:02X}{:02X}{:02X}{:02X}",
                color[0], color[1], color[2], color[3]
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let parameters = snapshot
        .parameters
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let mut source = format!(
        "format=4\nsaved_at_ms={}\nname={}\nmode_id={}\nmode_name={}\ngain={}\n\
         palette={}\nfinish={}\ncolors={colors}\nmaterial={},{},{}\n\
         color_motion={},{}\ncamera={},{},{}\nshow_handles={}\nselected_zone={}\nparameters={parameters}\n",
        snapshot.saved_at_ms,
        hex_encode(&snapshot.name),
        hex_encode(&snapshot.mode_id),
        hex_encode(&snapshot.mode_name),
        snapshot.gain,
        snapshot.palette,
        snapshot.finish,
        snapshot.glow,
        snapshot.gloss,
        snapshot.saturation,
        snapshot.color_motion[0],
        snapshot.color_motion[1],
        snapshot.camera_yaw,
        snapshot.camera_pitch,
        snapshot.camera_zoom,
        u8::from(snapshot.show_handles),
        snapshot.selected_zone.map_or(-1, |index| index as i32),
    );
    for zone in &snapshot.zones {
        source.push_str(&format!(
            "zone={},{},{},{},{},{}\n",
            zone.x,
            zone.y,
            zone.radius,
            zone.strength,
            zone.band,
            u8::from(zone.pinned)
        ));
    }
    if let Some(forge_state) = &snapshot.forge_state {
        source.push_str(&format!("forge_state={}\n", hex_encode(forge_state)));
    }
    if let Some(performance_state) = &snapshot.performance_state {
        source.push_str(&format!(
            "performance_state={}\n",
            hex_encode(performance_state)
        ));
    }
    source
}

fn decode(source: &str) -> Result<SceneSnapshot, String> {
    let mut format = None;
    let mut saved_at_ms = None;
    let mut name = None;
    let mut mode_id = None;
    let mut mode_name = None;
    let mut gain = None;
    let mut palette = None;
    let mut finish = None;
    let mut colors = None;
    let mut material = None;
    let mut color_motion = None;
    let mut camera = None;
    let mut show_handles = None;
    let mut selected_zone = None;
    let mut parameters = None;
    let mut zones = Vec::new();
    let mut forge_state = None;
    let mut performance_state = None;
    for line in source.lines().filter(|line| !line.trim().is_empty()) {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("invalid scene line `{line}`"))?;
        match key {
            "format" => set_once(&mut format, parse(value, "format")?, key)?,
            "saved_at_ms" => set_once(&mut saved_at_ms, parse(value, key)?, key)?,
            "name" => set_once(&mut name, hex_decode(value)?, key)?,
            "mode_id" => set_once(&mut mode_id, hex_decode(value)?, key)?,
            "mode_name" => set_once(&mut mode_name, hex_decode(value)?, key)?,
            "gain" => set_once(&mut gain, parse(value, key)?, key)?,
            "palette" => set_once(&mut palette, parse(value, key)?, key)?,
            "finish" => set_once(&mut finish, parse(value, key)?, key)?,
            "colors" => set_once(&mut colors, parse_colors(value)?, key)?,
            "material" => set_once(&mut material, parse_floats::<3>(value, key)?, key)?,
            "color_motion" => {
                set_once(&mut color_motion, parse_floats::<2>(value, key)?, key)?;
            }
            "camera" => set_once(&mut camera, parse_floats::<3>(value, key)?, key)?,
            "show_handles" => set_once(&mut show_handles, parse_bool(value, key)?, key)?,
            "selected_zone" => set_once(&mut selected_zone, parse(value, key)?, key)?,
            "parameters" => {
                set_once(
                    &mut parameters,
                    parse_floats::<SCENE_PARAMETER_COUNT>(value, key)?,
                    key,
                )?;
            }
            "forge_state" => set_once(&mut forge_state, hex_decode(value)?, key)?,
            "performance_state" => {
                set_once(&mut performance_state, hex_decode(value)?, key)?;
            }
            "zone" => {
                if zones.len() == SCENE_ZONE_LIMIT {
                    return Err(format!("scene exceeds {SCENE_ZONE_LIMIT} zones"));
                }
                let values = value.split(',').collect::<Vec<_>>();
                if values.len() != 6 {
                    return Err("zone must contain six values".to_owned());
                }
                zones.push(SceneZone {
                    x: parse(values[0], "zone x")?,
                    y: parse(values[1], "zone y")?,
                    radius: parse(values[2], "zone radius")?,
                    strength: parse(values[3], "zone strength")?,
                    band: parse(values[4], "zone band")?,
                    pinned: parse_bool(values[5], "zone pinned")?,
                });
            }
            _ => return Err(format!("unknown scene key `{key}`")),
        }
    }
    if !matches!(format, Some(1..=4)) {
        return Err("unsupported or missing scene format".to_owned());
    }
    let material = material.ok_or_else(|| "missing `material`".to_owned())?;
    let camera = camera.ok_or_else(|| "missing `camera`".to_owned())?;
    let selected: i32 = selected_zone.ok_or_else(|| "missing `selected_zone`".to_owned())?;
    let snapshot = SceneSnapshot {
        saved_at_ms: saved_at_ms.ok_or_else(|| "missing `saved_at_ms`".to_owned())?,
        name: name.ok_or_else(|| "missing `name`".to_owned())?,
        mode_id: mode_id.ok_or_else(|| "missing `mode_id`".to_owned())?,
        mode_name: mode_name.ok_or_else(|| "missing `mode_name`".to_owned())?,
        gain: gain.ok_or_else(|| "missing `gain`".to_owned())?,
        palette: palette.ok_or_else(|| "missing `palette`".to_owned())?,
        finish: finish.ok_or_else(|| "missing `finish`".to_owned())?,
        colors: colors.ok_or_else(|| "missing `colors`".to_owned())?,
        glow: material[0],
        gloss: material[1],
        saturation: material[2],
        color_motion: color_motion.unwrap_or([0.0, 0.0]),
        camera_yaw: camera[0],
        camera_pitch: camera[1],
        camera_zoom: camera[2],
        show_handles: show_handles.ok_or_else(|| "missing `show_handles`".to_owned())?,
        selected_zone: (selected >= 0).then_some(selected as usize),
        zones,
        parameters: parameters.ok_or_else(|| "missing `parameters`".to_owned())?,
        forge_state,
        performance_state,
    };
    validate(&snapshot)?;
    Ok(snapshot)
}

fn validate(snapshot: &SceneSnapshot) -> Result<(), String> {
    if snapshot.name.trim().is_empty() || snapshot.name.len() > 128 {
        return Err("scene name must contain 1–128 bytes".to_owned());
    }
    if snapshot.mode_id.trim().is_empty() || snapshot.mode_id.len() > 128 {
        return Err("mode id must contain 1–128 bytes".to_owned());
    }
    if snapshot.mode_name.trim().is_empty() || snapshot.mode_name.len() > 128 {
        return Err("mode name must contain 1–128 bytes".to_owned());
    }
    finite_range(snapshot.gain, 0.25, 6.0, "gain")?;
    if snapshot.palette > 7 || snapshot.finish > 4 {
        return Err("palette or finish is out of range".to_owned());
    }
    finite_range(snapshot.glow, 0.0, 2.0, "glow")?;
    finite_range(snapshot.gloss, 0.0, 1.0, "gloss")?;
    finite_range(snapshot.saturation, 0.0, 1.5, "saturation")?;
    finite_range(snapshot.color_motion[0], -1.0, 1.0, "hue shift")?;
    finite_range(snapshot.color_motion[1], -2.0, 2.0, "phase speed")?;
    finite_range(
        snapshot.camera_yaw,
        -std::f32::consts::TAU,
        std::f32::consts::TAU,
        "camera yaw",
    )?;
    finite_range(snapshot.camera_pitch, -1.2, 1.2, "camera pitch")?;
    finite_range(snapshot.camera_zoom, 0.35, 3.0, "camera zoom")?;
    if snapshot.zones.len() > SCENE_ZONE_LIMIT {
        return Err(format!("scene exceeds {SCENE_ZONE_LIMIT} zones"));
    }
    if snapshot
        .selected_zone
        .is_some_and(|index| index >= snapshot.zones.len())
    {
        return Err("selected zone does not exist".to_owned());
    }
    for zone in &snapshot.zones {
        finite_range(zone.x, 0.0, 1.0, "zone x")?;
        finite_range(zone.y, 0.0, 1.0, "zone y")?;
        finite_range(zone.radius, 0.04, 0.45, "zone radius")?;
        finite_range(zone.strength, 0.0, 3.0, "zone strength")?;
        if zone.band > 3 {
            return Err("zone band is out of range".to_owned());
        }
    }
    if snapshot.parameters.iter().any(|value| !value.is_finite()) {
        return Err("scene parameters must be finite".to_owned());
    }
    if snapshot
        .forge_state
        .as_ref()
        .is_some_and(|state| state.len() > 16 * 1024 || !state.is_ascii())
    {
        return Err("forge state is invalid or oversized".to_owned());
    }
    if snapshot
        .performance_state
        .as_ref()
        .is_some_and(|state| state.len() > 16 * 1024 || !state.is_ascii())
    {
        return Err("performance state is invalid or oversized".to_owned());
    }
    Ok(())
}

fn parse<T: std::str::FromStr>(value: &str, label: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid {label} value `{value}`"))
}

fn parse_bool(value: &str, label: &str) -> Result<bool, String> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(format!("{label} must be 0 or 1")),
    }
}

fn parse_floats<const N: usize>(value: &str, label: &str) -> Result<[f32; N], String> {
    let values = value.split(',').collect::<Vec<_>>();
    if values.len() != N {
        return Err(format!("{label} must contain {N} values"));
    }
    let parsed = values
        .iter()
        .map(|value| parse(value, label))
        .collect::<Result<Vec<f32>, _>>()?;
    parsed
        .try_into()
        .map_err(|_| format!("{label} must contain {N} values"))
}

fn parse_colors(value: &str) -> Result<[[u8; 4]; 5], String> {
    let values = value.split(',').collect::<Vec<_>>();
    if values.len() != 5 {
        return Err("colors must contain five RGBA values".to_owned());
    }
    let mut colors = [[0; 4]; 5];
    for (target, value) in colors.iter_mut().zip(values) {
        if value.len() != 8 {
            return Err("each color must be eight hexadecimal digits".to_owned());
        }
        for (byte, pair) in target.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
            let pair = std::str::from_utf8(pair).map_err(|_| "invalid color".to_owned())?;
            *byte = u8::from_str_radix(pair, 16).map_err(|_| "invalid color".to_owned())?;
        }
    }
    Ok(colors)
}

fn finite_range(value: f32, minimum: f32, maximum: f32, label: &str) -> Result<(), String> {
    if value.is_finite() && (minimum..=maximum).contains(&value) {
        Ok(())
    } else {
        Err(format!("{label} is out of range"))
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, key: &str) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err(format!("duplicate scene key `{key}`"))
    } else {
        Ok(())
    }
}

fn hex_encode(text: &str) -> String {
    text.bytes().map(|byte| format!("{byte:02X}")).collect()
}

fn hex_decode(text: &str) -> Result<String, String> {
    if !text.len().is_multiple_of(2) {
        return Err("invalid encoded text".to_owned());
    }
    let bytes = text
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| "invalid encoded text".to_owned())?;
    String::from_utf8(bytes).map_err(|_| "encoded text is not UTF-8".to_owned())
}

fn fingerprint(text: &str) -> u64 {
    text.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> SceneSnapshot {
        SceneSnapshot {
            saved_at_ms: 0,
            name: "Bass balcony".to_owned(),
            mode_id: "thevisualizer.cityscape".to_owned(),
            mode_name: "TheVisualCityScape".to_owned(),
            gain: 1.7,
            palette: 0,
            finish: 4,
            colors: [[255, 0, 0, 255]; 5],
            glow: 1.2,
            gloss: 0.7,
            saturation: 1.0,
            color_motion: [0.0, 0.0],
            camera_yaw: 0.4,
            camera_pitch: 0.1,
            camera_zoom: 1.3,
            show_handles: true,
            selected_zone: Some(0),
            zones: vec![SceneZone {
                x: 0.3,
                y: 0.48,
                radius: 0.18,
                strength: 1.35,
                band: 1,
                pinned: true,
            }],
            parameters: [0.5; SCENE_PARAMETER_COUNT],
            forge_state: None,
            performance_state: None,
        }
    }

    #[test]
    fn scene_snapshot_round_trips_through_discovery() {
        let directory = std::env::temp_dir().join(format!(
            "thevisualizer-scene-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let saved = save(&directory, snapshot()).expect("save scene");
        let discovery = discover(&directory);
        assert!(discovery.errors.is_empty());
        assert_eq!(discovery.scenes.len(), 1);
        assert_eq!(discovery.scenes[0].snapshot, saved.snapshot);
        fs::remove_dir_all(directory).expect("remove isolated scenes");
    }

    #[test]
    fn scene_snapshot_rejects_unbounded_values() {
        let mut invalid = snapshot();
        invalid.zones[0].radius = f32::INFINITY;
        assert!(validate(&invalid).is_err());
        invalid = snapshot();
        invalid.selected_zone = Some(5);
        assert!(validate(&invalid).is_err());
    }

    #[test]
    fn format_one_scene_migrates_without_forge_state() {
        let legacy = encode(&snapshot())
            .replacen("format=4", "format=1", 1)
            .lines()
            .filter(|line| !line.starts_with("color_motion="))
            .collect::<Vec<_>>()
            .join("\n");
        let restored = decode(&legacy).expect("decode legacy scene");
        assert!(restored.forge_state.is_none());
    }
}
