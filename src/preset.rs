use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_PRESET_BYTES: u64 = 128 * 1024;
const PRESET_FORMAT_VERSION: u32 = 1;

#[derive(Clone)]
pub struct PresetParameter {
    pub minimum: f32,
    pub maximum: f32,
    pub default: f32,
}

#[derive(Clone)]
pub struct Preset {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub license: String,
    pub response: PresetParameter,
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
        let mut response = None;

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
                "parameter" => set_once(&mut response, parse_response(value)?)?,
                key => return Err(format!("unknown metadata key `{key}`")),
            }
        }

        if format != Some(PRESET_FORMAT_VERSION) {
            return Err(format!(
                "unsupported preset format {}; expected {PRESET_FORMAT_VERSION}",
                format.map_or_else(|| "missing".to_owned(), |value| value.to_string())
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
        let response = response.ok_or("missing `parameter` metadata")?;
        let shader = shader.trim();
        if shader.is_empty() {
            return Err("shader source is empty".to_owned());
        }

        Ok(Self {
            id,
            name,
            version,
            author,
            license,
            response,
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
                    .is_some_and(|extension| extension == "tvpreset")
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

fn parse_response(value: &str) -> Result<PresetParameter, String> {
    let parts = value.split('|').map(str::trim).collect::<Vec<_>>();
    if parts.len() != 4 || parts[0] != "response" {
        return Err("parameter must be `response|min|max|default`".to_owned());
    }
    let minimum = parts[1]
        .parse::<f32>()
        .map_err(|_| "invalid response minimum")?;
    let maximum = parts[2]
        .parse::<f32>()
        .map_err(|_| "invalid response maximum")?;
    let default = parts[3]
        .parse::<f32>()
        .map_err(|_| "invalid response default")?;
    if !minimum.is_finite()
        || !maximum.is_finite()
        || !default.is_finite()
        || minimum >= maximum
        || !(minimum..=maximum).contains(&default)
    {
        return Err("response bounds/default are invalid".to_owned());
    }
    Ok(PresetParameter {
        minimum,
        maximum,
        default,
    })
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
        assert_eq!(bundled.presets.len(), 2);
    }
}
