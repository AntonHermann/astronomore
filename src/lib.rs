mod arrow;
mod camera;
mod celestial_body;
mod gpu;
mod grid;
mod loader;
mod mesh;
mod orbital;
mod pipelines;
mod planets;
mod scene;
mod scene_properties;
mod shader_loader;
mod sim;
mod texture;
mod ui;

pub use celestial_body::CelestialBody;
pub use mesh::Mesh;
pub use orbital::OrbitalModel;
pub use scene::{BodyId, Scene};
pub use shader_loader::validate_wgsl;
pub use texture::Texture;

use std::sync::Arc;

use glam::{Vec3, Vec4};
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

use crate::{
    arrow::{Arrow, ArrowMesh, DrawArrows},
    camera::{Camera, CameraRig},
    celestial_body::DrawCelestialBodyNormals,
    gpu::GpuContext,
    grid::{DrawGrid, GridMesh},
    mesh::DrawMesh,
    pipelines::Pipelines,
    planets::SolarSystemBody,
    scene::DrawScene,
    scene_properties::SceneProperties,
    sim::SimState,
    ui::{EguiLayer, ViewOptions},
};

pub struct State {
    gpu: GpuContext,
    last_update: web_time::Instant,
    last_frame_duration: web_time::Duration,
    pipelines: Pipelines,
    grid_xz: GridMesh,
    grid_xy: GridMesh,
    grid_yz: GridMesh,
    arrow_mesh: ArrowMesh,
    line_brightness_buffer: wgpu::Buffer,
    line_brightness_bind_group: wgpu::BindGroup,
    view: ViewOptions,
    meshes: Vec<Mesh>,
    diffuse_texture: Texture,
    // diffuse_bind_group: wgpu::BindGroup,
    identity_model_bind_group: wgpu::BindGroup,
    scene: Scene,
    scene_properties: SceneProperties,
    sim: SimState,
    camera_rig: CameraRig,
    ui: EguiLayer,
}

impl State {
    pub async fn new(window: Arc<Window>) -> miette::Result<Self> {
        let gpu = GpuContext::new(window).await?;
        let device = &gpu.device;
        let queue = &gpu.queue;
        let config = &gpu.config;
        let surface_format = config.format;
        let size = winit::dpi::PhysicalSize::new(config.width, config.height);

        let texture_bind_group_layout = Texture::bind_group_layout(device);
        let diffuse_bytes = loader::load_bytes("assets/textures/dbg.png").await?;
        let diffuse_texture = Texture::from_bytes(
            device,
            queue,
            &diffuse_bytes,
            "dbg.png",
            &texture_bind_group_layout,
        )?;

        // ==================== Scene setup =====================
        let mut scene = Scene::new(device);
        let mut body_ids: Vec<BodyId> = Vec::with_capacity(planets::BODIES.len());
        for def in planets::BODIES {
            let bytes = loader::load_bytes(def.texture_path).await?;
            let texture =
                Texture::from_bytes(device, queue, &bytes, def.name, &texture_bind_group_layout)?;
            let parent_id = def.parent.map(|p| body_ids[p as usize]);
            let id = scene.add_celestial_body(
                CelestialBody::new(
                    device,
                    def.name,
                    def.radius,
                    def.orbital_model,
                    texture,
                    &scene.model_bind_group_layout,
                ),
                parent_id,
            );
            body_ids.push(id);
        }
        let sun_id = body_ids[SolarSystemBody::Sun as usize];

        // ======= Camera setup =======
        // let initial_camera =
        //     Camera::new_fps((0.0, 8.0, 25.0), -90f32.to_radians(), -15f32.to_radians());
        let initial_camera = Camera::new_orbit(sun_id, 30.0, 0f32.to_radians(), 30f32.to_radians());
        let camera_rig = CameraRig::new(device, size.width, size.height, initial_camera, &scene);

        // ================= Scene properties =================
        let scene_properties = SceneProperties::new(device);

        // ================= Pipelines =================
        let pipelines = Pipelines::new(
            device,
            surface_format,
            &texture_bind_group_layout,
            &camera_rig.bind_group_layout,
            &scene.model_bind_group_layout,
            &scene_properties.bind_group_layout,
        )
        .await?;

        let grid_xz = GridMesh::xz_plane(device, 10);
        let grid_xy = GridMesh::xy_plane(device, 10);
        let grid_yz = GridMesh::yz_plane(device, 10);
        let arrow_mesh = ArrowMesh::new(device, 600);

        let line_brightness_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line Brightness Buffer"),
            contents: bytemuck::cast_slice(&[1.0f32]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let line_brightness_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &pipelines.brightness_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: line_brightness_buffer.as_entire_binding(),
            }],
            label: Some("Line Brightness Bind Group"),
        });

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
            // Mesh::default_sphere(&device),
            // Mesh::x_plane(&device),
            // Mesh::y_plane(&device),
            // Mesh::z_plane(&device),
        ];

        // ==================== egui setup ====================
        let ui_layer = ui::EguiLayer::new(device, &gpu.window, surface_format);

        Ok(Self {
            gpu,
            last_update: web_time::Instant::now(),
            last_frame_duration: web_time::Duration::ZERO,
            pipelines,
            grid_xz,
            grid_xy,
            grid_yz,
            arrow_mesh,
            line_brightness_buffer,
            line_brightness_bind_group,
            view: ui::ViewOptions::new(),
            meshes,
            identity_model_bind_group,
            diffuse_texture,
            scene,
            scene_properties,
            sim: SimState::new(),
            camera_rig,
            ui: ui_layer,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu.resize(width, height);
        self.camera_rig.resize(width, height);
    }

    /// Update application state before rendering
    pub fn update(&mut self) {
        let now = web_time::Instant::now();
        let dt = now - self.last_update;
        self.last_frame_duration = dt;
        self.last_update = now;

        self.sim.advance(dt);
        self.scene.update(self.sim.time, &self.gpu.queue);
        self.camera_rig.update(dt, &self.scene, &self.gpu.queue);
        self.scene_properties.update(&self.gpu.queue);
        if self.view.show_arrows {
            let arrows = self.build_arrows();
            self.arrow_mesh.update(&self.gpu.queue, &arrows);
        }
    }

    /// Build the list of debug arrows for the current frame.
    fn build_arrows(&self) -> Vec<Arrow> {
        const VEL_LEN: f32 = 3.0;
        const RAD_LEN: f32 = 3.0;
        const SPIN_LEN: f32 = 1.5;
        const VEL_COLOR: [f32; 4] = [0.0, 0.8, 1.0, 1.0];
        const RAD_COLOR: [f32; 4] = [1.0, 0.5, 0.0, 1.0];
        const SPIN_COLOR: [f32; 4] = [0.2, 1.0, 0.4, 1.0];

        let sim_time = self.sim.time;
        let mut arrows = Vec::new();

        for body in &self.scene.celestial_bodies {
            let origin = body.world_position;

            if self.view.arrows_velocity {
                let vel = orbital::orbital_velocity(body.orbital_parameters.model, sim_time);
                if vel.length_squared() > 1e-14 {
                    let dir = vel.normalize();
                    arrows.push(Arrow {
                        start: origin,
                        end: origin + dir * VEL_LEN,
                        color: VEL_COLOR,
                    });
                }
            }

            if self.view.arrows_radial {
                let parent_pos = match body.orbital_parameters.parent_id {
                    Some(pid) => self.scene.celestial_bodies[pid].world_position,
                    None => Vec3::ZERO,
                };
                let to_parent = parent_pos - origin;
                if to_parent.length_squared() > 1e-4 {
                    let dir = to_parent.normalize();
                    arrows.push(Arrow {
                        start: origin,
                        end: origin + dir * RAD_LEN,
                        color: RAD_COLOR,
                    });
                }
            }

            if self.view.arrows_spin {
                let spin_axis = body.spin_transform.transform_vector3(Vec3::Y).normalize();
                arrows.push(Arrow {
                    start: origin,
                    end: origin + spin_axis * SPIN_LEN,
                    color: SPIN_COLOR,
                });
            }
        }
        arrows
    }

    pub fn render(&mut self) -> miette::Result<()> {
        self.gpu.window.request_redraw();

        if !self.gpu.is_surface_configured {
            return Ok(());
        }

        let output = match self.gpu.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => {
                self.gpu
                    .surface
                    .configure(&self.gpu.device, &self.gpu.config);
                surface_texture
            }
            wgpu::CurrentSurfaceTexture::Timeout
            | wgpu::CurrentSurfaceTexture::Occluded
            | wgpu::CurrentSurfaceTexture::Validation => {
                // Skip this frame
                return Ok(());
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                self.gpu
                    .surface
                    .configure(&self.gpu.device, &self.gpu.config);
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
            .gpu
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
                            r: self.view.background_color[0] as f64,
                            g: self.view.background_color[1] as f64,
                            b: self.view.background_color[2] as f64,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.gpu.depth_texture.view,
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
            let _span = tracing::debug_span!("render pass");
            let _guard = _span.enter();

            let pipeline = if self.view.wireframe {
                &self.pipelines.wireframe
            } else {
                &self.pipelines.fill
            };
            tracing::trace!(
                mode = if self.view.wireframe {
                    "wireframe"
                } else {
                    "fill"
                },
                "set pipeline"
            );
            render_pass.set_pipeline(pipeline);
            tracing::trace!(group = 0, "set bind group: diffuse texture");
            render_pass.set_bind_group(0, &self.diffuse_texture.bind_group, &[]);
            tracing::trace!(group = 1, "set bind group: camera");
            render_pass.set_bind_group(1, &self.camera_rig.bind_group, &[]);
            tracing::trace!(group = 2, "set bind group: identity model");
            render_pass.set_bind_group(2, &self.identity_model_bind_group, &[]);
            tracing::trace!(group = 3, "set bind group: scene properties");
            render_pass.set_bind_group(3, &self.scene_properties.bind_group, &[]);

            tracing::trace!(count = self.meshes.len(), "draw meshes");
            for mesh in &self.meshes {
                render_pass.draw_mesh(mesh);
            }

            tracing::trace!("draw scene");
            render_pass.draw_scene(
                &self.scene,
                &self.camera_rig.bind_group,
                &self.scene_properties.bind_group,
            );

            if self.view.show_normals {
                tracing::trace!("set pipeline: normals");
                render_pass.set_pipeline(&self.pipelines.normals);
                tracing::trace!(
                    count = self.scene.celestial_bodies.len(),
                    "draw body normals"
                );
                for planet in &self.scene.celestial_bodies {
                    tracing::trace!(name = planet.name, "draw normals");
                    render_pass.draw_body_normals(planet, &self.camera_rig.bind_group);
                }
            }

            self.gpu.queue.write_buffer(
                &self.line_brightness_buffer,
                0,
                bytemuck::cast_slice(&[self.view.line_brightness]),
            );

            if self.view.show_arrows {
                tracing::trace!("set pipeline: grid (arrows)");
                render_pass.set_pipeline(&self.pipelines.grid);
                render_pass.set_bind_group(0, &self.camera_rig.bind_group, &[]);
                render_pass.set_bind_group(1, &self.line_brightness_bind_group, &[]);
                render_pass.draw_arrows(&self.arrow_mesh);
            }

            tracing::trace!("set pipeline: grid");
            render_pass.set_pipeline(&self.pipelines.grid);
            tracing::trace!(group = 0, "set bind group: camera (grid)");
            render_pass.set_bind_group(0, &self.camera_rig.bind_group, &[]);
            render_pass.set_bind_group(1, &self.line_brightness_bind_group, &[]);
            if self.view.show_grid_xz {
                tracing::trace!("draw grid: XZ");
                render_pass.draw_grid(&self.grid_xz);
            }
            if self.view.show_grid_xy {
                tracing::trace!("draw grid: XY");
                render_pass.draw_grid(&self.grid_xy);
            }
            if self.view.show_grid_yz {
                tracing::trace!("draw grid: YZ");
                render_pass.draw_grid(&self.grid_yz);
            }
        }

        // ==================== egui pass ====================
        let fps = if self.last_frame_duration.as_secs_f64() > 0.0 {
            1.0 / self.last_frame_duration.as_secs_f64()
        } else {
            0.0
        };
        let prev_meridians = self.view.sphere_meridians;
        let prev_parallels = self.view.sphere_parallels;
        let ui_scale = self.view.ui_scale;
        let sim = &mut self.sim;
        let view = &mut self.view;
        let mut cam_speed = self.camera_rig.controller.speed;
        let mut cam_sensitivity = self.camera_rig.controller.sensitivity;
        let mut cam_zoom_sensitivity = self.camera_rig.controller.zoom_sensitivity;
        let mut reset_camera = false;
        let cam_pos = match &self.camera_rig.camera {
            Camera::Fps(camera) => camera.position,
            Camera::Orbit(camera) => camera.position(
                self.camera_rig
                    .camera
                    .orbit_target(&self.scene)
                    .expect("orbit camera has target"),
            ),
        };
        let cam_is_fps = matches!(&self.camera_rig.camera, Camera::Fps(_));
        let body_list: Vec<(BodyId, String)> = self
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

        let mut props = self.scene_properties.uniform;

        let raw_input = self.ui.state.take_egui_input(&self.gpu.window);
        self.ui.ctx.set_pixels_per_point(ui_scale);
        self.ui.ctx.begin_pass(raw_input);
        egui::Window::new("Simulation")
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-8.0, 8.0))
            .show(&self.ui.ctx, |ui| {
                ui.horizontal(|ui| {
                    let wireframe_label = if view.wireframe {
                        "Wireframe: on"
                    } else {
                        "Wireframe: off"
                    };
                    if ui
                        .button(wireframe_label)
                        .on_hover_text("Toggle (Tab)")
                        .clicked()
                    {
                        view.toggle_wireframe();
                    }
                    let normals_label = if view.show_normals {
                        "Normals: on"
                    } else {
                        "Normals: off"
                    };
                    if ui
                        .button(normals_label)
                        .on_hover_text("Toggle (N)")
                        .clicked()
                    {
                        view.toggle_normals();
                    }
                    let names_label = if view.show_body_names {
                        "Labels: on"
                    } else {
                        "Labels: off"
                    };
                    if ui.button(names_label).on_hover_text("Toggle (L)").clicked() {
                        view.toggle_body_names();
                    }
                    let arrows_label = if view.show_arrows {
                        "Arrows: on"
                    } else {
                        "Arrows: off"
                    };
                    if ui
                        .button(arrows_label)
                        .on_hover_text("Toggle (V)")
                        .clicked()
                    {
                        view.toggle_arrows();
                    }
                    let offscreen_label = if view.show_offscreen_indicators {
                        "OffScreen: on"
                    } else {
                        "OffScreen: off"
                    };
                    if ui
                        .button(offscreen_label)
                        .on_hover_text("Toggle (I)")
                        .clicked()
                    {
                        view.toggle_offscreen_indicators();
                    }
                });
                ui.separator();
                egui::CollapsingHeader::new("Grid")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label("G = toggle all");
                        ui.checkbox(&mut view.show_grid_xz, "XZ plane (ground)");
                        ui.checkbox(&mut view.show_grid_xy, "XY plane");
                        ui.checkbox(&mut view.show_grid_yz, "YZ plane");
                    });
                ui.separator();
                egui::CollapsingHeader::new("Arrows")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.label("V = toggle all");
                        ui.checkbox(&mut view.arrows_velocity, "Velocity (cyan)");
                        ui.checkbox(&mut view.arrows_radial, "Radial (orange)");
                        ui.checkbox(&mut view.arrows_spin, "Spin axis (green)");
                    });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Line brightness:");
                    ui.add(
                        egui::Slider::new(&mut view.line_brightness, 0.0..=2.0).fixed_decimals(2),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("UI scale:")
                        .on_hover_text("[ = smaller, ] = bigger");
                    ui.add(egui::Slider::new(&mut view.ui_scale, 0.5..=4.0).fixed_decimals(2));
                });
                ui.horizontal(|ui| {
                    ui.label("Label size:");
                    ui.add(
                        egui::Slider::new(&mut view.label_font_size, 6.0..=48.0).fixed_decimals(1),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Offscreen label size:");
                    ui.add(
                        egui::Slider::new(&mut view.offscreen_label_font_size, 6.0..=48.0)
                            .fixed_decimals(1),
                    );
                });
                ui.separator();
                egui::CollapsingHeader::new("Scene Properties")
                    .default_open(true)
                    .show(ui, |ui| {
                        egui::Grid::new("scene_props")
                            .num_columns(2)
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label("Ambient:");
                                ui.add(
                                    egui::Slider::new(&mut props.ambient_strength, 0.0..=1.0)
                                        .fixed_decimals(2),
                                );
                                ui.end_row();
                                ui.label("Diffuse:");
                                ui.add(
                                    egui::Slider::new(&mut props.diffuse_factor, 0.0..=1.0)
                                        .fixed_decimals(2),
                                );
                                ui.end_row();
                                ui.label("Specular:");
                                ui.add(
                                    egui::Slider::new(&mut props.specular_intensity, 0.0..=1.0)
                                        .fixed_decimals(2),
                                );
                                ui.end_row();
                                ui.label("Shininess:");
                                ui.add(
                                    egui::Slider::new(&mut props.shininess, 1.0..=256.0)
                                        .logarithmic(true)
                                        .fixed_decimals(1),
                                );
                                ui.end_row();
                                ui.label("Light color:");
                                let mut lc = [
                                    props.light_color[0],
                                    props.light_color[1],
                                    props.light_color[2],
                                ];
                                ui.color_edit_button_rgb(&mut lc);
                                props.light_color[0] = lc[0];
                                props.light_color[1] = lc[1];
                                props.light_color[2] = lc[2];
                                ui.end_row();
                                ui.label("Background:");
                                ui.color_edit_button_rgb(&mut view.background_color);
                                ui.end_row();
                                ui.label("Light pos X:");
                                ui.add(
                                    egui::DragValue::new(&mut props.light_position[0]).speed(0.1),
                                );
                                ui.end_row();
                                ui.label("Light pos Y:");
                                ui.add(
                                    egui::DragValue::new(&mut props.light_position[1]).speed(0.1),
                                );
                                ui.end_row();
                                ui.label("Light pos Z:");
                                ui.add(
                                    egui::DragValue::new(&mut props.light_position[2]).speed(0.1),
                                );
                                ui.end_row();
                                ui.label("Texture:");
                                let mut use_tex = props.use_texture != 0;
                                ui.checkbox(&mut use_tex, "");
                                props.use_texture = use_tex as u32;
                                ui.end_row();
                                if !use_tex {
                                    ui.label("Object color:");
                                    let mut oc = [
                                        props.object_color[0],
                                        props.object_color[1],
                                        props.object_color[2],
                                    ];
                                    ui.color_edit_button_rgb(&mut oc);
                                    props.object_color[0] = oc[0];
                                    props.object_color[1] = oc[1];
                                    props.object_color[2] = oc[2];
                                    ui.end_row();
                                }
                                ui.separator();
                                ui.end_row();
                                ui.label("Meridians:");
                                ui.add(egui::Slider::new(&mut view.sphere_meridians, 3..=128));
                                ui.end_row();
                                ui.label("Parallels:");
                                ui.add(egui::Slider::new(&mut view.sphere_parallels, 1..=64));
                                ui.end_row();
                            });
                    });
                ui.separator();
                egui::CollapsingHeader::new("Camera")
                    .default_open(false)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label("Mode:");
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
                                    ui.label("Target:");
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
                                    ui.label("Distance:");
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
                        ui.add(egui::Slider::new(&mut cam_speed, 0.05..=4.0).text("Speed"));
                        ui.add(
                            egui::Slider::new(&mut cam_sensitivity, 0.1..=5.0).text("Sensitivity"),
                        );
                        ui.add(
                            egui::Slider::new(&mut cam_zoom_sensitivity, 0.1..=20.0)
                                .text("Zoom sensitivity"),
                        );
                        if ui.button("Reset").clicked() {
                            reset_camera = true;
                        }
                    });
            });
        egui::Window::new("Time Control")
            .resizable(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::Vec2::new(-8.0, -8.0))
            .show(&self.ui.ctx, |ui| {
                let (y, m, d) = orbital::jde_to_gregorian(orbital::sim_time_to_jde(sim.time));
                ui.horizontal(|ui| {
                    ui.label(format!("Date: {:04}-{:02}-{:02}", y, m, d));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(format!("FPS: {:.0}", fps));
                    });
                });

                fn fmt_float(v: f64) -> String {
                    if v.fract() == 0.0 {
                        format!("{}", v as i64)
                    } else {
                        format!("{:.2}", v)
                    }
                }
                let days_per_s = sim.sim_days_per_clock_sec();
                let rate_str = if days_per_s < 1.0 {
                    format!("{} min / s", fmt_float(sim.sim_mins_per_clock_sec()))
                } else {
                    format!("{} days / s", fmt_float(days_per_s))
                };
                ui.label(format!(
                    "Time factor: {}x ({})",
                    fmt_float(sim.multiplier),
                    rate_str,
                ));
                ui.horizontal(|ui| {
                    if ui.button("◀◀").on_hover_text("Halve (PageDown)").clicked() {
                        sim.halve_speed();
                    }
                    let pause_label = if sim.is_paused { "▶" } else { "⏸" };
                    if ui.button(pause_label).on_hover_text("Pause (P)").clicked() {
                        sim.toggle_pause();
                    }
                    if ui.button("▶▶").on_hover_text("Double (PageUp)").clicked() {
                        sim.double_speed();
                    }
                    if ui.button("1×").on_hover_text("Reset (0)").clicked() {
                        sim.reset_speed();
                    }
                    if ui
                        .button("1 min/s")
                        .on_hover_text("1 sim minute per second")
                        .clicked()
                    {
                        sim.set_sim_days_per_sec(1.0 / 1440.0);
                    }
                    if ui
                        .button("1 d/s")
                        .on_hover_text("1 sim day per second")
                        .clicked()
                    {
                        sim.set_sim_days_per_sec(1.);
                    }
                });
                let drag_speed = (sim.multiplier * 0.02).max(0.001);
                ui.horizontal(|ui| {
                    ui.label("Multiplier:");
                    ui.add(
                        egui::DragValue::new(&mut sim.multiplier)
                            .speed(drag_speed)
                            .range(0.0001..=1e15_f64)
                            .max_decimals(4),
                    );
                });
                ui.separator();
                ui.label("Jump to date");
                ui.horizontal(|ui| {
                    let y_ch = ui
                        .add(
                            egui::DragValue::new(&mut view.date_input_year)
                                .range(-4712..=9999)
                                .prefix("Y "),
                        )
                        .changed();
                    let m_ch = ui
                        .add(
                            egui::DragValue::new(&mut view.date_input_month)
                                .range(1u8..=12u8)
                                .prefix("M "),
                        )
                        .changed();
                    let d_ch = ui
                        .add(
                            egui::DragValue::new(&mut view.date_input_day)
                                .range(1u8..=31u8)
                                .prefix("D "),
                        )
                        .changed();
                    if y_ch || m_ch || d_ch {
                        sim.jump_to_date(
                            view.date_input_year,
                            view.date_input_month,
                            view.date_input_day,
                        );
                    }
                    if ui.button("Jump →").clicked() {
                        sim.jump_to_date(
                            view.date_input_year,
                            view.date_input_month,
                            view.date_input_day,
                        );
                    }
                });
            });
        let mut touch = camera::TouchInput::default();
        egui::Window::new("Navigation")
            .title_bar(false)
            .resizable(false)
            .anchor(egui::Align2::LEFT_BOTTOM, egui::Vec2::new(8.0, -8.0))
            .show(&self.ui.ctx, |ui| {
                let btn_size = egui::Vec2::splat(40.0);
                let dir_btn = |label: &str| egui::Button::new(label).min_size(btn_size);
                let held = |resp: &egui::Response| -> f32 {
                    if resp.is_pointer_button_down_on() {
                        1.0
                    } else {
                        0.0
                    }
                };
                ui.spacing_mut().item_spacing = egui::Vec2::splat(4.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.allocate_exact_size(btn_size, egui::Sense::hover());
                        touch.forward = held(&ui.add(dir_btn("↑")));
                        ui.allocate_exact_size(btn_size, egui::Sense::hover());
                    });
                    ui.horizontal(|ui| {
                        touch.left = held(&ui.add(dir_btn("◀")));
                        ui.allocate_exact_size(btn_size, egui::Sense::hover());
                        touch.right = held(&ui.add(dir_btn("▶")));
                    });
                    ui.horizontal(|ui| {
                        ui.allocate_exact_size(btn_size, egui::Sense::hover());
                        touch.backward = held(&ui.add(dir_btn("↓")));
                        ui.allocate_exact_size(btn_size, egui::Sense::hover());
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if cam_is_fps {
                            touch.up = held(&ui.add(dir_btn("↑")));
                            touch.down = held(&ui.add(dir_btn("↓")));
                        }
                        touch.zoom_in = held(&ui.add(dir_btn("+")));
                        touch.zoom_out = held(&ui.add(dir_btn("−")));
                    });
                });
            });
        if self.view.show_body_names {
            let painter = self.ui.ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("body_name_labels"),
            ));
            let ppp = self.ui.ctx.pixels_per_point();
            let screen_w = self.gpu.config.width as f32;
            let screen_h = self.gpu.config.height as f32;
            let view_proj = self.camera_rig.projection.cam_to_clip_matrix()
                * self
                    .camera_rig
                    .camera
                    .world_to_cam_matrix(self.camera_rig.camera.orbit_target(&self.scene));

            for (body_id, name) in &body_list {
                let world_pos = self
                    .scene
                    .get_body_orbital_transform(*body_id)
                    .transform_point3(Vec3::ZERO);
                let clip_pos = view_proj * Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);

                if clip_pos.w <= 0.0 {
                    continue;
                }
                let ndc_x = clip_pos.x / clip_pos.w;
                let ndc_y = clip_pos.y / clip_pos.w;
                let ndc_z = clip_pos.z / clip_pos.w;
                if !(0.0..=1.0).contains(&ndc_z)
                    || !(-1.0..=1.0).contains(&ndc_x)
                    || !(-1.0..=1.0).contains(&ndc_y)
                {
                    continue;
                }

                let screen_x = (ndc_x + 1.0) * 0.5 * screen_w / ppp;
                let screen_y = (1.0 - ndc_y) * 0.5 * screen_h / ppp;
                painter.text(
                    egui::pos2(screen_x, screen_y + 8.0),
                    egui::Align2::CENTER_TOP,
                    name.as_str(),
                    egui::FontId::proportional(self.view.label_font_size),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200),
                );
            }
        }
        if self.view.show_offscreen_indicators {
            let painter = self.ui.ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("offscreen_indicators"),
            ));
            let ppp = self.ui.ctx.pixels_per_point();
            let lw = self.gpu.config.width as f32 / ppp;
            let lh = self.gpu.config.height as f32 / ppp;
            let cx = lw * 0.5;
            let cy = lh * 0.5;
            const MARGIN: f32 = 30.0;
            const ARROW_LEN: f32 = 12.0;
            const ARROW_HALF_W: f32 = 6.0;
            const LABEL_OFFSET: f32 = 8.0;

            let view_proj = self.camera_rig.projection.cam_to_clip_matrix()
                * self
                    .camera_rig
                    .camera
                    .world_to_cam_matrix(self.camera_rig.camera.orbit_target(&self.scene));

            for (body_id, name) in &body_list {
                let world_pos = self
                    .scene
                    .get_body_orbital_transform(*body_id)
                    .transform_point3(Vec3::ZERO);
                let clip = view_proj * Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);

                let aw = clip.w.abs();
                if aw < 1e-6 {
                    continue;
                }

                // On-screen check requires w > 0 (in front of camera)
                let ndc_x = clip.x / clip.w;
                let ndc_y = clip.y / clip.w;
                let ndc_z = clip.z / clip.w;
                let on_screen = clip.w > 0.0
                    && (-1.0..=1.0).contains(&ndc_x)
                    && (-1.0..=1.0).contains(&ndc_y)
                    && (0.0..=1.0).contains(&ndc_z);
                if on_screen {
                    continue;
                }

                // Direction from screen center toward body. Using |w| auto-flips
                // the sign for behind-camera bodies (w < 0).
                let inv = 1.0 / aw;
                let mut dx = clip.x * inv * lw;
                let mut dy = -clip.y * inv * lh; // NDC y up → screen y down
                let dir_len = (dx * dx + dy * dy).sqrt();
                if dir_len < 1e-4 {
                    continue;
                }
                dx /= dir_len;
                dy /= dir_len;

                // Intersect ray from center with the margin-inset screen boundary
                let tx = if dx > 1e-5 {
                    (lw - MARGIN - cx) / dx
                } else if dx < -1e-5 {
                    (MARGIN - cx) / dx
                } else {
                    f32::MAX
                };
                let ty = if dy > 1e-5 {
                    (lh - MARGIN - cy) / dy
                } else if dy < -1e-5 {
                    (MARGIN - cy) / dy
                } else {
                    f32::MAX
                };
                let t = tx.min(ty);
                if t <= 0.0 {
                    continue;
                }

                let tip_x = cx + dx * t;
                let tip_y = cy + dy * t;
                let base_x = tip_x - dx * ARROW_LEN;
                let base_y = tip_y - dy * ARROW_LEN;
                let perp_x = -dy;
                let perp_y = dx;

                let tip = egui::pos2(tip_x, tip_y);
                let left = egui::pos2(
                    base_x + perp_x * ARROW_HALF_W,
                    base_y + perp_y * ARROW_HALF_W,
                );
                let right = egui::pos2(
                    base_x - perp_x * ARROW_HALF_W,
                    base_y - perp_y * ARROW_HALF_W,
                );

                let color = egui::Color32::from_rgba_unmultiplied(255, 200, 50, 220);
                painter.add(egui::Shape::convex_polygon(
                    vec![tip, left, right],
                    color,
                    egui::Stroke::NONE,
                ));
                painter.text(
                    egui::pos2(base_x - dx * LABEL_OFFSET, base_y - dy * LABEL_OFFSET),
                    egui::Align2::CENTER_CENTER,
                    name.as_str(),
                    egui::FontId::proportional(self.view.offscreen_label_font_size),
                    egui::Color32::from_rgba_unmultiplied(255, 200, 50, 200),
                );
            }
        }

        let full_output = self.ui.ctx.end_pass();

        self.scene_properties.uniform = props;
        if self.view.sphere_meridians != prev_meridians
            || self.view.sphere_parallels != prev_parallels
        {
            let (m, p) = (self.view.sphere_meridians, self.view.sphere_parallels);
            for body in &mut self.scene.celestial_bodies {
                body.rebuild_mesh(&self.gpu.device, m, p);
            }
        }
        self.camera_rig.controller.touch = touch;
        self.camera_rig.controller.speed = cam_speed;
        self.camera_rig.controller.sensitivity = cam_sensitivity;
        self.camera_rig.controller.zoom_sensitivity = cam_zoom_sensitivity;
        let mode_switched = selected_is_fps != cam_is_fps;
        if mode_switched || reset_camera {
            self.camera_rig.camera = if selected_is_fps {
                Camera::new_fps(
                    Vec3::new(0.0, 8.0, 25.0),
                    -90f32.to_radians(),
                    -20f32.to_radians(),
                )
            } else {
                Camera::new_orbit(selected_target, 25.0, 0.0, 45f32.to_radians())
            };
            self.camera_rig.controller.sensitivity = if selected_is_fps { 4.0 } else { 0.6 };
            self.camera_rig.controller.zoom_sensitivity = 6.0;
        } else {
            match &mut self.camera_rig.camera {
                Camera::Fps(c) => {
                    c.position = Vec3::new(fps_pos_x, fps_pos_y, fps_pos_z);
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
            .handle_platform_output(&self.gpu.window, full_output.platform_output);
        let clipped_primitives = self
            .ui
            .ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);
        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.gpu.config.width, self.gpu.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        for (id, image_delta) in &full_output.textures_delta.set {
            self.ui
                .renderer
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, image_delta);
        }
        self.ui.renderer.update_buffers(
            &self.gpu.device,
            &self.gpu.queue,
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
        self.gpu.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn handle_key(&mut self, event_loop: &ActiveEventLoop, code: KeyCode, state: ElementState) {
        if code == KeyCode::Escape && state.is_pressed() {
            event_loop.exit();
        } else if code == KeyCode::Tab && state.is_pressed() {
            self.view.toggle_wireframe();
        } else if code == KeyCode::KeyN && state.is_pressed() {
            self.view.toggle_normals();
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
        } else if code == KeyCode::KeyL && state.is_pressed() {
            self.view.toggle_body_names();
        } else if code == KeyCode::KeyV && state.is_pressed() {
            self.view.toggle_arrows();
        } else if code == KeyCode::KeyI && state.is_pressed() {
            self.view.toggle_offscreen_indicators();
        } else if code == KeyCode::BracketLeft && state.is_pressed() {
            self.view.decrease_ui_scale();
        } else if code == KeyCode::BracketRight && state.is_pressed() {
            self.view.increase_ui_scale();
        } else {
            self.camera_rig.controller.handle_key(code, state);
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
                                let msg = "Failed to send initialized state to event loop";
                                tracing::error!("{msg}");
                                wgpu::web_sys::console::error_1(&msg.into());
                            }
                        }
                        Err(e) => {
                            let msg = format!("Failed to initialize GPU state (WASM): {e:?}");
                            tracing::error!("{msg}");
                            wgpu::web_sys::console::error_1(&msg.into());
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
            event.gpu.window.request_redraw();
            event.resize(
                event.gpu.window.inner_size().width,
                event.gpu.window.inner_size().height,
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
            .on_window_event(&state.gpu.window, &event)
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
                            width = state.gpu.config.width,
                            height = state.gpu.config.height,
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
            state
                .pipelines
                .try_reload_main_shader(&state.gpu.device, state.gpu.config.format);
        }
        if reload_grid {
            state
                .pipelines
                .try_reload_grid_shader(&state.gpu.device, state.gpu.config.format);
        }
        if reload_normals {
            state
                .pipelines
                .try_reload_normals_shader(&state.gpu.device, state.gpu.config.format);
        }
        if reload_main || reload_grid || reload_normals {
            state.gpu.window.request_redraw();
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
