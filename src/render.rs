use std::num::NonZeroU64;

use eframe::{
    egui,
    egui_wgpu::{
        self,
        wgpu::{self, util::DeviceExt as _},
    },
};

use crate::analysis::{SPECTRUM_BANDS, WAVEFORM_POINTS};
use crate::preset::Preset;

const AUDIO_FEATURE_FLOATS: usize = WAVEFORM_POINTS + SPECTRUM_BANDS;
const FRAME_EXTRAS_FLOATS: usize = 8;
pub const PRESET_SCENE_FLOATS: usize = 96;
pub const PRESET_PARAMETER_FLOATS: usize = 40;

#[derive(Clone, Copy)]
pub struct PresetFrame<'a> {
    pub time: f32,
    pub delta: f32,
    pub gain: f32,
    pub waveform: &'a [f32],
    pub spectrum: &'a [f32],
    pub low: f32,
    pub mid: f32,
    pub high: f32,
    pub rms: f32,
    pub peak: f32,
    pub onset: f32,
    pub transient: f32,
    pub scene: &'a [f32],
    pub parameters: &'a [f32],
}

impl PresetFrame<'_> {
    fn uniforms(self, size: [u32; 2]) -> [f32; 8] {
        [
            size[0] as f32,
            size[1] as f32,
            self.time,
            self.gain,
            self.low,
            self.mid,
            self.high,
            self.rms,
        ]
    }

    fn extras(self) -> [f32; FRAME_EXTRAS_FLOATS] {
        [
            self.delta,
            self.peak,
            self.waveform.len() as f32,
            self.spectrum.len() as f32,
            self.onset,
            self.transient,
            0.0,
            0.0,
        ]
    }

    fn audio_features(self) -> [f32; AUDIO_FEATURE_FLOATS] {
        let mut features = [0.0; AUDIO_FEATURE_FLOATS];
        let waveform_len = self.waveform.len().min(WAVEFORM_POINTS);
        let spectrum_len = self.spectrum.len().min(SPECTRUM_BANDS);
        features[..waveform_len].copy_from_slice(&self.waveform[..waveform_len]);
        features[WAVEFORM_POINTS..WAVEFORM_POINTS + spectrum_len]
            .copy_from_slice(&self.spectrum[..spectrum_len]);
        features
    }

    fn scene_state(self) -> [f32; PRESET_SCENE_FLOATS] {
        let mut scene = [0.0; PRESET_SCENE_FLOATS];
        let length = self.scene.len().min(PRESET_SCENE_FLOATS);
        scene[..length].copy_from_slice(&self.scene[..length]);
        scene
    }

    fn parameter_state(self) -> [f32; PRESET_PARAMETER_FLOATS] {
        let mut parameters = [0.0; PRESET_PARAMETER_FLOATS];
        let length = self.parameters.len().min(PRESET_PARAMETER_FLOATS);
        parameters[..length].copy_from_slice(&self.parameters[..length]);
        parameters
    }
}

pub struct GpuPresetRenderer {
    adapter_name: String,
    state: egui_wgpu::RenderState,
    active_id: String,
}

impl GpuPresetRenderer {
    pub fn install(state: &egui_wgpu::RenderState, preset: &Preset) -> Result<Self, String> {
        let resources = create_resources(state, preset)?;
        state.renderer.write().callback_resources.insert(resources);
        Ok(Self {
            adapter_name: state.adapter.get_info().name,
            state: state.clone(),
            active_id: preset.id.clone(),
        })
    }

    pub fn load(&mut self, preset: &Preset) -> Result<(), String> {
        let resources = create_resources(&self.state, preset)?;
        self.state
            .renderer
            .write()
            .callback_resources
            .insert(resources);
        self.active_id.clone_from(&preset.id);
        Ok(())
    }

    pub fn active_id(&self) -> &str {
        &self.active_id
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, frame: PresetFrame<'_>) {
        painter.add(egui_wgpu::Callback::new_paint_callback(
            rect,
            PresetCallback {
                uniforms: frame.uniforms([1, 1]),
                extras: frame.extras(),
                audio_features: frame.audio_features(),
                scene: frame.scene_state(),
                parameters: frame.parameter_state(),
            },
        ));
    }
}

fn create_resources(
    state: &egui_wgpu::RenderState,
    preset: &Preset,
) -> Result<PresetResources, String> {
    let device = &state.device;
    let error_scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&preset.name),
        source: wgpu::ShaderSource::Wgsl(preset.shader.as_str().into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("TheVisualizer preset uniforms"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(32),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(
                        (AUDIO_FEATURE_FLOATS * size_of::<f32>()) as u64,
                    ),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(
                        (FRAME_EXTRAS_FLOATS * size_of::<f32>()) as u64,
                    ),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(
                        (PRESET_SCENE_FLOATS * size_of::<f32>()) as u64,
                    ),
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: NonZeroU64::new(
                        (PRESET_PARAMETER_FLOATS * size_of::<f32>()) as u64,
                    ),
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("TheVisualizer preset pipeline"),
        bind_group_layouts: &[Some(&bind_group_layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&preset.name),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            targets: &[Some(state.target_format.into())],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview_mask: None,
        cache: None,
    });
    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("TheVisualizer preset uniform buffer"),
        contents: bytemuck::cast_slice(&[0.0_f32; 8]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
    });
    let audio_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("TheVisualizer preset audio-feature buffer"),
        contents: bytemuck::cast_slice(&[0.0_f32; AUDIO_FEATURE_FLOATS]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
    });
    let extras_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("TheVisualizer preset frame-extras buffer"),
        contents: bytemuck::cast_slice(&[0.0_f32; FRAME_EXTRAS_FLOATS]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
    });
    let scene_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("TheVisualizer preset scene buffer"),
        contents: bytemuck::cast_slice(&[0.0_f32; PRESET_SCENE_FLOATS]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
    });
    let parameter_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("TheVisualizer preset parameter buffer"),
        contents: bytemuck::cast_slice(&[0.0_f32; PRESET_PARAMETER_FLOATS]),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("TheVisualizer preset bind group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: audio_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: extras_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: scene_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: parameter_buffer.as_entire_binding(),
            },
        ],
    });
    if let Some(error) = pollster::block_on(error_scope.pop()) {
        eprintln!("Preset {:?} was rejected:\n{error}", preset.path);
        return Err(format!(
            "{} rejected: {}",
            preset.name,
            concise_validation_error(&error.to_string())
        ));
    }
    Ok(PresetResources {
        pipeline,
        bind_group,
        uniform_buffer,
        audio_buffer,
        extras_buffer,
        scene_buffer,
        parameter_buffer,
    })
}

fn concise_validation_error(error: &str) -> String {
    let detail = error
        .lines()
        .map(str::trim)
        .find(|line| {
            !line.is_empty()
                && *line != "Validation Error"
                && *line != "Caused by:"
                && !line.starts_with("In Device::")
        })
        .unwrap_or("shader validation failed");
    let mut summary = detail.chars().take(220).collect::<String>();
    if detail.chars().count() > 220 {
        summary.push('…');
    }
    summary
}

struct PresetCallback {
    uniforms: [f32; 8],
    extras: [f32; FRAME_EXTRAS_FLOATS],
    audio_features: [f32; AUDIO_FEATURE_FLOATS],
    scene: [f32; PRESET_SCENE_FLOATS],
    parameters: [f32; PRESET_PARAMETER_FLOATS],
}

impl egui_wgpu::CallbackTrait for PresetCallback {
    fn prepare(
        &self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        screen: &egui_wgpu::ScreenDescriptor,
        _encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let Some(resources) = resources.get::<PresetResources>() {
            let mut uniforms = self.uniforms;
            uniforms[0] = screen.size_in_pixels[0] as f32;
            uniforms[1] = screen.size_in_pixels[1] as f32;
            queue.write_buffer(
                &resources.uniform_buffer,
                0,
                bytemuck::cast_slice(&uniforms),
            );
            queue.write_buffer(
                &resources.audio_buffer,
                0,
                bytemuck::cast_slice(&self.audio_features),
            );
            queue.write_buffer(
                &resources.extras_buffer,
                0,
                bytemuck::cast_slice(&self.extras),
            );
            queue.write_buffer(
                &resources.scene_buffer,
                0,
                bytemuck::cast_slice(&self.scene),
            );
            queue.write_buffer(
                &resources.parameter_buffer,
                0,
                bytemuck::cast_slice(&self.parameters),
            );
        }
        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        if let Some(resources) = resources.get::<PresetResources>() {
            render_pass.set_pipeline(&resources.pipeline);
            render_pass.set_bind_group(0, &resources.bind_group, &[]);
            render_pass.draw(0..3, 0..1);
        }
    }
}

struct PresetResources {
    pipeline: wgpu::RenderPipeline,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    audio_buffer: wgpu::Buffer,
    extras_buffer: wgpu::Buffer,
    scene_buffer: wgpu::Buffer,
    parameter_buffer: wgpu::Buffer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_uniform_layout_matches_wgsl_contract() {
        let frame = PresetFrame {
            time: 1.0,
            delta: 0.016,
            gain: 2.0,
            waveform: &[0.25, -0.5],
            spectrum: &[0.75],
            low: 3.0,
            mid: 4.0,
            high: 5.0,
            rms: 6.0,
            peak: 0.9,
            onset: 1.0,
            transient: 0.7,
            scene: &[7.0, 8.0],
            parameters: &[9.0],
        };
        assert_eq!(
            frame.uniforms([1920, 1080]),
            [1920.0, 1080.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
        assert_eq!(frame.extras(), [0.016, 0.9, 2.0, 1.0, 1.0, 0.7, 0.0, 0.0]);
        let audio = frame.audio_features();
        assert_eq!(&audio[..3], &[0.25, -0.5, 0.0]);
        assert_eq!(audio[WAVEFORM_POINTS], 0.75);
        assert_eq!(&frame.scene_state()[..3], &[7.0, 8.0, 0.0]);
        assert_eq!(&frame.parameter_state()[..2], &[9.0, 0.0]);
    }
}
