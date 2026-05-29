use std::f32::consts::FRAC_PI_2;
use web_time::Duration;
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalPosition;
use winit::event::*;
use winit::keyboard::KeyCode;

use crate::scene::{BodyId, Scene};

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
    /// For `Orbit` cameras, `orbit_target` must be `Some(world_pos_of_target_body)`.
    pub fn world_to_cam_matrix(&self, orbit_target: Option<glam::Vec3>) -> glam::Mat4 {
        match self {
            Camera::Fps(camera) => camera.world_to_cam_matrix(),
            Camera::Orbit(camera) => camera.world_to_cam_matrix(
                orbit_target.expect("orbit_target required for Camera::Orbit"),
            ),
        }
    }

    /// Returns the position of the camera in world space.
    pub fn position(&self, orbit_target: Option<glam::Vec3>) -> glam::Vec3 {
        match self {
            Camera::Fps(cam) => cam.position,
            Camera::Orbit(cam) => {
                cam.position(orbit_target.expect("orbit_target required for Camera::Orbit"))
            }
        }
    }

    /// Returns the world-space position of this camera's orbit target,
    /// or `None` if this is not an orbit camera.
    pub fn orbit_target(&self, scene: &Scene) -> Option<glam::Vec3> {
        match self {
            Camera::Orbit(c) => Some(
                scene
                    .get_body_orbital_transform(c.target)
                    .transform_point3(glam::Vec3::ZERO),
            ),
            _ => None,
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

    /// Returns the position of the camera in world space.
    pub fn position(&self, target_pos: glam::Vec3) -> glam::Vec3 {
        let rel_camera_transform = self.relative_camera_transform();
        rel_camera_transform.transform_point3(target_pos)
    }

    /// Calculate the view matrix for this camera. This is the transform that transforms world space to camera space.
    pub fn world_to_cam_matrix(&self, target_pos: glam::Vec3) -> glam::Mat4 {
        let camera_pos = self.position(target_pos);
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

    pub fn update_camera(&mut self, camera: &mut Camera, dt: Duration) {
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
                camera.position +=
                    scrollward * self.scroll * self.speed * self.zoom_sensitivity * dt;
                self.scroll = 0.0;
                // Touch zoom buttons drive the same forward/back motion as scroll.
                camera.position += scrollward
                    * (zoom_in_amt - zoom_out_amt)
                    * self.speed
                    * self.zoom_sensitivity
                    * dt;

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
                camera.dist +=
                    (zoom_out_amt - zoom_in_amt) * self.speed * self.zoom_sensitivity * dt;
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
    /// Camera position in world space
    camera_pos: [f32; 3],
    /// Padding to match WGSL's vec3<f32> alignment (16 bytes in uniforms)
    _padding: f32,
}

impl CameraUniform {
    /// Identity matrix; overwritten on the first `update_view_proj` call.
    pub fn new() -> Self {
        Self {
            view_proj: glam::Mat4::IDENTITY.to_cols_array_2d(),
            camera_pos: [0f32; 3],
            _padding: 0.0,
        }
    }

    /// Recompute `view_proj` from the current camera + projection + scene state.
    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection, scene: &Scene) {
        let orbit_target = match camera {
            Camera::Orbit(c) => Some(
                scene
                    .get_body_orbital_transform(c.target)
                    .transform_point3(glam::Vec3::ZERO),
            ),
            _ => None,
        };
        self.view_proj = (projection.cam_to_clip_matrix()
            * camera.world_to_cam_matrix(orbit_target))
        .to_cols_array_2d();
        self.camera_pos = camera.position(orbit_target).to_array();
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
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
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
        self.controller.update_camera(&mut self.camera, dt);
        self.uniform
            .update_view_proj(&self.camera, &self.projection, scene);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;
    use web_time::Duration;
    use winit::event::ElementState;
    use winit::keyboard::KeyCode;

    const EPS: f32 = 1e-5;

    fn approx_eq3(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < EPS
    }

    fn make_ctrl() -> CameraController {
        CameraController::new(1.0, 1.0, 1.0)
    }

    /// Step an FPS camera through the public controller for one second.
    fn step_fps(ctrl: &mut CameraController, cam: FpsCamera) -> FpsCamera {
        let mut camera = Camera::Fps(cam);
        ctrl.update_camera(&mut camera, Duration::from_secs(1));
        match camera {
            Camera::Fps(c) => c,
            _ => unreachable!(),
        }
    }

    // --- FpsCamera::world_to_cam_matrix ---

    #[test]
    fn view_matrix_eye_maps_to_origin() {
        let cam = FpsCamera::new(Vec3::new(3.0, 4.0, 5.0), 0.0, 0.0);
        let result = cam.world_to_cam_matrix().transform_point3(cam.position);
        assert!(approx_eq3(result, Vec3::ZERO), "got {result}");
    }

    #[test]
    fn view_matrix_look_direction_maps_to_neg_z() {
        // yaw=0, pitch=0 → forward direction in world space is +X
        let cam = FpsCamera::new(Vec3::ZERO, 0.0, 0.0);
        let forward_cam = cam.world_to_cam_matrix().transform_vector3(Vec3::X);
        assert!(approx_eq3(forward_cam, Vec3::NEG_Z), "got {forward_cam}");
    }

    #[test]
    fn view_matrix_world_up_stays_up() {
        let cam = FpsCamera::new(Vec3::ZERO, 0.0, 0.0);
        let up_cam = cam.world_to_cam_matrix().transform_vector3(Vec3::Y);
        assert!(approx_eq3(up_cam, Vec3::Y), "got {up_cam}");
    }

    // --- CameraController input → FPS camera movement ---

    #[test]
    fn space_moves_camera_up() {
        let mut ctrl = make_ctrl();
        ctrl.handle_key(KeyCode::Space, ElementState::Pressed);
        let cam = step_fps(&mut ctrl, FpsCamera::new(Vec3::ZERO, 0.0, 0.0));
        assert!(cam.position.y > 0.0, "expected y > 0, got {}", cam.position.y);
    }

    #[test]
    fn shift_moves_camera_down() {
        let mut ctrl = make_ctrl();
        ctrl.handle_key(KeyCode::ShiftLeft, ElementState::Pressed);
        let cam = step_fps(&mut ctrl, FpsCamera::new(Vec3::ZERO, 0.0, 0.0));
        assert!(cam.position.y < 0.0, "expected y < 0, got {}", cam.position.y);
    }

    #[test]
    fn key_release_stops_movement() {
        let mut ctrl = make_ctrl();
        ctrl.handle_key(KeyCode::Space, ElementState::Pressed);
        ctrl.handle_key(KeyCode::Space, ElementState::Released);
        let cam = step_fps(&mut ctrl, FpsCamera::new(Vec3::ZERO, 0.0, 0.0));
        assert_eq!(cam.position, Vec3::ZERO);
    }

    #[test]
    fn w_moves_forward_along_yaw() {
        // yaw=0 → forward is +X in world space
        let mut ctrl = make_ctrl();
        ctrl.handle_key(KeyCode::KeyW, ElementState::Pressed);
        let cam = step_fps(&mut ctrl, FpsCamera::new(Vec3::ZERO, 0.0, 0.0));
        assert!(cam.position.x > 0.0, "expected x > 0, got {}", cam.position.x);
        assert_eq!(cam.position.y, 0.0);
    }

    #[test]
    fn s_moves_backward() {
        // yaw=0 → backward is -X in world space
        let mut ctrl = make_ctrl();
        ctrl.handle_key(KeyCode::KeyS, ElementState::Pressed);
        let cam = step_fps(&mut ctrl, FpsCamera::new(Vec3::ZERO, 0.0, 0.0));
        assert!(cam.position.x < 0.0, "expected x < 0, got {}", cam.position.x);
    }

    #[test]
    fn arrow_up_aliases_forward() {
        let mut ctrl = make_ctrl();
        ctrl.handle_key(KeyCode::ArrowUp, ElementState::Pressed);
        let cam = step_fps(&mut ctrl, FpsCamera::new(Vec3::ZERO, 0.0, 0.0));
        assert!(cam.position.x > 0.0, "expected x > 0, got {}", cam.position.x);
    }

    #[test]
    fn touch_forward_moves_without_keys() {
        // Touch input should drive movement even when no key is held.
        let mut ctrl = make_ctrl();
        ctrl.touch.forward = 1.0;
        let cam = step_fps(&mut ctrl, FpsCamera::new(Vec3::ZERO, 0.0, 0.0));
        assert!(cam.position.x > 0.0, "expected x > 0, got {}", cam.position.x);
    }

    #[test]
    fn mouse_dx_changes_yaw() {
        let mut ctrl = make_ctrl();
        ctrl.handle_mouse(10.0, 0.0);
        let cam = step_fps(&mut ctrl, FpsCamera::new(Vec3::ZERO, 0.0, 0.0));
        assert!(cam.yaw_rad > 0.0, "expected yaw > 0, got {}", cam.yaw_rad);
    }

    #[test]
    fn mouse_dy_down_tilts_camera_down() {
        // positive mouse_dy (moving mouse downward) → negative pitch (looking down)
        let mut ctrl = make_ctrl();
        ctrl.handle_mouse(0.0, 5.0);
        let cam = step_fps(&mut ctrl, FpsCamera::new(Vec3::ZERO, 0.0, 0.0));
        assert!(cam.pitch_rad < 0.0, "expected pitch < 0, got {}", cam.pitch_rad);
    }

    #[test]
    fn pitch_clamped_to_safe_range() {
        let mut ctrl = CameraController::new(1.0, 100.0, 1.0);
        ctrl.handle_mouse(0.0, -10_000.0); // large upward movement
        let cam = step_fps(&mut ctrl, FpsCamera::new(Vec3::ZERO, 0.0, 0.0));
        assert!(
            cam.pitch_rad <= SAFE_FRAC_PI_2,
            "pitch not clamped: {}",
            cam.pitch_rad
        );
    }

    // --- Orbit camera ---

    #[test]
    fn orbit_zoom_keeps_positive_distance() {
        // Zooming out far should not drive distance negative (clamped at 0.1).
        let mut ctrl = CameraController::new(1.0, 1.0, 100.0);
        ctrl.handle_key(KeyCode::Minus, ElementState::Pressed); // zoom out
        let mut camera = Camera::Orbit(OrbitCamera::new(BodyId::TEST, 1.0, 0.0, 0.0));
        ctrl.update_camera(&mut camera, Duration::from_secs(10));
        match camera {
            Camera::Orbit(c) => assert!(c.dist >= 0.1, "dist not clamped: {}", c.dist),
            _ => unreachable!(),
        }
    }
}
