struct PresetUniforms {
    resolution: vec2f,
    time: f32,
    gain: f32,
    audio: vec4f,
}
@group(0) @binding(0) var<uniform> uniforms: PresetUniforms;

struct AudioFeatures {
    waveform: array<f32, 256>,
    spectrum: array<f32, 64>,
}
@group(0) @binding(1) var<storage, read> features: AudioFeatures;

struct FrameExtras {
    delta: f32,
    peak: f32,
    waveform_len: f32,
    spectrum_len: f32,
    onset: f32,
    transient: f32,
    history_rows: f32,
    history_columns: f32,
}
@group(0) @binding(2) var<uniform> extras: FrameExtras;

struct SceneState { values: array<f32, 160> }
@group(0) @binding(3) var<storage, read> scene: SceneState;
struct ModeParameters { values: array<f32, 40> }
@group(0) @binding(4) var<storage, read> controls: ModeParameters;
struct SpectrumHistory { values: array<f32, 8192> }
@group(0) @binding(5) var<storage, read> history: SpectrumHistory;

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4f {
    let vertices = array(vec2f(-1.0, -3.0), vec2f(3.0, 1.0), vec2f(-1.0, 1.0));
    return vec4f(vertices[index], 0.0, 1.0);
}

fn hash21(p: vec2f) -> f32 {
    return fract(sin(dot(p, vec2f(127.1, 311.7))) * 43758.5453);
}

fn rotate(p: vec2f, angle: f32) -> vec2f {
    let c = cos(angle);
    let s = sin(angle);
    return vec2f(c * p.x - s * p.y, s * p.x + c * p.y);
}

fn glow_line(distance: f32, width: f32) -> f32 {
    return exp(-distance * distance / max(width * width, 0.00001));
}

fn spectrum_at(value: f32) -> f32 {
    let index = u32(clamp(value, 0.0, 0.999) * 64.0);
    return clamp(features.spectrum[index] * uniforms.gain, 0.0, 2.0);
}

fn waveform_at(value: f32) -> f32 {
    let index = u32(clamp(value, 0.0, 0.999) * 256.0);
    return features.waveform[index];
}

fn history_at(x: f32, y: f32) -> f32 {
    let column = u32(clamp(x, 0.0, 0.999) * 64.0);
    let available = max(extras.history_rows, 1.0);
    let row = u32(clamp(y, 0.0, 0.999) * min(available, 128.0));
    return clamp(history.values[row * 64u + column] * uniforms.gain, 0.0, 2.0);
}

fn palette(kind: f32, shift: f32) -> vec3f {
    let bass = vec3f(scene.values[76], scene.values[77], scene.values[78]);
    let mid = vec3f(scene.values[80], scene.values[81], scene.values[82]);
    let high = vec3f(scene.values[84], scene.values[85], scene.values[86]);
    let phase = fract(kind * 0.137 + shift);
    var color = mix(bass, mid, smoothstep(0.0, 0.55, phase));
    color = mix(color, high, smoothstep(0.45, 1.0, phase));
    return mix(color, color.yzx, abs(shift) * 0.65);
}

fn pulse_trace(p: vec2f, density: f32, width: f32, time: f32) -> f32 {
    let x = p.x * 0.5 + 0.5;
    let wave = waveform_at(x) * (0.22 + extras.peak * 0.18);
    let main = glow_line(abs(p.y - wave), width);
    let echo = glow_line(abs(p.y + wave * 0.55 + sin(p.x * 9.0 + time) * 0.04), width * 1.8);
    let scan = 0.7 + 0.3 * sin((p.x + 1.0) * 30.0 * density);
    return main * scan + echo * 0.28;
}

fn spectrum_skyline(p: vec2f, density: f32, width: f32) -> f32 {
    let bars = max(8.0, floor(24.0 * density));
    let cell = floor((p.x * 0.5 + 0.5) * bars);
    let x = (cell + 0.5) / bars;
    let height = 0.08 + spectrum_at(x) * 0.6;
    let body = step(abs(p.x), 1.0) * step(-0.75, p.y) * step(p.y, -0.75 + height);
    let edge = glow_line(abs(p.y + 0.75 - height), width * 1.4);
    let blocks = step(0.18, fract((p.y + 0.75) * 28.0)) * step(0.12, fract((p.x + 1.0) * bars));
    let reflection = step(-0.75 - height * 0.55, p.y) * step(p.y, -0.75) * 0.25;
    return (body * blocks + edge + reflection) * step(abs(fract((p.x + 1.0) * bars) - 0.5), 0.43);
}

fn radial_burst(p: vec2f, density: f32, width: f32) -> f32 {
    let radius = length(p);
    let angle = fract(atan2(p.y, p.x) / 6.2831853 + 0.5);
    let energy = spectrum_at(angle);
    let spokes = pow(max(0.0, cos(angle * 6.2831853 * 28.0 * density)), 18.0);
    let ring = glow_line(abs(radius - 0.42), width * 1.5);
    let rays = spokes * step(0.38, radius) * step(radius, 0.48 + energy * 0.3);
    return ring + rays * (0.35 + energy);
}

fn spectrogram_city(p: vec2f, density: f32, width: f32) -> f32 {
    let x = p.x * 0.5 + 0.5;
    let depth = clamp((p.y + 0.8) / 1.6, 0.0, 1.0);
    let energy = history_at(x, depth);
    let cells = pow(max(0.0, sin((p.x + 1.0) * 34.0 * density)), 4.0);
    let horizon = glow_line(abs(p.y + 0.58 - energy * 0.22), width * 1.6);
    return cells * smoothstep(0.08, 0.8, energy) * (1.0 - depth * 0.45) + horizon;
}

fn spectral_terrain(p: vec2f, density: f32, width: f32) -> f32 {
    let depth = clamp((p.y + 1.0) * 0.5, 0.0, 1.0);
    let perspective_x = p.x * (0.35 + depth);
    let energy = history_at(perspective_x * 0.5 + 0.5, 1.0 - depth);
    let horizontal = glow_line(abs(fract((depth + energy * 0.08) * 12.0 * density) - 0.5), width * 8.0);
    let vertical = glow_line(abs(fract((perspective_x + 1.0) * 9.0 * density) - 0.5), width * 7.0);
    return (horizontal + vertical * 0.55) * smoothstep(-0.95, -0.2, p.y);
}

fn wave_tunnel(p: vec2f, density: f32, width: f32, time: f32) -> f32 {
    let angle = fract(atan2(p.y, p.x) / 6.2831853 + 0.5);
    let wave = waveform_at(angle) * 0.12;
    let radius = length(p) + wave;
    let rings = glow_line(abs(fract((radius - time * 0.08) * 7.0 * density) - 0.5), width * 8.0);
    let ribs = pow(max(0.0, cos(angle * 6.2831853 * 18.0)), 20.0);
    return (rings + ribs * 0.4) * (1.0 - smoothstep(0.12, 1.15, radius));
}

fn particle_ocean(p: vec2f, density: f32, width: f32, time: f32) -> f32 {
    let grid = max(8.0, 18.0 * density);
    let warped = p * grid + vec2f(time * 0.7, sin(p.x * 3.0 + time) * 2.0);
    let cell = floor(warped);
    let point = fract(warped) - 0.5;
    let flicker = hash21(cell);
    let flow = spectrum_at(fract(cell.x / grid));
    let dot = glow_line(length(point), width * (0.7 + flow) * 2.2);
    return dot * step(0.35, flicker) * (0.35 + flow);
}

fn wireframe_terrain(p: vec2f, density: f32, width: f32, time: f32) -> f32 {
    let depth = clamp(p.y * 0.5 + 0.5, 0.0, 1.0);
    let wave = history_at(p.x * 0.5 + 0.5, 1.0 - depth) * 0.12;
    let y = p.y - wave + sin(p.x * 4.0 + time) * 0.025;
    let rows = glow_line(abs(fract((y + 1.0) * 8.0 * density) - 0.5), width * 7.0);
    let columns = glow_line(abs(fract((p.x * (0.3 + depth) + 1.0) * 8.0 * density) - 0.5), width * 6.0);
    return rows + columns * 0.6;
}

fn halo_spectrum(p: vec2f, density: f32, width: f32) -> f32 {
    let radius = length(p);
    let angle = fract(atan2(p.y, p.x) / 6.2831853 + 0.5);
    let energy = spectrum_at(angle);
    let halo = glow_line(abs(radius - 0.46 - energy * 0.08), width * 1.5);
    let bars = pow(max(0.0, cos(angle * 6.2831853 * 24.0 * density)), 18.0)
        * step(0.47, radius) * step(radius, 0.52 + energy * 0.22);
    let wave = glow_line(abs(p.y - waveform_at(p.x * 0.5 + 0.5) * 0.16), width * 1.2)
        * step(abs(p.x), 0.95);
    return halo + bars * (0.4 + energy) + wave * 0.7;
}

fn atomic_orbits(p: vec2f, density: f32, width: f32, time: f32) -> f32 {
    var light = 0.0;
    for (var orbit = 0u; orbit < 5u; orbit += 1u) {
        let angle = f32(orbit) * 0.63 + time * (0.08 + f32(orbit) * 0.018);
        let q = rotate(p, angle);
        let ellipse = abs(length(vec2f(q.x, q.y * (2.2 + 0.2 * density))) - (0.42 + f32(orbit) * 0.045));
        light += glow_line(ellipse, width * 1.3) * (1.0 - f32(orbit) * 0.1);
    }
    let nucleus = glow_line(length(p), width * 5.0) * (1.0 + extras.transient * 2.0);
    return light + nucleus;
}

fn zone_visual(kind: u32, p: vec2f, density: f32, width: f32, time: f32) -> f32 {
    if kind == 0u { return pulse_trace(p, density, width, time); }
    if kind == 1u { return spectrum_skyline(p, density, width); }
    if kind == 2u { return radial_burst(p, density, width); }
    if kind == 3u { return spectrogram_city(p, density, width); }
    if kind == 4u { return spectral_terrain(p, density, width); }
    if kind == 5u { return wave_tunnel(p, density, width, time); }
    if kind == 6u { return particle_ocean(p, density, width, time); }
    if kind == 7u { return wireframe_terrain(p, density, width, time); }
    if kind == 8u { return halo_spectrum(p, density, width); }
    return atomic_orbits(p, density, width, time);
}

@fragment
fn fs_main(@builtin(position) position: vec4f) -> @location(0) vec4f {
    let resolution = max(uniforms.resolution, vec2f(1.0));
    let aspect = resolution.x / resolution.y;
    var uv = position.xy / resolution * 2.0 - 1.0;
    uv.y = -uv.y;
    uv.x *= aspect;

    var weighted_color = vec3f(0.0);
    var total_alpha = 0.0;
    for (var index = 0u; index < 8u; index += 1u) {
        if f32(index) >= scene.values[0] {
            continue;
        }
        let base = 8u + index * 8u;
        let extension = 96u + index * 8u;
        let center = vec2f(
            (scene.values[base] * 2.0 - 1.0) * aspect,
            1.0 - scene.values[base + 1u] * 2.0
        );
        let radius = max(scene.values[base + 2u] * 2.0, 0.04);
        let strength = scene.values[base + 3u];
        let energy = scene.values[base + 6u];
        let kind = u32(clamp(scene.values[extension], 0.0, 9.0));
        let rotation = scene.values[extension + 1u];
        let speed = scene.values[extension + 2u];
        let density = scene.values[extension + 3u];
        let thickness = scene.values[extension + 4u];
        let trail = scene.values[extension + 5u];
        let color_shift = scene.values[extension + 6u];
        let p = rotate((uv - center) / radius, rotation);
        if max(abs(p.x), abs(p.y)) > 1.35 {
            continue;
        }
        let width = 0.008 * thickness;
        let light = zone_visual(kind, p, density, width, uniforms.time * speed);
        let drive = clamp(0.16 + energy * strength * 0.8 + extras.transient * 0.18, 0.0, 1.7);
        let edge = 1.0 - smoothstep(0.92, 1.35, max(abs(p.x), abs(p.y)));
        let alpha = clamp(light * drive * edge * (0.45 + trail * 0.55), 0.0, 0.92);
        let color = palette(f32(kind), color_shift) * (0.75 + light * 0.35);
        weighted_color += color * alpha;
        total_alpha += alpha;
    }
    if total_alpha <= 0.0001 {
        return vec4f(0.0);
    }
    return vec4f(weighted_color / total_alpha, clamp(total_alpha, 0.0, 0.94));
}
