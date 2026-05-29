#![allow(dead_code)]

use crate::orbital::{OrbitalModel, Vsop87Body};

/// Typed index into the [`BODIES`] array. The discriminant equals the array index,
/// so `body_ids[SolarSystemBody::Earth as usize]` gives the corresponding [`crate::scene::BodyId`].
///
/// Invariant: every body's discriminant must be strictly greater than its parent's discriminant,
/// so that parents are always inserted into the scene before their children.
#[repr(usize)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Debug)]
pub enum SolarSystemBody {
    Sun = 0,
    Mercury = 1,
    Venus = 2,
    Earth = 3,
    Moon = 4,
    Mars = 5,
    Jupiter = 6,
    Saturn = 7,
    Uranus = 8,
    Neptune = 9,
}

/// Pure data describing a celestial body before GPU resources are allocated.
pub struct BodyDef {
    pub name: &'static str,
    pub texture_path: &'static str,
    /// Visual radius in scene units (Earth = 1.0).
    pub radius: f32,
    pub orbital_model: OrbitalModel,
    /// Parent body. Its discriminant must be less than this body's discriminant.
    pub parent: Option<SolarSystemBody>,
}

/// Moon orbital angular velocity: 2π / (27.32 days × 86 400 s/day) in rad/s.
const MOON_ANGULAR_VELOCITY: f32 = 2.662e-6;

pub const BODIES: &[BodyDef] = &[
    /* Sun = 0 */
    BodyDef {
        name: "Sun",
        texture_path: "assets/textures/2k_sun.jpg",
        radius: 4.0,
        orbital_model: OrbitalModel::Fixed,
        parent: None,
    },
    /* Mercury = 1 */
    BodyDef {
        name: "Mercury",
        texture_path: "assets/textures/2k_mercury.jpg",
        radius: 0.383,
        orbital_model: OrbitalModel::Vsop87 {
            body: Vsop87Body::Mercury,
        },
        parent: Some(SolarSystemBody::Sun),
    },
    /* Venus = 2 */
    BodyDef {
        name: "Venus",
        texture_path: "assets/textures/2k_venus_surface.jpg",
        radius: 0.949,
        orbital_model: OrbitalModel::Vsop87 {
            body: Vsop87Body::Venus,
        },
        parent: Some(SolarSystemBody::Sun),
    },
    /* Earth = 3 */
    BodyDef {
        name: "Earth",
        texture_path: "assets/textures/2k_earth_daymap.jpg",
        radius: 1.0,
        orbital_model: OrbitalModel::Vsop87 {
            body: Vsop87Body::Earth,
        },
        parent: Some(SolarSystemBody::Sun),
    },
    /* Moon = 4 — simplified parametric orbit around Earth; real distance is ~0.026 scene units
     * which would be invisible, so it is kept at 3.0 for visual clarity. */
    BodyDef {
        name: "Moon",
        texture_path: "assets/textures/2k_moon.jpg",
        radius: 0.273,
        orbital_model: OrbitalModel::Parametric {
            radius: 3.0,
            angular_velocity: MOON_ANGULAR_VELOCITY,
        },
        parent: Some(SolarSystemBody::Earth),
    },
    /* Mars = 5 */
    BodyDef {
        name: "Mars",
        texture_path: "assets/textures/2k_mars.jpg",
        radius: 0.532,
        orbital_model: OrbitalModel::Vsop87 {
            body: Vsop87Body::Mars,
        },
        parent: Some(SolarSystemBody::Sun),
    },
    /* Jupiter = 6 */
    BodyDef {
        name: "Jupiter",
        texture_path: "assets/textures/2k_jupiter.jpg",
        radius: 11.21,
        orbital_model: OrbitalModel::Vsop87 {
            body: Vsop87Body::Jupiter,
        },
        parent: Some(SolarSystemBody::Sun),
    },
    /* Saturn = 7 */
    BodyDef {
        name: "Saturn",
        texture_path: "assets/textures/2k_saturn.jpg",
        radius: 9.45,
        orbital_model: OrbitalModel::Vsop87 {
            body: Vsop87Body::Saturn,
        },
        parent: Some(SolarSystemBody::Sun),
    },
    /* Uranus = 8 */
    BodyDef {
        name: "Uranus",
        texture_path: "assets/textures/2k_uranus.jpg",
        radius: 4.01,
        orbital_model: OrbitalModel::Vsop87 {
            body: Vsop87Body::Uranus,
        },
        parent: Some(SolarSystemBody::Sun),
    },
    /* Neptune = 9 */
    BodyDef {
        name: "Neptune",
        texture_path: "assets/textures/2k_neptune.jpg",
        radius: 3.88,
        orbital_model: OrbitalModel::Vsop87 {
            body: Vsop87Body::Neptune,
        },
        parent: Some(SolarSystemBody::Sun),
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parent_precedes_child_in_array() {
        // The scene relies on parents being inserted before their children, which
        // holds iff every body's parent has a smaller discriminant (= array index).
        for (i, def) in BODIES.iter().enumerate() {
            if let Some(parent) = def.parent {
                assert!(
                    (parent as usize) < i,
                    "{} (index {i}) has parent with index {}",
                    def.name,
                    parent as usize
                );
            }
        }
    }

    #[test]
    fn discriminant_matches_array_index() {
        // body_ids[SolarSystemBody::X as usize] indexing relies on this.
        assert_eq!(SolarSystemBody::Sun as usize, 0);
        assert_eq!(SolarSystemBody::Earth as usize, 3);
        assert_eq!(SolarSystemBody::Neptune as usize, 9);
        assert_eq!(BODIES.len(), 10);
    }

    #[test]
    fn all_bodies_have_positive_radius() {
        for def in BODIES {
            assert!(def.radius > 0.0, "{} has radius {}", def.name, def.radius);
        }
    }
}
