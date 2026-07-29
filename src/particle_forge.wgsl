struct Uniforms {
    values: array<vec4<f32>, 8>,
};

struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    previous: vec4<f32>,
    data: vec4<f32>,
};

struct Node {
    position_radius: vec4<f32>,
    strength_kind_band_falloff: vec4<f32>,
    axis_pinned: vec4<f32>,
    reserved: vec4<f32>,
};

struct Colors {
    bass: vec4<f32>,
    mid: vec4<f32>,
    treble: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read_write> simulated_particles: array<Particle>;
@group(0) @binding(2) var<storage, read> nodes: array<Node>;
@group(0) @binding(3) var<storage, read> model_points: array<vec4<f32>>;
@group(1) @binding(0) var<uniform> render_uniforms: Uniforms;
@group(1) @binding(1) var<storage, read> rendered_particles: array<Particle>;
@group(1) @binding(2) var<storage, read> render_nodes: array<Node>;
@group(1) @binding(3) var<uniform> colors: Colors;

fn hash(value: u32) -> f32 {
    var x = value;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = (x >> 16u) ^ x;
    return f32(x) / 4294967295.0;
}

fn initial_position(index: u32, topology: f32, morph: f32) -> vec3<f32> {
    let u = hash(index * 3u + 1u) * 6.2831853;
    let v = hash(index * 3u + 2u) * 6.2831853;
    let jitter = hash(index * 3u + 3u);
    let ring = vec3<f32>(
        (1.25 + cos(v) * 0.42) * cos(u),
        sin(v) * 0.42,
        (1.25 + cos(v) * 0.42) * sin(u),
    );
    let sphere = vec3<f32>(cos(u) * sin(v), cos(v), sin(u) * sin(v)) * (0.8 + jitter * 0.55);
    let helix = vec3<f32>(cos(u * 4.0), (jitter - 0.5) * 2.4, sin(u * 4.0));
    let core = normalize(vec3<f32>(cos(u), sin(v), sin(u))) * (0.08 + jitter * 0.28);
    if topology < 0.5 {
        return mix(ring, sphere, morph);
    }
    if topology < 1.5 {
        return mix(ring * vec3<f32>(0.8, 1.5, 0.8), sphere, morph);
    }
    if topology < 2.5 {
        return mix(sphere, helix, morph);
    }
    if topology < 3.5 {
        return mix(helix, core, morph);
    }
    return mix(core, ring, morph);
}

fn band_energy(band: f32) -> f32 {
    if band < 0.5 { return uniforms.values[1].w; }
    if band < 1.5 { return uniforms.values[1].x; }
    if band < 2.5 { return uniforms.values[1].y; }
    return uniforms.values[1].z;
}

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) id: vec3<u32>) {
    let index = id.x;
    let count = u32(uniforms.values[2].w);
    if index >= count { return; }
    var particle = simulated_particles[index];
    let time = uniforms.values[0].z;
    let dt = uniforms.values[0].w;
    let topology = uniforms.values[3].y;
    let morph = uniforms.values[3].z;
    if particle.data.x <= 0.0 || particle.data.x > 24.0 || length(particle.position.xyz) > 8.0 {
        particle.position = vec4<f32>(initial_position(index, topology, morph), 1.0);
        particle.previous = particle.position;
        particle.velocity = vec4<f32>(0.0);
        particle.data = vec4<f32>(0.01, hash(index + 91u), hash(index + 177u), 1.0);
    }

    particle.previous = particle.position;
    var position = particle.position.xyz;
    var velocity = particle.velocity.xyz;
    let spin = uniforms.values[3].w * vec3<f32>(1.0, 0.0, 0.0)
        + uniforms.values[4].x * vec3<f32>(0.0, 1.0, 0.0)
        + uniforms.values[4].y * vec3<f32>(0.0, 0.0, 1.0);
    let radial = max(length(position.xz), 0.001);
    let torus_center = vec3<f32>(position.x / radial * 1.25, 0.0, position.z / radial * 1.25);
    let tube = position - torus_center;
    let tangent = normalize(vec3<f32>(-position.z, uniforms.values[4].z * sin(time + particle.data.y * 6.28), position.x));
    velocity += tangent * dt * (0.35 + uniforms.values[1].x * 1.5);
    velocity += cross(spin, position) * dt;
    velocity += normalize(tube + vec3<f32>(0.001)) * -dt * (0.25 + uniforms.values[4].z);

    let node_count = u32(uniforms.values[3].x);
    for (var node_index = 0u; node_index < node_count; node_index++) {
        let node = nodes[node_index];
        let delta = node.position_radius.xyz - position;
        let distance = max(length(delta), 0.04);
        let radius = max(node.position_radius.w, 0.05);
        let influence = pow(clamp(1.0 - distance / (radius * 4.0), 0.0, 1.0), max(node.strength_kind_band_falloff.w, 0.25));
        let strength = node.strength_kind_band_falloff.x * (0.25 + band_energy(node.strength_kind_band_falloff.z) * 2.0) * influence;
        let kind = node.strength_kind_band_falloff.y;
        let direction = delta / distance;
        if kind < 0.5 {
            velocity -= direction * strength * dt * 0.45;
        } else if kind < 1.5 {
            velocity += direction * strength * dt;
        } else if kind < 2.5 {
            velocity -= direction * strength * dt;
        } else {
            let axis = normalize(node.axis_pinned.xyz + vec3<f32>(0.001));
            velocity += cross(axis, -direction) * strength * dt * select(0.7, 1.35, kind > 3.5);
            if kind > 3.5 {
                velocity += direction * strength * dt * 0.18;
            }
        }
    }

    let event = uniforms.values[6].w;
    velocity += normalize(position + vec3<f32>(0.001)) * event * dt * 4.0;
    let source = uniforms.values[7].x;
    let model_count = u32(uniforms.values[7].y);
    if source > 0.5 && model_count > 0u {
        let model_target = model_points[index % model_count].xyz;
        let model_scale = 1.4 / max(length(model_target), 1.0);
        let attachment = model_target * model_scale;
        velocity += (attachment - position) * dt * select(0.45, 1.2, source > 1.5);
    }
    let max_speed = 1.8 + uniforms.values[1].w * 2.0;
    velocity = clamp(length(velocity), 0.0, max_speed) * normalize(velocity + vec3<f32>(0.0001));
    velocity *= pow(0.985, dt * 60.0);
    position += velocity * dt;
    particle.position = vec4<f32>(position, 1.0);
    particle.velocity = vec4<f32>(velocity, 0.0);
    particle.data.x += dt;
    simulated_particles[index] = particle;
}

fn rotate_x(point: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle); let s = sin(angle);
    return vec3<f32>(point.x, point.y * c - point.z * s, point.y * s + point.z * c);
}

fn rotate_y(point: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle); let s = sin(angle);
    return vec3<f32>(point.x * c + point.z * s, point.y, -point.x * s + point.z * c);
}

fn project(point: vec3<f32>) -> vec4<f32> {
    let yaw = render_uniforms.values[5].w + sin(render_uniforms.values[0].z * 0.07) * render_uniforms.values[7].w * 0.18;
    let pitch = render_uniforms.values[6].x;
    let zoom = max(render_uniforms.values[6].y, 0.35);
    let camera = rotate_x(rotate_y(point, yaw), pitch);
    let depth = camera.z + 4.5 / zoom;
    let aspect = render_uniforms.values[0].x / max(render_uniforms.values[0].y, 1.0);
    return vec4<f32>(camera.x / max(depth, 0.2) / aspect * 2.2, camera.y / max(depth, 0.2) * 2.2, clamp(depth / 9.0, 0.0, 1.0), 1.0);
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) frequency: f32,
    @location(2) depth: f32,
    @location(3) speed: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let particle = rendered_particles[instance];
    let clip = project(particle.position.xyz);
    let speed = length(particle.velocity.xyz);
    let size = (1.1 + speed * 1.8 + render_uniforms.values[1].w * 2.0) / max(render_uniforms.values[0].y, 1.0);
    var output: VertexOutput;
    output.position = clip + vec4<f32>(corners[vertex] * size * 2.0, 0.0, 0.0);
    output.local = corners[vertex];
    output.frequency = particle.data.y;
    output.depth = clip.z;
    output.speed = speed;
    return output;
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let distance = length(input.local);
    if distance > 1.0 { discard; }
    let shift = fract(input.frequency + render_uniforms.values[6].z);
    var color = mix(colors.bass.rgb, colors.mid.rgb, clamp(shift * 2.0, 0.0, 1.0));
    color = mix(color, colors.treble.rgb, clamp(shift * 2.0 - 1.0, 0.0, 1.0));
    let energy = render_uniforms.values[5].x;
    let cyber = render_uniforms.values[5].y;
    let cosmic = render_uniforms.values[5].z;
    color = mix(color, vec3<f32>(0.2, 0.9, 1.0), cyber * step(0.72, fract(input.frequency * 12.0)));
    color = mix(color, vec3<f32>(0.55, 0.35, 1.0), cosmic * 0.45);
    let glow = pow(1.0 - distance, mix(1.2, 4.0, cyber)) * (0.35 + energy * 0.65);
    let fog = mix(1.0, 0.18, input.depth * (0.45 + cosmic * 0.4));
    return vec4<f32>(color * glow * fog * (1.0 + input.speed * 0.35), glow * fog);
}
