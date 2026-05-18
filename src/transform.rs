pub struct Transform {
    pub position: glam::Vec3,
    pub rotation: glam::Quat,
    pub scale: glam::Vec3,
    pub parent_id: Option<usize>,
}

#[allow(dead_code)]
impl Transform {
    pub fn new(position: glam::Vec3, rotation: glam::Quat, scale: glam::Vec3) -> Self {
        Self {
            position,
            rotation,
            scale,
            parent_id: None,
        }
    }

    pub fn local_matrix(&self) -> glam::Mat4 {
        glam::Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            position: glam::Vec3::ZERO,
            rotation: glam::Quat::IDENTITY,
            scale: glam::Vec3::ONE,
            parent_id: None,
        }
    }
}
