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
impl std::fmt::Display for Vertex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "pos: [{:4.1} {:4.1} {:4.1}], uv: [{:4.3} {:4.3}]",
            self.position[0],
            self.position[1],
            self.position[2],
            self.tex_coords[0],
            self.tex_coords[1]
        )
    }
}

pub struct Mesh {
    #[allow(dead_code)]
    name: String,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_elements: u32,
}

#[allow(unused)]
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

    pub fn sphere(device: &wgpu::Device) -> Self {
        use std::f32::consts::PI;

        let num_meridians = 32u32;
        let num_parallels = 16u32;

        let lon_step = 2. * PI / num_meridians as f32;
        let lat_step = PI / num_parallels as f32;

        let mut vertices: Vec<Vertex> =
            Vec::with_capacity(((num_meridians + 1) * (num_parallels + 1)) as usize);
        let mut indices: Vec<[u32; 3]> =
            Vec::with_capacity((num_meridians * num_parallels) as usize * 2);

        // Vertices
        // in order to have correct texture coordinates:
        // - the top and bottom vertices are not generated separately, but `num_meridians + 1` x times at lat = 90° and lat = -90° respectively
        // - the last vertex of each parallel is the same as the first one (lon = 360° = 0°)
        for parallel_i in 0..=num_parallels {
            // lat: [90°, -90°] = [PI/2, -PI/2]
            let lat = PI / 2. - lat_step * parallel_i as f32;

            for meridian_i in 0..=num_meridians {
                let lon = lon_step * meridian_i as f32;

                vertices.push(Vertex {
                    position: [lat.cos() * lon.cos(), lat.sin(), lat.cos() * lon.sin()],
                    tex_coords: [1.0 - lon / (2. * PI), 0.5 - lat / PI],
                });
            }
        }

        // Indices
        for parallel_i in 0..num_parallels {
            for meridian_i in 0..num_meridians {
                // X--X  <- parallel_i   (s_curr)
                // |1/|
                // |/2|
                // X--X  <- parallel_i+1 (s_next)
                // ^  ^
                // i i+1
                let s_curr = parallel_i * (num_meridians + 1); // start of current parallel
                let s_next = s_curr + num_meridians + 1; // start of next parallel
                let i = meridian_i;
                indices.extend(&[
                    [s_curr + i, s_curr + i + 1, s_next + i],     // 1
                    [s_curr + i + 1, s_next + i + 1, s_next + i], // 2
                ]);
            }
        }

        let indices: Vec<u32> = indices.iter().flat_map(|tri| tri.to_vec()).collect();
        Self::new(device, "sphere", &vertices, &indices)
    }

    pub fn x_plane(device: &wgpu::Device) -> Self {
        #[rustfmt::skip]
        let vertices: &[Vertex] = &[
            Vertex { position: [0., -1., -1.], tex_coords: [0.2, 0.2], }, // A
            Vertex { position: [0.,  1., -1.], tex_coords: [0.2, 0.0], }, // B
            Vertex { position: [0.,  1.,  1.], tex_coords: [0.0, 0.0], }, // C
            Vertex { position: [0., -1.,  1.], tex_coords: [0.0, 0.2], }, // D
        ];

        #[rustfmt::skip]
        let indices: &[u32] = &[
            0, 1, 2, // ABC
            2, 3, 0, // CDA
        ];

        Self::new(device, "x-plane", vertices, indices)
    }
    pub fn y_plane(device: &wgpu::Device) -> Self {
        #[rustfmt::skip]
        let vertices: &[Vertex] = &[
            Vertex { position: [-1., 0., -1.], tex_coords: [0.8, 0.0], }, // A
            Vertex { position: [ 1., 0., -1.], tex_coords: [1.0, 0.0], }, // B
            Vertex { position: [ 1., 0.,  1.], tex_coords: [1.0, 0.2], }, // C
            Vertex { position: [-1., 0.,  1.], tex_coords: [0.8, 0.2], }, // D
        ];

        #[rustfmt::skip]
        let indices: &[u32] = &[
            0, 2, 1, // ACB
            2, 0, 3, // CDA
        ];

        Self::new(device, "y-plane", vertices, indices)
    }
    pub fn z_plane(device: &wgpu::Device) -> Self {
        #[rustfmt::skip]
        let vertices: &[Vertex] = &[
            Vertex { position: [-1., -1., 0.], tex_coords: [0.8, 1.0], }, // A
            Vertex { position: [ 1., -1., 0.], tex_coords: [1.0, 1.0], }, // B
            Vertex { position: [ 1.,  1., 0.], tex_coords: [1.0, 0.8], }, // C
            Vertex { position: [-1.,  1., 0.], tex_coords: [0.8, 0.8], }, // D
        ];

        #[rustfmt::skip]
        let indices: &[u32] = &[
            0, 1, 2, // ABC
            2, 3, 0, // CDA
        ];

        Self::new(device, "z-plane", vertices, indices)
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
