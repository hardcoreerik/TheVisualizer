use std::sync::Mutex;

use thevisualizer_plugin_sdk::{
    ABI_VERSION_V2, EVENT_BEAT, EVENT_TAKE_A, EVENT_TAKE_B, FeatureSnapshotV2, MAX_PLUGIN_COMMANDS,
    OP_SET, PLUGIN_OK, PluginCommandV2, PluginInitV2, PluginOutputV2, PluginV2, TARGET_CAMERA,
    TARGET_COLOR, TARGET_FORGE, TARGET_MACRO, TARGET_PERFORMANCE, TARGET_RESPONSE, TARGET_ZONE,
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
}

struct State {
    profile: Profile,
    phase: f32,
    onset_high: bool,
    take_b: bool,
    initialized: bool,
}

static STATE: Mutex<State> = Mutex::new(State {
    profile: Profile::Beat,
    phase: 0.0,
    onset_high: false,
    take_b: false,
    initialized: false,
});

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
    let profile = if id.contains("spectral-colorist") {
        Profile::Color
    } else if id.contains("zone-dancer") {
        Profile::Zone
    } else if id.contains("camera-pilot") {
        Profile::Camera
    } else if id.contains("particle-conductor") {
        Profile::Particle
    } else if id.contains("transition-dj") {
        Profile::Transition
    } else if id.contains("midi-performance-mapper") {
        Profile::Midi
    } else if id.contains("ambient-auto-director") {
        Profile::Director
    } else if id.contains("beat-choreographer") {
        Profile::Beat
    } else {
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
    if beat {
        output.event_flags |= EVENT_BEAT;
    }

    match state.profile {
        Profile::Beat => {
            push(output, TARGET_RESPONSE, 0, 0.9 + features.base.low * 0.9);
            push(output, TARGET_COLOR, 2, 0.7 + features.transient * 1.3);
            push(output, TARGET_FORGE, 3, 0.35 + features.base.low * 1.4);
        }
        Profile::Color => {
            let hue = (state.phase * (0.025 + features.base.high * 0.06)).fract();
            push(output, TARGET_COLOR, 0, hue);
            push(
                output,
                TARGET_COLOR,
                1,
                (features.base.mid - features.base.high) * 1.4,
            );
            push(output, TARGET_COLOR, 4, 0.65 + features.base.peak * 0.75);
        }
        Profile::Zone => {
            let orbit = state.phase * (0.45 + features.base.mid);
            push(output, TARGET_ZONE, 0, 0.5 + orbit.sin() * 0.28);
            push(output, TARGET_ZONE, 1, 0.5 + (orbit * 0.73).cos() * 0.24);
            push(output, TARGET_ZONE, 3, 0.8 + features.base.low * 1.8);
            push(
                output,
                TARGET_ZONE,
                4,
                ((features.base.low * 3.0 + features.base.high * 6.0) as u32 % 10) as f32,
            );
            push(output, TARGET_ZONE, 5, orbit);
        }
        Profile::Camera => {
            push(output, TARGET_CAMERA, 0, (state.phase * 0.18).sin() * 2.4);
            push(
                output,
                TARGET_CAMERA,
                1,
                (state.phase * 0.11).cos() * (0.18 + features.base.mid * 0.28),
            );
            push(output, TARGET_CAMERA, 2, 1.0 + features.base.low * 0.5);
        }
        Profile::Particle => {
            push(output, TARGET_FORGE, 0, (features.base.mid - 0.25) * 1.4);
            push(output, TARGET_FORGE, 1, 0.35 + features.base.low);
            push(output, TARGET_FORGE, 2, (features.base.high - 0.2) * 1.5);
            push(output, TARGET_FORGE, 3, 0.3 + features.transient * 1.7);
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
                0.5 + (state.phase * (0.18 + features.base.low * 0.2)).sin() * 0.5,
            );
            push(
                output,
                TARGET_PERFORMANCE,
                1,
                ((state.phase * 0.2) as u32 % 10) as f32,
            );
            push(
                output,
                TARGET_PERFORMANCE,
                2,
                0.3 + features.base.high * 0.7,
            );
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
            push(output, TARGET_COLOR, 2, 0.65 + features.base.rms * 0.9);
            push(output, TARGET_CAMERA, 0, slow.sin() * 1.2);
            push(output, TARGET_CAMERA, 2, 1.05 + slow.cos() * 0.18);
            push(output, TARGET_ZONE, 0, 0.5 + (slow * 1.7).sin() * 0.22);
            push(output, TARGET_ZONE, 1, 0.5 + (slow * 1.3).cos() * 0.18);
            push(output, TARGET_ZONE, 7, slow.sin());
            push(output, TARGET_FORGE, 3, 0.4 + features.transient);
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

    #[test]
    fn every_profile_initializes_and_emits_bounded_commands() {
        for id in [
            "thevisualizer.beat-choreographer",
            "thevisualizer.spectral-colorist",
            "thevisualizer.zone-dancer",
            "thevisualizer.camera-pilot",
            "thevisualizer.particle-conductor",
            "thevisualizer.transition-dj",
            "thevisualizer.midi-performance-mapper",
            "thevisualizer.ambient-auto-director",
        ] {
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
