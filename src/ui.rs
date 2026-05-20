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
        }
    }

    /// Flip the wireframe flag.
    pub fn toggle_wireframe(&mut self) {
        self.wireframe = !self.wireframe;
    }

    /// Flip the normals-overlay flag.
    pub fn toggle_normals(&mut self) {
        self.show_normals = !self.show_normals;
    }

    /// True if at least one grid plane is currently visible.
    pub fn any_grid_visible(&self) -> bool {
        self.show_grid_xz || self.show_grid_xy || self.show_grid_yz
    }

    /// G-key semantics: if any grid is visible, hide all; otherwise show all three.
    pub fn toggle_all_grids(&mut self) {
        let target = !self.any_grid_visible();
        self.show_grid_xz = target;
        self.show_grid_xy = target;
        self.show_grid_yz = target;
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
