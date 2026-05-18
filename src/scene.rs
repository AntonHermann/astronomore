use crate::celestial_body::CelestialBody;

pub struct Scene {
    pub celestial_bodies: Vec<CelestialBody>,
    pub model_bind_group_layout: wgpu::BindGroupLayout,
}

impl Scene {
    pub fn new(device: &wgpu::Device) -> Self {
        let model_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX, // only the vertex shader needs to see this
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
                label: Some("model_bind_group_layout"),
            });
        Self {
            celestial_bodies: Vec::new(),
            model_bind_group_layout,
        }
    }

    pub fn add_celestial_body(&mut self, body: CelestialBody) {
        self.celestial_bodies.push(body);
        self.validate_parent_ordering();
    }

    /// Validate that parents are always before their children in the list
    fn validate_parent_ordering(&self) {
        for (i, cb) in self.celestial_bodies.iter().enumerate() {
            if let Some(parent_id) = cb.orbital_parameters.parent_id {
                debug_assert!(
                    parent_id < i,
                    "Parent celestial body must be added before its children"
                );
            }
        }
    }

    /// TODO: change from Duration to sim_time
    pub fn update(&mut self, sim_time: f32, queue: &wgpu::Queue) {
        // update celestial bodies (including their local transforms)
        for cb in &mut self.celestial_bodies {
            cb.update(sim_time);
        }

        // calculate global transforms and update model uniforms for all celestial bodies
        // assumtion: parents are always before their children in the list, so we can just iterate through the list and update each celestial body
        let mut model_world_transforms = Vec::with_capacity(self.celestial_bodies.len());
        for i in 0..self.celestial_bodies.len() {
            // orbital transform relative to parent
            let relative_orbital_transform = self.celestial_bodies[i].orbital_transform;
            // orbital transform relative to world (i.e. combined with parent transforms)
            let world_transform = match self.celestial_bodies[i].orbital_parameters.parent_id {
                Some(parent_id) => model_world_transforms[parent_id] * relative_orbital_transform,
                None => relative_orbital_transform,
            };
            model_world_transforms.push(world_transform);

            // update the model uniform for this celestial body
            self.celestial_bodies[i].update_model_uniform(world_transform);
            // write the updated model uniform to the GPU
            queue.write_buffer(
                &self.celestial_bodies[i].model_buffer,
                0,
                bytemuck::cast_slice(&[self.celestial_bodies[i].model_uniform]),
            );
        }
    }
}
