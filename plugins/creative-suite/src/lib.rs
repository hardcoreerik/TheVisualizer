use std::sync::Mutex;

use thevisualizer_plugin_sdk::{
    ABI_VERSION_V2, EVENT_BEAT, EVENT_TAKE_A, EVENT_TAKE_B, FeatureSnapshotV2, MAX_PLUGIN_COMMANDS,
    OP_SET, PLUGIN_OK, PluginCommandV2, PluginInitV2, PluginOutputV2, PluginV2, TARGET_CAMERA,
    TARGET_COLOR, TARGET_FORGE, TARGET_MACRO, TARGET_MODE_PARAMETER, TARGET_PERFORMANCE,
    TARGET_RESPONSE, TARGET_ZONE,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Profile {
    Beat,
    Color,
    Zone,
    Camera,
    Particle,
    Transition,
    Midi,
    Director,
    // New profiles (10)
    AnalogFractal,
    SpectrumSculptor,
    OnsetArchitect,
    SilenceGardener,
    MacroWeaver,
    StereoNavigator,
    PulseDrummer,
    ColorStorm,
    DeckJuggler,
    AmbientOrbit,
}

struct State {
    profile: Profile,
    phase: f32,
    onset_high: bool,
    take_b: bool,
    quiet_frames: f32,
    initialized: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    profile: Profile::Beat,
    phase: 0.0,
    onset_high: false,
    take_b: false,
    quiet_frames: 0.0,
    initialized: false,
});

fn profile_from_id(id: &str) -> Option<Profile> {
    if id.contains("spectral-colorist") {
        Some(Profile::Color)
    } else if id.contains("zone-dancer") {
        Some(Profile::Zone)
    } else if id.contains("camera-pilot") {
        Some(Profile::Camera)
    } else if id.contains("particle-conductor") {
        Some(Profile::Particle)
    } else if id.contains("transition-dj") {
        Some(Profile::Transition)
    } else if id.contains("midi-performance-mapper") {
        Some(Profile::Midi)
    } else if id.contains("ambient-auto-director") {
        Some(Profile::Director)
    } else if id.contains("beat-choreographer") {
        Some(Profile::Beat)
    } else if id.contains("analog-fractal-pilot") {
        Some(Profile::AnalogFractal)
    } else if id.contains("spectrum-sculptor") {
        Some(Profile::SpectrumSculptor)
    } else if id.contains("onset-architect") {
        Some(Profile::OnsetArchitect)
    } else if id.contains("silence-gardener") {
        Some(Profile::SilenceGardener)
    } else if id.contains("macro-weaver") {
        Some(Profile::MacroWeaver)
    } else if id.contains("stereo-navigator") {
        Some(Profile::StereoNavigator)
    } else if id.contains("pulse-drummer") {
        Some(Profile::PulseDrummer)
    } else if id.contains("color-storm") {
        Some(Profile::ColorStorm)
    } else if id.contains("deck-juggler") {
        Some(Profile::DeckJuggler)
    } else if id.contains("ambient-orbit") {
        Some(Profile::AmbientOrbit)
    } else {
        None
    }
}

unsafe extern "C" fn initialize(init: *const PluginInitV2) -> i32 {
    if init.is_null() {
        return 1;
    }
    // SAFETY: The host guarantees the initialization value is readable for this call.
    let init = unsafe { &*init };
    if init.abi_version != ABI_VERSION_V2
        || init.struct_size < size_of::<PluginInitV2>() as u32
        || init.profile_id.is_null()
    {
        return 2;
    }
    // SAFETY: The host keeps the profile bytes alive for this synchronous call.
    let id = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(
            init.profile_id,
            init.profile_id_len as usize,
        ))
    }
    .unwrap_or_default();
    let Some(profile) = profile_from_id(id) else {
        return 3;
    };
    let Ok(mut state) = STATE.lock() else {
        return 4;
    };
    *state = State {
        profile,
        phase: 0.0,
        onset_high: false,
        take_b: false,
        quiet_frames: 0.0,
        initialized: true,
    };
    PLUGIN_OK
}

unsafe extern "C" fn process(
    features: *const FeatureSnapshotV2,
    output: *mut PluginOutputV2,
) -> i32 {
    if features.is_null() || output.is_null() {
        return 1;
    }
    // SAFETY: The host owns both ABI values for this synchronous call.
    let (features, output) = unsafe { (&*features, &mut *output) };
    let Ok(mut state) = STATE.lock() else {
        return 4;
    };
    if !state.initialized
        || features.abi_version != ABI_VERSION_V2
        || features.struct_size < size_of::<FeatureSnapshotV2>() as u32
        || output.abi_version != ABI_VERSION_V2
        || output.struct_size < size_of::<PluginOutputV2>() as u32
    {
        return 2;
    }
    let controls = if features.controls.is_null() {
        &[][..]
    } else {
        // SAFETY: The host keeps the bounded control slice valid for this call.
        unsafe {
            std::slice::from_raw_parts(features.controls, features.controls_len.min(18) as usize)
        }
    };
    *output = PluginOutputV2::default();
    state.phase = (state.phase + features.base.delta_seconds.clamp(0.0, 0.1)).rem_euclid(4096.0);
    let beat = features.onset > 0.5 && !state.onset_high;
    state.onset_high = features.onset > 0.2;
    if features.base.rms < 0.04 {
        state.quiet_frames = (state.quiet_frames + features.base.delta_seconds).min(30.0);
    } else {
        state.quiet_frames = (state.quiet_frames - features.base.delta_seconds * 2.0).max(0.0);
    }
    if beat {
        output.event_flags |= EVENT_BEAT;
    }

    let low = features.base.low;
    let mid = features.base.mid;
    let high = features.base.high;
    let rms = features.base.rms;
    let peak = features.base.peak;
    let transient = features.transient;

    match state.profile {
        Profile::Beat => {
            push(output, TARGET_RESPONSE, 0, 0.9 + low * 0.9);
            push(output, TARGET_COLOR, 2, 0.7 + transient * 1.3);
            push(output, TARGET_FORGE, 3, 0.35 + low * 1.4);
        }
        Profile::Color => {
            let hue = (state.phase * (0.025 + high * 0.06)).fract();
            push(output, TARGET_COLOR, 0, hue);
            push(output, TARGET_COLOR, 1, (mid - high) * 1.4);
            push(output, TARGET_COLOR, 4, 0.65 + peak * 0.75);
        }
        Profile::Zone => {
            let orbit = state.phase * (0.45 + mid);
            push(output, TARGET_ZONE, 0, 0.5 + orbit.sin() * 0.28);
            push(output, TARGET_ZONE, 1, 0.5 + (orbit * 0.73).cos() * 0.24);
            push(output, TARGET_ZONE, 3, 0.8 + low * 1.8);
            push(
                output,
                TARGET_ZONE,
                4,
                ((low * 3.0 + high * 6.0) as u32 % 10) as f32,
            );
            push(output, TARGET_ZONE, 5, orbit);
        }
        Profile::Camera => {
            push(output, TARGET_CAMERA, 0, (state.phase * 0.18).sin() * 2.4);
            push(
                output,
                TARGET_CAMERA,
                1,
                (state.phase * 0.11).cos() * (0.18 + mid * 0.28),
            );
            push(output, TARGET_CAMERA, 2, 1.0 + low * 0.5);
        }
        Profile::Particle => {
            push(output, TARGET_FORGE, 0, (mid - 0.25) * 1.4);
            push(output, TARGET_FORGE, 1, 0.35 + low);
            push(output, TARGET_FORGE, 2, (high - 0.2) * 1.5);
            push(output, TARGET_FORGE, 3, 0.3 + transient * 1.7);
            push(output, TARGET_FORGE, 5, (state.phase * 0.08).fract());
            if beat {
                push(output, TARGET_FORGE, 4, (state.phase as u32 % 5) as f32);
            }
        }
        Profile::Transition => {
            push(
                output,
                TARGET_PERFORMANCE,
                0,
                0.5 + (state.phase * (0.18 + low * 0.2)).sin() * 0.5,
            );
            push(
                output,
                TARGET_PERFORMANCE,
                1,
                ((state.phase * 0.2) as u32 % 10) as f32,
            );
            push(output, TARGET_PERFORMANCE, 2, 0.3 + high * 0.7);
            push(output, TARGET_PERFORMANCE, 3, 1.0);
            if beat {
                output.event_flags |= if state.take_b {
                    EVENT_TAKE_A
                } else {
                    EVENT_TAKE_B
                };
                state.take_b = !state.take_b;
            }
        }
        Profile::Midi => {
            for index in 0..8 {
                let value = controls.get(10 + index).copied().unwrap_or(0.0);
                push(output, TARGET_MACRO, index as u32, value);
            }
        }
        Profile::Director => {
            let slow = state.phase * 0.08;
            push(output, TARGET_COLOR, 0, slow.fract());
            push(output, TARGET_COLOR, 2, 0.65 + rms * 0.9);
            push(output, TARGET_CAMERA, 0, slow.sin() * 1.2);
            push(output, TARGET_CAMERA, 2, 1.05 + slow.cos() * 0.18);
            push(output, TARGET_ZONE, 0, 0.5 + (slow * 1.7).sin() * 0.22);
            push(output, TARGET_ZONE, 1, 0.5 + (slow * 1.3).cos() * 0.18);
            push(output, TARGET_ZONE, 7, slow.sin());
            push(output, TARGET_FORGE, 3, 0.4 + transient);
        }
        // Conducts Fractal Reef mode parameters: dive, path, formation, travel.
        Profile::AnalogFractal => {
            push(output, TARGET_RESPONSE, 0, 1.1 + low * 0.7 + transient * 0.4);
            // dive-power (1), path-steer (2), travel-speed (7), wave-formation (27)
            push(output, TARGET_MODE_PARAMETER, 1, 0.8 + low * 2.2);
            push(output, TARGET_MODE_PARAMETER, 2, 0.5 + mid * 2.0);
            push(
                output,
                TARGET_MODE_PARAMETER,
                7,
                0.35 + rms * 1.1 + transient * 0.5,
            );
            push(
                output,
                TARGET_MODE_PARAMETER,
                27,
                0.7 + peak * 1.4 + high * 0.8,
            );
            push(
                output,
                TARGET_MODE_PARAMETER,
                28,
                0.4 + high * 1.5 + mid * 0.5,
            );
            push(output, TARGET_MODE_PARAMETER, 29, 0.5 + high * 1.2 + beat as u8 as f32 * 0.4);
            if beat {
                push(output, TARGET_MODE_PARAMETER, 4, 1.2 + transient * 1.5);
                push(output, TARGET_MODE_PARAMETER, 5, 0.4 + transient);
            }
            push(output, TARGET_COLOR, 2, 0.6 + peak * 0.9);
        }
        Profile::SpectrumSculptor => {
            let brightness = (low * 0.4 + mid * 0.35 + high * 0.4).clamp(0.0, 1.5);
            push(output, TARGET_COLOR, 0, (state.phase * 0.04 + high * 0.2).fract());
            push(output, TARGET_COLOR, 1, (high - low) * 1.2);
            push(output, TARGET_COLOR, 2, 0.55 + brightness * 0.9);
            push(output, TARGET_COLOR, 4, 0.7 + high * 0.6);
            push(output, TARGET_RESPONSE, 0, 0.85 + mid * 0.9);
            push(output, TARGET_MODE_PARAMETER, 21, 6.0 + high * 14.0);
            push(output, TARGET_MODE_PARAMETER, 22, -0.4 + mid * 1.8);
        }
        Profile::OnsetArchitect => {
            push(output, TARGET_RESPONSE, 0, 1.0 + transient * 1.1);
            if beat {
                let zone = ((state.phase * 3.0) as u32 % 4) * 8;
                push(output, TARGET_ZONE, zone, 0.35 + mid * 0.3);
                push(output, TARGET_ZONE, zone + 1, 0.35 + high * 0.3);
                push(output, TARGET_ZONE, zone + 3, 1.2 + peak);
                push(output, TARGET_ZONE, zone + 4, ((state.phase as u32) % 10) as f32);
                push(output, TARGET_FORGE, 3, 0.8 + transient * 1.4);
                push(output, TARGET_FORGE, 4, (state.phase as u32 % 5) as f32);
            }
            push(output, TARGET_COLOR, 2, 0.5 + transient * 1.5);
        }
        Profile::SilenceGardener => {
            let quiet = (state.quiet_frames * 0.15).clamp(0.0, 1.0);
            push(output, TARGET_RESPONSE, 0, 1.4 - quiet * 0.85);
            push(output, TARGET_COLOR, 2, 0.95 - quiet * 0.55);
            push(output, TARGET_COLOR, 4, 0.9 - quiet * 0.35);
            push(output, TARGET_CAMERA, 2, 1.0 + quiet * 0.35);
            push(output, TARGET_MODE_PARAMETER, 34, 0.25 + quiet * 1.1);
            push(output, TARGET_MODE_PARAMETER, 36, 0.4 + quiet * 0.8);
            if quiet < 0.2 {
                push(output, TARGET_MODE_PARAMETER, 7, 0.6 + rms * 1.2);
            } else {
                push(output, TARGET_MODE_PARAMETER, 7, 0.15);
            }
        }
        Profile::MacroWeaver => {
            push(output, TARGET_MACRO, 0, low.clamp(0.0, 1.0));
            push(output, TARGET_MACRO, 1, mid.clamp(0.0, 1.0));
            push(output, TARGET_MACRO, 2, high.clamp(0.0, 1.0));
            push(output, TARGET_MACRO, 3, rms.clamp(0.0, 1.0));
            push(output, TARGET_MACRO, 4, peak.clamp(0.0, 1.0));
            push(output, TARGET_MACRO, 5, transient.clamp(0.0, 1.0));
            push(
                output,
                TARGET_MACRO,
                6,
                (0.5 + 0.5 * (state.phase * 0.3).sin()).clamp(0.0, 1.0),
            );
            push(
                output,
                TARGET_MACRO,
                7,
                if beat { 1.0 } else { (features.onset).clamp(0.0, 1.0) },
            );
        }
        Profile::StereoNavigator => {
            // Approximate stereo motion from band imbalance and phase.
            let yaw = (mid - high) * 2.2 + (state.phase * 0.21).sin() * 0.6;
            let pitch = (low - mid) * 0.7 + (state.phase * 0.13).cos() * 0.25;
            push(output, TARGET_CAMERA, 0, yaw.clamp(-6.0, 6.0));
            push(output, TARGET_CAMERA, 1, pitch.clamp(-1.1, 1.1));
            push(output, TARGET_CAMERA, 2, 0.85 + rms * 0.9 + beat as u8 as f32 * 0.15);
            push(output, TARGET_MODE_PARAMETER, 31, 0.4 + (mid - high).abs() * 2.0);
        }
        Profile::PulseDrummer => {
            push(output, TARGET_RESPONSE, 0, 0.95 + low * 0.8);
            if beat {
                output.event_flags |= EVENT_BEAT;
                push(output, TARGET_ZONE, 3, 2.2 + peak);
                push(output, TARGET_ZONE, 11, 2.0 + transient);
                push(output, TARGET_FORGE, 3, 1.0 + transient * 1.2);
                push(output, TARGET_COLOR, 2, 1.3 + peak * 0.5);
                push(output, TARGET_MODE_PARAMETER, 4, 1.5 + transient);
            } else {
                push(output, TARGET_ZONE, 3, 0.4 + low * 0.6);
                push(output, TARGET_COLOR, 2, 0.55 + rms * 0.5);
            }
            push(output, TARGET_ZONE, 6, 0.8 + low * 1.5);
        }
        Profile::ColorStorm => {
            let storm = state.phase * (0.12 + peak * 0.35 + transient * 0.2);
            push(output, TARGET_COLOR, 0, storm.fract());
            push(output, TARGET_COLOR, 1, (0.4 + high).clamp(-2.0, 2.0));
            push(output, TARGET_COLOR, 2, 0.8 + peak * 1.1);
            push(output, TARGET_COLOR, 3, 0.35 + mid * 0.5);
            push(output, TARGET_COLOR, 4, 0.9 + high * 0.5);
            push(output, TARGET_MODE_PARAMETER, 22, -1.0 + peak * 2.5);
            push(output, TARGET_MODE_PARAMETER, 23, storm.fract());
        }
        Profile::DeckJuggler => {
            let xfade = 0.5 + (state.phase * (0.15 + low * 0.25)).sin() * 0.5;
            push(output, TARGET_PERFORMANCE, 0, xfade.clamp(0.0, 1.0));
            push(
                output,
                TARGET_PERFORMANCE,
                1,
                ((state.phase * 0.35 + high * 2.0) as u32 % 10) as f32,
            );
            push(output, TARGET_PERFORMANCE, 2, 0.25 + mid * 0.7);
            push(output, TARGET_PERFORMANCE, 3, if rms > 0.08 { 1.0 } else { 0.0 });
            if beat {
                output.event_flags |= if state.take_b {
                    EVENT_TAKE_A
                } else {
                    EVENT_TAKE_B
                };
                state.take_b = !state.take_b;
            }
        }
        Profile::AmbientOrbit => {
            let slow = state.phase * 0.05;
            push(output, TARGET_CAMERA, 0, slow.sin() * 1.8);
            push(output, TARGET_CAMERA, 1, (slow * 0.7).cos() * 0.35);
            push(output, TARGET_CAMERA, 2, 1.05 + (slow * 0.4).sin() * 0.2);
            push(output, TARGET_COLOR, 0, (slow * 0.15).fract());
            push(output, TARGET_COLOR, 2, 0.55 + rms * 0.6);
            push(output, TARGET_ZONE, 0, 0.5 + (slow * 1.1).sin() * 0.2);
            push(output, TARGET_ZONE, 1, 0.5 + (slow * 0.9).cos() * 0.2);
            push(output, TARGET_ZONE, 5, slow);
            push(output, TARGET_MODE_PARAMETER, 7, 0.25 + rms * 0.5);
            push(output, TARGET_MODE_PARAMETER, 9, 0.85);
        }
    }
    PLUGIN_OK
}

fn push(output: &mut PluginOutputV2, target: u32, index: u32, value: f32) {
    let position = output.command_count as usize;
    if position >= MAX_PLUGIN_COMMANDS {
        return;
    }
    output.commands[position] = PluginCommandV2 {
        target,
        index,
        operation: OP_SET,
        value,
    };
    output.command_count += 1;
}

unsafe extern "C" fn shutdown() {
    if let Ok(mut state) = STATE.lock() {
        state.initialized = false;
    }
}

static PLUGIN: PluginV2 = PluginV2 {
    struct_size: size_of::<PluginV2>() as u32,
    abi_version: ABI_VERSION_V2,
    initialize: Some(initialize),
    process: Some(process),
    shutdown: Some(shutdown),
};

#[unsafe(no_mangle)]
pub extern "C" fn thevisualizer_plugin_v2() -> *const PluginV2 {
    &PLUGIN
}

#[cfg(test)]
mod tests {
    use super::*;
    use thevisualizer_plugin_sdk::{ABI_VERSION, FeatureSnapshotV1};

    const ALL_PROFILES: &[&str] = &[
        "thevisualizer.beat-choreographer",
        "thevisualizer.spectral-colorist",
        "thevisualizer.zone-dancer",
        "thevisualizer.camera-pilot",
        "thevisualizer.particle-conductor",
        "thevisualizer.transition-dj",
        "thevisualizer.midi-performance-mapper",
        "thevisualizer.ambient-auto-director",
        "thevisualizer.analog-fractal-pilot",
        "thevisualizer.spectrum-sculptor",
        "thevisualizer.onset-architect",
        "thevisualizer.silence-gardener",
        "thevisualizer.macro-weaver",
        "thevisualizer.stereo-navigator",
        "thevisualizer.pulse-drummer",
        "thevisualizer.color-storm",
        "thevisualizer.deck-juggler",
        "thevisualizer.ambient-orbit",
    ];

    #[test]
    fn every_profile_initializes_and_emits_bounded_commands() {
        for id in ALL_PROFILES {
            let init = PluginInitV2 {
                struct_size: size_of::<PluginInitV2>() as u32,
                abi_version: ABI_VERSION_V2,
                profile_id: id.as_ptr(),
                profile_id_len: id.len() as u32,
            };
            let controls = [0.5; 18];
            let features = FeatureSnapshotV2 {
                struct_size: size_of::<FeatureSnapshotV2>() as u32,
                abi_version: ABI_VERSION_V2,
                base: FeatureSnapshotV1 {
                    struct_size: size_of::<FeatureSnapshotV1>() as u32,
                    abi_version: ABI_VERSION,
                    time_seconds: 1.0,
                    delta_seconds: 1.0 / 60.0,
                    waveform: std::ptr::null(),
                    waveform_len: 0,
                    spectrum: std::ptr::null(),
                    spectrum_len: 0,
                    rms: 0.4,
                    peak: 0.8,
                    low: 0.6,
                    mid: 0.3,
                    high: 0.2,
                },
                onset: 1.0,
                transient: 0.7,
                controls: controls.as_ptr(),
                controls_len: controls.len() as u32,
            };
            let mut output = PluginOutputV2::default();
            // SAFETY: This test owns all ABI values for the synchronous calls.
            unsafe {
                assert_eq!(initialize(&init), PLUGIN_OK);
                assert_eq!(process(&features, &mut output), PLUGIN_OK);
                assert!((1..=MAX_PLUGIN_COMMANDS as u32).contains(&output.command_count));
                assert!(
                    output.commands[..output.command_count as usize]
                        .iter()
                        .all(|command| command.value.is_finite())
                );
                shutdown();
            }
        }
    }
}
