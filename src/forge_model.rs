use std::{
    fs,
    path::{Path, PathBuf},
};

const MAX_GLB_BYTES: u64 = 64 * 1024 * 1024;
const MAX_MODEL_VERTICES: usize = 500_000;

#[derive(Clone, Debug)]
pub struct ForgeModel {
    pub path: PathBuf,
    pub points: Vec<[f32; 4]>,
    pub mesh_count: usize,
    pub skin_count: usize,
    pub animation_count: usize,
}

impl ForgeModel {
    pub fn load(path: &Path) -> Result<Self, String> {
        if path
            .extension()
            .is_none_or(|extension| !extension.eq_ignore_ascii_case("glb"))
        {
            return Err("Particle Forge accepts binary glTF `.glb` models only.".to_owned());
        }
        let metadata =
            fs::metadata(path).map_err(|error| format!("Could not inspect GLB: {error}"))?;
        if metadata.len() > MAX_GLB_BYTES {
            return Err(format!(
                "GLB exceeds the {} MiB limit.",
                MAX_GLB_BYTES / 1024 / 1024
            ));
        }
        let (document, buffers, images) =
            gltf::import(path).map_err(|error| format!("Invalid GLB: {error}"))?;
        if images
            .iter()
            .any(|image| image.width > 4096 || image.height > 4096)
        {
            return Err("GLB texture exceeds the 4096 × 4096 limit.".to_owned());
        }
        let mut points = Vec::new();
        for mesh in document.meshes() {
            for primitive in mesh.primitives() {
                let reader = primitive.reader(|buffer| Some(&buffers[buffer.index()]));
                if let Some(positions) = reader.read_positions() {
                    for position in positions {
                        if points.len() == MAX_MODEL_VERTICES {
                            return Err(format!(
                                "GLB exceeds the {MAX_MODEL_VERTICES} vertex limit."
                            ));
                        }
                        points.push([position[0], position[1], position[2], 1.0]);
                    }
                }
            }
        }
        if points.is_empty() {
            return Err("GLB contains no readable mesh positions.".to_owned());
        }
        Ok(Self {
            path: path.to_owned(),
            points,
            mesh_count: document.meshes().count(),
            skin_count: document.skins().count(),
            animation_count: document.animations().count(),
        })
    }
}
