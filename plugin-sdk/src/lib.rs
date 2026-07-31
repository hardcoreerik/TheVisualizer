#![no_std]
//! TheVisualizer native-plugin ABI v1 and v2.
//!
//! Feature pointers are read-only and valid only for the synchronous `process` call. The host
//! retains audio, window, GPU, and frame ownership. Native plugins are explicitly trusted,
//! in-process code; these ABIs do not sandbox them. ABI v2 adds bounded host-owned modulation
//! commands without exposing GPU, window, capture, or frame-lifecycle handles.

pub const ABI_VERSION: u32 = 1;
pub const ABI_VERSION_V2: u32 = 2;
pub const ENTRY_POINT: &[u8] = b"thevisualizer_plugin_v1\0";
pub const ENTRY_POINT_V2: &[u8] = b"thevisualizer_plugin_v2\0";
pub const PLUGIN_OK: i32 = 0;
pub const MAX_PLUGIN_COMMANDS: usize = 24;
pub const MAX_PLUGIN_CONTROLS: usize = 18;

pub const TARGET_RESPONSE: u32 = 0;
pub const TARGET_MODE_PARAMETER: u32 = 1;
pub const TARGET_COLOR: u32 = 2;
pub const TARGET_CAMERA: u32 = 3;
pub const TARGET_ZONE: u32 = 4;
pub const TARGET_FORGE: u32 = 5;
pub const TARGET_PERFORMANCE: u32 = 6;
pub const TARGET_MACRO: u32 = 7;

pub const OP_SET: u32 = 0;
pub const OP_ADD: u32 = 1;
pub const OP_MULTIPLY: u32 = 2;

pub const EVENT_BEAT: u32 = 1 << 0;
pub const EVENT_TAKE_A: u32 = 1 << 1;
pub const EVENT_TAKE_B: u32 = 1 << 2;

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

#[repr(C)]
pub struct PluginInitV2 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub profile_id: *const u8,
    pub profile_id_len: u32,
}

#[repr(C)]
pub struct FeatureSnapshotV2 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub base: FeatureSnapshotV1,
    pub onset: f32,
    pub transient: f32,
    pub controls: *const f32,
    pub controls_len: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PluginCommandV2 {
    pub target: u32,
    pub index: u32,
    pub operation: u32,
    pub value: f32,
}

#[repr(C)]
pub struct PluginOutputV2 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub command_count: u32,
    pub event_flags: u32,
    pub commands: [PluginCommandV2; MAX_PLUGIN_COMMANDS],
}

impl Default for PluginOutputV2 {
    fn default() -> Self {
        Self {
            struct_size: size_of::<Self>() as u32,
            abi_version: ABI_VERSION_V2,
            command_count: 0,
            event_flags: 0,
            commands: [PluginCommandV2::default(); MAX_PLUGIN_COMMANDS],
        }
    }
}

pub type InitializeFnV2 = unsafe extern "C" fn(*const PluginInitV2) -> i32;
pub type ProcessFnV2 = unsafe extern "C" fn(*const FeatureSnapshotV2, *mut PluginOutputV2) -> i32;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct PluginV2 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub initialize: Option<InitializeFnV2>,
    pub process: Option<ProcessFnV2>,
    pub shutdown: Option<ShutdownFn>,
}

pub type EntryPointFnV2 = unsafe extern "C" fn() -> *const PluginV2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_struct_sizes_are_stable() {
        assert_eq!(PluginOutputV1::default().struct_size, 12);
        assert_eq!(size_of::<PluginV1>(), 32);
        assert_eq!(size_of::<FeatureSnapshotV1>(), 64);
        assert_eq!(size_of::<PluginCommandV2>(), 16);
        assert_eq!(size_of::<PluginOutputV2>(), 400);
        assert_eq!(size_of::<PluginV2>(), 32);
        assert_eq!(size_of::<FeatureSnapshotV2>(), 96);
    }
}
