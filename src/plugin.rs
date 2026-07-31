use std::{
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use libloading::Library;
use sha2::{Digest, Sha256};
use thevisualizer_plugin_sdk::{
    ABI_VERSION, ABI_VERSION_V2, ENTRY_POINT, ENTRY_POINT_V2, EntryPointFn, EntryPointFnV2,
    FeatureSnapshotV1, FeatureSnapshotV2, MAX_PLUGIN_COMMANDS, OP_MULTIPLY, PLUGIN_OK,
    PluginCommandV2, PluginInitV2, PluginOutputV1, PluginOutputV2, PluginV1, PluginV2,
};

use crate::analysis::Features;

const PLUGIN_FORMAT_VERSION: u32 = 1;
const MAX_MANIFEST_BYTES: u64 = 16 * 1024;
const MAX_LIBRARY_BYTES: u64 = 64 * 1024 * 1024;
const MIN_RESPONSE_MULTIPLIER: f32 = 0.25;
const MAX_RESPONSE_MULTIPLIER: f32 = 2.0;

#[derive(Clone)]
pub struct PluginPackage {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub license: String,
    pub abi: u32,
    pub library_path: PathBuf,
    pub manifest_path: PathBuf,
    artifact_hash: [u8; 32],
}

impl PluginPackage {
    pub fn artifact_hash(&self) -> &[u8; 32] {
        &self.artifact_hash
    }

    pub fn hash_hex(&self) -> String {
        self.artifact_hash
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    pub fn display_library_path(&self) -> String {
        let path = self.library_path.display().to_string();
        path.strip_prefix(r"\\?\").unwrap_or(&path).to_owned()
    }
}

pub struct PluginDiscovery {
    pub packages: Vec<PluginPackage>,
    pub errors: Vec<String>,
}

pub struct LoadedPlugin {
    descriptor: LoadedDescriptor,
    _library: Library,
    package_id: String,
    artifact_hash: [u8; 32],
}

#[derive(Clone, Copy)]
enum LoadedDescriptor {
    V1(PluginV1),
    V2(PluginV2),
}

pub struct PluginFrame {
    pub response_multiplier: f32,
    pub commands: [PluginCommandV2; MAX_PLUGIN_COMMANDS],
    pub command_count: usize,
    pub event_flags: u32,
}

impl LoadedPlugin {
    pub fn load(package: &PluginPackage) -> Result<Self, String> {
        let metadata = fs::metadata(&package.library_path)
            .map_err(|error| format!("Could not inspect approved library: {error}"))?;
        if metadata.len() > MAX_LIBRARY_BYTES {
            return Err(format!(
                "Approved library now exceeds {MAX_LIBRARY_BYTES} bytes"
            ));
        }
        let current_hash = hash_file(&package.library_path)?;
        if current_hash != package.artifact_hash {
            return Err(format!(
                "{} changed after approval; refresh and review it again",
                package.name
            ));
        }

        // SAFETY: Loading native code is allowed only after the UI's explicit approval action.
        let library = unsafe { Library::new(&package.library_path) }.map_err(|error| {
            format!("Could not load {}: {error}", package.library_path.display())
        })?;
        // SAFETY: The fixed symbol is the versioned ABI entry point defined by the SDK.
        let (descriptor, result) = if package.abi == ABI_VERSION {
            // SAFETY: The fixed symbol is the versioned ABI entry point defined by the SDK.
            let entry = unsafe {
                *library
                    .get::<EntryPointFn>(ENTRY_POINT)
                    .map_err(|error| format!("Missing `thevisualizer_plugin_v1`: {error}"))?
            };
            // SAFETY: An approved plugin promises that the entry point returns a static descriptor.
            let descriptor = unsafe { checked_descriptor(entry(), ABI_VERSION)? };
            let initialize = descriptor
                .initialize
                .ok_or("Plugin does not provide initialize")?;
            descriptor
                .process
                .ok_or("Plugin does not provide process")?;
            descriptor
                .shutdown
                .ok_or("Plugin does not provide shutdown")?;
            // SAFETY: Function pointer and ABI were validated above.
            (LoadedDescriptor::V1(descriptor), unsafe { initialize() })
        } else {
            // SAFETY: The fixed symbol is the versioned ABI entry point defined by the SDK.
            let entry = unsafe {
                *library
                    .get::<EntryPointFnV2>(ENTRY_POINT_V2)
                    .map_err(|error| format!("Missing `thevisualizer_plugin_v2`: {error}"))?
            };
            // SAFETY: An approved plugin promises that the entry point returns a static descriptor.
            let descriptor = unsafe { checked_descriptor(entry(), ABI_VERSION_V2)? };
            let initialize = descriptor
                .initialize
                .ok_or("Plugin does not provide initialize")?;
            descriptor
                .process
                .ok_or("Plugin does not provide process")?;
            descriptor
                .shutdown
                .ok_or("Plugin does not provide shutdown")?;
            let init = PluginInitV2 {
                struct_size: size_of::<PluginInitV2>() as u32,
                abi_version: ABI_VERSION_V2,
                profile_id: package.id.as_ptr(),
                profile_id_len: package.id.len() as u32,
            };
            // SAFETY: The profile bytes remain valid for this synchronous initialization call.
            (LoadedDescriptor::V2(descriptor), unsafe {
                initialize(&init)
            })
        };
        if result != PLUGIN_OK {
            return Err(format!("Plugin initialization failed with code {result}"));
        }

        Ok(Self {
            descriptor,
            _library: library,
            package_id: package.id.clone(),
            artifact_hash: package.artifact_hash,
        })
    }

    pub fn package_id(&self) -> &str {
        &self.package_id
    }

    pub fn artifact_hash(&self) -> &[u8; 32] {
        &self.artifact_hash
    }

    pub fn process(
        &self,
        features: &Features,
        time_seconds: f32,
        delta_seconds: f32,
        controls: &[f32],
    ) -> Result<PluginFrame, String> {
        let base = FeatureSnapshotV1 {
            struct_size: size_of::<FeatureSnapshotV1>() as u32,
            abi_version: ABI_VERSION,
            time_seconds,
            delta_seconds,
            waveform: features.waveform.as_ptr(),
            waveform_len: features.waveform.len() as u32,
            spectrum: features.spectrum.as_ptr(),
            spectrum_len: features.spectrum.len() as u32,
            rms: features.rms,
            peak: features.peak,
            low: features.low,
            mid: features.mid,
            high: features.high,
        };
        match self.descriptor {
            LoadedDescriptor::V1(descriptor) => {
                let mut output = PluginOutputV1::default();
                let process = descriptor
                    .process
                    .ok_or("Plugin process function disappeared")?;
                // SAFETY: Both values remain alive and exclusively borrowed for this synchronous call.
                let result = unsafe { process(&base, &mut output) };
                if result != PLUGIN_OK {
                    return Err(format!("Plugin processing failed with code {result}"));
                }
                if output.struct_size < size_of::<PluginOutputV1>() as u32
                    || output.abi_version != ABI_VERSION
                    || !output.response_multiplier.is_finite()
                {
                    return Err("Plugin returned an invalid v1 output".to_owned());
                }
                Ok(PluginFrame {
                    response_multiplier: output
                        .response_multiplier
                        .clamp(MIN_RESPONSE_MULTIPLIER, MAX_RESPONSE_MULTIPLIER),
                    commands: [PluginCommandV2::default(); MAX_PLUGIN_COMMANDS],
                    command_count: 0,
                    event_flags: 0,
                })
            }
            LoadedDescriptor::V2(descriptor) => {
                let snapshot = FeatureSnapshotV2 {
                    struct_size: size_of::<FeatureSnapshotV2>() as u32,
                    abi_version: ABI_VERSION_V2,
                    base,
                    onset: features.onset,
                    transient: features.transient,
                    controls: controls.as_ptr(),
                    controls_len: controls.len() as u32,
                };
                let mut output = PluginOutputV2::default();
                let process = descriptor
                    .process
                    .ok_or("Plugin process function disappeared")?;
                // SAFETY: All pointers remain valid for this synchronous call.
                let result = unsafe { process(&snapshot, &mut output) };
                if result != PLUGIN_OK {
                    return Err(format!("Plugin processing failed with code {result}"));
                }
                if output.struct_size < size_of::<PluginOutputV2>() as u32
                    || output.abi_version != ABI_VERSION_V2
                    || output.command_count as usize > MAX_PLUGIN_COMMANDS
                    || output.commands[..output.command_count as usize]
                        .iter()
                        .any(|command| {
                            command.target > 7
                                || command.operation > OP_MULTIPLY
                                || !command.value.is_finite()
                        })
                {
                    return Err("Plugin returned an invalid v2 output".to_owned());
                }
                let response_multiplier = output.commands[..output.command_count as usize]
                    .iter()
                    .rev()
                    .find(|command| command.target == 0)
                    .map_or(1.0, |command| command.value)
                    .clamp(MIN_RESPONSE_MULTIPLIER, MAX_RESPONSE_MULTIPLIER);
                Ok(PluginFrame {
                    response_multiplier,
                    commands: output.commands,
                    command_count: output.command_count as usize,
                    event_flags: output.event_flags,
                })
            }
        }
    }
}

impl Drop for LoadedPlugin {
    fn drop(&mut self) {
        let shutdown = match self.descriptor {
            LoadedDescriptor::V1(descriptor) => descriptor.shutdown,
            LoadedDescriptor::V2(descriptor) => descriptor.shutdown,
        };
        if let Some(shutdown) = shutdown {
            // SAFETY: The library remains loaded until after this Drop implementation returns.
            unsafe { shutdown() };
        }
    }
}

unsafe fn checked_descriptor<T: Copy>(pointer: *const T, abi: u32) -> Result<T, String> {
    if pointer.is_null() {
        return Err("Plugin returned a null ABI descriptor".to_owned());
    }
    // SAFETY: The plugin contract guarantees a readable common header.
    let header = unsafe { &*pointer.cast::<AbiHeader>() };
    if header.struct_size < size_of::<T>() as u32 {
        return Err(format!(
            "Plugin descriptor is {} bytes; host requires at least {}",
            header.struct_size,
            size_of::<T>()
        ));
    }
    if header.abi_version != abi {
        return Err(format!(
            "Plugin ABI {} is incompatible with host ABI {abi}",
            header.abi_version
        ));
    }
    // SAFETY: The validated size covers the complete descriptor.
    Ok(unsafe { *pointer })
}

#[repr(C)]
struct AbiHeader {
    struct_size: u32,
    abi_version: u32,
}

struct Manifest {
    id: String,
    name: String,
    version: String,
    author: String,
    license: String,
    abi: u32,
    platform: String,
    library: PathBuf,
}

pub fn discover(directory: &Path) -> PluginDiscovery {
    let mut paths = match fs::read_dir(directory) {
        Ok(entries) => entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "tvplugin")
            })
            .collect::<Vec<_>>(),
        Err(error) => {
            return PluginDiscovery {
                packages: Vec::new(),
                errors: vec![format!("Could not read {}: {error}", directory.display())],
            };
        }
    };
    paths.sort();

    let mut packages = Vec::new();
    let mut errors = Vec::new();
    for path in paths {
        let result = read_manifest(&path).and_then(|manifest| {
            if manifest.platform != host_platform() {
                return Err(format!(
                    "targets {}; this host is {}",
                    manifest.platform,
                    host_platform()
                ));
            }
            let parent = path.parent().unwrap_or_else(|| Path::new("."));
            let library_path = parent
                .join(&manifest.library)
                .canonicalize()
                .map_err(|error| format!("could not resolve library: {error}"))?;
            let metadata = fs::metadata(&library_path)
                .map_err(|error| format!("could not inspect library: {error}"))?;
            if !metadata.is_file() {
                return Err("library path is not a file".to_owned());
            }
            if metadata.len() > MAX_LIBRARY_BYTES {
                return Err(format!("library exceeds {MAX_LIBRARY_BYTES} bytes"));
            }
            let artifact_hash = hash_file(&library_path)?;
            Ok(PluginPackage {
                id: manifest.id,
                name: manifest.name,
                version: manifest.version,
                author: manifest.author,
                license: manifest.license,
                abi: manifest.abi,
                library_path,
                manifest_path: path.clone(),
                artifact_hash,
            })
        });
        match result {
            Ok(package)
                if packages
                    .iter()
                    .any(|known: &PluginPackage| known.id == package.id) =>
            {
                errors.push(format!(
                    "{}: duplicate plugin id `{}`",
                    path.display(),
                    package.id
                ));
            }
            Ok(package) => packages.push(package),
            Err(error) => errors.push(format!("{}: {error}", path.display())),
        }
    }

    PluginDiscovery { packages, errors }
}

fn read_manifest(path: &Path) -> Result<Manifest, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    if metadata.len() > MAX_MANIFEST_BYTES {
        return Err(format!("manifest exceeds {MAX_MANIFEST_BYTES} bytes"));
    }
    let source = fs::read_to_string(path).map_err(|error| error.to_string())?;
    parse_manifest(&source)
}

fn parse_manifest(source: &str) -> Result<Manifest, String> {
    let mut format = None;
    let mut id = None;
    let mut name = None;
    let mut version = None;
    let mut author = None;
    let mut license = None;
    let mut abi = None;
    let mut platform = None;
    let mut library = None;

    for line in source.lines().filter(|line| !line.trim().is_empty()) {
        let (key, value) = line
            .split_once('=')
            .ok_or_else(|| format!("invalid manifest line: {line}"))?;
        let value = value.trim();
        match key.trim() {
            "format" => set_once(&mut format, parse_u32(value, "format")?)?,
            "id" => set_once(&mut id, value.to_owned())?,
            "name" => set_once(&mut name, value.to_owned())?,
            "version" => set_once(&mut version, value.to_owned())?,
            "author" => set_once(&mut author, value.to_owned())?,
            "license" => set_once(&mut license, value.to_owned())?,
            "abi" => set_once(&mut abi, parse_u32(value, "abi")?)?,
            "platform" => set_once(&mut platform, value.to_owned())?,
            "library" => set_once(&mut library, PathBuf::from(value))?,
            key => return Err(format!("unknown manifest key `{key}`")),
        }
    }

    if format != Some(PLUGIN_FORMAT_VERSION) {
        return Err(format!(
            "unsupported plugin format {}; expected {PLUGIN_FORMAT_VERSION}",
            optional_number(format)
        ));
    }
    if !matches!(abi, Some(ABI_VERSION | ABI_VERSION_V2)) {
        return Err(format!(
            "unsupported plugin ABI {}; expected {ABI_VERSION} or {ABI_VERSION_V2}",
            optional_number(abi)
        ));
    }
    let id = required(id, "id")?;
    if !valid_id(&id) {
        return Err("id must contain only lowercase letters, digits, `-`, or `.`".to_owned());
    }
    let library = library.ok_or("missing `library`")?;
    if library.is_absolute() {
        return Err("library path must be relative to the manifest".to_owned());
    }

    Ok(Manifest {
        id,
        name: required(name, "name")?,
        version: required(version, "version")?,
        author: required(author, "author")?,
        license: required(license, "license")?,
        abi: abi.expect("validated above"),
        platform: required(platform, "platform")?,
        library,
    })
}

fn hash_file(path: &Path) -> Result<[u8; 32], String> {
    let mut file = File::open(path).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer).map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hasher.finalize().into())
}

fn host_platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn parse_u32(value: &str, name: &str) -> Result<u32, String> {
    value.parse().map_err(|_| format!("invalid `{name}`"))
}

fn optional_number(value: Option<u32>) -> String {
    value.map_or_else(|| "missing".to_owned(), |value| value.to_string())
}

fn set_once<T>(slot: &mut Option<T>, value: T) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err("duplicate manifest key".to_owned())
    } else {
        Ok(())
    }
}

fn required(value: Option<String>, name: &str) -> Result<String, String> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("missing `{name}`"))
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

    const VALID: &str = "format=1\nid=test.plugin\nname=Test Plugin\nversion=0.1.0\nauthor=Test\nlicense=Test-only\nabi=1\nplatform=windows-x86_64\nlibrary=plugin.dll\n";

    #[test]
    fn plugin_manifest_is_strict_before_any_library_load() {
        let manifest = parse_manifest(VALID).unwrap();
        assert_eq!(manifest.id, "test.plugin");
        assert_eq!(manifest.library, PathBuf::from("plugin.dll"));
        assert_eq!(
            parse_manifest(&VALID.replace("abi=1", "abi=2"))
                .unwrap()
                .abi,
            ABI_VERSION_V2
        );
        assert!(parse_manifest(&VALID.replace("abi=1", "abi=3")).is_err());
        assert!(parse_manifest(&format!("{VALID}unknown=value\n")).is_err());
        assert!(parse_manifest(&VALID.replace("plugin.dll", "C:\\plugin.dll")).is_err());

        let path = std::env::temp_dir().join(format!(
            "thevisualizer-changed-plugin-{}.dll",
            std::process::id()
        ));
        fs::write(&path, b"changed artifact").unwrap();
        let package = PluginPackage {
            id: "test.changed".to_owned(),
            name: "Changed Plugin".to_owned(),
            version: "0.1.0".to_owned(),
            author: "Test".to_owned(),
            license: "Test-only".to_owned(),
            abi: ABI_VERSION,
            library_path: path.clone(),
            manifest_path: PathBuf::new(),
            artifact_hash: [0; 32],
        };
        let error = match LoadedPlugin::load(&package) {
            Ok(_) => panic!("changed artifact loaded"),
            Err(error) => error,
        };
        assert!(error.contains("changed after approval"));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn bundled_plugins_discover_both_abi_generations() {
        let discovery = discover(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/plugins")));
        assert!(discovery.errors.is_empty(), "{:?}", discovery.errors);
        assert_eq!(discovery.packages.len(), 19);
        assert_eq!(
            discovery
                .packages
                .iter()
                .filter(|package| package.abi == ABI_VERSION_V2)
                .count(),
            18
        );
        for package in discovery.packages {
            let plugin = LoadedPlugin::load(&package).expect("load bundled plugin");
            let frame = plugin
                .process(&Features::default(), 1.0, 1.0 / 60.0, &[0.5; 18])
                .expect("process bundled plugin");
            assert!(frame.response_multiplier.is_finite());
            assert!(frame.command_count <= MAX_PLUGIN_COMMANDS);
            drop(plugin);
        }
    }
}
