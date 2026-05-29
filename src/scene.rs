use crate::celestial_body::{CelestialBody, DrawCelestialBody};

/// Index of a celestial body in the scene's celestial_bodies list.
/// Can only be constructed by the scene when adding a celestial body, and is used to reference celestial bodies (e.g. as parents in orbital parameters) without exposing the internal list structure of the scene.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct BodyId(usize);

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

    pub fn add_celestial_body(
        &mut self,
        mut body: CelestialBody,
        parent: Option<BodyId>,
    ) -> BodyId {
        let id = BodyId(self.celestial_bodies.len());
        assert!(
            parent.as_ref().is_none_or(|p| p.0 < id.0),
            "Parent celestial body must be added before its children"
        );
        body.orbital_parameters.parent_id = parent.map(|p| p.0);
        self.celestial_bodies.push(body);
        self.validate_parent_ordering();
        id
    }

    /// Validate that parents are always before their children in the list
    fn validate_parent_ordering(&self) {
        for (i, cb) in self.celestial_bodies.iter().enumerate() {
            if let Some(parent_id) = cb.orbital_parameters.parent_id {
                assert!(
                    parent_id < i,
                    "Parent celestial body must be added before its children"
                );
            }
        }
    }

    pub fn update(&mut self, sim_time: f64, queue: &wgpu::Queue) {
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

    /// Iterate over all bodies in the scene, yielding `(BodyId, name)` pairs.
    /// Useful for building UI selectors without exposing the internal index.
    pub fn iter_bodies(&self) -> impl Iterator<Item = (BodyId, &str)> {
        self.celestial_bodies
            .iter()
            .enumerate()
            .map(|(i, b)| (BodyId(i), b.name.as_str()))
    }

    /// Iterate through ancestor chain until root body is reached,
    /// multiply parent transform (relative to its parent) to the left of current transform
    pub fn get_body_orbital_transform(&self, body_id: BodyId) -> glam::Mat4 {
        let mut curr_id = body_id.0;
        let mut orbital_transform = self.celestial_bodies[curr_id].orbital_transform;
        while let Some(parent_id) = self.celestial_bodies[curr_id].orbital_parameters.parent_id {
            let relative_parent_transform = self.celestial_bodies[parent_id].orbital_transform;
            orbital_transform = relative_parent_transform * orbital_transform;
            curr_id = parent_id;
        }
        orbital_transform
    }
}

pub trait DrawScene<'a>: DrawCelestialBody<'a> {
    fn draw_scene(
        &mut self,
        scene: &'a Scene,
        camera_bind_group: &wgpu::BindGroup,
        scene_properties_bind_group: &wgpu::BindGroup,
    );
}
impl<'a, 'b: 'a> DrawScene<'b> for wgpu::RenderPass<'a> {
    fn draw_scene(
        &mut self,
        scene: &'b Scene,
        camera_bind_group: &wgpu::BindGroup,
        scene_properties_bind_group: &wgpu::BindGroup,
    ) {
        self.set_bind_group(3, scene_properties_bind_group, &[]);

        tracing::trace!(
            count = scene.celestial_bodies.len(),
            "draw celestial bodies"
        );
        for cb in &scene.celestial_bodies {
            tracing::trace!(name = cb.name, "draw body");
            self.draw_celestial_body(cb, camera_bind_group);
        }
    }
}
