//! Headless performance benchmark.
//!
//! Outputs a single JSON object to stdout. Run from the project root:
//!   cargo build --release --bin bench && ./target/release/bench
//!
//! Benchmarks:
//!   A. shader_validation_us — naga parse+validate on both WGSL shaders (CPU only)
//!   B. sphere_128_64_us     — Mesh::sphere(128, 64) creation (CPU tess + GPU upload)
//!   C. scene_update_us      — Scene::update() per frame on a 2-body scene

use astronomore::{
    celestial_body::CelestialBody,
    mesh::Mesh,
    scene::Scene,
    shader_loader::validate_wgsl,
    texture::Texture,
};
use image::{DynamicImage, Rgba, RgbaImage};
use web_time::Instant;

const N_SHADER: u32 = 100;
const N_SPHERE: u32 = 50;
const N_SCENE: u32 = 1000;

fn main() {
    pollster::block_on(run());
}

async fn run() {
    let shader_us = bench_shader_validation();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::PRIMARY,
        flags: Default::default(),
        memory_budget_thresholds: Default::default(),
        backend_options: Default::default(),
        display: None,
    });

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::None,
            compatible_surface: None,
            force_fallback_adapter: false,
        })
        .await;

    let adapter = match adapter {
        Ok(a) => a,
        Err(e) => {
            eprintln!("[perf] Warning: no GPU adapter ({e}), GPU benchmarks skipped");
            println!(
                r#"{{"shader_validation_us":{shader_us},"n_shader":{N_SHADER},"gpu_skipped":true}}"#
            );
            return;
        }
    };

    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            required_limits: wgpu::Limits::default(),
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off,
        })
        .await
        .expect("failed to create wgpu device");

    let sphere_us = bench_sphere(&device);
    let scene_us = bench_scene_update(&device, &queue);

    println!(
        r#"{{"shader_validation_us":{shader_us},"sphere_128_64_us":{sphere_us},"scene_update_us":{scene_us},"n_shader":{N_SHADER},"n_sphere":{N_SPHERE},"n_scene":{N_SCENE},"gpu_skipped":false}}"#
    );
}

fn bench_shader_validation() -> u64 {
    let shader_src = std::fs::read_to_string("src/shaders/shader.wgsl")
        .expect("shader.wgsl not found — run bench from project root");
    let grid_src = std::fs::read_to_string("src/shaders/grid.wgsl")
        .expect("grid.wgsl not found — run bench from project root");

    let start = Instant::now();
    for _ in 0..N_SHADER {
        validate_wgsl("shader.wgsl", &shader_src).expect("shader validation failed");
        validate_wgsl("grid.wgsl", &grid_src).expect("grid shader validation failed");
    }
    start.elapsed().as_micros() as u64 / N_SHADER as u64
}

fn bench_sphere(device: &wgpu::Device) -> u64 {
    let start = Instant::now();
    for _ in 0..N_SPHERE {
        std::hint::black_box(Mesh::sphere(device, 128, 64));
    }
    start.elapsed().as_micros() as u64 / N_SPHERE as u64
}

fn bench_scene_update(device: &wgpu::Device, queue: &wgpu::Queue) -> u64 {
    // Replicate the group 0 texture bind group layout from lib.rs
    let texture_bgl = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("bench_texture_bgl"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });

    let mut scene = Scene::new(device);

    let body1 = {
        let layout = &scene.model_bind_group_layout;
        let tex = make_dummy_texture(device, queue, &texture_bgl);
        CelestialBody::new(device, "root", 0.0, 1.0, 0.0, tex, layout)
    };
    let root_id = scene.add_celestial_body(body1, None);

    let body2 = {
        let layout = &scene.model_bind_group_layout;
        let tex = make_dummy_texture(device, queue, &texture_bgl);
        CelestialBody::new(device, "child", 10.0, 0.5, 1.0, tex, layout)
    };
    scene.add_celestial_body(body2, Some(root_id));

    let mut sim_time = 0.0f64;
    let start = Instant::now();
    for _ in 0..N_SCENE {
        scene.update(sim_time, queue);
        sim_time += 0.016;
    }
    start.elapsed().as_micros() as u64 / N_SCENE as u64
}

fn make_dummy_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    bgl: &wgpu::BindGroupLayout,
) -> Texture {
    let img = DynamicImage::ImageRgba8(RgbaImage::from_pixel(1, 1, Rgba([255, 0, 0, 255])));
    Texture::from_image(device, queue, &img, "bench_dummy", bgl)
        .expect("failed to create dummy texture")
}
