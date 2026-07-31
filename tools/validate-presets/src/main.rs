fn main() {
    let instance = eframe::wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&eframe::wgpu::RequestAdapterOptions {
        power_preference: eframe::wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    })).expect("adapter");
    let (device, _) = pollster::block_on(adapter.request_device(&eframe::wgpu::DeviceDescriptor::default())).unwrap();
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let path = root.join("presets/pixel-valley-flight.tvpreset");
    let src = std::fs::read_to_string(&path).unwrap();
    let shader = src.split("//! ---").nth(1).unwrap();
    let scope = device.push_error_scope(eframe::wgpu::ErrorFilter::Validation);
    let _ = device.create_shader_module(eframe::wgpu::ShaderModuleDescriptor {
        label: Some("valley"),
        source: eframe::wgpu::ShaderSource::Wgsl(shader.into()),
    });
    match pollster::block_on(scope.pop()) {
        Some(err) => println!("FAIL: {err}"),
        None => println!("OK"),
    }
}
