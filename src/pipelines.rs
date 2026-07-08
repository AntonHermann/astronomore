//! Render pipelines for the main scene draw pass.
//!
//! The main scene draw pass uses three pipelines:
//! 1. textured fill pipeline for celestial bodies,
//! 2. wireframe pipeline for celestial bodies (debug)
//! 3. unlit grid pipeline for coordinate grids and arrows.
//!
//! Pipelines 1 and 2 share the same vertex shader and layout, but have different fragment shaders and polygon modes.
//! Pipeline 3 uses a different vertex shader and layout, and draws line lists instead of triangles.
//!
//! ## Bind group layouts:
//! - Main: 0 = texture, 1 = camera, 2 = model, 3 = scene properties.
//! - Grid: 0 = camera, 1 = line brightness uniform.

use crate::{grid::ColorVertex, loader, mesh::Vertex, shader_loader, texture::Texture};

/// All render pipelines used by the main scene draw pass: the textured fill
/// pipeline for celestial bodies, its wireframe twin, and the unlit grid
/// pipeline.
///
/// Pipeline layouts are kept around so shader hot-reload can rebuild a
/// pipeline against the same layout without rederiving the bind group
/// arrangement.
pub struct Pipelines {
    /// Layout for the textured/wireframe pipelines (texture + camera + model bind groups).
    pub main_layout: wgpu::PipelineLayout,
    /// Pipeline used for solid-shaded geometry.
    pub fill: wgpu::RenderPipeline,
    /// Pipeline used when the wireframe toggle is on.
    pub wireframe: wgpu::RenderPipeline,
    /// Layout for the grid pipeline (camera + brightness bind groups).
    pub grid_layout: wgpu::PipelineLayout,
    /// Unlit line-list pipeline drawing the coordinate grids.
    pub grid: wgpu::RenderPipeline,
    /// Bind group layout for the line-brightness scalar uniform (group 1 of grid pipeline).
    pub brightness_bind_group_layout: wgpu::BindGroupLayout,
    /// Layout for the normals-overlay pipeline (camera + model bind groups).
    pub normals_layout: wgpu::PipelineLayout,
    /// Line-list pipeline drawing per-vertex normal vectors.
    pub normals: wgpu::RenderPipeline,
}

impl Pipelines {
    /// Load both shaders, validate them via naga, and build the three pipelines.
    ///
    /// Errors propagate as miette diagnostics from `shader_loader::validate_wgsl`
    /// or from asset loading; the constructor never panics on a bad shader.
    pub async fn new(
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
        texture_bg_layout: &wgpu::BindGroupLayout,
        camera_bg_layout: &wgpu::BindGroupLayout,
        model_bg_layout: &wgpu::BindGroupLayout,
        scene_props_bg_layout: &wgpu::BindGroupLayout,
    ) -> miette::Result<Self> {
        let main_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                Some(texture_bg_layout),
                Some(camera_bg_layout),
                Some(model_bg_layout),
                Some(scene_props_bg_layout),
            ],
            immediate_size: 0,
        });
        let brightness_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Line Brightness BGL"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });
        let grid_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Grid Pipeline Layout"),
            bind_group_layouts: &[Some(camera_bg_layout), Some(&brightness_bind_group_layout)],
            immediate_size: 0,
        });
        let normals_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Normals Pipeline Layout"),
            bind_group_layouts: &[Some(camera_bg_layout), Some(model_bg_layout)],
            immediate_size: 0,
        });

        let main_src = loader::load_str("src/shaders/shader.wgsl").await?;
        shader_loader::validate_wgsl("shader.wgsl", &main_src)?;
        let main_module = shader_loader::make_shader_module(device, "shader.wgsl", &main_src);
        let (fill, wireframe) =
            build_main_pipelines(device, &main_layout, &main_module, surface_format);

        let grid_src = loader::load_str("src/shaders/grid.wgsl").await?;
        shader_loader::validate_wgsl("grid.wgsl", &grid_src)?;
        let grid_module = shader_loader::make_shader_module(device, "grid.wgsl", &grid_src);
        let grid = build_grid_pipeline(device, &grid_layout, &grid_module, surface_format);

        let normals_src = loader::load_str("src/shaders/normals.wgsl").await?;
        shader_loader::validate_wgsl("normals.wgsl", &normals_src)?;
        let normals_module =
            shader_loader::make_shader_module(device, "normals.wgsl", &normals_src);
        let normals =
            build_normals_pipeline(device, &normals_layout, &normals_module, surface_format);

        Ok(Self {
            main_layout,
            fill,
            wireframe,
            grid_layout,
            grid,
            brightness_bind_group_layout,
            normals_layout,
            normals,
        })
    }

    /// Reread `shader.wgsl` from disk and rebuild the fill + wireframe pipelines.
    ///
    /// On validation error the miette diagnostic is printed to stderr and the
    /// existing pipelines are left unchanged. Native-only.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_reload_main_shader(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) {
        let src = match std::fs::read_to_string("src/shaders/shader.wgsl") {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("shader.wgsl lesen fehlgeschlagen: {e}");
                return;
            }
        };
        match shader_loader::validate_wgsl("shader.wgsl", &src) {
            Ok(()) => {
                let module = shader_loader::make_shader_module(device, "shader.wgsl", &src);
                (self.fill, self.wireframe) =
                    build_main_pipelines(device, &self.main_layout, &module, surface_format);
                tracing::info!("shader.wgsl neu geladen");
            }
            Err(e) => tracing::error!(error = ?e, "shader validation failed"),
        }
    }

    /// Reread `grid.wgsl` from disk and rebuild the grid pipeline.
    ///
    /// On validation error the miette diagnostic is printed to stderr and the
    /// existing pipeline is left unchanged. Native-only.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_reload_grid_shader(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) {
        let src = match std::fs::read_to_string("src/shaders/grid.wgsl") {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("grid.wgsl lesen fehlgeschlagen: {e}");
                return;
            }
        };
        match shader_loader::validate_wgsl("grid.wgsl", &src) {
            Ok(()) => {
                let module = shader_loader::make_shader_module(device, "grid.wgsl", &src);
                self.grid = build_grid_pipeline(device, &self.grid_layout, &module, surface_format);
                tracing::info!("grid.wgsl neu geladen");
            }
            Err(e) => tracing::error!(error = ?e, "shader validation failed"),
        }
    }

    /// Reread `normals.wgsl` from disk and rebuild the normals pipeline.
    ///
    /// On validation error the miette diagnostic is printed to stderr and the
    /// existing pipeline is left unchanged. Native-only.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn try_reload_normals_shader(
        &mut self,
        device: &wgpu::Device,
        surface_format: wgpu::TextureFormat,
    ) {
        let src = match std::fs::read_to_string("src/shaders/normals.wgsl") {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("normals.wgsl lesen fehlgeschlagen: {e}");
                return;
            }
        };
        match shader_loader::validate_wgsl("normals.wgsl", &src) {
            Ok(()) => {
                let module = shader_loader::make_shader_module(device, "normals.wgsl", &src);
                self.normals =
                    build_normals_pipeline(device, &self.normals_layout, &module, surface_format);
                tracing::info!("normals.wgsl neu geladen");
            }
            Err(e) => tracing::error!(error = ?e, "shader validation failed"),
        }
    }
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
                buffers: &[Vertex::LAYOUT],
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
                format: Texture::DEPTH_FORMAT,
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
            format: Texture::DEPTH_FORMAT,
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
            format: Texture::DEPTH_FORMAT,
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
