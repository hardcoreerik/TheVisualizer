use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_PRESET_BYTES: u64 = 128 * 1024;
const MAX_PRESET_FORMAT_VERSION: u32 = 2;
const MAX_PRESET_PARAMETERS: usize = 40;

#[derive(Clone)]
pub struct PresetParameter {
    pub id: String,
    pub label: String,
    pub group: String,
    pub minimum: f32,
    pub maximum: f32,
    pub default: f32,
}

#[derive(Clone)]
pub struct Preset {
    pub format: u32,
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub license: String,
    pub response: PresetParameter,
    pub parameters: Vec<PresetParameter>,
    pub shader: String,
    pub path: PathBuf,
}

pub struct PresetDiscovery {
    pub presets: Vec<Preset>,
    pub errors: Vec<String>,
}

impl Preset {
    fn parse(source: &str, path: PathBuf) -> Result<Self, String> {
        if source.len() as u64 > MAX_PRESET_BYTES {
            return Err(format!("preset exceeds {MAX_PRESET_BYTES} bytes"));
        }

        let (header, shader) = source
            .split_once("//! ---")
            .ok_or("missing `//! ---` metadata terminator")?;
        let mut format = None;
        let mut id = None;
        let mut name = None;
        let mut version = None;
        let mut author = None;
        let mut license = None;
        let mut parameter_values = Vec::new();

        for line in header.lines().filter(|line| !line.trim().is_empty()) {
            let metadata = line
                .strip_prefix("//! ")
                .ok_or_else(|| format!("invalid metadata line: {line}"))?;
            let (key, value) = metadata
                .split_once(':')
                .ok_or_else(|| format!("invalid metadata line: {line}"))?;
            let value = value.trim();
            match key.trim() {
                "format" => set_once(&mut format, value.parse().map_err(|_| "invalid format")?)?,
                "id" => set_once(&mut id, value.to_owned())?,
                "name" => set_once(&mut name, value.to_owned())?,
                "version" => set_once(&mut version, value.to_owned())?,
                "author" => set_once(&mut author, value.to_owned())?,
                "license" => set_once(&mut license, value.to_owned())?,
                "parameter" => parameter_values.push(value.to_owned()),
                key => return Err(format!("unknown metadata key `{key}`")),
            }
        }

        let format = format.ok_or("missing `format` metadata")?;
        if !(1..=MAX_PRESET_FORMAT_VERSION).contains(&format) {
            return Err(format!(
                "unsupported preset format {format}; expected 1..={MAX_PRESET_FORMAT_VERSION}"
            ));
        }
        let id = required(id, "id")?;
        if !valid_id(&id) {
            return Err("id must contain only lowercase letters, digits, `-`, or `.`".to_owned());
        }
        let name = required(name, "name")?;
        let version = required(version, "version")?;
        let author = required(author, "author")?;
        let license = required(license, "license")?;
        let parameters = match format {
            1 => {
                if parameter_values.len() != 1 {
                    return Err("format 1 requires exactly one `parameter`".to_owned());
                }
                vec![parse_response(&parameter_values[0])?]
            }
            2 => {
                if parameter_values.is_empty() || parameter_values.len() > MAX_PRESET_PARAMETERS {
                    return Err(format!(
                        "format 2 requires 1..={MAX_PRESET_PARAMETERS} parameters"
                    ));
                }
                let mut parameters = Vec::with_capacity(parameter_values.len());
                for value in parameter_values {
                    let parameter = parse_parameter(&value)?;
                    if parameters
                        .iter()
                        .any(|known: &PresetParameter| known.id == parameter.id)
                    {
                        return Err(format!("duplicate parameter id `{}`", parameter.id));
                    }
                    parameters.push(parameter);
                }
                parameters
            }
            _ => unreachable!(),
        };
        let response = parameters
            .iter()
            .find(|parameter| parameter.id == "response")
            .cloned()
            .ok_or("missing `response` parameter")?;
        let shader = shader.trim();
        if shader.is_empty() {
            return Err("shader source is empty".to_owned());
        }

        Ok(Self {
            format,
            id,
            name,
            version,
            author,
            license,
            response,
            parameters,
            shader: shader.to_owned(),
            path,
        })
    }
}

pub fn discover(directory: &Path) -> PresetDiscovery {
    let mut paths = match fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("tvpreset"))
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            return PresetDiscovery {
                presets: Vec::new(),
                errors: vec![format!("Could not read {}: {error}", directory.display())],
            };
        }
    };
    paths.sort();

    let mut presets = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        let result = fs::metadata(&path)
            .map_err(|error| error.to_string())
            .and_then(|metadata| {
                if metadata.len() > MAX_PRESET_BYTES {
                    Err(format!("exceeds {MAX_PRESET_BYTES} bytes"))
                } else {
                    fs::read_to_string(&path).map_err(|error| error.to_string())
                }
            })
            .and_then(|source| Preset::parse(&source, path.clone()));
        match result {
            Ok(preset) if presets.iter().any(|known: &Preset| known.id == preset.id) => {
                errors.push(format!(
                    "{}: duplicate preset id `{}`",
                    path.display(),
                    preset.id
                ));
            }
            Ok(preset) => presets.push(preset),
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }

    PresetDiscovery { presets, errors }
}

pub fn directory_signature(directory: &Path) -> u64 {
    let Ok(entries) = fs::read_dir(directory) else {
        return 0;
    };
    let mut items = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("tvpreset"))
        })
        .take(512)
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            let modified = metadata
                .modified()
                .ok()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()?
                .as_nanos();
            Some(format!(
                "{}:{}:{modified}",
                entry.file_name().to_string_lossy(),
                metadata.len()
            ))
        })
        .collect::<Vec<_>>();
    items.sort();
    items
        .iter()
        .flat_map(|item| item.bytes())
        .fold(0xcbf29ce484222325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        })
}

fn parse_response(value: &str) -> Result<PresetParameter, String> {
    let parts = value.split('|').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 || parts[0] != "response" {
        return Err("parameter must be `response|min|max|default`".to_owned());
    }
    let (minimum, maximum, default) = parse_bounds(parts[1], parts[2], parts[3])?;
    Ok(PresetParameter {
        id: "response".to_owned(),
        label: "Response".to_owned(),
        group: "Audio".to_owned(),
        minimum,
        maximum,
        default,
    })
}

fn parse_parameter(value: &str) -> Result<PresetParameter, String> {
    let parts = value.split('|').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 6 {
        return Err("format 2 parameter must be `group|id|label|min|max|default`".to_owned());
    }
    if parts[..3].iter().any(|part| part.is_empty()) || !valid_id(parts[1]) {
        return Err("parameter group, id, and label must be valid and non-empty".to_owned());
    }
    let (minimum, maximum, default) = parse_bounds(parts[3], parts[4], parts[5])?;
    Ok(PresetParameter {
        id: parts[1].to_owned(),
        label: parts[2].to_owned(),
        group: parts[0].to_owned(),
        minimum,
        maximum,
        default,
    })
}

fn parse_bounds(minimum: &str, maximum: &str, default: &str) -> Result<(f32, f32, f32), String> {
    let minimum = minimum
        .parse::<f32>()
        .map_err(|_| "invalid parameter minimum")?;
    let maximum = maximum
        .parse::<f32>()
        .map_err(|_| "invalid parameter maximum")?;
    let default = default
        .parse::<f32>()
        .map_err(|_| "invalid parameter default")?;
    if !minimum.is_finite()
        || !maximum.is_finite()
        || !default.is_finite()
        || minimum >= maximum
        || !(minimum..=maximum).contains(&default)
    {
        return Err("parameter bounds/default are invalid".to_owned());
    }
    Ok((minimum, maximum, default))
}

fn set_once<T>(slot: &mut Option<T>, value: T) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err("duplicate metadata key".to_owned())
    } else {
        Ok(())
    }
}

fn required(value: Option<String>, name: &str) -> Result<String, String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing `{name}` metadata"))
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-.".contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID: &str = "//! format: 1\n//! id: test.preset\n//! name: Test\n//! version: 0.1.0\n//! author: Test Author\n//! license: CC0-1.0\n//! parameter: response|0.5|6|2.2\n//! ---\n@fragment fn fs_main() {}";

    #[test]
    fn preset_metadata_is_strict_and_bounded() {
        let preset = Preset::parse(VALID, "test.tvpreset".into()).unwrap();
        assert_eq!(preset.id, "test.preset");
        assert_eq!(preset.response.default, 2.2);

        assert!(Preset::parse(&VALID.replace("format: 1", "format: 2"), PathBuf::new()).is_err());
        assert!(
            Preset::parse(
                &VALID.replace("response|0.5|6|2.2", "response|6|0.5|2.2"),
                PathBuf::new()
            )
            .is_err()
        );

        let bundled = discover(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/presets")));
        assert!(bundled.errors.is_empty(), "{:?}", bundled.errors);
        assert_eq!(bundled.presets.len(), 36);
        let city = bundled
            .presets
            .iter()
            .find(|preset| preset.id == "thevisualizer.cityscape")
            .unwrap();
        assert_eq!(city.format, 2);
        assert_eq!(city.parameters.len(), 35);
        for preset in &bundled.presets {
            assert_eq!(preset.format, 2);
            assert!((10..=40).contains(&preset.parameters.len()));
        }
        assert_eq!(
            bundled
                .presets
                .iter()
                .filter(|preset| preset.id.starts_with("isf."))
                .count(),
            8
        );
        assert_eq!(
            bundled
                .presets
                .iter()
                .filter(|preset| {
                    matches!(
                        preset.id.as_str(),
                        "thevisualizer.pulse-trace"
                            | "thevisualizer.spectrum-skyline"
                            | "thevisualizer.radial-burst"
                            | "thevisualizer.spectrogram-city"
                            | "thevisualizer.spectral-terrain"
                            | "thevisualizer.wave-tunnel"
                            | "thevisualizer.particle-ocean"
                            | "thevisualizer.wireframe-terrain"
                            | "thevisualizer.halo-spectrum"
                            | "thevisualizer.atomic-orbits"
                    )
                })
                .count(),
            10
        );
    }

    #[test]
    fn bundled_wgsl_compiles() {
        let instance = eframe::wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(
            &eframe::wgpu::RequestAdapterOptions {
                power_preference: eframe::wgpu::PowerPreference::LowPower,
                compatible_surface: None,
                force_fallback_adapter: false,
            },
        ))
        .expect("a graphics adapter is required to validate bundled WGSL");
        let (device, _) =
            pollster::block_on(adapter.request_device(&eframe::wgpu::DeviceDescriptor::default()))
                .unwrap();
        let bundled = discover(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/presets")));
        for preset in bundled.presets {
            let error_scope = device.push_error_scope(eframe::wgpu::ErrorFilter::Validation);
            let _ = device.create_shader_module(eframe::wgpu::ShaderModuleDescriptor {
                label: Some(&preset.name),
                source: eframe::wgpu::ShaderSource::Wgsl(preset.shader.as_str().into()),
            });
            assert!(
                pollster::block_on(error_scope.pop()).is_none(),
                "{} failed WGSL validation",
                preset.name
            );
        }
        let error_scope = device.push_error_scope(eframe::wgpu::ErrorFilter::Validation);
        let _ = device.create_shader_module(eframe::wgpu::ShaderModuleDescriptor {
            label: Some("Zone Studio"),
            source: eframe::wgpu::ShaderSource::Wgsl(include_str!("zone_overlay.wgsl").into()),
        });
        assert!(
            pollster::block_on(error_scope.pop()).is_none(),
            "Zone Studio failed WGSL validation"
        );
    }

    #[test]
    fn discovery_accepts_mixed_case_extension() {
        let directory =
            std::env::temp_dir().join(format!("thevisualizer-preset-{}", std::process::id()));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("test.TVPRESET"), VALID).unwrap();

        let discovery = discover(&directory);

        fs::remove_dir_all(directory).unwrap();
        assert!(discovery.errors.is_empty(), "{:?}", discovery.errors);
        assert_eq!(discovery.presets.len(), 1);
    }
}
