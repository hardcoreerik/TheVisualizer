// Particle Forge — fully GPU compute simulation (wgpu / WGSL).
// Architecture (standard GPU particle pipeline):
//   • Single RW storage buffer for particles
//   • Workgroup shared memory tile for O(wg²) local neighbor forces
//   • Formation lock + hard respawn on mode/epoch change
//   • Instanced GPU rasterization of billboards / motion streaks
//
// 10 music-driven physics modes:
//   0 Vortex ring  1 Boids  2 Plummer wells  3 Lorentz  4 Curl
//   5 Spectrum fountain  6 Spectral Form (Lucio-style 3D sound)  7 Spring mesh
//   8 Fireworks  9 Accretion
//
// Uniform layout: array<vec4, 14> (56 floats)
// [0] w,h,time,dt  [1] low,mid,high,rms  [2] onset,transient,gain,count
// [3] nodes,reseedEpoch,fieldX,spinX  [4] spinY,spinZ,twist,fieldY
// [5] energy,cyber,cosmic,yaw  [6] pitch,zoom,grad,event
// [7] source,modelCount,mode,autoCam
// [8] modeParams0..3  [9] modeParams4..7
// [10..13] spectrum0..15

struct Uniforms {
    values: array<vec4<f32>, 14>,
};

struct Particle {
    position: vec4<f32>,
    velocity: vec4<f32>,
    previous: vec4<f32>,
    data: vec4<f32>, // age, phase, seed, physics_mode
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

// Single storage buffer (read_write) — simpler and reliable for mode swaps.
@group(0) @binding(0) var<uniform> uniforms: Uniforms;
@group(0) @binding(1) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(2) var<storage, read> nodes: array<Node>;
@group(0) @binding(3) var<storage, read> model_points: array<vec4<f32>>;

@group(1) @binding(0) var<uniform> render_uniforms: Uniforms;
@group(1) @binding(1) var<storage, read> rendered_particles: array<Particle>;
@group(1) @binding(2) var<storage, read> render_nodes: array<Node>;
@group(1) @binding(3) var<uniform> colors: Colors;

// Workgroup tile for GPU-local neighbor forces (classic compute boids / SPH tile).
const WG_SIZE: u32 = 256u;
var<workgroup> tile_pos: array<vec3<f32>, 256>;
var<workgroup> tile_vel: array<vec3<f32>, 256>;
var<workgroup> tile_valid: array<u32, 256>;

fn hash(value: u32) -> f32 {
    var x = value;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = ((x >> 16u) ^ x) * 0x45d9f3bu;
    x = (x >> 16u) ^ x;
    return f32(x) / 4294967295.0;
}

fn hash3(p: vec3<f32>) -> vec3<f32> {
    let n = vec3<f32>(
        dot(p, vec3<f32>(127.1, 311.7, 74.7)),
        dot(p, vec3<f32>(269.5, 183.3, 246.1)),
        dot(p, vec3<f32>(113.5, 271.9, 124.6)),
    );
    return fract(sin(n) * 43758.5453);
}

fn value_noise(p: vec3<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash3(i);
    let b = hash3(i + vec3<f32>(1.0, 0.0, 0.0));
    let c = hash3(i + vec3<f32>(0.0, 1.0, 0.0));
    let d = hash3(i + vec3<f32>(1.0, 1.0, 0.0));
    let e = hash3(i + vec3<f32>(0.0, 0.0, 1.0));
    let g = hash3(i + vec3<f32>(1.0, 0.0, 1.0));
    let h = hash3(i + vec3<f32>(0.0, 1.0, 1.0));
    let j = hash3(i + vec3<f32>(1.0, 1.0, 1.0));
    return mix(
        mix(mix(a.x, b.x, u.x), mix(c.x, d.x, u.x), u.y),
        mix(mix(e.x, g.x, u.x), mix(h.x, j.x, u.x), u.y),
        u.z,
    );
}

fn curl_noise(p: vec3<f32>) -> vec3<f32> {
    let e = 0.09;
    let nxy = value_noise(p + vec3<f32>(0.0, e, 0.0)) - value_noise(p - vec3<f32>(0.0, e, 0.0));
    let nxz = value_noise(p + vec3<f32>(0.0, 0.0, e)) - value_noise(p - vec3<f32>(0.0, 0.0, e));
    let nyx = value_noise(p + vec3<f32>(e, 0.0, 0.0) + 19.0) - value_noise(p - vec3<f32>(e, 0.0, 0.0) + 19.0);
    let nyz = value_noise(p + vec3<f32>(0.0, 0.0, e) + 19.0) - value_noise(p - vec3<f32>(0.0, 0.0, e) + 19.0);
    let nzx = value_noise(p + vec3<f32>(e, 0.0, 0.0) + 41.0) - value_noise(p - vec3<f32>(e, 0.0, 0.0) + 41.0);
    let nzy = value_noise(p + vec3<f32>(0.0, e, 0.0) + 41.0) - value_noise(p - vec3<f32>(0.0, e, 0.0) + 41.0);
    return vec3<f32>(nyz - nzy, nzx - nxz, nxy - nyx) / (2.0 * e);
}

fn mp(i: u32) -> f32 {
    if i < 4u { return uniforms.values[8][i]; }
    return uniforms.values[9][i - 4u];
}

// ── Music / waveform drive ──────────────────────────────────────────────────
// Host packs: values[1]=low,mid,high,rms  values[2]=onset,transient,gain,count
// Gain is the Mode “Field/Music response” slider. Soft-knee keeps peaks hot but bounded.
fn audio_gain() -> f32 {
    return max(uniforms.values[2].z, 0.35);
}

fn punch(x: f32) -> f32 {
    // Nonlinear lift so quiet music still moves, loud music spikes harder.
    let g = audio_gain();
    let v = max(x, 0.0) * g * 1.35;
    return clamp(pow(v, 0.82) * 1.25, 0.0, 3.2);
}

fn a_low() -> f32 { return punch(uniforms.values[1].x); }
fn a_mid() -> f32 { return punch(uniforms.values[1].y); }
fn a_high() -> f32 { return punch(uniforms.values[1].z); }
fn a_rms() -> f32 { return punch(uniforms.values[1].w); }
fn a_onset() -> f32 { return punch(uniforms.values[2].x) * 1.4; }
fn a_transient() -> f32 { return punch(uniforms.values[2].y) * 1.2; }
fn a_event() -> f32 { return punch(uniforms.values[6].w); }

// 0..~2.5 overall energy for size, glow, and shared kicks.
fn music_drive() -> f32 {
    return clamp(
        a_rms() * 0.45 + a_low() * 0.4 + a_mid() * 0.25 + a_onset() * 0.55 + a_transient() * 0.2,
        0.0,
        2.8,
    );
}

fn spectrum_band(i: u32) -> f32 {
    // 16 packed bands across values[10..13]
    let idx = i % 16u;
    let base = 10u + idx / 4u;
    let comp = idx % 4u;
    var raw = 0.0;
    if base == 10u { raw = uniforms.values[10][comp]; }
    else if base == 11u { raw = uniforms.values[11][comp]; }
    else if base == 12u { raw = uniforms.values[12][comp]; }
    else { raw = uniforms.values[13][comp]; }
    return punch(raw);
}

// Smooth spectrum sample for Spectral Form (fractional bin).
fn spectrum_sample(freq01: f32) -> f32 {
    let f = clamp(freq01, 0.0, 1.0) * 15.0;
    let i0 = u32(floor(f));
    let i1 = min(i0 + 1u, 15u);
    let t = fract(f);
    return mix(spectrum_band(i0), spectrum_band(i1), t);
}

fn band_energy(band: f32) -> f32 {
    if band < 0.5 { return a_rms(); }
    if band < 1.5 { return a_low(); }
    if band < 2.5 { return a_mid(); }
    return a_high();
}

// Shared per-particle music impulse applied after mode forces.
fn music_impulse(position: vec3<f32>, velocity: vec3<f32>, phase: f32, seed: f32, dt: f32) -> vec3<f32> {
    var vel = velocity;
    let origin = field_origin();
    let local = position - origin;
    let drive = music_drive();
    let onset = a_onset();
    let bass = a_low();
    let treble = a_high();
    let mid = a_mid();
    // Bass: expand/contract shell around field center.
    let radial = normalize(local + vec3<f32>(0.001));
    vel += radial * (bass * 1.1 - mid * 0.25) * dt * 2.2;
    // Onset / transient kick along a hashed axis (waveform “hit”).
    let kick_dir = normalize(hash3(vec3<f32>(phase * 9.1, seed * 5.3, onset)) * 2.0 - 1.0);
    vel += kick_dir * (onset * 2.4 + a_transient() * 1.3 + a_event() * 1.1) * dt * 3.0;
    // Treble shimmer (high-frequency wiggle).
    vel += (hash3(position * 3.1 + vec3<f32>(uniforms.values[0].z * 8.0, seed, mid)) - 0.5)
        * treble * dt * 2.8;
    // Overall energy scales motion so louder music = livelier field.
    vel *= 1.0 + drive * 0.12;
    return vel;
}

// Right-drag field pan (host field_offset packed into morph/precession slots).
fn field_origin() -> vec3<f32> {
    return vec3<f32>(uniforms.values[3].z, uniforms.values[4].w, 0.0);
}

// Mountain height field on the spring mesh (rx/rz in 0..1 mesh UV).
// peaks = mp(2), height = mp(6); bounce driven by bass + onset + event.
fn mesh_mountain_height(rx: f32, rz: f32) -> f32 {
    let peaks = u32(clamp(mp(2) + 0.5, 0.0, 12.0));
    if peaks == 0u { return 0.0; }
    let mtn_h = mp(6);
    let bass = a_low();
    let mid = a_mid();
    let onset = a_onset();
    let event = a_event();
    let bounce = 0.2 + bass * 1.15 + mid * 0.35 + onset * 2.1 + event * 1.4 + music_drive() * 0.25;
    var height = 0.0;
    for (var p = 0u; p < peaks; p++) {
        let cx = hash(p * 17u + 3u);
        let cz = hash(p * 31u + 11u);
        let radius = 0.12 + hash(p * 7u + 5u) * 0.14;
        let dx = rx - cx;
        let dz = rz - cz;
        let d2 = dx * dx + dz * dz;
        let r2 = radius * radius;
        let lobe = exp(-d2 / max(r2, 0.002));
        let peak_scale = 0.65 + hash(p * 13u + 2u) * 0.7;
        let phase = hash(p * 19u + 1u) * 6.2831853;
        let pulse = 0.85 + 0.15 * sin(uniforms.values[0].z * (6.0 + f32(p)) + phase);
        height += lobe * peak_scale * pulse;
    }
    return height * mtn_h * bounce;
}

fn initial_position(index: u32, mode_id: u32, phase: f32, seed: f32) -> vec3<f32> {
    let u = phase * 6.2831853;
    let v = seed * 6.2831853;
    let j = hash(index * 17u + 3u);
    let origin = field_origin();

    var local = vec3<f32>(0.0);
    switch mode_id {
        // 0 Vortex — fat torus (classic ring)
        case 0u: {
            let R = max(mp(0), 0.8);
            let r = max(mp(1), 0.15);
            local = vec3<f32>((R + cos(v) * r) * cos(u), sin(v) * r, (R + cos(v) * r) * sin(u));
        }
        // 1 Boids — wide loose cloud
        case 1u: {
            local = (hash3(vec3<f32>(f32(index), phase, seed)) - 0.5) * 3.4;
        }
        // 2 Gravity wells — flat orbital plane
        case 2u: {
            local = vec3<f32>((phase - 0.5) * 3.4, (seed - 0.5) * 0.25, (j - 0.5) * 3.4);
        }
        // 3 Plasma — hollow sphere shell
        case 3u: {
            let dir = normalize(vec3<f32>(cos(u) * sin(v), cos(v), sin(u) * sin(v)) + vec3<f32>(0.001));
            local = dir * (1.15 + j * 0.25);
        }
        // 4 Curl nebula — dense ball
        case 4u: {
            let dir = normalize(hash3(vec3<f32>(f32(index), 2.1, 3.3)) * 2.0 - 1.0);
            local = dir * (j * 1.9);
        }
        // 5 Spectrum fountain — floor line of bars (very distinct)
        case 5u: {
            let bar = f32(index % 48u) / 47.0;
            local = vec3<f32>((bar - 0.5) * 3.8, -1.35, (seed - 0.5) * 0.35);
        }
        // 6 Spectral Form — frequency ring / layered manifold seed
        case 6u: {
            let freq = phase;
            let amp = 0.35 + seed * 0.4;
            let ang = freq * 6.2831853 * 2.0 + seed * 6.28;
            let r = 0.4 + amp * 0.8;
            local = vec3<f32>(cos(ang) * r, (freq - 0.5) * 2.0, sin(ang) * r);
        }
        // 7 Spring mesh — flat regular grid
        case 7u: {
            let side = 64u;
            let x = f32(index % side) / f32(side - 1u);
            let z = f32((index / side) % side) / f32(side - 1u);
            let mtn = mesh_mountain_height(x, z) * 0.55;
            local = vec3<f32>((x - 0.5) * 2.8, -0.15 + mtn, (z - 0.5) * 2.8);
        }
        // 8 Fireworks — packed core
        case 8u: {
            local = (hash3(vec3<f32>(f32(index), phase * 3.0, seed)) - 0.5) * 0.08;
        }
        // 9 Accretion — thin wide disk
        default: {
            let R = 0.55 + j * 1.9;
            local = vec3<f32>(cos(u) * R, (seed - 0.5) * 0.04, sin(u) * R);
        }
    }
    return local + origin;
}

fn apply_nodes(position: vec3<f32>, velocity: vec3<f32>, dt: f32) -> vec3<f32> {
    var vel = velocity;
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
            vel -= direction * strength * dt * 0.45;
        } else if kind < 1.5 {
            vel += direction * strength * dt;
        } else if kind < 2.5 {
            vel -= direction * strength * dt;
        } else {
            let axis = normalize(node.axis_pinned.xyz + vec3<f32>(0.001));
            vel += cross(axis, -direction) * strength * dt * select(0.7, 1.35, kind > 3.5);
            if kind > 3.5 {
                vel += direction * strength * dt * 0.18;
            }
        }
    }
    return vel;
}

fn apply_model(position: vec3<f32>, velocity: vec3<f32>, index: u32, dt: f32) -> vec3<f32> {
    var vel = velocity;
    let source = uniforms.values[7].x;
    let model_count = u32(uniforms.values[7].y);
    if source > 0.5 && model_count > 0u {
        let model_target = model_points[index % model_count].xyz;
        let model_scale = 1.4 / max(length(model_target), 1.0);
        vel += (model_target * model_scale - position) * dt * select(0.45, 1.2, source > 1.5);
    }
    return vel;
}

// ─── Mode 0: Original Particle Vortex ───────────────────────────────────────
// mp: majorR, tubeR, swirl, tension, lift, damping, bassDrive, nodeScale
fn force_vortex(position: vec3<f32>, velocity: vec3<f32>, spin: vec3<f32>, time: f32, phase: f32, dt: f32) -> vec3<f32> {
    var vel = velocity;
    let origin = field_origin();
    let local = position - origin;
    let major_r = max(mp(0), 0.4) * (1.0 + a_low() * 0.18);
    let tube_r = max(mp(1), 0.08) * (1.0 + a_mid() * 0.22);
    let swirl = mp(2) * (1.0 + a_mid() * 0.9 + music_drive() * 0.35);
    let tension = mp(3) * (1.0 + a_high() * 0.4);
    let lift = mp(4) * (1.0 + a_onset() * 0.5);
    let bass = a_low() * mp(6);
    let radial = max(length(local.xz), 0.001);
    let torus_center = origin + vec3<f32>(local.x / radial * major_r, 0.0, local.z / radial * major_r);
    let tube = position - torus_center;
    let tangent = normalize(vec3<f32>(-local.z, lift * sin(time + phase * 6.28 + a_mid() * 3.0), local.x));
    vel += tangent * dt * (swirl + bass * 2.6 + a_rms() * 0.8);
    vel += cross(spin * (1.0 + a_low() * 0.5), local) * dt;
    vel += normalize(tube + vec3<f32>(0.001)) * -dt * (tension + uniforms.values[4].z + a_transient() * 0.5);
    let tube_len = length(tube);
    if tube_len > 0.001 {
        vel += (tube / tube_len) * (tube_r - tube_len) * dt * (1.2 + a_onset() * 0.8);
    }
    return vel;
}

// ─── Mode 1: Reynolds Boids on GPU ──────────────────────────────────────────
// Workgroup shared-memory tile (256) + global storage samples from particles_in.
// mp: separation, alignment, cohesion, perception, maxSpeed, scatter, centerPull, climb
fn force_boids_from_samples(
    position: vec3<f32>,
    velocity: vec3<f32>,
    index: u32,
    count: u32,
    local_idx: u32,
    dt: f32,
) -> vec3<f32> {
    var vel = velocity;
    let sep_w = mp(0) * (1.0 + a_high() * 0.35);
    let ali_w = mp(1) * (1.0 + a_mid() * 0.45);
    let coh_w = mp(2) * (1.0 + a_low() * 0.35);
    let perception = max(mp(3), 0.15) * (1.0 + music_drive() * 0.2);
    let scatter = mp(5) * (a_onset() + a_transient() * 0.6 + 0.05);
    let center_pull = mp(6);
    let climb = mp(7) * (1.0 + a_mid() * 0.8);

    var sep = vec3<f32>(0.0);
    var ali = vec3<f32>(0.0);
    var coh = vec3<f32>(0.0);
    var neighbors = 0.0;

    // Local workgroup neighbors (GPU shared memory — primary flock signal).
    for (var j = 0u; j < WG_SIZE; j++) {
        if tile_valid[j] == 0u || j == local_idx { continue; }
        let other_p = tile_pos[j];
        let delta = position - other_p;
        let dist = length(delta);
        if dist < perception && dist > 0.001 {
            sep += delta / (dist * dist + 0.05);
            ali += tile_vel[j];
            coh += other_p;
            neighbors += 1.0;
        }
    }

    // Global storage samples (long-range cohesion across the full buffer).
    for (var k = 1u; k <= 12u; k++) {
        let other_i = (index + k * 977u + k * k * 131u) % max(count, 1u);
        if other_i == index { continue; }
        let other = particles[other_i];
        let delta = position - other.position.xyz;
        let dist = length(delta);
        if dist < perception * 1.6 && dist > 0.001 {
            sep += delta / (dist * dist + 0.08) * 0.55;
            ali += other.velocity.xyz * 0.55;
            coh += other.position.xyz * 0.55;
            neighbors += 0.55;
        }
    }

    if neighbors > 0.5 {
        sep = sep / neighbors;
        ali = ali / neighbors - velocity;
        coh = coh / neighbors - position;
        vel += sep * sep_w * dt;
        vel += ali * ali_w * dt;
        vel += coh * coh_w * dt;
    }

    vel += (hash3(position + velocity) * 2.0 - 1.0) * scatter * dt * 5.5;
    vel.y += (a_mid() - 0.2) * climb * dt * 1.4;
    let origin = field_origin();
    vel += -(position - origin) * center_pull * dt * (1.0 - a_onset() * 0.15);
    let local = position - origin;
    vel += vec3<f32>(-local.z, 0.0, local.x) * a_low() * dt * 0.55;
    return vel;
}

// Soft density pressure from workgroup tile (SPH-inspired GPU pressure force).
fn force_tile_pressure(position: vec3<f32>, local_idx: u32, strength: f32, radius: f32, dt: f32) -> vec3<f32> {
    var force = vec3<f32>(0.0);
    let r2 = radius * radius;
    for (var j = 0u; j < WG_SIZE; j++) {
        if tile_valid[j] == 0u || j == local_idx { continue; }
        let delta = position - tile_pos[j];
        let d2 = dot(delta, delta);
        if d2 < r2 && d2 > 1e-6 {
            let d = sqrt(d2);
            let w = 1.0 - d / radius;
            force += (delta / d) * (w * w) * strength;
        }
    }
    return force * dt;
}

// ─── Mode 2: Soft gravity wells from spectrum bands ─────────────────────────
// mp: G, softening, orbitBoost, wellSpread, planeFlatten, wellCount, drag, bassMass
fn force_gravity_wells(position: vec3<f32>, velocity: vec3<f32>, dt: f32) -> vec3<f32> {
    var vel = velocity;
    let origin = field_origin();
    let g = mp(0);
    let soft = max(mp(1), 0.05);
    let orbit = mp(2);
    let spread = mp(3);
    let flatten = mp(4);
    let well_count = u32(clamp(mp(5), 2.0, 8.0));
    let bass_mass = mp(7);

    for (var i = 0u; i < well_count; i++) {
        let t = f32(i) / f32(well_count);
        let ang = t * 6.2831853 + uniforms.values[0].z * 0.15;
        let mass = (0.2 + spectrum_band(i) * 2.6 + a_low() * bass_mass + a_onset() * 0.4) * g;
        let well = origin + vec3<f32>(cos(ang) * spread, sin(ang * 2.0) * 0.15, sin(ang) * spread);
        let delta = well - position;
        let r2 = dot(delta, delta) + soft * soft;
        vel += delta * (mass / pow(r2, 1.5)) * dt * (1.0 + music_drive() * 0.25);
        let tang = normalize(cross(vec3<f32>(0.0, 1.0, 0.0), delta) + vec3<f32>(0.001));
        vel += tang * orbit * mass * dt * (0.15 + a_mid() * 0.12);
    }
    vel.y += -(position.y - origin.y) * flatten * dt;
    return vel;
}

// ─── Mode 3: Lorentz plasma ─────────────────────────────────────────────────
// mp: Bscale, Escale, chargeMix, dipole, cyclotron, confine, trebleB, bassE
fn force_lorentz(position: vec3<f32>, velocity: vec3<f32>, time: f32, seed: f32, dt: f32) -> vec3<f32> {
    var vel = velocity;
    let b_scale = mp(0) * (1.0 + a_high() * 0.55 + music_drive() * 0.2);
    let e_scale = mp(1) * (1.0 + a_low() * 0.6);
    let dipole = mp(3);
    let cyclotron = mp(4) * (1.0 + a_mid() * 0.4);
    let confine = mp(5);
    let origin = field_origin();
    let local = position - origin;
    let r2 = max(dot(local, local), 0.06);
    let m = dipole * (1.0 + a_high() * mp(6));
    let b = (3.0 * local.y * local - vec3<f32>(0.0, r2, 0.0)) * (m / pow(r2, 2.5)) * b_scale
        + vec3<f32>(0.0, 0.55 + a_mid() * 0.55, 0.0) * b_scale * 0.35;
    let charge = select(-1.0, 1.0, seed > mp(2));
    let e_field = -local * confine
        + vec3<f32>(sin(time + seed * 6.28), cos(time * 1.3 + a_onset()), sin(time * 0.7 + seed))
            * e_scale * (0.25 + a_low() * mp(7) + a_onset() * 0.5);
    vel += charge * (cross(vel, b) * cyclotron + e_field) * dt * (1.0 + a_transient() * 0.35);
    return vel;
}

// ─── Mode 4: Curl-noise nebula ──────────────────────────────────────────────
// mp: scale, speed, octaves, confine, swirl, rise, bassWarp, fine
fn force_curl(position: vec3<f32>, velocity: vec3<f32>, time: f32, dt: f32) -> vec3<f32> {
    var vel = velocity;
    let scale = mp(0) * (1.0 + a_high() * 0.25);
    let speed = mp(1) * (0.7 + music_drive() * 0.9 + a_mid() * 0.5);
    let confine = mp(3);
    let swirl = mp(4) * (1.0 + a_low() * 0.5);
    let rise = mp(5);
    let bass_warp = mp(6);
    let fine = mp(7) * (1.0 + a_high() * 0.6);
    let warp = 1.0 + a_low() * bass_warp + a_onset() * 0.35;
    let p = position * scale * warp + vec3<f32>(0.0, time * (0.25 + a_mid() * 0.2), time * 0.18);
    var flow = curl_noise(p) * speed;
    if mp(2) > 0.5 {
        flow += curl_noise(p * 2.3 + 10.0) * fine * 0.55;
    }
    if mp(2) > 1.5 {
        flow += curl_noise(p * 4.7 - 5.0) * fine * 0.25;
    }
    vel += flow * dt;
    let origin = field_origin();
    let local = position - origin;
    vel += vec3<f32>(-local.z, 0.0, local.x) * swirl * dt * 0.2;
    vel.y += (a_mid() - local.y * 0.2) * rise * dt * 1.3;
    vel += -local * confine * dt;
    return vel;
}

// ─── Mode 5: Spectrum fountain ──────────────────────────────────────────────
// mp: gravity, emitPower, spread, lifetimeBoost, barWidth, forward, drag, heightGain
fn force_fountain(position: vec3<f32>, velocity: vec3<f32>, index: u32, phase: f32, seed: f32, dt: f32) -> vec3<f32> {
    var vel = velocity;
    let gravity = mp(0);
    let emit = mp(1);
    let spread = mp(2);
    let bar_w = mp(4);
    let forward = mp(5);
    let height_gain = mp(7);

    let bar = index % 32u;
    let band = bar / 4u; // 0..7
    let energy = spectrum_band(band) * (0.55 + a_rms() * 0.9 + a_onset() * 0.35);
    let origin = field_origin();
    let x_target = origin.x + (f32(bar) / 31.0 - 0.5) * 3.4;
    let z_target = origin.z + (seed - 0.5) * bar_w;

    vel.x += (x_target - position.x) * dt * 3.5;
    vel.z += (z_target - position.z) * dt * 2.0;

    // Launch from floor — height tracks spectrum bar + beat.
    if position.y < origin.y - 1.05 {
        let launch = (0.35 + energy * height_gain) * emit * (0.85 + a_onset() * 2.2 + a_transient());
        vel.y = max(vel.y, launch);
        vel.x += (phase - 0.5) * spread * energy;
        vel.z += (seed - 0.5) * spread * 0.5;
        vel.z -= forward * energy * 0.3;
    }

    vel.y -= gravity * dt * (1.0 - a_low() * 0.08);
    vel.x += sin(uniforms.values[0].z * 2.0 + f32(bar) + a_mid()) * a_mid() * dt * 0.7;
    return vel;
}

// ─── Mode 6: Spectral Form (Lucio Arese–style 3D sound sculpture) ───────────
// Every particle is a frequency fragment in a layered 3D manifold:
//   Y ≈ frequency, radius ≈ amplitude, layers scroll as a spatial spectrogram.
// Color/frequency identity is stored in particle phase (data.y).
// mp: spread, height, scroll, spring, layers, rotate, network, bloom
fn force_spectral_form(
    position: vec3<f32>,
    velocity: vec3<f32>,
    index: u32,
    count: u32,
    phase: f32,
    seed: f32,
    age: f32,
    dt: f32,
) -> vec3<f32> {
    var vel = velocity;
    let origin = field_origin();
    let spread = max(mp(0), 0.2);
    let height = max(mp(1), 0.4);
    let scroll = mp(2);
    let spring = mp(3);
    let layers_f = clamp(mp(4), 1.0, 48.0);
    let rotate = mp(5);
    let network = mp(6);
    let bloom = mp(7);

    // Frequency assignment: stable per particle, jittered for density.
    let freq = fract(phase * 0.97 + seed * 0.03);
    let amp = spectrum_sample(freq);
    let amp2 = spectrum_sample(fract(freq + 0.04));
    let energy = max(amp, amp2 * 0.7);

    // Layer = depth history ring (spatial spectrogram stack).
    let bins = 96u;
    let layer = f32((index / bins) % u32(layers_f));
    let layer_t = layer / max(layers_f - 1.0, 1.0);

    // Live spectral radius + beat bloom.
    let r = (0.12 + energy * spread) * (1.0 + a_onset() * bloom * 0.45 + music_drive() * 0.12);
    let ang = freq * 6.2831853 * (1.5 + mp(0) * 0.15)
        + uniforms.values[0].z * rotate
        + layer * 0.11
        + seed * 0.7;
    // Scroll older layers backward in Z (time → space).
    let z_scroll = -layer_t * (0.35 + scroll * 1.8) - age * scroll * 0.15;
    // Mild helical drift so sustained tones form continuous trajectories.
    let helix = sin(uniforms.values[0].z * 1.7 + freq * 12.0) * energy * 0.08;

    let target = origin + vec3<f32>(
        cos(ang) * r + helix,
        (freq - 0.5) * height + a_low() * 0.05 * sin(ang * 2.0),
        sin(ang) * r + z_scroll,
    );

    // Strong spring so geometry reads the spectrum every frame.
    let pull = spring * (2.2 + energy * 3.5 + a_onset() * 1.5);
    vel += (target - position) * pull * dt;

    // Local network cohesion with same-frequency neighbors (layered lattice).
    if network > 0.01 {
        var coh = vec3<f32>(0.0);
        var n = 0.0;
        for (var k = 1u; k <= 6u; k++) {
            let other_i = (index + k * 97u) % max(count, 1u);
            let other = particles[other_i];
            let ofreq = other.data.y;
            if abs(ofreq - freq) < 0.08 {
                coh += other.position.xyz;
                n += 1.0;
            }
        }
        if n > 0.5 {
            coh = coh / n;
            vel += (coh - position) * network * dt * (0.8 + energy);
        }
    }

    // Onset: radial burst of active frequencies (exploding song texture).
    let radial = normalize(position - origin + vec3<f32>(0.001));
    vel += radial * a_onset() * energy * bloom * dt * 4.5;
    // Highs shimmer along the frequency axis.
    vel.y += (hash(index + 3u) - 0.5) * a_high() * dt * 1.8;

    return vel;
}

// ─── Mode 7: Spring-mass mesh ────────────────────────────────────────────────
// mp: stiffness, restLen, mountainPeaks, waveAmp, waveSpeed, shear, mountainHeight, pinStrength
fn force_springs(position: vec3<f32>, velocity: vec3<f32>, index: u32, count: u32, dt: f32) -> vec3<f32> {
    var vel = velocity;
    let k = mp(0);
    let rest = mp(1);
    let wave_amp = mp(3);
    let wave_speed = mp(4);
    let shear = mp(5);
    let pin = mp(7);
    let side = 64u;

    let offsets = array<i32, 4>(1, -1, i32(side), -i32(side));
    for (var n = 0u; n < 4u; n++) {
        let oi = i32(index) + offsets[n];
        if oi < 0 || oi >= i32(count) { continue; }
        let other = particles[u32(oi)].position.xyz;
        let delta = other - position;
        let dist = max(length(delta), 0.001);
        vel += (delta / dist) * ((dist - rest) * k) * dt;
    }
    let d0 = i32(index) + i32(side) + 1;
    let d1 = i32(index) + i32(side) - 1;
    if d0 >= 0 && d0 < i32(count) {
        let other = particles[u32(d0)].position.xyz;
        let delta = other - position;
        let dist = max(length(delta), 0.001);
        vel += (delta / dist) * (dist - rest * 1.414) * k * shear * dt;
    }
    if d1 >= 0 && d1 < i32(count) {
        let other = particles[u32(d1)].position.xyz;
        let delta = other - position;
        let dist = max(length(delta), 0.001);
        vel += (delta / dist) * (dist - rest * 1.414) * k * shear * dt;
    }

    let x = f32(index % side);
    let z = f32((index / side) % side);
    let rx = x / f32(side - 1u);
    let rz = z / f32(side - 1u);

    // Traveling surface waves (music mids + highs as faster ripples).
    let wave = sin(x * 0.2 + uniforms.values[0].z * wave_speed * (1.0 + a_mid() * 0.4))
        * cos(z * 0.18 - uniforms.values[0].z * wave_speed * 0.7);
    let treble_ripple = sin(x * 0.55 + uniforms.values[0].z * 9.0) * a_high();
    vel.y += (wave * wave_amp * (0.55 + a_mid()) + treble_ripple * 0.8) * dt * 1.4;

    // Beat-reactive mountains: pin rest height under peaks, spring strongly toward it.
    let mountain = mesh_mountain_height(rx, rz);
    let rest_y = -0.2 + mountain;
    let rest_pos = vec3<f32>((rx - 0.5) * 2.6, rest_y, (rz - 0.5) * 2.6) + field_origin();
    let peak_pin = pin * (1.0 + mountain * 1.8);
    vel += (rest_pos - position) * peak_pin * dt;
    // Extra upward kick on onset so mountains "thump" with the beat.
    vel.y += mountain * (a_onset() * 10.0 + a_transient() * 4.0 + a_low() * 1.5) * dt;
    return vel;
}

// ─── Mode 8: Firework bursts on onset ───────────────────────────────────────
// mp: blast, gravity, drag, sparkSpread, shellSpeed, recover, spinKick, flash
fn force_fireworks(position: vec3<f32>, velocity: vec3<f32>, phase: f32, seed: f32, dt: f32) -> vec3<f32> {
    var vel = velocity;
    let blast = mp(0);
    let gravity = mp(1);
    let spread = mp(3);
    let shell = mp(4);
    let recover = mp(5);
    let spin_kick = mp(6);
    let flash = mp(7);

    let hit = a_onset() * blast + a_event() * flash + a_transient() * 0.65;
    let dir = normalize(
        hash3(vec3<f32>(phase * 17.0, seed * 13.0, uniforms.values[0].z * hit + 1.0)) * 2.0 - 1.0
        + (position - field_origin()) * 0.15
        + vec3<f32>(0.001),
    );
    if hit > 0.04 {
        vel = dir * (shell + hit * 3.2 + a_rms() * 1.2 + music_drive() * 0.5) * (0.65 + spread * seed);
        vel += cross(vec3<f32>(0.0, 1.0, 0.0), dir) * spin_kick * hit;
    }

    vel.y -= gravity * dt * (1.0 - a_low() * 0.05);
    // Pull home slowly for next burst
    vel += -position * recover * dt * 0.35;
    // Twinkle drift
    vel += (hash3(position + vec3<f32>(uniforms.values[0].z, 0.0, 0.0)) - 0.5) * dt * 0.2;
    return vel;
}

// ─── Mode 9: Accretion disk + polar jets ─────────────────────────────────────
// mp: mass, spinDrag, diskFlat, jetPower, jetWidth, isco, outer, feed
fn force_accretion(position: vec3<f32>, velocity: vec3<f32>, seed: f32, dt: f32) -> vec3<f32> {
    var vel = velocity;
    let origin = field_origin();
    let local = position - origin;
    let mass = mp(0) * (0.65 + a_low() * mp(7) + music_drive() * 0.35);
    let spin_drag = mp(1) * (1.0 + a_mid() * 0.35);
    let flat = mp(2);
    let jet = mp(3);
    let jet_w = max(mp(4), 0.05);
    let isco = max(mp(5), 0.15);
    let outer = mp(6);

    let r = max(length(local.xz), 0.05);
    let radial = vec3<f32>(local.x, 0.0, local.z) / r;

    let r3 = pow(length(local) + 0.08, 3.0);
    vel += -local * (mass / r3) * dt;

    let tang = vec3<f32>(-local.z, 0.0, local.x) / r;
    let v_circ = sqrt(mass / max(r, isco));
    vel += tang * (v_circ * spin_drag - dot(vel, tang)) * dt * (0.9 + a_onset() * 0.25);

    vel.y += -local.y * flat * dt * 2.5;

    if r < isco * 1.4 {
        vel += -radial * dt * 0.8;
        let axis = exp(-(r * r) / (jet_w * jet_w));
        vel.y += sign(local.y + (seed - 0.5) * 0.01) * jet * axis
            * (0.55 + a_high() + a_onset() * 1.2 + a_event() * 0.8) * dt * 3.4;
    }

    if r > outer {
        vel += -radial * (r - outer) * dt * 1.5;
    }
    return vel;
}

@compute @workgroup_size(256)
fn cs_main(
    @builtin(global_invocation_id) id: vec3<u32>,
    @builtin(local_invocation_index) local_idx: u32,
) {
    let index = id.x;
    let count = u32(uniforms.values[2].w);
    // Note: `active` is a reserved WGSL keyword — do not use that name.
    let in_bounds = index < count;

    // GPU tile load into shared memory for local neighbor forces.
    if in_bounds {
        let src = particles[index];
        tile_pos[local_idx] = src.position.xyz;
        tile_vel[local_idx] = src.velocity.xyz;
        tile_valid[local_idx] = 1u;
    } else {
        tile_pos[local_idx] = vec3<f32>(0.0);
        tile_vel[local_idx] = vec3<f32>(0.0);
        tile_valid[local_idx] = 0u;
    }
    workgroupBarrier();

    if !in_bounds { return; }

    var particle = particles[index];
    let time = uniforms.values[0].z;
    let dt = max(uniforms.values[0].w, 0.0001);
    // Mode is packed at uniforms[7].z (host physics_mode 0..9).
    let mode_id = u32(clamp(uniforms.values[7].z + 0.5, 0.0, 9.0));
    let mode = f32(mode_id);
    // Reseed epoch at uniforms[3].y — bumped on every host mode change.
    let epoch = uniforms.values[3].y;

    var recycle_age = 30.0;
    if mode_id == 5u { recycle_age = 3.5; }
    else if mode_id == 6u { recycle_age = 10.0 + mp(4) * 0.2; }
    else if mode_id == 8u { recycle_age = 2.8; }

    // ALWAYS hard-respawn when mode or epoch changes (unmistakable mode swap).
    let needs_respawn = particle.data.x <= 0.0
        || particle.data.x > recycle_age
        || length(particle.position.xyz) > 12.0
        || abs(particle.data.w - mode) > 0.1
        || abs(particle.data.z - epoch) > 0.1;

    let phase = select(particle.data.y, hash(index + 91u), needs_respawn);
    let seed = hash(index + 177u);

    if needs_respawn {
        let spawn = initial_position(index, mode_id, phase, seed);
        particle.position = vec4<f32>(spawn, 1.0);
        particle.previous = particle.position;
        particle.velocity = vec4<f32>(0.0);
        if mode_id == 9u {
            let r = max(length(spawn.xz), 0.05);
            let tang = vec3<f32>(-spawn.z, 0.0, spawn.x) / r * sqrt(max(mp(0), 0.3) / r) * 0.9;
            particle.velocity = vec4<f32>(tang, 0.0);
        }
        particle.data = vec4<f32>(0.01, phase, epoch, mode);
        tile_pos[local_idx] = spawn;
        tile_vel[local_idx] = particle.velocity.xyz;
    }

    particle.previous = particle.position;
    var position = particle.position.xyz;
    var velocity = particle.velocity.xyz;
    let spin = vec3<f32>(uniforms.values[3].w, uniforms.values[4].x, uniforms.values[4].y);

    // Mode-specific physics.
    switch mode_id {
        case 0u: {
            velocity = force_vortex(position, velocity, spin, time, phase, dt);
            velocity += force_tile_pressure(position, local_idx, 0.35, 0.35, dt);
            velocity = apply_nodes(position, velocity, dt * max(mp(7), 0.25));
        }
        case 1u: {
            velocity = force_boids_from_samples(position, velocity, index, count, local_idx, dt);
        }
        case 2u: {
            velocity = force_gravity_wells(position, velocity, dt);
            velocity += force_tile_pressure(position, local_idx, 0.3, 0.45, dt);
        }
        case 3u: {
            velocity = force_lorentz(position, velocity, time, phase, dt);
        }
        case 4u: {
            velocity = force_curl(position, velocity, time, dt);
            velocity += force_tile_pressure(position, local_idx, 0.6 + uniforms.values[1].w, 0.5, dt);
        }
        case 5u: {
            velocity = force_fountain(position, velocity, index, phase, seed, dt);
        }
        case 6u: {
            velocity = force_spectral_form(
                position,
                velocity,
                index,
                count,
                phase,
                seed,
                particle.data.x,
                dt,
            );
            // Keep phase = frequency identity for coloring / networks.
            particle.data.y = phase;
        }
        case 7u: {
            velocity = force_springs(position, velocity, index, count, dt);
        }
        case 8u: {
            velocity = force_fireworks(position, velocity, phase, seed, dt);
        }
        default: {
            velocity = force_accretion(position, velocity, phase, dt);
            velocity += force_tile_pressure(position, local_idx, 0.25, 0.25, dt);
        }
    }

    // Shared waveform impulse (Spectral Form already is pure spectrum geometry).
    if mode_id != 6u {
        velocity = music_impulse(position, velocity, phase, seed, dt);
    }

    // Formation lock: pull toward this mode's canonical shape (includes field pan origin).
    // Loosen when music is loud so audio motion isn't over-damped.
    let formation = initial_position(index, mode_id, phase, seed);
    var formation_k = 1.8;
    if mode_id == 1u { formation_k = 0.35; }
    else if mode_id == 4u { formation_k = 0.45; }
    else if mode_id == 6u { formation_k = 0.08; }
    else if mode_id == 8u { formation_k = 0.2; }
    else if mode_id == 5u { formation_k = 0.9; }
    else if mode_id == 7u { formation_k = 0.55; }
    formation_k *= max(0.35, 1.0 - music_drive() * 0.28);
    velocity += (formation - position) * formation_k * dt;

    let event = a_event();
    if mode_id != 8u {
        let origin = field_origin();
        velocity += normalize(position - origin + vec3<f32>(0.001)) * event * dt * select(1.6, 4.2, mode_id != 9u);
    }
    velocity = apply_model(position, velocity, index, dt);

    var max_speed = 2.4 + uniforms.values[1].w * 2.2;
    var drag = 0.985;
    switch mode_id {
        case 0u: { drag = mix(0.99, 0.96, clamp(mp(5), 0.0, 1.0)); }
        case 1u: { max_speed = max(mp(4), 0.5); drag = 0.96; }
        case 5u: { max_speed = 4.5; drag = 0.995; }
        case 6u: { max_speed = 3.8; drag = 0.999; }
        case 7u: { max_speed = 2.2; drag = 0.90; }
        case 8u: { max_speed = 5.5; drag = mix(0.992, 0.95, mp(2)); }
        case 9u: { max_speed = 3.2; drag = 0.988; }
        default: {}
    }

    let speed = length(velocity);
    if speed > 0.0001 {
        velocity = normalize(velocity) * min(speed, max_speed);
    }
    velocity *= pow(drag, dt * 60.0);
    position += velocity * dt;

    if mode_id == 5u && position.y < -1.25 {
        position.y = -1.25;
        velocity.y *= -0.12;
    }

    particle.position = vec4<f32>(position, 1.0);
    particle.velocity = vec4<f32>(velocity, 0.0);
    particle.data.x += dt;
    particle.data.y = phase;
    particle.data.z = epoch;
    particle.data.w = mode;
    particles[index] = particle;
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
    return vec4<f32>(
        camera.x / max(depth, 0.2) / aspect * 2.2,
        camera.y / max(depth, 0.2) * 2.2,
        clamp(depth / 9.0, 0.0, 1.0),
        1.0,
    );
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) frequency: f32,
    @location(2) depth: f32,
    @location(3) speed: f32,
    @location(4) mode: f32,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) instance: u32) -> VertexOutput {
    let corners = array<vec2<f32>, 6>(
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, -1.0), vec2<f32>(1.0, 1.0),
        vec2<f32>(-1.0, -1.0), vec2<f32>(1.0, 1.0), vec2<f32>(-1.0, 1.0),
    );
    let particle = rendered_particles[instance];
    let mode_id = u32(clamp(render_uniforms.values[7].z + 0.5, 0.0, 9.0));
    let mode = f32(mode_id);
    let speed = length(particle.velocity.xyz);
    let curr = project(particle.position.xyz);

    // Motion streaks for fountain / fireworks / plasma / spectral trajectories
    let use_streak = (mode_id == 3u) || (mode_id == 5u) || (mode_id == 6u) || (mode_id == 8u);
    var clip = curr;
    var local = corners[vertex];
    // Size tracks velocity + music energy (use render_uniforms — group 1 only in this pass).
    let gain = max(render_uniforms.values[2].z, 0.35);
    let rms = clamp(pow(max(render_uniforms.values[1].w, 0.0) * gain * 1.35, 0.82) * 1.25, 0.0, 3.2);
    let onset = clamp(pow(max(render_uniforms.values[2].x, 0.0) * gain * 1.35, 0.82) * 1.75, 0.0, 3.2);
    var size = (1.35 + speed * 1.7 + rms * 2.1 + onset * 1.4) / max(render_uniforms.values[0].y, 1.0);
    size *= (0.7 + f32(mode_id) * 0.16) * (0.85 + clamp(gain * 0.12, 0.0, 0.55));

    if mode_id == 7u {
        size *= 0.55;
    } else if mode_id == 5u {
        size *= 1.25;
    } else if mode_id == 8u {
        size *= 1.4;
    }

    if use_streak && speed > 0.08 {
        let prev = project(particle.previous.xyz);
        let mid = (curr.xy + prev.xy) * 0.5;
        let dir = curr.xy - prev.xy;
        let len = max(length(dir), 0.001);
        let tangent = dir / len;
        let normal = vec2<f32>(-tangent.y, tangent.x);
        // Longer streaks when the beat hits.
        let half_len = min(len * (0.55 + onset * 0.35) + size * 0.55, 0.14);
        let half_w = size * select(1.15, 0.75, mode_id == 5u);
        let c = corners[vertex];
        clip = vec4<f32>(mid + tangent * c.x * half_len + normal * c.y * half_w, curr.z, 1.0);
        local = c;
    } else {
        clip = curr + vec4<f32>(corners[vertex] * size * 2.2, 0.0, 0.0);
    }

    var output: VertexOutput;
    output.position = clip;
    output.local = local;
    output.frequency = particle.data.y;
    output.depth = curr.z;
    output.speed = speed;
    output.mode = mode;
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
    let mode = input.mode;

    // Pure mode palette (spectrum only lightly tints) so swaps are obvious.
    let mode_id = u32(clamp(mode + 0.5, 0.0, 9.0));
    var mode_color = vec3<f32>(1.0, 0.55, 0.15);
    switch mode_id {
        case 0u: { mode_color = vec3<f32>(1.0, 0.55, 0.12); }      // amber vortex
        case 1u: { mode_color = vec3<f32>(1.0, 0.82, 0.28); }      // gold flock
        case 2u: { mode_color = vec3<f32>(0.55, 0.3, 1.0); }       // purple gravity
        case 3u: { mode_color = vec3<f32>(0.1, 0.95, 1.0); }        // cyan plasma
        case 4u: { mode_color = vec3<f32>(0.78, 0.35, 1.0); }       // violet curl
        case 5u: { mode_color = mix(vec3<f32>(0.2, 1.0, 0.45), vec3<f32>(1.0, 0.9, 0.2), shift); } // green→yellow fountain
        case 6u: {
            // Spectral Form: color = frequency (low→warm, high→cool) like Lucio maps.
            let f = fract(input.frequency);
            mode_color = mix(
                mix(vec3<f32>(0.95, 0.25, 0.55), vec3<f32>(0.2, 0.95, 0.75), clamp(f * 2.0, 0.0, 1.0)),
                vec3<f32>(0.55, 0.75, 1.0),
                clamp(f * 2.0 - 1.0, 0.0, 1.0),
            );
            mode_color = mix(mode_color, vec3<f32>(1.0, 1.0, 1.0), clamp(input.speed * 0.15, 0.0, 0.35));
        }
        case 7u: { mode_color = vec3<f32>(0.05, 1.0, 0.72); }       // teal mesh
        case 8u: { mode_color = vec3<f32>(1.0, 0.97, 0.92); }       // white fireworks
        default: { mode_color = mix(vec3<f32>(1.0, 0.7, 0.15), vec3<f32>(0.3, 0.6, 1.0), clamp(input.speed * 0.3, 0.0, 1.0)); }
    }
    // Music tints (render_uniforms only — no compute group bindings here).
    let gain_f = max(render_uniforms.values[2].z, 0.35);
    let bass = clamp(pow(max(render_uniforms.values[1].x, 0.0) * gain_f * 1.35, 0.82) * 1.25, 0.0, 3.2);
    let treble = clamp(pow(max(render_uniforms.values[1].z, 0.0) * gain_f * 1.35, 0.82) * 1.25, 0.0, 3.2);
    let onset_g = clamp(pow(max(render_uniforms.values[2].x, 0.0) * gain_f * 1.35, 0.82) * 1.75, 0.0, 3.2);
    let rms_g = clamp(pow(max(render_uniforms.values[1].w, 0.0) * gain_f * 1.35, 0.82) * 1.25, 0.0, 3.2);
    color = mix(mode_color, color, 0.12);
    color = mix(color, vec3<f32>(1.0, 0.45, 0.12), clamp(bass * 0.28, 0.0, 0.45));
    color = mix(color, vec3<f32>(0.45, 0.85, 1.0), clamp(treble * 0.22, 0.0, 0.4));
    color = mix(color, vec3<f32>(1.0, 1.0, 1.0), clamp(onset_g * 0.35, 0.0, 0.55));
    color = mix(color, vec3<f32>(0.2, 0.9, 1.0), cyber * 0.12);
    color = mix(color, vec3<f32>(0.55, 0.35, 1.0), cosmic * 0.1);

    var glow_pow = mix(1.1, 3.5, cyber);
    if mode_id == 8u { glow_pow = 0.75; }
    if mode_id == 7u { glow_pow = 2.4; }
    let music_glow = 0.45 + energy * 0.55 + bass * 0.25 + onset_g * 0.55 + rms_g * 0.35;
    let glow = pow(1.0 - distance, glow_pow) * music_glow;
    let fog = mix(1.0, 0.2, input.depth * (0.4 + cosmic * 0.35));
    let speed_boost = 1.0 + input.speed * select(0.65, 0.35, mode_id == 7u) + onset_g * 0.4;
    return vec4<f32>(color * glow * fog * speed_boost, glow * fog);
}
