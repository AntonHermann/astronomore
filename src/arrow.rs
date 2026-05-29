use glam::Vec3;

use crate::grid::ColorVertex;

/// A colored 3-D arrow defined by start and end points in world space.
pub struct Arrow {
    /// World-space start position of the arrow.
    pub start: Vec3,
    /// World-space end position (tip) of the arrow.
    pub end: Vec3,
    /// Linear RGBA color for this arrow.
    pub color: [f32; 4],
}

/// GPU vertex buffer for a batch of world-space arrows (shaft + cone-outline head).
pub struct ArrowMesh {
    /// Vertex buffer containing [`ColorVertex`] line-list data for all arrows.
    pub vertex_buffer: wgpu::Buffer,
    /// Number of vertices currently written into the buffer.
    pub vertex_count: u32,
    capacity: u32,
}

impl ArrowMesh {
    /// Allocate a vertex buffer large enough for `capacity` vertices.
    pub fn new(device: &wgpu::Device, capacity: u32) -> Self {
        let byte_size = capacity as u64 * std::mem::size_of::<ColorVertex>() as u64;
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Arrow Vertex Buffer"),
            size: byte_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            vertex_buffer,
            vertex_count: 0,
            capacity,
        }
    }

    /// Rebuild the vertex buffer contents from `arrows`, clamping to the allocated capacity.
    pub fn update(&mut self, queue: &wgpu::Queue, arrows: &[Arrow]) {
        let vertices: Vec<ColorVertex> = arrows.iter().flat_map(arrow_vertices).collect();
        let count = vertices.len().min(self.capacity as usize);
        self.vertex_count = count as u32;
        if count > 0 {
            queue.write_buffer(
                &self.vertex_buffer,
                0,
                bytemuck::cast_slice(&vertices[..count]),
            );
        }
    }
}

/// Generate [`ColorVertex`] line-list pairs for one arrow (shaft + 4-spoke cone head).
fn arrow_vertices(arrow: &Arrow) -> Vec<ColorVertex> {
    let mut verts = Vec::with_capacity(18);
    let c = arrow.color;
    let start = arrow.start;
    let end = arrow.end;
    let shaft_vec = end - start;
    let len = shaft_vec.length();
    if len < 1e-6 {
        return verts;
    }
    let dir = shaft_vec / len;

    // --- shaft ---
    verts.push(ColorVertex {
        position: start.into(),
        color: c,
    });
    verts.push(ColorVertex {
        position: end.into(),
        color: c,
    });

    // --- arrowhead (cone outline) ---
    let head_len = (len * 0.3).max(0.2);
    let head_rad = head_len * 0.5;

    // pick a vector not parallel to dir for the cross product
    let up = if dir.y.abs() < 0.9 { Vec3::Y } else { Vec3::X };
    let perp1 = dir.cross(up).normalize();
    let perp2 = dir.cross(perp1).normalize();

    let tip = end;
    let back = end - dir * head_len;
    let p = [
        back + perp1 * head_rad,
        back - perp1 * head_rad,
        back + perp2 * head_rad,
        back - perp2 * head_rad,
    ];

    // 4 spoke lines: tip → ring point
    for &pt in &p {
        verts.push(ColorVertex {
            position: tip.into(),
            color: c,
        });
        verts.push(ColorVertex {
            position: pt.into(),
            color: c,
        });
    }
    // 4 ring lines connecting adjacent ring points
    let ring = [p[0], p[2], p[1], p[3]];
    for i in 0..4 {
        verts.push(ColorVertex {
            position: ring[i].into(),
            color: c,
        });
        verts.push(ColorVertex {
            position: ring[(i + 1) % 4].into(),
            color: c,
        });
    }

    verts
}

/// Extension trait — draw the arrow mesh on a render pass that already has the
/// grid pipeline and camera bind group set.
pub trait DrawArrows<'a> {
    /// Bind the arrow vertex buffer and issue a non-indexed draw call.
    fn draw_arrows(&mut self, mesh: &'a ArrowMesh);
}

impl<'a, 'b: 'a> DrawArrows<'b> for wgpu::RenderPass<'a> {
    fn draw_arrows(&mut self, mesh: &'b ArrowMesh) {
        if mesh.vertex_count == 0 {
            return;
        }
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.draw(0..mesh.vertex_count, 0..1);
    }
}
