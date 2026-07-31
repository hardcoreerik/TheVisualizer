use std::sync::atomic::{AtomicBool, Ordering};

use thevisualizer_plugin_sdk::{
    ABI_VERSION, FeatureSnapshotV1, PLUGIN_OK, PluginOutputV1, PluginV1,
};

static INITIALIZED: AtomicBool = AtomicBool::new(false);

unsafe extern "C" fn initialize() -> i32 {
    INITIALIZED.store(true, Ordering::Release);
    PLUGIN_OK
}

unsafe extern "C" fn process(
    features: *const FeatureSnapshotV1,
    output: *mut PluginOutputV1,
) -> i32 {
    if !INITIALIZED.load(Ordering::Acquire) || features.is_null() || output.is_null() {
        return 1;
    }
    // SAFETY: The host guarantees both pointers are non-null and valid for this call.
    let (features, output) = unsafe { (&*features, &mut *output) };
    if features.abi_version != ABI_VERSION
        || features.struct_size < size_of::<FeatureSnapshotV1>() as u32
        || output.abi_version != ABI_VERSION
        || output.struct_size < size_of::<PluginOutputV1>() as u32
    {
        return 2;
    }

    output.response_multiplier = (0.8 + features.low * 0.7 + features.high * 0.35).clamp(0.5, 1.8);
    PLUGIN_OK
}

unsafe extern "C" fn shutdown() {
    INITIALIZED.store(false, Ordering::Release);
}

static PLUGIN: PluginV1 = PluginV1 {
    struct_size: size_of::<PluginV1>() as u32,
    abi_version: ABI_VERSION,
    initialize: Some(initialize),
    process: Some(process),
    shutdown: Some(shutdown),
};

#[unsafe(no_mangle)]
pub extern "C" fn thevisualizer_plugin_v1() -> *const PluginV1 {
    &PLUGIN
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_is_bounded_and_requires_initialization() {
        let features = FeatureSnapshotV1 {
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
        };
        let mut output = PluginOutputV1::default();

        // SAFETY: This test owns both ABI values for each direct lifecycle call.
        unsafe {
            assert_ne!(process(&features, &mut output), PLUGIN_OK);
            assert_eq!(initialize(), PLUGIN_OK);
            assert_eq!(process(&features, &mut output), PLUGIN_OK);
            assert!((0.5..=1.8).contains(&output.response_multiplier));
            shutdown();
            assert_ne!(process(&features, &mut output), PLUGIN_OK);
        }
    }
}
