use std::mem::size_of;

use crate::forge_model::ForgeModel;
use eframe::{
    egui,
    egui_wgpu::{
        self,
        wgpu::{self, util::DeviceExt as _},
    },
};

pub const MAX_FORGE_NODES: usize = 16;
pub const MAX_FORGE_ROUTES: usize = 24;
const MAX_PARTICLES: u32 = 262_144;
const MAX_MODEL_POINTS: usize = 500_000;
const PARTICLE_FLOATS: usize = 16;
const NODE_FLOATS: usize = 16;
const UNIFORM_FLOATS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForgeQuality {
    Fps60,
    Fps90,
    Fps120,
    Unlimited,
}

impl ForgeQuality {
    pub const ALL: [Self; 4] = [Self::Fps60, Self::Fps90, Self::Fps120, Self::Unlimited];

    pub fn label(self) -> &'static str {
        match self {
            Self::Fps60 => "60 · Cinematic",
            Self::Fps90 => "90 · Balanced",
            Self::Fps120 => "120 · Performance",
            Self::Unlimited => "Unlimited",
        }
    }

    pub fn particle_count(self) -> u32 {
        match self {
            Self::Fps60 => 262_144,
            Self::Fps90 => 131_072,
            Self::Fps120 | Self::Unlimited => 65_536,
        }
    }

    pub fn frame_rate(self) -> Option<u32> {
        match self {
            Self::Fps60 => Some(60),
            Self::Fps90 => Some(90),
            Self::Fps120 => Some(120),
            Self::Unlimited => None,
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Fps60 => 0,
            Self::Fps90 => 1,
            Self::Fps120 => 2,
            Self::Unlimited => 3,
        }
    }

    fn from_code(code: u8) -> Result<Self, String> {
        Self::ALL
            .get(code as usize)
            .copied()
            .ok_or_else(|| "invalid forge quality".to_owned())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForgeTopology {
    Ring,
    Horn,
    Sphere,
    Helix,
    Core,
}

impl ForgeTopology {
    pub const ALL: [Self; 5] = [
        Self::Ring,
        Self::Horn,
        Self::Sphere,
        Self::Helix,
        Self::Core,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Ring => "Ring",
            Self::Horn => "Horn torus",
            Self::Sphere => "Sphere",
            Self::Helix => "Helix",
            Self::Core => "Collapsed core",
        }
    }

    fn code(self) -> f32 {
        match self {
            Self::Ring => 0.0,
            Self::Horn => 1.0,
            Self::Sphere => 2.0,
            Self::Helix => 3.0,
            Self::Core => 4.0,
        }
    }

    fn from_code(code: u8) -> Result<Self, String> {
        Self::ALL
            .get(code as usize)
            .copied()
            .ok_or_else(|| "invalid forge topology".to_owned())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForgeSceneSource {
    Procedural,
    Hybrid,
    Model,
}

impl ForgeSceneSource {
    pub const ALL: [Self; 3] = [Self::Procedural, Self::Hybrid, Self::Model];

    pub fn label(self) -> &'static str {
        match self {
            Self::Procedural => "Procedural",
            Self::Hybrid => "Hybrid",
            Self::Model => "Model",
        }
    }

    fn code(self) -> u8 {
        match self {
            Self::Procedural => 0,
            Self::Hybrid => 1,
            Self::Model => 2,
        }
    }

    fn from_code(code: u8) -> Result<Self, String> {
        Self::ALL
            .get(code as usize)
            .copied()
            .ok_or_else(|| "invalid forge scene source".to_owned())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ForgeForceKind {
    Emit,
    Attract,
    Repel,
    Spin,
    Vortex,
}

impl ForgeForceKind {
    pub const ALL: [Self; 5] = [
        Self::Emit,
        Self::Attract,
        Self::Repel,
        Self::Spin,
        Self::Vortex,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Emit => "Emit",
            Self::Attract => "Attract",
            Self::Repel => "Repel",
            Self::Spin => "Spin",
            Self::Vortex => "Vortex",
        }
    }

    fn code(self) -> f32 {
        match self {
            Self::Emit => 0.0,
            Self::Attract => 1.0,
            Self::Repel => 2.0,
            Self::Spin => 3.0,
            Self::Vortex => 4.0,
        }
    }

    fn from_code(code: u8) -> Result<Self, String> {
        Self::ALL
            .get(code as usize)
            .copied()
            .ok_or_else(|| "invalid forge force kind".to_owned())
    }
}

#[derive(Clone, Debug)]
pub struct ForgeNode {
    pub position: [f32; 3],
    pub radius: f32,
    pub strength: f32,
    pub falloff: f32,
    pub spin_axis: [f32; 3],
    pub band: u8,
    pub kind: ForgeForceKind,
    pub pinned: bool,
}

impl Default for ForgeNode {
    fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            radius: 0.45,
            strength: 1.0,
            falloff: 1.5,
            spin_axis: [0.0, 1.0, 0.0],
            band: 0,
            kind: ForgeForceKind::Vortex,
            pinned: false,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ForgeMaterialMix {
    pub energy: f32,
    pub cyber: f32,
    pub cosmic: f32,
}

impl Default for ForgeMaterialMix {
    fn default() -> Self {
        Self {
            energy: 1.0,
            cyber: 0.0,
            cosmic: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ForgeModRoute {
    pub source: u8,
    pub target: u8,
    pub amount: f32,
    pub enabled: bool,
}

#[derive(Clone, Debug)]
pub struct ParticleForgeState {
    pub quality: ForgeQuality,
    pub source: ForgeSceneSource,
    pub topology: ForgeTopology,
    pub topology_morph: f32,
    pub spin: [f32; 3],
    pub twist: f32,
    pub precession: f32,
    pub materials: ForgeMaterialMix,
    pub camera_yaw: f32,
    pub camera_pitch: f32,
    pub camera_zoom: f32,
    pub auto_camera: bool,
    pub camera_override_until: f32,
    pub gizmo: bool,
    pub reverse: bool,
    pub gradient_shift: f32,
    pub event_envelope: f32,
    pub nodes: Vec<ForgeNode>,
    pub selected: Option<usize>,
    pub routes: Vec<ForgeModRoute>,
    pub model: Option<ForgeModel>,
    pub model_notice: Option<String>,
    pub drag_origin: Option<[f32; 3]>,
    pub camera_drag_origin: Option<[f32; 2]>,
    pub secondary_drag_origin: Option<[f32; 2]>,
}

impl Default for ParticleForgeState {
    fn default() -> Self {
        Self {
            quality: ForgeQuality::Fps120,
            source: ForgeSceneSource::Procedural,
            topology: ForgeTopology::Ring,
            topology_morph: 0.0,
            spin: [0.12, 0.28, 0.08],
            twist: 0.35,
            precession: 0.18,
            materials: ForgeMaterialMix::default(),
            camera_yaw: 0.0,
            camera_pitch: 0.12,
            camera_zoom: 1.0,
            auto_camera: true,
            camera_override_until: 0.0,
            gizmo: false,
            reverse: false,
            gradient_shift: 0.0,
            event_envelope: 0.0,
            nodes: vec![ForgeNode::default()],
            selected: Some(0),
            routes: Vec::new(),
            model: None,
            model_notice: None,
            drag_origin: None,
            camera_drag_origin: None,
            secondary_drag_origin: None,
        }
    }
}

impl ParticleForgeState {
    pub fn add_node(&mut self, position: [f32; 3]) {
        if self.nodes.len() >= MAX_FORGE_NODES {
            return;
        }
        self.nodes.push(ForgeNode {
            position,
            ..ForgeNode::default()
        });
        self.selected = Some(self.nodes.len() - 1);
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn remove_selected(&mut self) {
        let Some(index) = self.selected else {
            return;
        };
        if self.nodes.get(index).is_none_or(|node| node.pinned) {
            return;
        }
        self.nodes.remove(index);
        self.selected =
            (!self.nodes.is_empty()).then(|| index.min(self.nodes.len().saturating_sub(1)));
    }

    pub fn apply_modulation(&mut self, sources: [f32; 6]) {
        for route in self.routes.iter().filter(|route| route.enabled) {
            let value = sources
                .get(route.source as usize)
                .copied()
                .unwrap_or_default()
                * route.amount;
            match route.target {
                0 => self.topology_morph = (self.topology_morph + value).clamp(0.0, 1.0),
                1..=3 => {
                    let axis = route.target as usize - 1;
                    self.spin[axis] = (self.spin[axis] + value).clamp(-1.5, 1.5);
                }
                4 => self.twist = (self.twist + value).clamp(0.0, 2.0),
                5 => self.materials.energy = (self.materials.energy + value).clamp(0.0, 1.0),
                6 => self.materials.cyber = (self.materials.cyber + value).clamp(0.0, 1.0),
                7 => self.materials.cosmic = (self.materials.cosmic + value).clamp(0.0, 1.0),
                _ => {}
            }
        }
    }

    pub fn encode_scene(&self) -> String {
        let selected = self.selected.map_or(-1, |index| index as i32);
        let mut output = format!(
            "state={},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            self.quality.code(),
            self.source.code(),
            self.topology.code() as u8,
            self.topology_morph,
            self.spin[0],
            self.spin[1],
            self.spin[2],
            self.twist,
            self.precession,
            self.materials.energy,
            self.materials.cyber,
            self.materials.cosmic,
            self.camera_yaw,
            self.camera_pitch,
            self.camera_zoom,
            u8::from(self.auto_camera),
            u8::from(self.gizmo),
            u8::from(self.reverse),
            self.gradient_shift,
            selected,
        );
        for node in self.nodes.iter().take(MAX_FORGE_NODES) {
            output.push_str(&format!(
                "node={},{},{},{},{},{},{},{},{},{},{},{}\n",
                node.position[0],
                node.position[1],
                node.position[2],
                node.radius,
                node.strength,
                node.falloff,
                node.spin_axis[0],
                node.spin_axis[1],
                node.spin_axis[2],
                node.band,
                node.kind.code() as u8,
                u8::from(node.pinned),
            ));
        }
        for route in self.routes.iter().take(MAX_FORGE_ROUTES) {
            output.push_str(&format!(
                "route={},{},{},{}\n",
                route.source,
                route.target,
                route.amount,
                u8::from(route.enabled),
            ));
        }
        if let Some(model) = &self.model {
            output.push_str(&format!(
                "model={}\n",
                model
                    .path
                    .to_string_lossy()
                    .bytes()
                    .map(|byte| format!("{byte:02X}"))
                    .collect::<String>()
            ));
        }
        output
    }

    pub fn decode_scene(source: &str) -> Result<Self, String> {
        if source.len() > 16 * 1024 {
            return Err("forge state is oversized".to_owned());
        }
        let mut state_values = None;
        let mut nodes = Vec::new();
        let mut routes = Vec::new();
        let mut model_path = None;
        for line in source.lines().filter(|line| !line.is_empty()) {
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| "invalid forge state line".to_owned())?;
            let values = value.split(',').collect::<Vec<_>>();
            match key {
                "state" if state_values.is_none() && values.len() == 20 => {
                    state_values = Some(values);
                }
                "node" if values.len() == 12 && nodes.len() < MAX_FORGE_NODES => {
                    nodes.push(ForgeNode {
                        position: [parse(values[0])?, parse(values[1])?, parse(values[2])?],
                        radius: parse_range(values[3], 0.08, 1.5)?,
                        strength: parse_range(values[4], 0.0, 3.0)?,
                        falloff: parse_range(values[5], 0.25, 4.0)?,
                        spin_axis: [parse(values[6])?, parse(values[7])?, parse(values[8])?],
                        band: parse_range::<u8>(values[9], 0, 3)?,
                        kind: ForgeForceKind::from_code(parse(values[10])?)?,
                        pinned: parse_bool(values[11])?,
                    });
                }
                "route" if values.len() == 4 && routes.len() < MAX_FORGE_ROUTES => {
                    routes.push(ForgeModRoute {
                        source: parse_range(values[0], 0, 5)?,
                        target: parse_range(values[1], 0, 7)?,
                        amount: parse_range(values[2], -2.0, 2.0)?,
                        enabled: parse_bool(values[3])?,
                    });
                }
                "model" if model_path.is_none() && value.len().is_multiple_of(2) => {
                    let bytes = value
                        .as_bytes()
                        .chunks_exact(2)
                        .map(|pair| {
                            let pair = std::str::from_utf8(pair).ok()?;
                            u8::from_str_radix(pair, 16).ok()
                        })
                        .collect::<Option<Vec<_>>>()
                        .ok_or_else(|| "invalid forge model path".to_owned())?;
                    model_path = Some(
                        String::from_utf8(bytes)
                            .map_err(|_| "forge model path is not UTF-8".to_owned())?,
                    );
                }
                _ => return Err(format!("invalid or duplicate forge state key `{key}`")),
            }
        }
        let values = state_values.ok_or_else(|| "missing forge state".to_owned())?;
        let selected: i32 = parse(values[19])?;
        let mut state = Self {
            quality: ForgeQuality::from_code(parse(values[0])?)?,
            source: ForgeSceneSource::from_code(parse(values[1])?)?,
            topology: ForgeTopology::from_code(parse(values[2])?)?,
            topology_morph: parse_range(values[3], 0.0, 1.0)?,
            spin: [
                parse_range(values[4], -1.5, 1.5)?,
                parse_range(values[5], -1.5, 1.5)?,
                parse_range(values[6], -1.5, 1.5)?,
            ],
            twist: parse_range(values[7], 0.0, 2.0)?,
            precession: parse_range(values[8], 0.0, 1.0)?,
            materials: ForgeMaterialMix {
                energy: parse_range(values[9], 0.0, 1.0)?,
                cyber: parse_range(values[10], 0.0, 1.0)?,
                cosmic: parse_range(values[11], 0.0, 1.0)?,
            },
            camera_yaw: parse_range(values[12], -std::f32::consts::TAU, std::f32::consts::TAU)?,
            camera_pitch: parse_range(values[13], -1.2, 1.2)?,
            camera_zoom: parse_range(values[14], 0.35, 3.0)?,
            auto_camera: parse_bool(values[15])?,
            camera_override_until: 0.0,
            gizmo: parse_bool(values[16])?,
            reverse: parse_bool(values[17])?,
            gradient_shift: parse_range(values[18], 0.0, 1.0)?,
            event_envelope: 0.0,
            nodes,
            selected: (selected >= 0).then_some(selected as usize),
            routes,
            model: None,
            model_notice: None,
            drag_origin: None,
            camera_drag_origin: None,
            secondary_drag_origin: None,
        };
        if state.nodes.is_empty() {
            state.nodes.push(ForgeNode::default());
            state.selected = Some(0);
        }
        if state
            .selected
            .is_some_and(|index| index >= state.nodes.len())
        {
            return Err("selected forge node does not exist".to_owned());
        }
        if let Some(path) = model_path {
            match ForgeModel::load(std::path::Path::new(&path)) {
                Ok(model) => state.model = Some(model),
                Err(error) => state.model_notice = Some(error),
            }
        }
        Ok(state)
    }
}

fn parse<T: std::str::FromStr>(value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid forge value `{value}`"))
}

fn parse_range<T>(value: &str, minimum: T, maximum: T) -> Result<T, String>
where
    T: std::str::FromStr + PartialOrd + Copy,
{
    let parsed = parse(value)?;
    if parsed >= minimum && parsed <= maximum {
        Ok(parsed)
    } else {
        Err(format!("forge value `{value}` is out of range"))
    }
}

fn parse_bool(value: &str) -> Result<bool, String> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err("forge boolean must be 0 or 1".to_owned()),
    }
}

pub struct ForgeFrame<'a> {
    pub time: f32,
    pub delta: f32,
    pub gain: f32,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
    pub rms: f32,
    pub onset: f32,
    pub transient: f32,
    pub state: &'a ParticleForgeState,
    pub colors: [[f32; 4]; 3],
}

pub struct ParticleForgeRenderer {
    state: egui_wgpu::RenderState,
    adapter_name: String,
}

impl ParticleForgeRenderer {
    pub fn install(state: &egui_wgpu::RenderState) -> Result<Self, String> {
        let resources = create_resources(state)?;
        state.renderer.write().callback_resources.insert(resources);
        Ok(Self {
            state: state.clone(),
            adapter_name: state.adapter.get_info().name,
        })
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn upload_model(&self, model: &ForgeModel) {
        if let Some(resources) = self
            .state
            .renderer
            .write()
            .callback_resources
            .get::<ForgeResources>()
        {
            self.state.queue.write_buffer(
                &resources.model_points,
                0,
                bytemuck::cast_slice(&model.points),
            );
        }
    }

    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, frame: ForgeFrame<'_>) {
        let mut nodes = [0.0_f32; MAX_FORGE_NODES * NODE_FLOATS];
        for (index, node) in frame.state.nodes.iter().take(MAX_FORGE_NODES).enumerate() {
            let offset = index * NODE_FLOATS;
            nodes[offset..offset + 4].copy_from_slice(&[
                node.position[0],
                node.position[1],
                node.position[2],
                node.radius,
            ]);
            nodes[offset + 4..offset + 8].copy_from_slice(&[
                node.strength,
                node.kind.code(),
                node.band as f32,
                node.falloff,
            ]);
            nodes[offset + 8..offset + 12].copy_from_slice(&[
                node.spin_axis[0],
                node.spin_axis[1],
                node.spin_axis[2],
                f32::from(node.pinned),
            ]);
        }
        let state = frame.state;
        let direction = if state.reverse { -1.0 } else { 1.0 };
        let uniforms = [
            rect.width(),
            rect.height(),
            frame.time,
            frame.delta.min(0.05),
            frame.low,
            frame.mid,
            frame.high,
            frame.rms,
            frame.onset,
            frame.transient,
            frame.gain,
            state.quality.particle_count() as f32,
            state.nodes.len().min(MAX_FORGE_NODES) as f32,
            state.topology.code(),
            state.topology_morph,
            state.spin[0] * direction,
            state.spin[1] * direction,
            state.spin[2] * direction,
            state.twist,
            state.precession,
            state.materials.energy,
            state.materials.cyber,
            state.materials.cosmic,
            state.camera_yaw,
            state.camera_pitch,
            state.camera_zoom,
            state.gradient_shift,
            state.event_envelope,
            state.source.code() as f32,
            state
                .model
                .as_ref()
                .map_or(0.0, |model| model.points.len() as f32),
            0.0,
            f32::from(state.auto_camera && frame.time >= state.camera_override_until),
        ];
        painter.add(egui_wgpu::Callback::new_paint_callback(
            rect,
            ForgeCallback {
                uniforms,
                nodes,
                colors: frame.colors,
                particle_count: state.quality.particle_count(),
            },
        ));
    }
}

fn create_resources(state: &egui_wgpu::RenderState) -> Result<ForgeResources, String> {
    let device = &state.device;
    let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Particle Forge 3D"),
        source: wgpu::ShaderSource::Wgsl(include_str!("particle_forge.wgsl").into()),
    });
    let compute_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Particle Forge compute bind group"),
        entries: &[
            buffer_entry(
                0,
                wgpu::ShaderStages::COMPUTE,
                wgpu::BufferBindingType::Uniform,
                UNIFORM_FLOATS,
            ),
            buffer_entry(
                1,
                wgpu::ShaderStages::COMPUTE,
                wgpu::BufferBindingType::Storage { read_only: false },
                MAX_PARTICLES as usize * PARTICLE_FLOATS,
            ),
            buffer_entry(
                2,
                wgpu::ShaderStages::COMPUTE,
                wgpu::BufferBindingType::Storage { read_only: true },
                MAX_FORGE_NODES * NODE_FLOATS,
            ),
            buffer_entry(
                3,
                wgpu::ShaderStages::COMPUTE,
                wgpu::BufferBindingType::Storage { read_only: true },
                MAX_MODEL_POINTS * 4,
            ),
        ],
    });
    let render_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Particle Forge render bind group"),
        entries: &[
            buffer_entry(
                0,
                wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                wgpu::BufferBindingType::Uniform,
                UNIFORM_FLOATS,
            ),
            buffer_entry(
                1,
                wgpu::ShaderStages::VERTEX,
                wgpu::BufferBindingType::Storage { read_only: true },
                MAX_PARTICLES as usize * PARTICLE_FLOATS,
            ),
            buffer_entry(
                2,
                wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                wgpu::BufferBindingType::Storage { read_only: true },
                MAX_FORGE_NODES * NODE_FLOATS,
            ),
            buffer_entry(
                3,
                wgpu::ShaderStages::FRAGMENT,
                wgpu::BufferBindingType::Uniform,
                12,
            ),
        ],
    });
    let compute_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Particle Forge compute pipeline layout"),
        bind_group_layouts: &[Some(&compute_layout)],
        immediate_size: 0,
    });
    let compute = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("Particle Forge simulation"),
        layout: Some(&compute_pipeline_layout),
        module: &shader,
        entry_point: Some("cs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Particle Forge render pipeline layout"),
        bind_group_layouts: &[None, Some(&render_layout)],
        immediate_size: 0,
    });
    let render = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Particle Forge particles"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: state.target_format,
                blend: Some(wgpu::BlendState {
                    color: wgpu::BlendComponent {
                        src_factor: wgpu::BlendFactor::SrcAlpha,
                        dst_factor: wgpu::BlendFactor::One,
                        operation: wgpu::BlendOperation::Add,
                    },
                    alpha: wgpu::BlendComponent::OVER,
                }),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    let uniforms = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Particle Forge uniforms"),
        contents: bytemuck::cast_slice(&[0.0_f32; UNIFORM_FLOATS]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
    });
    let particles = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Particle Forge particles"),
        size: (MAX_PARTICLES as usize * PARTICLE_FLOATS * size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let nodes = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Particle Forge nodes"),
        contents: bytemuck::cast_slice(&[0.0_f32; MAX_FORGE_NODES * NODE_FLOATS]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
    });
    let model_points = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Particle Forge model points"),
        size: (MAX_MODEL_POINTS * 4 * size_of::<f32>()) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
        mapped_at_creation: false,
    });
    let colors = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Particle Forge colors"),
        contents: bytemuck::cast_slice(&[0.0_f32; 12]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
    });
    let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Particle Forge compute bind group"),
        layout: &compute_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: particles.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: nodes.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: model_points.as_entire_binding(),
            },
        ],
    });
    let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Particle Forge render bind group"),
        layout: &render_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniforms.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: particles.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: nodes.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: colors.as_entire_binding(),
            },
        ],
    });
    if let Some(error) = pollster::block_on(error_scope.pop()) {
        return Err(format!("Particle Forge GPU pipeline rejected: {error}"));
    }
    Ok(ForgeResources {
        compute,
        render,
        compute_bind_group,
        render_bind_group,
        uniforms,
        nodes,
        colors,
        model_points,
    })
}

fn buffer_entry(
    binding: u32,
    visibility: wgpu::ShaderStages,
    ty: wgpu::BufferBindingType,
    floats: usize,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility,
        ty: wgpu::BindingType::Buffer {
            ty,
            has_dynamic_offset: false,
            min_binding_size: std::num::NonZeroU64::new((floats * size_of::<f32>()) as u64),
        },
        count: None,
    }
}

struct ForgeCallback {
    uniforms: [f32; UNIFORM_FLOATS],
    nodes: [f32; MAX_FORGE_NODES * NODE_FLOATS],
    colors: [[f32; 4]; 3],
    particle_count: u32,
}

impl egui_wgpu::CallbackTrait for ForgeCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(resources) = resources.get::<ForgeResources>() {
            queue.write_buffer(&resources.uniforms, 0, bytemuck::cast_slice(&self.uniforms));
            queue.write_buffer(&resources.nodes, 0, bytemuck::cast_slice(&self.nodes));
            queue.write_buffer(&resources.colors, 0, bytemuck::cast_slice(&self.colors));
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Particle Forge simulation"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&resources.compute);
            pass.set_bind_group(0, &resources.compute_bind_group, &[]);
            pass.dispatch_workgroups(self.particle_count.div_ceil(256), 1, 1);
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(resources) = resources.get::<ForgeResources>() {
            render_pass.set_pipeline(&resources.render);
            render_pass.set_bind_group(1, &resources.render_bind_group, &[]);
            render_pass.draw(0..6, 0..self.particle_count);
        }
    }
}

struct ForgeResources {
    compute: wgpu::ComputePipeline,
    render: wgpu::RenderPipeline,
    compute_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,
    uniforms: wgpu::Buffer,
    nodes: wgpu::Buffer,
    colors: wgpu::Buffer,
    model_points: wgpu::Buffer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quality_profiles_are_fixed_and_bounded() {
        assert_eq!(ForgeQuality::Fps60.particle_count(), MAX_PARTICLES);
        assert_eq!(ForgeQuality::Fps90.frame_rate(), Some(90));
        assert_eq!(ForgeQuality::Unlimited.frame_rate(), None);
    }

    #[test]
    fn forge_nodes_and_routes_are_bounded() {
        let mut state = ParticleForgeState::default();
        for index in 0..MAX_FORGE_NODES + 4 {
            state.add_node([index as f32, 0.0, 0.0]);
        }
        assert_eq!(state.nodes.len(), MAX_FORGE_NODES);
        assert!(MAX_FORGE_ROUTES >= state.routes.len());
    }

    #[test]
    fn selected_pinned_node_cannot_be_removed() {
        let mut state = ParticleForgeState::default();
        state.nodes[0].pinned = true;
        state.remove_selected();
        assert_eq!(state.nodes.len(), 1);
        state.nodes[0].pinned = false;
        state.remove_selected();
        assert!(state.nodes.is_empty());
        assert_eq!(state.selected, None);
    }

    #[test]
    fn modulation_routes_are_bounded_at_the_target() {
        let mut state = ParticleForgeState::default();
        state.routes.push(ForgeModRoute {
            source: 0,
            target: 1,
            amount: 10.0,
            enabled: true,
        });
        state.apply_modulation([1.0; 6]);
        assert_eq!(state.spin[0], 1.5);
    }

    #[test]
    fn forge_scene_state_round_trips() {
        let mut state = ParticleForgeState::default();
        state.add_node([1.0, -0.5, 0.75]);
        state.routes.push(ForgeModRoute {
            source: 2,
            target: 6,
            amount: -0.4,
            enabled: true,
        });
        let restored = ParticleForgeState::decode_scene(&state.encode_scene()).unwrap();
        assert_eq!(restored.nodes.len(), 2);
        assert_eq!(restored.routes.len(), 1);
        assert_eq!(restored.nodes[1].position, [1.0, -0.5, 0.75]);
    }
}
