#![no_std]
//! TheVisualizer native-plugin ABI v1.
//!
//! Feature pointers are read-only and valid only for the synchronous `process` call. The host
//! retains audio, window, GPU, and frame ownership. Native plugins are explicitly trusted,
//! in-process code; this ABI does not sandbox them.

pub const ABI_VERSION: u32 = 1;
pub const ENTRY_POINT: &[u8] = b"thevisualizer_plugin_v1\0";
pub const PLUGIN_OK: i32 = 0;

#[repr(C)]
pub struct FeatureSnapshotV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub time_seconds: f32,
    pub delta_seconds: f32,
    pub waveform: *const f32,
    pub waveform_len: u32,
    pub spectrum: *const f32,
    pub spectrum_len: u32,
    pub rms: f32,
    pub peak: f32,
    pub low: f32,
    pub mid: f32,
    pub high: f32,
}

#[repr(C)]
pub struct PluginOutputV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub response_multiplier: f32,
}

impl Default for PluginOutputV1 {
    fn default() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: ABI_VERSION,
            response_multiplier: 1.0,
        }
    }
}

pub type InitializeFn = unsafe extern "C" fn() -> i32;
pub type ProcessFn = unsafe extern "C" fn(*const FeatureSnapshotV1, *mut PluginOutputV1) -> i32;
pub type ShutdownFn = unsafe extern "C" fn();

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PluginV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub initialize: Option<InitializeFn>,
    pub process: Option<ProcessFn>,
    pub shutdown: Option<ShutdownFn>,
}

pub type EntryPointFn = unsafe extern "C" fn() -> *const PluginV1;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_struct_sizes_are_stable() {
        assert_eq!(PluginOutputV1::default().struct_size, 12);
        assert_eq!(size_of::<PluginV1>(), 32);
        assert_eq!(size_of::<FeatureSnapshotV1>(), 64);
    }
}
