#![allow(dead_code)]

/// Typed index into the [`BODIES`] array. The discriminant equals the array index,
/// so `body_ids[SolarSystemBody::Earth as usize]` gives the corresponding [`crate::scene::BodyId`].
///
/// Invariant: every body's discriminant must be strictly greater than its parent's discriminant,
/// so that parents are always inserted into the scene before their children.
#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum SolarSystemBody {
    Sun = 0,
    Earth = 1,
    Moon = 2,
}

/// Pure data describing a celestial body before GPU resources are allocated.
pub struct BodyDef {
    pub name: &'static str,
    pub texture_path: &'static str,
    pub distance_from_parent: f32,
    pub radius: f32,
    pub angular_velocity: f32,
    /// Parent body. Its discriminant must be less than this body's discriminant.
    pub parent: Option<SolarSystemBody>,
}

pub const BODIES: &[BodyDef] = &[
    /* Sun   = 0 */
    BodyDef {
        name: "Sun",
        texture_path: "assets/textures/2k_sun.jpg",
        distance_from_parent: 0.0,
        radius: 4.0,
        angular_velocity: 0.0,
        parent: None,
    },
    /* Earth = 1 */
    BodyDef {
        name: "Earth",
        texture_path: "assets/textures/2k_earth_daymap.jpg",
        distance_from_parent: 10.0,
        radius: 1.0,
        angular_velocity: 0.2,
        parent: Some(SolarSystemBody::Sun),
    },
    /* Moon  = 2 */
    BodyDef {
        name: "Moon",
        texture_path: "assets/textures/2k_moon.jpg",
        distance_from_parent: 3.0,
        radius: 0.27,
        angular_velocity: 0.5,
        parent: Some(SolarSystemBody::Earth),
    },
];
