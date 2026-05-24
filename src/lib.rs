mod camera;
mod celestial_body;
mod gpu;
mod grid;
mod loader;
mod mesh;
pub mod orbital;
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
pub use scene::{BodyId, Scene};
pub use shader_loader::validate_wgsl;
pub use texture::Texture;

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

use crate::grid::{DrawGrid, GridMesh};
use crate::mesh::DrawMesh;
use crate::{
    camera::{Camera, CameraRig},
    celestial_body::{DrawCelestialBody, DrawCelestialBodyNormals},
    gpu::GpuContext,
    pipelines::Pipelines,
};

pub struct State {
    gpu: GpuContext,
    last_update: web_time::Instant,
    last_frame_duration: web_time::Duration,
    pipelines: Pipelines,
    grid_xz: GridMesh,
    grid_xy: GridMesh,
    grid_yz: GridMesh,
    view: ui::ViewOptions,
    meshes: Vec<mesh::Mesh>,
    diffuse_texture: texture::Texture,
    // diffuse_bind_group: wgpu::BindGroup,
    identity_model_bind_group: wgpu::BindGroup,
    scene: scene::Scene,
    scene_properties: scene_properties::SceneProperties,
    sim: sim::SimState,
    camera_rig: CameraRig,
    ui: ui::EguiLayer,
}

impl State {
    pub async fn new(window: Arc<Window>) -> miette::Result<Self> {
        let gpu = GpuContext::new(window).await?;
        let device = &gpu.device;
        let queue = &gpu.queue;
        let config = &gpu.config;
        let surface_format = config.format;
        let size = winit::dpi::PhysicalSize::new(config.width, config.height);

        let texture_bind_group_layout = texture::Texture::bind_group_layout(device);
        let diffuse_bytes = loader::load_bytes("assets/textures/dbg.png").await?;
        let diffuse_texture = texture::Texture::from_bytes(
            device,
            queue,
            &diffuse_bytes,
            "dbg.png",
            &texture_bind_group_layout,
        )?;

        // ==================== Scene setup =====================
        let mut scene = scene::Scene::new(device);
        let mut body_ids: Vec<scene::BodyId> = Vec::with_capacity(planets::BODIES.len());
        for def in planets::BODIES {
            let bytes = loader::load_bytes(def.texture_path).await?;
            let texture = texture::Texture::from_bytes(
                device,
                queue,
                &bytes,
                def.name,
                &texture_bind_group_layout,
            )?;
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
        let sun_id = body_ids[planets::SolarSystemBody::Sun as usize];

        // ======= Camera setup =======
        // let initial_camera =
        //     Camera::new_fps((0.0, 8.0, 25.0), -90f32.to_radians(), -15f32.to_radians());
        let initial_camera = Camera::new_orbit(sun_id, 30.0, 0f32.to_radians(), 30f32.to_radians());
        let camera_rig = CameraRig::new(device, size.width, size.height, initial_camera, &scene);

        // ================= Scene properties =================
        let scene_properties = scene_properties::SceneProperties::new(device);

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
        let ui_layer = ui::EguiLayer::new(device, &gpu.window, surface_format);

        Ok(Self {
            gpu,
            last_update: web_time::Instant::now(),
            last_frame_duration: web_time::Duration::ZERO,
            pipelines,
            grid_xz,
            grid_xy,
            grid_yz,
            view: ui::ViewOptions::new(),
            meshes,
            identity_model_bind_group,
            diffuse_texture,
            scene,
            scene_properties,
            sim: sim::SimState::new(),
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
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
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

            // TODO: move this logic into the scene and/or celestial body
            tracing::trace!(
                count = self.scene.celestial_bodies.len(),
                "draw celestial bodies"
            );
            for planet in &self.scene.celestial_bodies {
                tracing::trace!(name = planet.name, "draw body");
                render_pass.draw_celestial_body(planet, &self.camera_rig.bind_group);
            }

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

            tracing::trace!("set pipeline: grid");
            render_pass.set_pipeline(&self.pipelines.grid);
            tracing::trace!(group = 0, "set bind group: camera (grid)");
            render_pass.set_bind_group(0, &self.camera_rig.bind_group, &[]);
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
        let sim = &mut self.sim;
        let view = &mut self.view;
        let mut cam_speed = self.camera_rig.controller.speed;
        let mut cam_sensitivity = self.camera_rig.controller.sensitivity;
        let mut cam_zoom_sensitivity = self.camera_rig.controller.zoom_sensitivity;
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

        let mut props = self.scene_properties.uniform;

        let raw_input = self.ui.state.take_egui_input(&self.gpu.window);
        self.ui.ctx.begin_pass(raw_input);
        egui::Window::new("Simulation")
            .resizable(false)
            .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-8.0, 8.0))
            .show(&self.ui.ctx, |ui| {
                ui.label(format!("FPS: {:.0}", fps));
                let (y, m, d) = orbital::jde_to_gregorian(orbital::sim_time_to_jde(sim.time));
                ui.label(format!("Datum: {:04}-{:02}-{:02}", y, m, d));
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
                let names_label = if view.show_body_names {
                    "Beschriftungen: an"
                } else {
                    "Beschriftungen: aus"
                };
                if ui
                    .button(names_label)
                    .on_hover_text("Umschalten (L)")
                    .clicked()
                {
                    view.toggle_body_names();
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
                                ui.label("Meridians:");
                                ui.add(egui::Slider::new(&mut view.sphere_meridians, 3..=128));
                                ui.end_row();
                                ui.label("Parallels:");
                                ui.add(egui::Slider::new(&mut view.sphere_parallels, 1..=64));
                                ui.end_row();
                            });
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
                        ui.add(
                            egui::Slider::new(&mut cam_zoom_sensitivity, 0.1..=20.0)
                                .text("Zoom-Empfindlichkeit"),
                        );
                        if ui.button("Zurücksetzen").clicked() {
                            reset_camera = true;
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
                * self.camera_rig.camera.world_to_cam_matrix(&self.scene);

            for (body_id, name) in &body_list {
                let world_pos = self
                    .scene
                    .get_body_orbital_transform(*body_id)
                    .transform_point3(glam::Vec3::ZERO);
                let clip_pos =
                    view_proj * glam::Vec4::new(world_pos.x, world_pos.y, world_pos.z, 1.0);

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
                    egui::pos2(screen_x, screen_y),
                    egui::Align2::CENTER_BOTTOM,
                    name.as_str(),
                    egui::FontId::proportional(13.0),
                    egui::Color32::from_rgba_unmultiplied(255, 255, 255, 200),
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
                    glam::Vec3::new(0.0, 8.0, 25.0),
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
