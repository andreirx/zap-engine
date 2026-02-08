/// Planetary data — J2000 orbital elements and visual properties.
///
/// Orbital elements from Standish (1992) / JPL approximate planetary positions.
/// Visual properties are exaggerated for readability (real planets would be sub-pixel).

use crate::orbit::OrbitalElements;

/// Planet index constants.
pub const MERCURY: usize = 0;
pub const VENUS: usize = 1;
pub const EARTH: usize = 2;
pub const MARS: usize = 3;
pub const JUPITER: usize = 4;
pub const SATURN: usize = 5;
pub const URANUS: usize = 6;
pub const NEPTUNE: usize = 7;
pub const PLUTO: usize = 8;
pub const PLANET_COUNT: usize = 9;

/// Names for UI display (indexed by planet constant).
pub const PLANET_NAMES: [&str; PLANET_COUNT] = [
    "Mercury", "Venus", "Earth", "Mars", "Jupiter", "Saturn", "Uranus", "Neptune", "Pluto",
];

/// Orbital periods in Earth days (for UI display).
pub const ORBITAL_PERIODS: [f64; PLANET_COUNT] = [
    87.97,    // Mercury
    224.70,   // Venus
    365.26,   // Earth
    686.98,   // Mars
    4332.59,  // Jupiter
    10759.22, // Saturn
    30688.5,  // Uranus
    60182.0,  // Neptune
    90560.0,  // Pluto
];

// ── Sun ──────────────────────────────────────────────────────────────

pub const SUN_RADIUS_PX: f32 = 16.0;
pub const SUN_COLOR: (f32, f32, f32) = (1.0, 0.9, 0.5);
pub const SUN_EMISSIVE: f32 = 3.5;
pub const SUN_SHININESS: f32 = 8.0;

// ── Planets ──────────────────────────────────────────────────────────

/// Visual properties for one planet.
pub struct PlanetVisuals {
    pub radius_px: f32,
    pub color: (f32, f32, f32),
    pub emissive: f32,
    pub shininess: f32,
}

/// J2000 Keplerian elements for all 8 planets.
pub fn planet_elements() -> [OrbitalElements; PLANET_COUNT] {
    [
        // Mercury
        OrbitalElements {
            a0: 0.38710, e0: 0.20563,
            l0: 252.251, l_dot: 149472.675,
            w0: 77.457,  w_dot: 0.159,
        },
        // Venus
        OrbitalElements {
            a0: 0.72333, e0: 0.00677,
            l0: 181.980, l_dot: 58517.816,
            w0: 131.564, w_dot: 0.053,
        },
        // Earth
        OrbitalElements {
            a0: 1.00000, e0: 0.01671,
            l0: 100.464, l_dot: 35999.373,
            w0: 102.937, w_dot: 0.323,
        },
        // Mars
        OrbitalElements {
            a0: 1.52368, e0: 0.09340,
            l0: 355.453, l_dot: 19140.300,
            w0: 336.060, w_dot: 0.443,
        },
        // Jupiter
        OrbitalElements {
            a0: 5.20260, e0: 0.04849,
            l0: 34.351,  l_dot: 3034.906,
            w0: 14.331,  w_dot: 0.172,
        },
        // Saturn
        OrbitalElements {
            a0: 9.55491, e0: 0.05551,
            l0: 50.077,  l_dot: 1222.114,
            w0: 93.057,  w_dot: 0.312,
        },
        // Uranus
        OrbitalElements {
            a0: 19.21845, e0: 0.04630,
            l0: 314.055,  l_dot: 428.467,
            w0: 173.005,  w_dot: 0.030,
        },
        // Neptune
        OrbitalElements {
            a0: 30.11039, e0: 0.00899,
            l0: 304.349,  l_dot: 218.486,
            w0: 48.120,   w_dot: 0.012,
        },
        // Pluto (dwarf planet — included by popular demand)
        OrbitalElements {
            a0: 39.482,  e0: 0.2488,
            l0: 238.929, l_dot: 145.18,
            w0: 224.067, w_dot: 0.006,
        },
    ]
}

/// Visual properties per planet (indexed by planet constant).
pub fn planet_visuals() -> [PlanetVisuals; PLANET_COUNT] {
    [
        PlanetVisuals { radius_px: 3.0,  color: (0.60, 0.55, 0.50), emissive: 0.0, shininess: 16.0 }, // Mercury
        PlanetVisuals { radius_px: 4.5,  color: (0.90, 0.75, 0.40), emissive: 0.0, shininess: 16.0 }, // Venus
        PlanetVisuals { radius_px: 5.0,  color: (0.20, 0.40, 0.80), emissive: 0.0, shininess: 32.0 }, // Earth
        PlanetVisuals { radius_px: 4.0,  color: (0.80, 0.30, 0.15), emissive: 0.0, shininess: 16.0 }, // Mars
        PlanetVisuals { radius_px: 14.0, color: (0.80, 0.70, 0.50), emissive: 0.0, shininess: 16.0 }, // Jupiter
        PlanetVisuals { radius_px: 12.0, color: (0.85, 0.75, 0.50), emissive: 0.0, shininess: 16.0 }, // Saturn
        PlanetVisuals { radius_px: 7.0,  color: (0.50, 0.75, 0.85), emissive: 0.0, shininess: 16.0 }, // Uranus
        PlanetVisuals { radius_px: 6.5,  color: (0.25, 0.35, 0.80), emissive: 0.0, shininess: 16.0 }, // Neptune
        PlanetVisuals { radius_px: 2.0,  color: (0.70, 0.60, 0.50), emissive: 0.0, shininess: 16.0 }, // Pluto
    ]
}

// ── Moons ────────────────────────────────────────────────────────────

/// Simplified moon description — circular orbit relative to parent planet.
pub struct MoonDesc {
    pub name: &'static str,
    /// Index into PLANETS array.
    pub parent: usize,
    /// Screen-space orbital radius (pixels from parent center).
    pub orbit_radius_px: f32,
    /// Orbital period in Earth days.
    pub period_days: f64,
    /// Visual radius in pixels.
    pub radius_px: f32,
    /// SDF color (r, g, b).
    pub color: (f32, f32, f32),
}

pub fn moon_data() -> [MoonDesc; 10] {
    [
        MoonDesc { name: "Moon",     parent: EARTH,   orbit_radius_px: 15.0, period_days: 27.32,  radius_px: 2.0, color: (0.70, 0.70, 0.70) },
        MoonDesc { name: "Phobos",   parent: MARS,    orbit_radius_px: 8.0,  period_days: 0.32,   radius_px: 1.0, color: (0.50, 0.45, 0.40) },
        MoonDesc { name: "Deimos",   parent: MARS,    orbit_radius_px: 12.0, period_days: 1.26,   radius_px: 0.8, color: (0.55, 0.50, 0.45) },
        MoonDesc { name: "Io",       parent: JUPITER, orbit_radius_px: 20.0, period_days: 1.77,   radius_px: 2.0, color: (0.90, 0.80, 0.30) },
        MoonDesc { name: "Europa",   parent: JUPITER, orbit_radius_px: 25.0, period_days: 3.55,   radius_px: 1.8, color: (0.80, 0.70, 0.50) },
        MoonDesc { name: "Ganymede", parent: JUPITER, orbit_radius_px: 30.0, period_days: 7.15,   radius_px: 2.5, color: (0.60, 0.55, 0.50) },
        MoonDesc { name: "Callisto", parent: JUPITER, orbit_radius_px: 36.0, period_days: 16.69,  radius_px: 2.2, color: (0.35, 0.30, 0.28) },
        MoonDesc { name: "Titan",    parent: SATURN,  orbit_radius_px: 24.0, period_days: 15.95,  radius_px: 2.5, color: (0.85, 0.70, 0.30) },
        MoonDesc { name: "Triton",   parent: NEPTUNE, orbit_radius_px: 14.0, period_days: -5.877, radius_px: 2.0, color: (0.60, 0.70, 0.80) },
        MoonDesc { name: "Charon",   parent: PLUTO,   orbit_radius_px: 8.0,  period_days: 6.387,  radius_px: 1.2, color: (0.55, 0.55, 0.55) },
    ]
}

// ── Asteroid belt ────────────────────────────────────────────────────

pub const ASTEROID_COUNT: usize = 50;
/// Semi-major axis range for the main belt (AU).
pub const ASTEROID_AU_MIN: f64 = 2.2;
pub const ASTEROID_AU_MAX: f64 = 3.2;
/// Eccentricity range.
pub const ASTEROID_ECC_MAX: f64 = 0.15;

/// Deterministic hash for asteroid generation (no external rand crate).
pub fn asteroid_hash(seed: u32) -> u32 {
    let mut n = seed;
    n = n.wrapping_mul(2654435761);
    n ^= n >> 16;
    n = n.wrapping_mul(2246822519);
    n ^= n >> 13;
    n
}

/// Generate orbital elements for N asteroids using deterministic pseudo-random.
pub fn generate_asteroid_orbits() -> Vec<OrbitalElements> {
    let mut orbits = Vec::with_capacity(ASTEROID_COUNT);
    for i in 0..ASTEROID_COUNT {
        let h1 = asteroid_hash(i as u32 * 7 + 31);
        let h2 = asteroid_hash(i as u32 * 13 + 97);
        let h3 = asteroid_hash(i as u32 * 19 + 151);
        let h4 = asteroid_hash(i as u32 * 23 + 211);
        let h5 = asteroid_hash(i as u32 * 29 + 277);

        let frac = |h: u32| (h as f64) / (u32::MAX as f64);

        let a = ASTEROID_AU_MIN + frac(h1) * (ASTEROID_AU_MAX - ASTEROID_AU_MIN);
        let e = frac(h2) * ASTEROID_ECC_MAX;
        let l0 = frac(h3) * 360.0;
        let w0 = frac(h4) * 360.0;
        // Kepler's 3rd law: period ∝ a^(3/2) → mean motion = 360 / (a^1.5 * 365.25) deg/day
        // l_dot = degrees per Julian century = (360 / (a^1.5 * 365.25)) * 36525
        let l_dot = 36525.0 * 360.0 / (a.powf(1.5) * 365.25);
        // Perihelion precession — small random drift
        let w_dot = (frac(h5) - 0.5) * 0.1;

        orbits.push(OrbitalElements { a0: a, e0: e, l0, l_dot, w0, w_dot });
    }
    orbits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planet_count_matches() {
        assert_eq!(planet_elements().len(), PLANET_COUNT);
        assert_eq!(planet_visuals().len(), PLANET_COUNT);
        assert_eq!(PLANET_NAMES.len(), PLANET_COUNT);
        assert_eq!(ORBITAL_PERIODS.len(), PLANET_COUNT);
    }

    #[test]
    fn moon_parents_valid() {
        for moon in &moon_data() {
            assert!(moon.parent < PLANET_COUNT, "moon {} has invalid parent", moon.name);
        }
    }

    #[test]
    fn asteroid_orbits_in_range() {
        let orbits = generate_asteroid_orbits();
        assert_eq!(orbits.len(), ASTEROID_COUNT);
        for orbit in &orbits {
            assert!(orbit.a0 >= ASTEROID_AU_MIN, "asteroid too close: {}", orbit.a0);
            assert!(orbit.a0 <= ASTEROID_AU_MAX, "asteroid too far: {}", orbit.a0);
            assert!(orbit.e0 >= 0.0 && orbit.e0 <= ASTEROID_ECC_MAX);
        }
    }

    #[test]
    fn asteroid_hash_deterministic() {
        assert_eq!(asteroid_hash(42), asteroid_hash(42));
        assert_ne!(asteroid_hash(0), asteroid_hash(1));
    }
}
