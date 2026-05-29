use glam::Vec3;

/// Scale factor: 1 astronomical unit = `AU_TO_SCENE` scene units.
///
/// Earth's semi-major axis (1 AU) maps to 10 scene units, consistent with the
/// existing setup.
pub const AU_TO_SCENE: f64 = 10.0;

/// Julian Day of the J2000.0 epoch (2000-01-01 12:00 TT).
const J2000_JDE: f64 = 2_451_545.0;

/// Seconds in one day (24 h × 60 min × 60 s).
pub(crate) const SEC_PER_DAY: f64 = 24.0 * 60.0 * 60.0;

/// Identifies which VSOP87A planetary solution to use for position computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Vsop87Body {
    Mercury,
    Venus,
    Earth,
    Moon,
    Mars,
    Jupiter,
    Saturn,
    Uranus,
    Neptune,
}

/// Determines how a celestial body's orbital position is computed each frame.
#[derive(Clone, Copy, Debug)]
pub enum OrbitalModel {
    /// The body sits permanently at the scene origin (the Sun).
    Fixed,
    /// Simple circular orbit in the parent's XZ plane.
    Parametric { radius: f32, angular_velocity: f32 },
    /// Position from the VSOP87A heliocentric rectangular theory (in AU, J2000 ecliptic).
    Vsop87 { body: Vsop87Body },
}

/// Converts simulation time (seconds since J2000.0) to a Julian Ephemeris Date.
pub fn sim_time_to_jde(sim_time_s: f64) -> f64 {
    J2000_JDE + sim_time_s / SEC_PER_DAY
}

/// Converts a proleptic Gregorian date to a Julian Day at 0:00 UT of that date.
///
/// Algorithm from Meeus, *Astronomical Algorithms*, Chapter 7 (inverse of [`jde_to_gregorian`]).
pub fn gregorian_to_jde(year: i32, month: u8, day: u8) -> f64 {
    let (y, m) = if month <= 2 {
        (year - 1, month as i32 + 12)
    } else {
        (year, month as i32)
    };
    let a = (y as f64 / 100.0).floor();
    let b = 2.0 - a + (a / 4.0).floor();
    (365.25 * (y as f64 + 4716.0)).floor() + (30.6001 * (m as f64 + 1.0)).floor() + day as f64 + b
        - 1524.5
}

/// Converts a Julian Day back to simulation time (seconds since J2000.0).
pub fn jde_to_sim_time(jde: f64) -> f64 {
    (jde - J2000_JDE) * SEC_PER_DAY
}

/// Converts a Julian Day to a proleptic Gregorian date `(year, month, day)`.
///
/// Algorithm from Meeus, *Astronomical Algorithms*, Chapter 7.
pub fn jde_to_gregorian(jde: f64) -> (i32, u8, u8) {
    let z = (jde + 0.5).floor();
    let a = if z < 2_299_161.0 {
        z
    } else {
        let alpha = ((z - 1_867_216.25) / 36_524.25).floor();
        z + 1.0 + alpha - (alpha / 4.0).floor()
    };
    let b = a + 1524.0;
    let c = ((b - 122.1) / 365.25).floor();
    let d = (365.25 * c).floor();
    let e = ((b - d) / 30.6001).floor();

    let day = (b - d - (30.6001 * e).floor()) as u8;
    let month = (if e < 14.0 { e - 1.0 } else { e - 13.0 }) as u8;
    let year = (if month > 2 { c - 4716.0 } else { c - 4715.0 }) as i32;

    (year, month, day)
}

/// Compute the approximate orbital velocity of a body in scene units per second.
///
/// Uses a 1-hour forward difference for VSOP87, analytic tangent for Parametric,
/// and zero for Fixed bodies.
pub fn orbital_velocity(model: OrbitalModel, sim_time: f64) -> Vec3 {
    match model {
        OrbitalModel::Fixed => Vec3::ZERO,
        OrbitalModel::Parametric {
            radius,
            angular_velocity,
        } => {
            let angle = angular_velocity * sim_time as f32;
            Vec3::new(
                -angle.sin() * radius * angular_velocity,
                0.0,
                angle.cos() * radius * angular_velocity,
            )
        }
        OrbitalModel::Vsop87 { body } => {
            let dt = 3_600.0_f64; // 1 hour
            let p0 = heliocentric_position(body, sim_time);
            let p1 = heliocentric_position(body, sim_time + dt);
            (p1 - p0) / dt as f32
        }
    }
}

/// Computes the heliocentric scene-space position of a planet using VSOP87A.
///
/// `sim_time_s` is seconds elapsed since J2000.0 (2000-01-01 12:00 TT).
///
/// VSOP87A uses ecliptic J2000 rectangular coordinates: X and Y lie in the
/// ecliptic plane, Z points toward the ecliptic north pole. We remap to the
/// scene convention (Y = up, orbits in XZ plane):
///
/// ```text
/// scene_x = vsop87_x * AU_TO_SCENE
/// scene_y = vsop87_z * AU_TO_SCENE   (ecliptic north → scene up)
/// scene_z = vsop87_y * AU_TO_SCENE
/// ```
pub fn heliocentric_position(body: Vsop87Body, sim_time_s: f64) -> Vec3 {
    let jde = sim_time_to_jde(sim_time_s);
    let c = match body {
        Vsop87Body::Mercury => vsop87::vsop87a::mercury(jde),
        Vsop87Body::Venus => vsop87::vsop87a::venus(jde),
        Vsop87Body::Earth => vsop87::vsop87a::earth(jde),
        Vsop87Body::Moon => vsop87::vsop87a::earth_moon(jde),
        Vsop87Body::Mars => vsop87::vsop87a::mars(jde),
        Vsop87Body::Jupiter => vsop87::vsop87a::jupiter(jde),
        Vsop87Body::Saturn => vsop87::vsop87a::saturn(jde),
        Vsop87Body::Uranus => vsop87::vsop87a::uranus(jde),
        Vsop87Body::Neptune => vsop87::vsop87a::neptune(jde),
    };
    Vec3::new(
        (c.x * AU_TO_SCENE) as f32,
        (c.z * AU_TO_SCENE) as f32,
        (c.y * AU_TO_SCENE) as f32,
    )
}
