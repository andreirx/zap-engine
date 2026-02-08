/// Keplerian orbital mechanics — pure math, no engine dependencies.
///
/// Uses f64 throughout for precision (centuries × deg/century = large numbers).
/// Only convert to f32 at the final screen-coordinate step in game.rs.

const DEG_TO_RAD: f64 = std::f64::consts::PI / 180.0;

/// Keplerian orbital elements at J2000 epoch with secular rates per century.
/// Source: Standish (1992) / JPL approximate planetary positions.
#[derive(Debug, Clone, Copy)]
pub struct OrbitalElements {
    /// Semi-major axis (AU) at J2000
    pub a0: f64,
    /// Eccentricity at J2000
    pub e0: f64,
    /// Mean longitude (degrees) at J2000
    pub l0: f64,
    /// Mean longitude rate (degrees per Julian century)
    pub l_dot: f64,
    /// Longitude of perihelion (degrees) at J2000
    pub w0: f64,
    /// Longitude of perihelion rate (degrees per Julian century)
    pub w_dot: f64,
}

/// Convert days from J2000 to Julian centuries from J2000.
pub fn days_to_centuries(days_from_j2000: f64) -> f64 {
    days_from_j2000 / 36525.0
}

/// Solve Kepler's equation: E - e·sin(E) = M
/// Using Newton-Raphson iteration.
/// `mean_anomaly` in radians, returns eccentric anomaly in radians.
pub fn solve_kepler(mean_anomaly: f64, eccentricity: f64) -> f64 {
    let mut ea = mean_anomaly; // initial guess
    for _ in 0..15 {
        let delta = ea - eccentricity * ea.sin() - mean_anomaly;
        let derivative = 1.0 - eccentricity * ea.cos();
        ea -= delta / derivative;
        if delta.abs() < 1e-12 {
            break;
        }
    }
    ea
}

/// Calculate heliocentric (x, y) position in AU for given orbital elements
/// at `t_centuries` Julian centuries from J2000.
/// Returns (x_au, y_au) projected onto the ecliptic plane.
pub fn heliocentric_position(elements: &OrbitalElements, t_centuries: f64) -> (f64, f64) {
    let a = elements.a0; // semi-major axis stays ~constant over ±100 years
    let e = elements.e0;

    // Current mean longitude and longitude of perihelion
    let l = (elements.l0 + elements.l_dot * t_centuries) * DEG_TO_RAD;
    let w = (elements.w0 + elements.w_dot * t_centuries) * DEG_TO_RAD;

    // Mean anomaly = mean longitude - longitude of perihelion
    let m = l - w;

    // Solve Kepler's equation for eccentric anomaly
    let ea = solve_kepler(m, e);

    // True anomaly from eccentric anomaly
    let true_anomaly = 2.0
        * ((1.0 + e).sqrt() * (ea / 2.0).sin())
            .atan2((1.0 - e).sqrt() * (ea / 2.0).cos());

    // Heliocentric distance
    let r = a * (1.0 - e * ea.cos());

    // Position in ecliptic plane (rotated by longitude of perihelion)
    let angle = true_anomaly + w;
    (r * angle.cos(), r * angle.sin())
}

/// Convert days from J2000 to (year, month, day).
/// J2000.0 = January 1, 2000, 12:00 TT (Julian Day 2451545.0).
pub fn days_to_date(days_from_j2000: f64) -> (i32, u32, u32) {
    let jd = days_from_j2000 + 2451545.0;
    let z = (jd + 0.5).floor() as i64;
    let a = if z < 2299161 {
        z
    } else {
        let alpha = ((z as f64 - 1867216.25) / 36524.25).floor() as i64;
        z + 1 + alpha - alpha / 4
    };
    let b = a + 1524;
    let c = ((b as f64 - 122.1) / 365.25).floor() as i64;
    let d = (365.25 * c as f64).floor() as i64;
    let e = ((b - d) as f64 / 30.6001).floor() as i64;

    let day = (b - d - (30.6001 * e as f64).floor() as i64) as u32;
    let month = if e < 14 { (e - 1) as u32 } else { (e - 13) as u32 };
    let year = if month > 2 { (c - 4716) as i32 } else { (c - 4715) as i32 };

    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kepler_circular_orbit() {
        // For e=0, eccentric anomaly = mean anomaly
        let ea = solve_kepler(1.0, 0.0);
        assert!((ea - 1.0).abs() < 1e-10);
    }

    #[test]
    fn kepler_mercury_eccentricity() {
        // Mercury has e=0.2056 — solver should converge
        let m = 1.5; // radians
        let ea = solve_kepler(m, 0.2056);
        // Verify: ea - e*sin(ea) should equal m
        let residual = ea - 0.2056 * ea.sin() - m;
        assert!(residual.abs() < 1e-12, "residual = {residual}");
    }

    #[test]
    fn earth_at_j2000() {
        // At t=0 (J2000), Earth should be ~1 AU from the Sun
        let earth = OrbitalElements {
            a0: 1.00000, e0: 0.01671,
            l0: 100.464, l_dot: 35999.373,
            w0: 102.937, w_dot: 0.323,
        };
        let (x, y) = heliocentric_position(&earth, 0.0);
        let dist = (x * x + y * y).sqrt();
        assert!((dist - 1.0).abs() < 0.02, "Earth distance = {dist} AU");
    }

    #[test]
    fn date_j2000_epoch() {
        // days=0 should give approximately Jan 1, 2000
        // (Actually J2000.0 is Jan 1.5, so day=1 or 2 depending on rounding)
        let (year, month, _day) = days_to_date(0.0);
        assert_eq!(year, 2000);
        assert_eq!(month, 1);
    }

    #[test]
    fn date_known_date() {
        // March 20, 2000 = J2000 + 79 days (approx)
        let (year, month, day) = days_to_date(79.0);
        assert_eq!(year, 2000);
        assert_eq!(month, 3);
        assert!(day >= 20 && day <= 21, "day = {day}");
    }

    #[test]
    fn date_negative_days() {
        // 365 days before J2000 ≈ Jan 1999
        let (year, _month, _day) = days_to_date(-365.0);
        assert_eq!(year, 1999);
    }
}
