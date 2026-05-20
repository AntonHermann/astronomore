mod camera;
mod celestial_body;
mod grid;
mod loader;
mod mesh;
mod scene;
mod shader_loader;
mod texture;

use std::sync::Arc;

use miette::IntoDiagnostic;
use wgpu::util::DeviceExt;
use winit::{
    application::ApplicationHandler,
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::Window,
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")]
use winit::platform::web::EventLoopExtWebSys;

use crate::celestial_body::{CelestialBody, DrawCelestialBody};
use crate::grid::{ColorVertex, DrawGrid, GridMesh};
use crate::mesh::{DrawMesh, Vertex};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}
impl CameraUniform {
    fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }
    fn update_view_proj(&mut self, camera: &camera::Camera, projection: &camera::Projection) {
        self.view_proj = (projection.calc_matrix() * camera.calc_matrix()).to_cols_array_2d();
    }
}

pub struct State {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    window: Arc<Window>,
    last_update: web_time::Instant,
    last_frame_duration: web_time::Duration,
    render_pipeline_layout: wgpu::PipelineLayout,
    render_pipeline: wgpu::RenderPipeline,
    wireframe_pipeline: wgpu::RenderPipeline,
    wireframe: bool,
    grid_pipeline_layout: wgpu::PipelineLayout,
    grid_pipeline: wgpu::RenderPipeline,
    grid_xz: GridMesh,
    grid_xy: GridMesh,
    grid_yz: GridMesh,
    show_grid_xz: bool,
    show_grid_xy: bool,
    show_grid_yz: bool,
    meshes: Vec<mesh::Mesh>,
    diffuse_texture: texture::Texture,
    // diffuse_bind_group: wgpu::BindGroup,
    identity_model_bind_group: wgpu::BindGroup,
    scene: scene::Scene,
    sim_time: f64,
    sim_time_multiplier: f64,
    is_paused: bool,
    camera: camera::Camera,
    projection: camera::Projection,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: camera::FpsCameraController,
    mouse_pressed: bool,
    depth_texture: texture::Texture,
    egui_ctx: egui::Context,
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

fn build_main_pipelines(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    surface_format: wgpu::TextureFormat,
) -> (wgpu::RenderPipeline, wgpu::RenderPipeline) {
    let make = |polygon_mode: wgpu::PolygonMode, label: &str, fs_entry: &'static str| {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(label),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some(fs_entry),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: texture::Texture::DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview_mask: None,
            cache: None,
        })
    };

    let fill = make(wgpu::PolygonMode::Fill, "Fill Pipeline", "fs_main");
    #[cfg(not(target_arch = "wasm32"))]
    let wire = make(
        wgpu::PolygonMode::Line,
        "Wireframe Pipeline",
        "fs_wireframe",
    );
    #[cfg(target_arch = "wasm32")]
    let wire = make(
        wgpu::PolygonMode::Fill,
        "Wireframe Pipeline",
        "fs_wireframe",
    );
    (fill, wire)
}

fn build_grid_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Grid Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            buffers: &[ColorVertex::desc()],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::LineList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: texture::Texture::DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview_mask: None,
        cache: None,
    })
}

impl State {
    pub async fn new(window: Arc<Window>) -> miette::Result<Self> {
        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            flags: Default::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: Default::default(),
            display: None,
        });

        // part of the window that we draw to
        let surface = instance.create_surface(window.clone()).into_diagnostic()?;

        // handle to the actual graphics card, locked to specific backend
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .into_diagnostic()?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: None,
                required_features: if cfg!(target_arch = "wasm32") {
                    wgpu::Features::empty()
                } else {
                    wgpu::Features::POLYGON_MODE_LINE
                },
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                memory_hints: Default::default(),
                trace: wgpu::Trace::Off,
            })
            .await
            .into_diagnostic()?;

        let adapter_info = adapter.get_info();
        tracing::info!(
            "Using adapter: {} ({:?})",
            adapter_info.name,
            adapter_info.device_type
        );
        tracing::info!("Backend: {}", adapter_info.backend);
        tracing::trace!("Adapter features: {:#?}", adapter.features());
        tracing::debug!("Device features: {:#?}", device.features());
        tracing::trace!("Device limits: {:#?}", device.limits());

        let surface_caps = surface.get_capabilities(&adapter);
        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });
        let diffuse_bytes = loader::load_bytes("assets/textures/dbg.png").await?;
        let diffuse_texture = texture::Texture::from_bytes(
            &device,
            &queue,
            &diffuse_bytes,
            "dbg.png",
            &texture_bind_group_layout,
        )?;
        let sun_texture = texture::Texture::from_bytes(
            &device,
            &queue,
            &diffuse_bytes,
            "sun",
            &texture_bind_group_layout,
        )?;
        let earth_bytes = loader::load_bytes("assets/textures/2k_earth_daymap.jpg").await?;
        let earth_texture = texture::Texture::from_bytes(
            &device,
            &queue,
            &earth_bytes,
            "earth.jpg",
            &texture_bind_group_layout,
        )?;
        let moon_bytes = loader::load_bytes("assets/textures/2k_moon.jpg").await?;
        let moon_texture = texture::Texture::from_bytes(
            &device,
            &queue,
            &moon_bytes,
            "moon.jpg",
            &texture_bind_group_layout,
        )?;

        // ======= Camera setup =======

        let camera =
            camera::Camera::new_fps((0.0, 8.0, 25.0), -90f32.to_radians(), -15f32.to_radians());
        let projection =
            camera::Projection::new(size.width, size.height, 45.0f32.to_radians(), 0.1, 100.0);

        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(&camera, &projection);

        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[camera_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX, // only the vertex shader needs to see this
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("camera_bind_group_layout"),
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("Camera Bind Group"),
        });

        let camera_controller = camera::FpsCameraController::new(4.0, 2.0);

        // ================= Depth Texture setup =================
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        // ==================== Scene setup =====================
        let mut scene = scene::Scene::new(&device);
        let sun_id = scene.add_celestial_body(
            CelestialBody::new(
                &device,
                "Sun",
                0.,
                4.,
                0.0,
                sun_texture,
                &scene.model_bind_group_layout,
            ),
            None,
        );
        let earth_id = scene.add_celestial_body(
            CelestialBody::new(
                &device,
                "Earth",
                10.,
                1.,
                0.2,
                earth_texture,
                &scene.model_bind_group_layout,
            ),
            Some(sun_id),
        );
        scene.add_celestial_body(
            CelestialBody::new(
                &device,
                "Moon",
                3.,
                0.27,
                0.5,
                moon_texture,
                &scene.model_bind_group_layout,
            ),
            Some(earth_id),
        );

        // ================= Render Pipeline =================

        let shader_src = loader::load_str("src/shaders/shader.wgsl").await?;
        shader_loader::validate_wgsl("shader.wgsl", &shader_src)?;
        let shader = shader_loader::make_shader_module(&device, "shader.wgsl", &shader_src);
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&texture_bind_group_layout),
                    Some(&camera_bind_group_layout),
                    Some(&scene.model_bind_group_layout),
                ],
                immediate_size: 0,
            });

        let (render_pipeline, wireframe_pipeline) =
            build_main_pipelines(&device, &render_pipeline_layout, &shader, config.format);

        // ================= Grid Pipeline =================
        let grid_src = loader::load_str("src/shaders/grid.wgsl").await?;
        shader_loader::validate_wgsl("grid.wgsl", &grid_src)?;
        let grid_shader = shader_loader::make_shader_module(&device, "grid.wgsl", &grid_src);
        let grid_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Grid Pipeline Layout"),
            bind_group_layouts: &[Some(&camera_bind_group_layout)],
            immediate_size: 0,
        });
        let grid_pipeline =
            build_grid_pipeline(&device, &grid_pipeline_layout, &grid_shader, config.format);

        let grid_xz = GridMesh::xz_plane(&device, 10);
        let grid_xy = GridMesh::xy_plane(&device, 10);
        let grid_yz = GridMesh::yz_plane(&device, 10);

        let identity_model_uniform = celestial_body::ModelUniform::default();
        let identity_model_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Identity Model Buffer"),
            contents: bytemuck::cast_slice(&[identity_model_uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let identity_model_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &scene.model_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: identity_model_buffer.as_entire_binding(),
            }],
            label: Some("Identity Model Bind Group"),
        });
        let meshes = vec![
            // mesh::Mesh::default_sphere(&device),
            // mesh::Mesh::x_plane(&device),
            // mesh::Mesh::y_plane(&device),
            // mesh::Mesh::z_plane(&device),
        ];

        // ==================== egui setup ====================
        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            None,
            None,
            Some(device.limits().max_texture_dimension_2d as usize),
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            &device,
            surface_format,
            egui_wgpu::RendererOptions::default(),
        );

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            last_update: web_time::Instant::now(),
            last_frame_duration: web_time::Duration::ZERO,
            render_pipeline_layout,
            render_pipeline,
            wireframe_pipeline,
            wireframe: false,
            grid_pipeline_layout,
            grid_pipeline,
            grid_xz,
            grid_xy,
            grid_yz,
            show_grid_xz: true,
            show_grid_xy: false,
            show_grid_yz: false,
            meshes,
            identity_model_bind_group,
            diffuse_texture,
            scene,
            sim_time: 0.0f64,
            sim_time_multiplier: 1.0f64,
            is_paused: false,
            camera,
            projection,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            mouse_pressed: false,
            depth_texture,
            egui_ctx,
            egui_state,
            egui_renderer,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
            self.projection.resize(width, height);
            self.is_surface_configured = true;
        }
    }

    /// Update application state before rendering
    pub fn update(&mut self) {
        let now = web_time::Instant::now();
        let dt = now - self.last_update;
        self.last_frame_duration = dt;
        self.last_update = now;

        if !self.is_paused {
            self.sim_time += dt.as_secs_f64() * self.sim_time_multiplier;
        }
        self.scene.update(self.sim_time, &self.queue);

        // Alternatives for updating the camera:
        // 1. We can create a separate buffer and copy its contents to our camera_buffer
        //    The new buffer is known as a staging buffer.
        //    This method is usually how it's done as it allows the contents of the main buffer
        //    (in this case, camera_buffer) to be accessible only by the GPU.
        //    The GPU can do some speed optimizations, which it couldn't if we could access the buffer via the CPU.
        // 2. We can call one of the mapping methods map_read_async, and map_write_async on the buffer itself.
        //    These allow us to access a buffer's contents directly but require us to deal with the
        //    async aspect of these methods. This also requires our buffer to use the
        //    BufferUsages::MAP_READ and/or BufferUsages::MAP_WRITE.
        //    We won't talk about it here, but check out the Wgpu without a window tutorial if you want to know more.
        // 3. We can use write_buffer on queue.
        // --> we chose 3. src: https://sotrh.github.io/learn-wgpu/beginner/tutorial6-uniforms/#demo
        let camera::Camera::Fps(camera) = &mut self.camera;
        self.camera_controller.update_camera(camera, dt);
        self.camera_uniform
            .update_view_proj(&self.camera, &self.projection);
        self.queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(&[self.camera_uniform]),
        );
    }

    pub fn render(&mut self) -> miette::Result<()> {
        self.window.request_redraw();

        if !self.is_surface_configured {
            return Ok(());
        }

        let output = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.surface.configure(&self.device, &self.config);
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.surface.configure(&self.device, &self.config);
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                // You could recreate the devices and all resources
                // created with it here, but we'll just bail
                miette::bail!("Lost device");
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });

            let pipeline = if self.wireframe {
                &self.wireframe_pipeline
            } else {
                &self.render_pipeline
            };
            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &self.diffuse_texture.bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
            render_pass.set_bind_group(2, &self.identity_model_bind_group, &[]);

            for mesh in &self.meshes {
                render_pass.draw_mesh(mesh);
            }

            // TODO: move this logic into the scene and/or celestial body
            for planet in &self.scene.celestial_bodies {
                render_pass.draw_celestial_body(planet, &self.camera_bind_group);
            }

            render_pass.set_pipeline(&self.grid_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            if self.show_grid_xz {
                render_pass.draw_grid(&self.grid_xz);
            }
            if self.show_grid_xy {
                render_pass.draw_grid(&self.grid_xy);
            }
            if self.show_grid_yz {
                render_pass.draw_grid(&self.grid_yz);
            }
        }

        // ==================== egui pass ====================
        let fps = if self.last_frame_duration.as_secs_f64() > 0.0 {
            1.0 / self.last_frame_duration.as_secs_f64()
        } else {
            0.0
        };
        let sim_time_multiplier = self.sim_time_multiplier;
        let is_paused = self.is_paused;
        let wireframe = self.wireframe;
        let mut toggle_pause = false;
        let mut toggle_wireframe = false;
        let mut new_multiplier: Option<f64> = None;
        let show_grid_xz = self.show_grid_xz;
        let show_grid_xy = self.show_grid_xy;
        let show_grid_yz = self.show_grid_yz;
        let mut new_grid_xz: Option<bool> = None;
        let mut new_grid_xy: Option<bool> = None;
        let mut new_grid_yz: Option<bool> = None;
        let mut cam_speed = self.camera_controller.speed;
        let mut cam_sensitivity = self.camera_controller.sensitivity;
        let mut reset_camera = false;
        let camera::Camera::Fps(cam_ref) = &self.camera;
        let cam_pos = cam_ref.position;

        let raw_input = self.egui_state.take_egui_input(&self.window);
        self.egui_ctx.begin_pass(raw_input);
        egui::Window::new("Simulation")
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-8.0, 8.0))
            .show(&self.egui_ctx, |ui| {
                ui.label(format!("FPS: {:.0}", fps));
                ui.separator();
                ui.label(format!(
                    "Zeit-Faktor: {}x",
                    if sim_time_multiplier.fract() == 0.0 {
                        format!("{}", sim_time_multiplier as i32)
                    } else {
                        format!("{:.2}", sim_time_multiplier)
                    }
                ));
                ui.horizontal(|ui| {
                    if ui
                        .button("◀◀")
                        .on_hover_text("Halbieren (PageDown)")
                        .clicked()
                    {
                        new_multiplier = Some(sim_time_multiplier / 2.0);
                    }
                    let pause_label = if is_paused { "▶" } else { "⏸" };
                    if ui.button(pause_label).on_hover_text("Pause (P)").clicked() {
                        toggle_pause = true;
                    }
                    if ui
                        .button("▶▶")
                        .on_hover_text("Verdoppeln (PageUp)")
                        .clicked()
                    {
                        new_multiplier = Some(sim_time_multiplier * 2.0);
                    }
                    if ui.button("1×").on_hover_text("Zurücksetzen (0)").clicked() {
                        new_multiplier = Some(1.0);
                    }
                });
                ui.separator();
                let wireframe_label = if wireframe {
                    "Wireframe: an"
                } else {
                    "Wireframe: aus"
                };
                if ui
                    .button(wireframe_label)
                    .on_hover_text("Umschalten (Tab)")
                    .clicked()
                {
                    toggle_wireframe = true;
                }
                ui.separator();
                egui::CollapsingHeader::new("Gitternetz")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label("G = alle umschalten");
                        let mut xz = show_grid_xz;
                        if ui.checkbox(&mut xz, "XZ-Ebene (Boden)").changed() {
                            new_grid_xz = Some(xz);
                        }
                        let mut xy = show_grid_xy;
                        if ui.checkbox(&mut xy, "XY-Ebene").changed() {
                            new_grid_xy = Some(xy);
                        }
                        let mut yz = show_grid_yz;
                        if ui.checkbox(&mut yz, "YZ-Ebene").changed() {
                            new_grid_yz = Some(yz);
                        }
                    });
                ui.separator();
                egui::CollapsingHeader::new("Kamera")
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.label(format!(
                            "Position: ({:.1}, {:.1}, {:.1})",
                            cam_pos.x, cam_pos.y, cam_pos.z
                        ));
                        ui.add(
                            egui::Slider::new(&mut cam_speed, 0.5..=30.0).text("Geschwindigkeit"),
                        );
                        ui.add(
                            egui::Slider::new(&mut cam_sensitivity, 0.1..=5.0)
                                .text("Empfindlichkeit"),
                        );
                        if ui.button("Zurücksetzen").clicked() {
                            reset_camera = true;
                        }
                    });
            });
        let full_output = self.egui_ctx.end_pass();

        if toggle_pause {
            self.is_paused = !self.is_paused;
        }
        if toggle_wireframe {
            self.wireframe = !self.wireframe;
        }
        if let Some(v) = new_grid_xz {
            self.show_grid_xz = v;
        }
        if let Some(v) = new_grid_xy {
            self.show_grid_xy = v;
        }
        if let Some(v) = new_grid_yz {
            self.show_grid_yz = v;
        }
        if let Some(m) = new_multiplier {
            self.sim_time_multiplier = m;
        }
        self.camera_controller.speed = cam_speed;
        self.camera_controller.sensitivity = cam_sensitivity;
        if reset_camera {
            let camera::Camera::Fps(cam) = &mut self.camera;
            cam.position = glam::Vec3::new(0.0, 8.0, 25.0);
            cam.yaw_rad = -90f32.to_radians();
            cam.pitch_rad = -20f32.to_radians();
        }

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);
        let clipped_primitives = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }
        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &clipped_primitives,
            &screen_descriptor,
        );

        {
            let mut egui_pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("egui Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                })
                .forget_lifetime();
            self.egui_renderer
                .render(&mut egui_pass, &clipped_primitives, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, state: ElementState) {
        if code == KeyCode::Escape && state.is_pressed() {
            event_loop.exit();
        } else if code == KeyCode::Tab && state.is_pressed() {
            self.wireframe = !self.wireframe;
            tracing::info!("Wireframe mode: {}", self.wireframe);
        } else if code == KeyCode::PageUp && state.is_pressed() {
            self.sim_time_multiplier *= 2.0;
            tracing::info!("Sim time mult: {}x", self.sim_time_multiplier);
        } else if code == KeyCode::PageDown && state.is_pressed() {
            self.sim_time_multiplier /= 2.0;
            tracing::info!("Sim time mult: {}x", self.sim_time_multiplier);
        } else if code == KeyCode::Digit0 && state.is_pressed() {
            self.sim_time_multiplier = 1.0;
            tracing::info!("Sim time mult reset: {}x", self.sim_time_multiplier);
        } else if code == KeyCode::KeyP && state.is_pressed() {
            self.is_paused = !self.is_paused;
            tracing::info!("Simulation paused: {}", self.is_paused);
        } else if code == KeyCode::KeyG && state.is_pressed() {
            let any = self.show_grid_xz || self.show_grid_xy || self.show_grid_yz;
            self.show_grid_xz = !any;
            self.show_grid_xy = !any;
            self.show_grid_yz = !any;
            tracing::info!("Grids: {}", !any);
        } else {
            self.camera_controller.handle_key(code, state);
        }
    }

    /// Reloads `shader.wgsl` from disk and recreates the main render pipelines.
    ///
    /// On validation error the miette diagnostic is printed to stderr and the
    /// existing pipelines are left unchanged.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_reload_main_shader(&mut self) {
        let src = match std::fs::read_to_string("src/shaders/shader.wgsl") {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("shader.wgsl lesen fehlgeschlagen: {e}");
                return;
            }
        };
        match shader_loader::validate_wgsl("shader.wgsl", &src) {
            Ok(()) => {
                let module = shader_loader::make_shader_module(&self.device, "shader.wgsl", &src);
                (self.render_pipeline, self.wireframe_pipeline) = build_main_pipelines(
                    &self.device,
                    &self.render_pipeline_layout,
                    &module,
                    self.config.format,
                );
                tracing::info!("shader.wgsl neu geladen");
            }
            Err(e) => eprintln!("{e:?}"),
        }
    }

    /// Reloads `grid.wgsl` from disk and recreates the grid render pipeline.
    ///
    /// On validation error the miette diagnostic is printed to stderr and the
    /// existing pipeline is left unchanged.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_reload_grid_shader(&mut self) {
        let src = match std::fs::read_to_string("src/shaders/grid.wgsl") {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("grid.wgsl lesen fehlgeschlagen: {e}");
                return;
            }
        };
        match shader_loader::validate_wgsl("grid.wgsl", &src) {
            Ok(()) => {
                let module = shader_loader::make_shader_module(&self.device, "grid.wgsl", &src);
                self.grid_pipeline = build_grid_pipeline(
                    &self.device,
                    &self.grid_pipeline_layout,
                    &module,
                    self.config.format,
                );
                tracing::info!("grid.wgsl neu geladen");
            }
            Err(e) => eprintln!("{e:?}"),
        }
    }
}

pub struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
    #[cfg(not(target_arch = "wasm32"))]
    shader_rx: Option<std::sync::mpsc::Receiver<std::path::PathBuf>>,
    #[cfg(not(target_arch = "wasm32"))]
    _debouncer:
        Option<notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>>,
}

impl App {
    pub fn new(#[cfg(target_arch = "wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch = "wasm32")]
        let proxy = Some(event_loop.create_proxy());
        Self {
            state: None,
            #[cfg(target_arch = "wasm32")]
            proxy,
            #[cfg(not(target_arch = "wasm32"))]
            shader_rx: None,
            #[cfg(not(target_arch = "wasm32"))]
            _debouncer: None,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationHandler<State> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        #[allow(unused_mut)]
        let mut window_attributes = Window::default_attributes();

        #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        {
            self.state = Some(pollster::block_on(State::new(window)).unwrap());

            let (tx, rx) = std::sync::mpsc::channel();
            let mut debouncer = notify_debouncer_mini::new_debouncer(
                std::time::Duration::from_millis(150),
                move |res: notify_debouncer_mini::DebounceEventResult| {
                    if let Ok(events) = res {
                        for event in events {
                            if event.kind == notify_debouncer_mini::DebouncedEventKind::Any {
                                let _ = tx.send(event.path);
                            }
                        }
                    }
                },
            )
            .expect("Shader-Watcher konnte nicht gestartet werden");
            debouncer
                .watcher()
                .watch(
                    std::path::Path::new("src/shaders"),
                    notify_debouncer_mini::notify::RecursiveMode::NonRecursive,
                )
                .expect("src/shaders kann nicht beobachtet werden");
            self._debouncer = Some(debouncer);
            self.shader_rx = Some(rx);
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(
                        proxy
                            .send_event(
                                State::new(window)
                                    .await
                                    .expect("Unable to create canvas!!!")
                            )
                            .is_ok()
                    )
                });
            }
        }
    }

    #[allow(unused_mut)]
    fn user_event(&mut self, _event_loop: &ActiveEventLoop, mut event: State) {
        #[cfg(target_arch = "wasm32")]
        {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size().width,
                event.window.inner_size().height,
            );
        }
        self.state = Some(event);
    }

    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let DeviceEvent::MouseMotion { delta } = event
            && let Some(state) = &mut self.state
            && state.mouse_pressed
        {
            let (mouse_dx, mouse_dy) = delta;
            state.camera_controller.handle_mouse(mouse_dx, mouse_dy);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let state = match &mut self.state {
            Some(s) => s,
            None => return,
        };

        let egui_consumed = state
            .egui_state
            .on_window_event(&state.window, &event)
            .consumed;

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => state.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                state.update();
                match state.render() {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!("Render error: {e:?}");
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key: PhysicalKey::Code(code),
                        state: key_state,
                        ..
                    },
                ..
            } if !egui_consumed => state.handle_key(event_loop, code, key_state),
            WindowEvent::MouseWheel { delta, .. } if !egui_consumed => {
                state.camera_controller.handle_mouse_scroll(&delta)
            }
            WindowEvent::MouseInput {
                state: mouse_state,
                button: MouseButton::Left,
                ..
            } if !egui_consumed => state.mouse_pressed = mouse_state.is_pressed(),
            _ => {}
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(rx) = &self.shader_rx else { return };
        let Some(state) = &mut self.state else { return };

        let mut reload_main = false;
        let mut reload_grid = false;
        while let Ok(path) = rx.try_recv() {
            match path.file_name().and_then(|n| n.to_str()) {
                Some("shader.wgsl") => reload_main = true,
                Some("grid.wgsl") => reload_grid = true,
                _ => {}
            }
        }
        if reload_main {
            state.try_reload_main_shader();
        }
        if reload_grid {
            state.try_reload_grid_shader();
        }
        if reload_main || reload_grid {
            state.window.request_redraw();
        }
    }
}

pub fn run() -> miette::Result<()> {
    let event_loop = EventLoop::with_user_event().build().into_diagnostic()?;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut app = App::new();
        event_loop.run_app(&mut app).into_diagnostic()?;
    }
    #[cfg(target_arch = "wasm32")]
    {
        let app = App::new(&event_loop);
        event_loop.spawn_app(app);
    }

    Ok(())
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(start)]
pub fn run_web() -> Result<(), wasm_bindgen::JsValue> {
    console_error_panic_hook::set_once();
    wasm_tracing::set_as_global_default();
    run().unwrap_throw();
    Ok(())
}
