use std::sync::Arc;

use miette::IntoDiagnostic;
use winit::window::Window;

use crate::texture;

/// Window-bound wgpu plumbing: instance/adapter/device/queue plus the swapchain
/// surface and its companion depth buffer.
///
/// The surface starts unconfigured; the first `resize` call configures it and
/// flips `is_surface_configured` to true.
pub struct GpuContext {
    /// The render surface backed by the window.
    pub surface: wgpu::Surface<'static>,
    /// Logical GPU device used to create resources.
    pub device: wgpu::Device,
    /// Submission queue paired with `device`.
    pub queue: wgpu::Queue,
    /// Current surface configuration (size, format, present mode).
    pub config: wgpu::SurfaceConfiguration,
    /// Whether `surface.configure` has been called at least once. The first
    /// frame is skipped until this is true.
    pub is_surface_configured: bool,
    /// The window the surface is bound to.
    pub window: Arc<Window>,
    /// Depth texture matching the current swapchain size; recreated on resize.
    pub depth_texture: texture::Texture,
}

impl GpuContext {
    /// Bring up the full wgpu stack against the given window.
    ///
    /// Picks the primary backend natively and WebGL in the browser, requests
    /// `POLYGON_MODE_LINE` on native, prefers an sRGB surface format, and
    /// creates the initial depth texture matched to the window size.
    pub async fn new(window: Arc<Window>) -> miette::Result<Self> {
        let size = window.inner_size();

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

        let surface = instance.create_surface(window.clone()).into_diagnostic()?;

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

        let depth_texture =
            texture::Texture::create_depth_texture(&device, &config, "depth_texture");

        Ok(Self {
            surface,
            device,
            queue,
            config,
            is_surface_configured: false,
            window,
            depth_texture,
        })
    }

    /// Reconfigure the swapchain to the new size and recreate the depth texture.
    /// No-op if either dimension is zero.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        self.depth_texture =
            texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        self.is_surface_configured = true;
    }
}
