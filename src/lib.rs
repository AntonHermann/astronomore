mod camera;
mod celestial_body;
mod grid;
mod loader;
mod mesh;
mod planets;
mod scene;
mod shader_loader;
mod sim;
mod texture;
mod ui;

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

use crate::grid::{ColorVertex, DrawGrid, GridMesh};
use crate::mesh::{DrawMesh, Vertex};
use crate::{
    camera::{Camera, CameraRig},
    celestial_body::{CelestialBody, DrawCelestialBody, DrawCelestialBodyNormals},
};

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
    grid_pipeline_layout: wgpu::PipelineLayout,
    grid_pipeline: wgpu::RenderPipeline,
    grid_xz: GridMesh,
    grid_xy: GridMesh,
    grid_yz: GridMesh,
    normals_pipeline_layout: wgpu::PipelineLayout,
    normals_pipeline: wgpu::RenderPipeline,
    view: ui::ViewOptions,
    meshes: Vec<mesh::Mesh>,
    diffuse_texture: texture::Texture,
    // diffuse_bind_group: wgpu::BindGroup,
    identity_model_bind_group: wgpu::BindGroup,
    scene: scene::Scene,
    sim: sim::SimState,
    camera_rig: CameraRig,
    depth_texture: texture::Texture,
    ui: ui::EguiLayer,
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

fn build_normals_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    surface_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Normals Pipeline"),
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
            adapter = %adapter_info.name,
            device_type = ?adapter_info.device_type,
            "Using adapter"
        );
        tracing::info!(backend = ?adapter_info.backend, "GPU backend selected");
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

        // ================= Depth Texture setup =================
        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        // ==================== Scene setup =====================
        let mut scene = scene::Scene::new(&device);
        let mut body_ids: Vec<scene::BodyId> = Vec::with_capacity(planets::BODIES.len());
        for def in planets::BODIES {
            let bytes = loader::load_bytes(def.texture_path).await?;
            let texture = texture::Texture::from_bytes(
                &device,
                &queue,
                &bytes,
                def.name,
                &texture_bind_group_layout,
            )?;
            let parent_id = def.parent.map(|p| body_ids[p as usize]);
            let id = scene.add_celestial_body(
                CelestialBody::new(
                    &device,
                    def.name,
                    def.distance_from_parent,
                    def.radius,
                    def.angular_velocity,
                    texture,
                    &scene.model_bind_group_layout,
                ),
                parent_id,
            );
            body_ids.push(id);
        }
        let sun_id = body_ids[planets::SolarSystemBody::Sun as usize];

        // ======= Camera setup =======
        // let initial_camera =
        //     Camera::new_fps((0.0, 8.0, 25.0), -90f32.to_radians(), -15f32.to_radians());
        let initial_camera =
            Camera::new_orbit(sun_id, 30.0, 0f32.to_radians(), 30f32.to_radians());
        let camera_rig = CameraRig::new(&device, size.width, size.height, initial_camera, &scene);

        // ================= Render Pipeline =================

        let shader_src = loader::load_str("src/shaders/shader.wgsl").await?;
        shader_loader::validate_wgsl("shader.wgsl", &shader_src)?;
        let shader = shader_loader::make_shader_module(&device, "shader.wgsl", &shader_src);
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&texture_bind_group_layout),
                    Some(&camera_rig.bind_group_layout),
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
            bind_group_layouts: &[Some(&camera_rig.bind_group_layout)],
            immediate_size: 0,
        });
        let grid_pipeline =
            build_grid_pipeline(&device, &grid_pipeline_layout, &grid_shader, config.format);

        let grid_xz = GridMesh::xz_plane(&device, 10);
        let grid_xy = GridMesh::xy_plane(&device, 10);
        let grid_yz = GridMesh::yz_plane(&device, 10);

        // ================= Normals Pipeline =================
        let normals_src = loader::load_str("src/shaders/normals.wgsl").await?;
        shader_loader::validate_wgsl("normals.wgsl", &normals_src)?;
        let normals_shader =
            shader_loader::make_shader_module(&device, "normals.wgsl", &normals_src);
        let normals_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Normals Pipeline Layout"),
                bind_group_layouts: &[
                    Some(&camera_rig.bind_group_layout),
                    Some(&scene.model_bind_group_layout),
                ],
                immediate_size: 0,
            });
        let normals_pipeline = build_normals_pipeline(
            &device,
            &normals_pipeline_layout,
            &normals_shader,
            config.format,
        );

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
        let ui_layer = ui::EguiLayer::new(&device, &window, surface_format);

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
            grid_pipeline_layout,
            grid_pipeline,
            grid_xz,
            grid_xy,
            grid_yz,
            normals_pipeline_layout,
            normals_pipeline,
            view: ui::ViewOptions::new(),
            meshes,
            identity_model_bind_group,
            diffuse_texture,
            scene,
            sim: sim::SimState::new(),
            camera_rig,
            depth_texture,
            ui: ui_layer,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
            self.depth_texture =
                texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
            self.camera_rig.resize(width, height);
            self.is_surface_configured = true;
        }
    }

    /// Update application state before rendering
    pub fn update(&mut self) {
        let now = web_time::Instant::now();
        let dt = now - self.last_update;
        self.last_frame_duration = dt;
        self.last_update = now;

        self.sim.advance(dt);
        self.scene.update(self.sim.time, &self.queue);
        self.camera_rig.update(dt, &self.scene, &self.queue);
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

        let frame_view = output
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
                    view: &frame_view,
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

            let pipeline = if self.view.wireframe {
                &self.wireframe_pipeline
            } else {
                &self.render_pipeline
            };
            render_pass.set_pipeline(pipeline);
            render_pass.set_bind_group(0, &self.diffuse_texture.bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_rig.bind_group, &[]);
            render_pass.set_bind_group(2, &self.identity_model_bind_group, &[]);

            for mesh in &self.meshes {
                render_pass.draw_mesh(mesh);
            }

            // TODO: move this logic into the scene and/or celestial body
            for planet in &self.scene.celestial_bodies {
                render_pass.draw_celestial_body(planet, &self.camera_rig.bind_group);
            }

            if self.view.show_normals {
                render_pass.set_pipeline(&self.normals_pipeline);
                for planet in &self.scene.celestial_bodies {
                    render_pass.draw_body_normals(planet, &self.camera_rig.bind_group);
                }
            }

            render_pass.set_pipeline(&self.grid_pipeline);
            render_pass.set_bind_group(0, &self.camera_rig.bind_group, &[]);
            if self.view.show_grid_xz {
                render_pass.draw_grid(&self.grid_xz);
            }
            if self.view.show_grid_xy {
                render_pass.draw_grid(&self.grid_xy);
            }
            if self.view.show_grid_yz {
                render_pass.draw_grid(&self.grid_yz);
            }
        }

        // ==================== egui pass ====================
        let fps = if self.last_frame_duration.as_secs_f64() > 0.0 {
            1.0 / self.last_frame_duration.as_secs_f64()
        } else {
            0.0
        };
        let sim = &mut self.sim;
        let view = &mut self.view;
        let mut cam_speed = self.camera_rig.controller.speed;
        let mut cam_sensitivity = self.camera_rig.controller.sensitivity;
        let mut reset_camera = false;
        let cam_pos = match &self.camera_rig.camera {
            Camera::Fps(camera) => camera.position,
            Camera::Orbit(camera) => camera.target_and_camera_pos(&self.scene).1,
        };
        let cam_is_fps = matches!(&self.camera_rig.camera, Camera::Fps(_));
        let body_list: Vec<(scene::BodyId, String)> = self
            .scene
            .iter_bodies()
            .map(|(id, n)| (id, n.to_string()))
            .collect();
        let default_target = body_list.first().expect("scene has at least one body").0;
        let mut selected_is_fps = cam_is_fps;
        // Defaults for whichever mode is not currently active, so the UI shows
        // coherent values immediately when the user toggles the mode.
        let mut fps_pos_x = 0.0f32;
        let mut fps_pos_y = 8.0f32;
        let mut fps_pos_z = 25.0f32;
        let mut fps_yaw_deg = -90.0f32;
        let mut fps_pitch_deg = -20.0f32;
        let mut orbit_dist = 25.0f32;
        let mut orbit_yaw_deg = 0.0f32;
        let mut orbit_pitch_deg = 0.0f32;
        let mut selected_target = default_target;
        match &self.camera_rig.camera {
            Camera::Fps(c) => {
                fps_pos_x = c.position.x;
                fps_pos_y = c.position.y;
                fps_pos_z = c.position.z;
                fps_yaw_deg = c.yaw_rad.to_degrees();
                fps_pitch_deg = c.pitch_rad.to_degrees();
            }
            Camera::Orbit(c) => {
                orbit_dist = c.dist;
                orbit_yaw_deg = c.yaw_rad.to_degrees();
                orbit_pitch_deg = c.pitch_rad.to_degrees();
                selected_target = c.target;
            }
        }

        let raw_input = self.ui.state.take_egui_input(&self.window);
        self.ui.ctx.begin_pass(raw_input);
        egui::Window::new("Simulation")
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-8.0, 8.0))
            .show(&self.ui.ctx, |ui| {
                ui.label(format!("FPS: {:.0}", fps));
                ui.separator();
                ui.label(format!(
                    "Zeit-Faktor: {}x",
                    if sim.multiplier.fract() == 0.0 {
                        format!("{}", sim.multiplier as i32)
                    } else {
                        format!("{:.2}", sim.multiplier)
                    }
                ));
                ui.horizontal(|ui| {
                    if ui
                        .button("◀◀")
                        .on_hover_text("Halbieren (PageDown)")
                        .clicked()
                    {
                        sim.halve_speed();
                    }
                    let pause_label = if sim.is_paused { "▶" } else { "⏸" };
                    if ui.button(pause_label).on_hover_text("Pause (P)").clicked() {
                        sim.toggle_pause();
                    }
                    if ui
                        .button("▶▶")
                        .on_hover_text("Verdoppeln (PageUp)")
                        .clicked()
                    {
                        sim.double_speed();
                    }
                    if ui.button("1×").on_hover_text("Zurücksetzen (0)").clicked() {
                        sim.reset_speed();
                    }
                });
                ui.separator();
                let wireframe_label = if view.wireframe {
                    "Wireframe: an"
                } else {
                    "Wireframe: aus"
                };
                if ui
                    .button(wireframe_label)
                    .on_hover_text("Umschalten (Tab)")
                    .clicked()
                {
                    view.toggle_wireframe();
                }
                let normals_label = if view.show_normals {
                    "Normalen: an"
                } else {
                    "Normalen: aus"
                };
                if ui
                    .button(normals_label)
                    .on_hover_text("Umschalten (N)")
                    .clicked()
                {
                    view.toggle_normals();
                }
                ui.separator();
                egui::CollapsingHeader::new("Gitternetz")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label("G = alle umschalten");
                        ui.checkbox(&mut view.show_grid_xz, "XZ-Ebene (Boden)");
                        ui.checkbox(&mut view.show_grid_xy, "XY-Ebene");
                        ui.checkbox(&mut view.show_grid_yz, "YZ-Ebene");
                    });
                ui.separator();
                egui::CollapsingHeader::new("Kamera")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Modus:");
                            ui.radio_value(&mut selected_is_fps, true, "FPS");
                            ui.radio_value(&mut selected_is_fps, false, "Orbit");
                        });
                        ui.separator();
                        egui::Grid::new("cam_params")
                            .num_columns(2)
                            .striped(true)
                            .show(ui, |ui| {
                                if selected_is_fps {
                                    ui.label("X:");
                                    ui.add(egui::DragValue::new(&mut fps_pos_x).speed(0.1));
                                    ui.end_row();
                                    ui.label("Y:");
                                    ui.add(egui::DragValue::new(&mut fps_pos_y).speed(0.1));
                                    ui.end_row();
                                    ui.label("Z:");
                                    ui.add(egui::DragValue::new(&mut fps_pos_z).speed(0.1));
                                    ui.end_row();
                                    ui.label("Yaw:");
                                    ui.add(
                                        egui::DragValue::new(&mut fps_yaw_deg)
                                            .suffix("°")
                                            .speed(0.5),
                                    );
                                    ui.end_row();
                                    ui.label("Pitch:");
                                    ui.add(
                                        egui::DragValue::new(&mut fps_pitch_deg)
                                            .suffix("°")
                                            .speed(0.5)
                                            .range(-89.9..=89.9f32),
                                    );
                                    ui.end_row();
                                } else {
                                    ui.label("Ziel:");
                                    let selected_text = body_list
                                        .iter()
                                        .find(|(id, _)| *id == selected_target)
                                        .map(|(_, n)| n.as_str())
                                        .unwrap_or("");
                                    egui::ComboBox::from_id_salt("orbit_target")
                                        .selected_text(selected_text)
                                        .show_ui(ui, |ui| {
                                            for (id, name) in &body_list {
                                                ui.selectable_value(
                                                    &mut selected_target,
                                                    *id,
                                                    name,
                                                );
                                            }
                                        });
                                    ui.end_row();
                                    ui.label("Abstand:");
                                    ui.add(
                                        egui::DragValue::new(&mut orbit_dist)
                                            .speed(0.1)
                                            .range(0.1..=f32::MAX),
                                    );
                                    ui.end_row();
                                    ui.label("Yaw:");
                                    ui.add(
                                        egui::DragValue::new(&mut orbit_yaw_deg)
                                            .suffix("°")
                                            .speed(0.5),
                                    );
                                    ui.end_row();
                                    ui.label("Pitch:");
                                    ui.add(
                                        egui::DragValue::new(&mut orbit_pitch_deg)
                                            .suffix("°")
                                            .speed(0.5)
                                            .range(-89.9..=89.9f32),
                                    );
                                    ui.end_row();
                                    ui.label("Position:");
                                    ui.label(format!(
                                        "({:.1}, {:.1}, {:.1})",
                                        cam_pos.x, cam_pos.y, cam_pos.z
                                    ));
                                    ui.end_row();
                                }
                            });
                        ui.separator();
                        ui.add(
                            egui::Slider::new(&mut cam_speed, 0.05..=4.0).text("Geschwindigkeit"),
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
        let full_output = self.ui.ctx.end_pass();

        self.camera_rig.controller.speed = cam_speed;
        self.camera_rig.controller.sensitivity = cam_sensitivity;
        let mode_switched = selected_is_fps != cam_is_fps;
        if mode_switched || reset_camera {
            self.camera_rig.camera = if selected_is_fps {
                Camera::new_fps(
                    glam::Vec3::new(0.0, 8.0, 25.0),
                    -90f32.to_radians(),
                    -20f32.to_radians(),
                )
            } else {
                Camera::new_orbit(selected_target, 25.0, 0.0, 0.0)
            };
        } else {
            match &mut self.camera_rig.camera {
                Camera::Fps(c) => {
                    c.position = glam::Vec3::new(fps_pos_x, fps_pos_y, fps_pos_z);
                    c.yaw_rad = fps_yaw_deg.to_radians();
                    c.pitch_rad = fps_pitch_deg.to_radians();
                }
                Camera::Orbit(c) => {
                    c.target = selected_target;
                    c.dist = orbit_dist;
                    c.yaw_rad = orbit_yaw_deg.to_radians();
                    c.pitch_rad = orbit_pitch_deg.to_radians();
                }
            }
        }

        self.ui
            .state
            .handle_platform_output(&self.window, full_output.platform_output);
        let clipped_primitives = self
            .ui
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.config.width, self.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.ui
                .renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }
        self.ui.renderer.update_buffers(
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
                        view: &frame_view,
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
            self.ui
                .renderer
                .render(&mut egui_pass, &clipped_primitives, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.ui.renderer.free_texture(id);
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
            self.view.toggle_wireframe();
            tracing::info!("Wireframe mode: {}", self.view.wireframe);
        } else if code == KeyCode::KeyN && state.is_pressed() {
            self.view.toggle_normals();
            tracing::info!("Normalen-Visualisierung: {}", self.view.show_normals);
        } else if code == KeyCode::PageUp && state.is_pressed() {
            self.sim.double_speed();
        } else if code == KeyCode::PageDown && state.is_pressed() {
            self.sim.halve_speed();
        } else if code == KeyCode::Digit0 && state.is_pressed() {
            self.sim.reset_speed();
        } else if code == KeyCode::KeyP && state.is_pressed() {
            self.sim.toggle_pause();
        } else if code == KeyCode::KeyG && state.is_pressed() {
            self.view.toggle_all_grids();
            tracing::info!("Grids: {}", self.view.any_grid_visible());
        } else {
            self.camera_rig.controller.handle_key(code, state);
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
            Err(e) => tracing::error!(error = ?e, "shader.wgsl validation failed"),
        }
    }

    /// Reloads `normals.wgsl` from disk and recreates the normals render pipeline.
    ///
    /// On validation error the miette diagnostic is printed to stderr and the
    /// existing pipeline is left unchanged.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_reload_normals_shader(&mut self) {
        let src = match std::fs::read_to_string("src/shaders/normals.wgsl") {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("normals.wgsl lesen fehlgeschlagen: {e}");
                return;
            }
        };
        match shader_loader::validate_wgsl("normals.wgsl", &src) {
            Ok(()) => {
                let module = shader_loader::make_shader_module(&self.device, "normals.wgsl", &src);
                self.normals_pipeline = build_normals_pipeline(
                    &self.device,
                    &self.normals_pipeline_layout,
                    &module,
                    self.config.format,
                );
                tracing::info!("normals.wgsl neu geladen");
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
            Err(e) => tracing::error!(error = ?e, "grid.wgsl validation failed"),
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

        let window = match event_loop.create_window(window_attributes) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                tracing::error!(error = %e, "Failed to create window");
                event_loop.exit();
                return;
            }
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let state = match pollster::block_on(State::new(window)) {
                Ok(s) => s,
                Err(e) => {
                    tracing::error!(error = %e, "Failed to initialize GPU state");
                    event_loop.exit();
                    return;
                }
            };
            self.state = Some(state);

            let (tx, rx) = std::sync::mpsc::channel();
            match notify_debouncer_mini::new_debouncer(
                std::time::Duration::from_millis(150),
                move |res: notify_debouncer_mini::DebounceEventResult| {
                    if let Ok(events) = res {
                        for event in events {
                            if event.kind == notify_debouncer_mini::DebouncedEventKind::Any {
                                // channel closed when shader_rx is dropped — ignore
                                let _ = tx.send(event.path);
                            }
                        }
                    }
                },
            ) {
                Ok(mut debouncer) => {
                    match debouncer.watcher().watch(
                        std::path::Path::new("src/shaders"),
                        notify_debouncer_mini::notify::RecursiveMode::NonRecursive,
                    ) {
                        Ok(()) => {
                            self._debouncer = Some(debouncer);
                            self.shader_rx = Some(rx);
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "Cannot watch src/shaders - hot-reload disabled");
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Cannot start shader watcher - hot-reload disabled");
                }
            }
        }

        #[cfg(target_arch = "wasm32")]
        {
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    match State::new(window).await {
                        Ok(state) => {
                            if proxy.send_event(state).is_err() {
                                tracing::error!("Failed to send initialized state to event loop");
                            }
                        }
                        Err(e) => {
                            tracing::error!(error = %e, "Failed to initialize GPU state (WASM)")
                        }
                    }
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
            && state.camera_rig.mouse_pressed
        {
            let (mouse_dx, mouse_dy) = delta;
            state.camera_rig.controller.handle_mouse(mouse_dx, mouse_dy);
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
            .ui
            .state
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
                        tracing::error!(
                            error = ?e,
                            width = state.config.width,
                            height = state.config.height,
                            "Render failed; exiting"
                        );
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
                state.camera_rig.controller.handle_mouse_scroll(&delta)
            }
            WindowEvent::MouseInput {
                state: mouse_state,
                button: MouseButton::Left,
                ..
            } if !egui_consumed => state.camera_rig.mouse_pressed = mouse_state.is_pressed(),
            _ => {}
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(rx) = &self.shader_rx else { return };
        let Some(state) = &mut self.state else { return };

        let mut reload_main = false;
        let mut reload_grid = false;
        let mut reload_normals = false;
        while let Ok(path) = rx.try_recv() {
            match path.file_name().and_then(|n| n.to_str()) {
                Some("shader.wgsl") => reload_main = true,
                Some("grid.wgsl") => reload_grid = true,
                Some("normals.wgsl") => reload_normals = true,
                _ => {}
            }
        }
        if reload_main {
            state.try_reload_main_shader();
        }
        if reload_grid {
            state.try_reload_grid_shader();
        }
        if reload_normals {
            state.try_reload_normals_shader();
        }
        if reload_main || reload_grid || reload_normals {
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
