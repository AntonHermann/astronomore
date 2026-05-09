use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    // // could be used instead of speciying `attributes` explicitly below:
    // const ATTRIBS: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2];

    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
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
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

pub struct Mesh {
    #[allow(dead_code)]
    name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

impl Mesh {
    pub fn new(device: &wgpu::Device, name: &str, vertices: &[Vertex], indices: &[u32]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("[{name}] Vertex Buffer")),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(&format!("[{name}] Index Buffer")),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        let num_elements = indices.len() as u32;

        Self {
            name: name.into(),
            vertex_buffer,
            index_buffer,
            num_elements,
        }
    }

    pub fn simple_pentagon(device: &wgpu::Device) -> Self {
        #[rustfmt::skip]
        let vertices: &[Vertex] = &[
            Vertex { position: [-0.0868241 ,  0.49240386, 0.0], tex_coords: [0.4131759   , 0.00759614], }, // A
            Vertex { position: [-0.49513406,  0.06958647, 0.0], tex_coords: [0.0048659444, 0.43041354], }, // B
            Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453  , 0.949397  ], }, // C
            Vertex { position: [ 0.35966998, -0.3473291 , 0.0], tex_coords: [0.85967     , 0.84732914], }, // D
            Vertex { position: [ 0.44147372,  0.2347359 , 0.0], tex_coords: [0.9414737   , 0.2652641 ], }, // E
        ];

        #[rustfmt::skip]
        let indices: &[u32] = &[
            0, 1, 4, // ABE
            1, 2, 4, // BCE
            2, 3, 4, // CDE
        ];

        Self::new(device, "pentagon", vertices, indices)
    }
}

pub trait DrawMesh<'a> {
    fn draw_mesh(&mut self, mesh: &'a Mesh);
    fn draw_mesh_instanced(&mut self, mesh: &'a Mesh, instances: std::ops::Range<u32>);
}
impl<'a, 'b: 'a> DrawMesh<'b> for wgpu::RenderPass<'a> {
    fn draw_mesh(&mut self, mesh: &'b Mesh) {
        self.draw_mesh_instanced(mesh, 0..1);
    }

    fn draw_mesh_instanced(&mut self, mesh: &'b Mesh, instances: std::ops::Range<u32>) {
        self.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        self.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        self.draw_indexed(0..mesh.num_elements, 0, instances);
    }
}
