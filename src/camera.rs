use std::f32::consts::FRAC_PI_2;
use web_time::Duration;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::KeyCode;

use crate::scene::{self, BodyId, Scene};

#[rustfmt::skip]
const OPENGL_TO_WGPU_MATRIX: glam::Mat4 = glam::Mat4::from_cols(
    glam::Vec4::new(1.0, 0.0, 0.0, 0.0),
    glam::Vec4::new(0.0, 1.0, 0.0, 0.0),
    glam::Vec4::new(0.0, 0.0, 0.5, 0.0),
    glam::Vec4::new(0.0, 0.0, 0.5, 1.0),
);
const SAFE_FRAC_PI_2: f32 = FRAC_PI_2 - 0.0001;

#[derive(Debug, Clone)]
pub enum Camera {
    Fps(FpsCamera),
    Orbit(OrbitCamera),
}
impl Camera {
    #[allow(dead_code)]
    pub fn new_fps(position: impl Into<glam::Vec3>, yaw_rad: f32, pitch_rad: f32) -> Self {
        Self::Fps(FpsCamera::new(position, yaw_rad, pitch_rad))
    }
    #[allow(dead_code)]
    pub fn new_orbit(target: BodyId, dist: f32, yaw_rad: f32, pitch_rad: f32) -> Self {
        Self::Orbit(OrbitCamera::new(target, dist, yaw_rad, pitch_rad))
    }

    /// Calculate the view matrix for this camera. This is the transform that transforms world space to camera space.
    pub fn world_to_cam_matrix(&self, scene: &Scene) -> glam::Mat4 {
        match self {
            Camera::Fps(camera) => camera.world_to_cam_matrix(),
            Camera::Orbit(camera) => camera.world_to_cam_matrix(scene),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FpsCamera {
    /// Position of the camera in world space.
    pub position: glam::Vec3,
    pub yaw_rad: f32,
    pub pitch_rad: f32,
}

impl FpsCamera {
    pub fn new(position: impl Into<glam::Vec3>, yaw_rad: f32, pitch_rad: f32) -> Self {
        Self {
            position: position.into(),
            yaw_rad,
            pitch_rad,
        }
    }

    /// Calculate the view matrix for this camera. This is the transform that transforms world space to camera space.
    pub fn world_to_cam_matrix(&self) -> glam::Mat4 {
        let (sin_pitch, cos_pitch) = self.pitch_rad.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw_rad.sin_cos();
        glam::Mat4::look_to_rh(
            self.position,
            glam::Vec3::new(cos_pitch * cos_yaw, sin_pitch, cos_pitch * sin_yaw).normalize(),
            glam::Vec3::Y,
        )
    }
}

#[derive(Debug, Clone)]
pub struct OrbitCamera {
    pub target: BodyId,
    pub dist: f32,
    /// Rotation around y axis. 0 means the camera is looking at the target from the positive z axis, and positive values mean the camera is rotating clockwise around the target (when looking from above).
    pub yaw_rad: f32,
    /// The angle between the camera's forward vector and the horizontal plane. 0 means the camera is looking at the target from the same height, positive values mean the camera is looking from above, and negative values mean the camera is looking from below.
    pub pitch_rad: f32,
}

impl OrbitCamera {
    pub fn new(target: BodyId, dist: f32, yaw_rad: f32, pitch_rad: f32) -> Self {
        Self {
            target,
            dist,
            yaw_rad,
            pitch_rad,
        }
    }

    /// Transform from `target` to `camera` (world space)
    pub fn relative_camera_transform(&self) -> glam::Mat4 {
        // Negate pitch so positive pitch lifts the camera above the target (elevation convention).
        let camera_rotation = glam::Mat4::from_rotation_y(self.yaw_rad)
            * glam::Mat4::from_rotation_x(-self.pitch_rad);
        let camera_translation = glam::Mat4::from_translation(glam::Vec3::new(0.0, 0.0, self.dist));
        camera_rotation * camera_translation
    }

    pub fn target_and_camera_pos(&self, scene: &Scene) -> (glam::Vec3, glam::Vec3) {
        let target_pos = scene
            .get_body_orbital_transform(self.target)
            .transform_point3(glam::Vec3::ZERO);
        let rel_camera_transform = self.relative_camera_transform();
        let camera_pos = rel_camera_transform.transform_point3(target_pos);
        (target_pos, camera_pos)
    }

    /// Calculate the view matrix for this camera. This is the transform that transforms world space to camera space.
    pub fn world_to_cam_matrix(&self, scene: &Scene) -> glam::Mat4 {
        let (target_pos, camera_pos) = self.target_and_camera_pos(scene);

        glam::Mat4::look_at_rh(camera_pos, target_pos, glam::Vec3::Y)
    }
}

/// Represents the perspective projection parameters used to compute the cam to clip matrix.
pub struct Projection {
    aspect_ratio: f32,
    fov_y_rad: f32,
    z_near: f32,
    z_far: f32,
}
impl Projection {
    pub fn new(width: u32, height: u32, fov_y_rad: f32, z_near: f32, z_far: f32) -> Self {
        let aspect_ratio = width as f32 / height as f32;
        Self {
            aspect_ratio,
            fov_y_rad,
            z_near,
            z_far,
        }
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect_ratio = width as f32 / height as f32;
    }

    /// Calculate the projection matrix that transforms camera space to clip space.
    /// Combines a right-handed perspective projection with `OPENGL_TO_WGPU_MATRIX`
    /// to remap OpenGL's `[-1, 1]` z-range to wgpu's `[0, 1]` convention.
    pub fn cam_to_clip_matrix(&self) -> glam::Mat4 {
        OPENGL_TO_WGPU_MATRIX
            * glam::Mat4::perspective_rh(self.fov_y_rad, self.aspect_ratio, self.z_near, self.z_far)
    }
}

#[derive(Debug)]
pub struct CameraController {
    amount_left: f32,
    amount_right: f32,
    amount_forward: f32,
    amount_backward: f32,
    amount_up: f32,
    amount_down: f32,
    amount_zoom_in: f32,
    amount_zoom_out: f32,
    /// Touch/UI-driven directional inputs, merged with the keyboard amounts in `update_camera`.
    /// Kept separate so a held key isn't clobbered by a frame in which no button is pressed.
    pub touch: TouchInput,
    rotate_horizontal: f32,
    rotate_vertical: f32,
    scroll: f32,
    pub speed: f32,
    pub sensitivity: f32,
    /// Independent sensitivity multiplier applied to scroll and touch-zoom inputs.
    pub zoom_sensitivity: f32,
}

/// On-screen directional buttons, set every frame from egui responses.
#[derive(Debug, Default, Clone, Copy)]
pub struct TouchInput {
    pub left: f32,
    pub right: f32,
    pub forward: f32,
    pub backward: f32,
    pub up: f32,
    pub down: f32,
    pub zoom_in: f32,
    pub zoom_out: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32, zoom_sensitivity: f32) -> Self {
        Self {
            amount_left: 0.0,
            amount_right: 0.0,
            amount_forward: 0.0,
            amount_backward: 0.0,
            amount_up: 0.0,
            amount_down: 0.0,
            amount_zoom_in: 0.0,
            amount_zoom_out: 0.0,
            touch: TouchInput::default(),
            rotate_horizontal: 0.0,
            rotate_vertical: 0.0,
            scroll: 0.0,
            speed,
            sensitivity,
            zoom_sensitivity,
        }
    }

    pub fn handle_key(&mut self, key: KeyCode, state: ElementState) -> bool {
        let amount = if state.is_pressed() { 1.0 } else { 0.0 };
        match key {
            KeyCode::KeyW | KeyCode::ArrowUp => {
                self.amount_forward = amount;
                true
            }
            KeyCode::KeyS | KeyCode::ArrowDown => {
                self.amount_backward = amount;
                true
            }
            KeyCode::KeyA | KeyCode::ArrowLeft => {
                self.amount_left = amount;
                true
            }
            KeyCode::KeyD | KeyCode::ArrowRight => {
                self.amount_right = amount;
                true
            }
            KeyCode::Space => {
                self.amount_up = amount;
                true
            }
            KeyCode::ShiftLeft => {
                self.amount_down = amount;
                true
            }
            KeyCode::Equal | KeyCode::NumpadAdd => {
                self.amount_zoom_in = amount;
                true
            }
            KeyCode::Minus | KeyCode::NumpadSubtract => {
                self.amount_zoom_out = amount;
                true
            }
            _ => false,
        }
    }

    pub fn handle_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotate_horizontal = mouse_dx as f32;
        self.rotate_vertical = mouse_dy as f32;
    }

    pub fn handle_mouse_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = -match delta {
            // I'm assuming a line is about 100 pixels
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 100.0,
            MouseScrollDelta::PixelDelta(PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration, _scene: &scene::Scene) {
        let dt = dt.as_secs_f32();

        let forward_amt = self.amount_forward.max(self.touch.forward);
        let backward_amt = self.amount_backward.max(self.touch.backward);
        let left_amt = self.amount_left.max(self.touch.left);
        let right_amt = self.amount_right.max(self.touch.right);
        let up_amt = self.amount_up.max(self.touch.up);
        let down_amt = self.amount_down.max(self.touch.down);
        let zoom_in_amt = self.amount_zoom_in.max(self.touch.zoom_in);
        let zoom_out_amt = self.amount_zoom_out.max(self.touch.zoom_out);

        match camera {
            Camera::Fps(camera) => {
                // Move forward/backward and left/right
                let (yaw_sin, yaw_cos) = camera.yaw_rad.sin_cos();
                let forward = glam::Vec3::new(yaw_cos, 0.0, yaw_sin).normalize();
                let right = glam::Vec3::new(-yaw_sin, 0.0, yaw_cos).normalize();
                camera.position += forward * (forward_amt - backward_amt) * self.speed * dt;
                camera.position += right * (right_amt - left_amt) * self.speed * dt;

                // Move in/out (aka. "zoom")
                // Note: this isn't an actual zoom. The camera's position
                // changes when zooming. I've added this to make it easier
                // to get closer to an object you want to focus on.
                let (pitch_sin, pitch_cos) = camera.pitch_rad.sin_cos();
                let scrollward =
                    glam::Vec3::new(pitch_cos * yaw_cos, pitch_sin, pitch_cos * yaw_sin)
                        .normalize();
                camera.position += scrollward * self.scroll * self.speed * self.zoom_sensitivity * dt;
                self.scroll = 0.0;
                // Touch zoom buttons drive the same forward/back motion as scroll.
                camera.position +=
                    scrollward * (zoom_in_amt - zoom_out_amt) * self.speed * self.zoom_sensitivity * dt;

                // Move up/down. Since we don't use roll, we can just
                // modify the y coordinate directly.
                camera.position.y += (up_amt - down_amt) * self.speed * dt;

                // Rotate
                camera.yaw_rad += self.rotate_horizontal * self.sensitivity * dt;
                camera.pitch_rad += -self.rotate_vertical * self.sensitivity * dt;

                // If process_mouse isn't called every frame, these values
                // will not get set to zero, and the camera will rotate
                // when moving in a non-cardinal direction.
                self.rotate_horizontal = 0.0;
                self.rotate_vertical = 0.0;

                // Keep the camera's angle from going too high/low.
                camera.pitch_rad = camera.pitch_rad.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
            }
            Camera::Orbit(camera) => {
                camera.yaw_rad += self.rotate_horizontal * self.sensitivity * dt;
                camera.pitch_rad += -self.rotate_vertical * self.sensitivity * dt;
                camera.yaw_rad += (right_amt - left_amt) * self.speed * dt;
                camera.pitch_rad += (forward_amt - backward_amt) * self.speed * dt;
                camera.pitch_rad = camera.pitch_rad.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
                self.rotate_horizontal = 0.0;
                self.rotate_vertical = 0.0;

                camera.dist += self.scroll * self.speed * self.zoom_sensitivity * dt;
                self.scroll = 0.0;
                camera.dist += (zoom_out_amt - zoom_in_amt) * self.speed * self.zoom_sensitivity * dt;
                camera.dist = camera.dist.max(0.1);
            }
        }
    }
}

/// View-projection matrix mapping world space to clip space.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    /// Maps world space to clip space
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    /// Identity matrix; overwritten on the first `update_view_proj` call.
    pub fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    /// Recompute `view_proj` from the current camera + projection + scene state.
    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection, scene: &Scene) {
        self.view_proj = (projection.cam_to_clip_matrix() * camera.world_to_cam_matrix(scene))
            .to_cols_array_2d();
    }
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self::new()
    }
}

/// Owns everything camera-related: the camera enum, projection, input controller,
/// and the uniform buffer + bind group that ship the view-projection matrix to the GPU.
pub struct CameraRig {
    /// Active camera (FPS or orbit). Mutated by the controller and the UI.
    pub camera: Camera,
    /// Perspective projection; resized when the surface size changes.
    pub projection: Projection,
    /// Keyboard/mouse-driven input integrator.
    pub controller: CameraController,
    /// Whether the left mouse button is currently held; gates mouse-look.
    pub mouse_pressed: bool,
    /// Bind group layout for the camera uniform; needed when building pipelines.
    pub bind_group_layout: wgpu::BindGroupLayout,
    /// Bind group bound at group 1 during the main render pass.
    pub bind_group: wgpu::BindGroup,
    uniform: CameraUniform,
    buffer: wgpu::Buffer,
}

impl CameraRig {
    /// Create the rig with all its GPU resources. The initial uniform is computed
    /// from `camera`, `scene`, and a 45° perspective projection.
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        camera: Camera,
        scene: &Scene,
    ) -> Self {
        let projection = Projection::new(width, height, 45.0f32.to_radians(), 0.1, 100.0);

        let mut uniform = CameraUniform::new();
        uniform.update_view_proj(&camera, &projection, scene);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
            label: Some("Camera Bind Group"),
        });

        Self {
            camera,
            projection,
            controller: CameraController::new(0.5, 0.6, 6.0),
            mouse_pressed: false,
            bind_group_layout,
            bind_group,
            uniform,
            buffer,
        }
    }

    /// Update the projection aspect ratio after a surface resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.projection.resize(width, height);
    }

    /// Step the controller, recompute the view-projection uniform, and upload it.
    pub fn update(&mut self, dt: Duration, scene: &Scene, queue: &wgpu::Queue) {
        self.controller.update_camera(&mut self.camera, dt, scene);
        self.uniform
            .update_view_proj(&self.camera, &self.projection, scene);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}
