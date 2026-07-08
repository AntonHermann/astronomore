//! Dbg helper: draw colored vertex + grids (used for xy, xz, yz planes)

use wgpu::util::DeviceExt;

/// A vertex carrying a world-space position and an RGBA color, used for unlit geometry like grids.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorVertex {
    /// World-space position.
    pub position: [f32; 3],
    /// Linear RGBA color.
    pub color: [f32; 4],
}

impl ColorVertex {
    /// Returns the [`wgpu::VertexBufferLayout`] for this vertex type.
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<ColorVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

const GRAY: [f32; 4] = [0.35, 0.35, 0.35, 1.0];
const RED: [f32; 4] = [0.9, 0.2, 0.2, 1.0];
const GREEN: [f32; 4] = [0.2, 0.9, 0.2, 1.0];
const BLUE: [f32; 4] = [0.2, 0.2, 0.9, 1.0];

/// A GPU-resident line-list mesh used to render a coordinate-plane grid.
pub struct GridMesh {
    /// Vertex buffer containing [`ColorVertex`] data for all grid lines.
    pub vertex_buffer: wgpu::Buffer,
    /// Number of vertices (two per line segment).
    pub num_vertices: u32,
}

impl GridMesh {
    /// Uploads `vertices` to a new GPU vertex buffer and returns a [`GridMesh`].
    pub fn build(device: &wgpu::Device, name: &str, vertices: Vec<ColorVertex>) -> Self {
        let num_vertices = vertices.len() as u32;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(name),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            vertex_buffer,
            num_vertices,
        }
    }

    /// Grid in the XZ plane (y = 0). X-axis = red, Z-axis = blue.
    pub fn xz_plane(device: &wgpu::Device, half_size: i32) -> Self {
        let mut verts = Vec::new();
        let n = half_size as f32;
        for i in -half_size..=half_size {
            let p = i as f32;
            // lines along Z (fixed X position)
            if i == 0 {
                verts.push(ColorVertex {
                    position: [0.0, 0.0, -n],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: RED,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, n],
                    color: RED,
                });
            } else {
                verts.push(ColorVertex {
                    position: [p, 0.0, -n],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [p, 0.0, n],
                    color: GRAY,
                });
            }
            // lines along X (fixed Z position)
            if i == 0 {
                verts.push(ColorVertex {
                    position: [-n, 0.0, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: BLUE,
                });
                verts.push(ColorVertex {
                    position: [n, 0.0, 0.0],
                    color: BLUE,
                });
            } else {
                verts.push(ColorVertex {
                    position: [-n, 0.0, p],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [n, 0.0, p],
                    color: GRAY,
                });
            }
        }
        Self::build(device, "Grid XZ", verts)
    }

    /// Grid in the XY plane (z = 0). X-axis = red, Y-axis = green.
    pub fn xy_plane(device: &wgpu::Device, half_size: i32) -> Self {
        let mut verts = Vec::new();
        let n = half_size as f32;
        for i in -half_size..=half_size {
            let p = i as f32;
            // lines along Y (fixed X position)
            if i == 0 {
                verts.push(ColorVertex {
                    position: [0.0, -n, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: RED,
                });
                verts.push(ColorVertex {
                    position: [0.0, n, 0.0],
                    color: RED,
                });
            } else {
                verts.push(ColorVertex {
                    position: [p, -n, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [p, n, 0.0],
                    color: GRAY,
                });
            }
            // lines along X (fixed Y position)
            if i == 0 {
                verts.push(ColorVertex {
                    position: [-n, 0.0, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: GREEN,
                });
                verts.push(ColorVertex {
                    position: [n, 0.0, 0.0],
                    color: GREEN,
                });
            } else {
                verts.push(ColorVertex {
                    position: [-n, p, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [n, p, 0.0],
                    color: GRAY,
                });
            }
        }
        Self::build(device, "Grid XY", verts)
    }

    /// Grid in the YZ plane (x = 0). Y-axis = green, Z-axis = blue.
    pub fn yz_plane(device: &wgpu::Device, half_size: i32) -> Self {
        let mut verts = Vec::new();
        let n = half_size as f32;
        for i in -half_size..=half_size {
            let p = i as f32;
            // lines along Z (fixed Y position)
            if i == 0 {
                verts.push(ColorVertex {
                    position: [0.0, 0.0, -n],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: GREEN,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, n],
                    color: GREEN,
                });
            } else {
                verts.push(ColorVertex {
                    position: [0.0, p, -n],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, p, n],
                    color: GRAY,
                });
            }
            // lines along Y (fixed Z position)
            if i == 0 {
                verts.push(ColorVertex {
                    position: [0.0, -n, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, 0.0, 0.0],
                    color: BLUE,
                });
                verts.push(ColorVertex {
                    position: [0.0, n, 0.0],
                    color: BLUE,
                });
            } else {
                verts.push(ColorVertex {
                    position: [0.0, -n, p],
                    color: GRAY,
                });
                verts.push(ColorVertex {
                    position: [0.0, n, p],
                    color: GRAY,
                });
            }
        }
        Self::build(device, "Grid YZ", verts)
    }
}

/// Extension trait for drawing a [`GridMesh`] in a render pass.
pub trait DrawGrid<'a> {
    /// Binds the grid's vertex buffer and issues a non-indexed draw call.
    fn draw_grid(&mut self, grid: &'a GridMesh);
}

impl<'a, 'b: 'a> DrawGrid<'b> for wgpu::RenderPass<'a> {
    fn draw_grid(&mut self, grid: &'b GridMesh) {
        self.set_vertex_buffer(0, grid.vertex_buffer.slice(..));
        self.draw(0..grid.num_vertices, 0..1);
    }
}
