use std::sync::Arc;

/// Toggleable render options driven by keyboard shortcuts and the egui panel.
#[derive(Debug, Clone)]
pub struct ViewOptions {
    /// Use the wireframe pipeline (PolygonMode::Line on native, fallback on WASM).
    pub wireframe: bool,
    /// Overlay the per-vertex normals as line segments via the normals pipeline.
    pub show_normals: bool,
    /// Show the XZ grid (ground plane).
    pub show_grid_xz: bool,
    /// Show the XY grid (front-facing plane).
    pub show_grid_xy: bool,
    /// Show the YZ grid (side-facing plane).
    pub show_grid_yz: bool,
    /// Longitude segments for sphere tessellation (meridians).
    pub sphere_meridians: u32,
    /// Latitude segments for sphere tessellation (parallels).
    pub sphere_parallels: u32,
    /// Show name labels for each celestial body as a screen-space overlay.
    pub show_body_names: bool,
    /// Year field in the "Jump to date" input (Gregorian).
    pub date_input_year: i32,
    /// Month field in the "Jump to date" input (1–12).
    pub date_input_month: u8,
    /// Day field in the "Jump to date" input (1–31).
    pub date_input_day: u8,
    /// Master toggle: show debug vector arrows.
    pub show_arrows: bool,
    /// Show velocity-direction arrows (cyan).
    pub arrows_velocity: bool,
    /// Show radial arrows toward parent / sun (orange).
    pub arrows_radial: bool,
    /// Show spin-axis arrows (green).
    pub arrows_spin: bool,
    /// Background clear color (RGB, linear).
    pub background_color: [f32; 3],
    /// Show edge arrows pointing toward off-screen bodies.
    pub show_offscreen_indicators: bool,
    /// Brightness multiplier for all line geometry (grids and arrows), 0–2.
    pub line_brightness: f32,
}

impl ViewOptions {
    /// Default view: ground grid on, others off, solid fill, no normals overlay.
    pub fn new() -> Self {
        Self {
            wireframe: false,
            show_normals: false,
            show_grid_xz: true,
            show_grid_xy: false,
            show_grid_yz: false,
            sphere_meridians: 128,
            sphere_parallels: 64,
            show_body_names: false,
            date_input_year: 2000,
            date_input_month: 1,
            date_input_day: 1,
            show_arrows: false,
            arrows_velocity: true,
            arrows_radial: true,
            arrows_spin: true,
            background_color: [0.0, 0.0, 0.0],
            show_offscreen_indicators: true,
            line_brightness: 1.0,
        }
    }

    /// Flip the wireframe flag.
    pub fn toggle_wireframe(&mut self) {
        self.wireframe = !self.wireframe;
        tracing::info!("Wireframe mode: {}", self.wireframe);
    }

    /// Flip the normals-overlay flag.
    pub fn toggle_normals(&mut self) {
        self.show_normals = !self.show_normals;
        tracing::info!("Normals visualization: {}", self.show_normals);
    }

    /// True if at least one grid plane is currently visible.
    pub fn any_grid_visible(&self) -> bool {
        self.show_grid_xz || self.show_grid_xy || self.show_grid_yz
    }

    /// Flip the body-name overlay flag.
    pub fn toggle_body_names(&mut self) {
        self.show_body_names = !self.show_body_names;
        tracing::info!("Body name labels: {}", self.show_body_names);
    }

    /// G-key semantics: if any grid is visible, hide all; otherwise show all three.
    pub fn toggle_all_grids(&mut self) {
        let target = !self.any_grid_visible();
        self.show_grid_xz = target;
        self.show_grid_xy = target;
        self.show_grid_yz = target;
        tracing::info!("Grids: {}", self.any_grid_visible());
    }

    /// Flip the debug-arrows overlay flag.
    pub fn toggle_arrows(&mut self) {
        self.show_arrows = !self.show_arrows;
        tracing::info!("Debug arrows: {}", self.show_arrows);
    }

    /// Flip the off-screen body indicators flag.
    pub fn toggle_offscreen_indicators(&mut self) {
        self.show_offscreen_indicators = !self.show_offscreen_indicators;
        tracing::info!("Off-screen indicators: {}", self.show_offscreen_indicators);
    }
}

impl Default for ViewOptions {
    fn default() -> Self {
        Self::new()
    }
}

/// Egui rendering plumbing — context, winit input state, and wgpu renderer.
pub struct EguiLayer {
    /// Egui context shared between input handling and rendering.
    pub ctx: egui::Context,
    /// Winit integration: translates window events into egui input.
    pub state: egui_winit::State,
    /// Wgpu integration: tessellated egui draw calls.
    pub renderer: egui_wgpu::Renderer,
}

impl EguiLayer {
    /// Initialize all three egui sub-systems against the given window and surface.
    pub fn new(
        device: &wgpu::Device,
        window: &Arc<winit::window::Window>,
        surface_format: wgpu::TextureFormat,
    ) -> Self {
        let ctx = egui::Context::default();
        let state = egui_winit::State::new(
            ctx.clone(),
            egui::ViewportId::ROOT,
            window.as_ref(),
            None,
            None,
            Some(device.limits().max_texture_dimension_2d as usize),
        );
        let renderer = egui_wgpu::Renderer::new(
            device,
            surface_format,
            egui_wgpu::RendererOptions::default(),
        );
        Self {
            ctx,
            state,
            renderer,
        }
    }
}
