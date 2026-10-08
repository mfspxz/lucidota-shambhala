// sham_spatial — sham-002: spatial engine.
// Coordinates, distances (haversine), bearings, turning radii, road widths,
// parking-geometry checks. Hand-rolled deterministic property testing
// (seeded xorshift) — no external crates.
// Depends on sham-001 (sham_domain).
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

use sham_domain::VehicleClass;

pub const EARTH_RADIUS_M: f64 = 6_371_008.8;

/// Latitude/longitude in decimal degrees.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GeoCoord {
    pub lat: f64,
    pub lon: f64,
}
impl GeoCoord {
    pub fn new(lat: f64, lon: f64) -> Result<Self, String> {
        if !(-90.0..=90.0).contains(&lat) {
            return Err(format!("latitude {} out of range", lat));
        }
        if !(-180.0..=180.0).contains(&lon) {
            return Err(format!("longitude {} out of range", lon));
        }
        Ok(Self { lat, lon })
    }
    /// Fixed-point E7 pair (matches Point.lat_e7/lon_e7).
    pub fn to_e7(self) -> (i32, i32) {
        (
            (self.lat * 1e7).round() as i32,
            (self.lon * 1e7).round() as i32,
        )
    }
    pub fn from_e7(lat_e7: i32, lon_e7: i32) -> Self {
        Self {
            lat: lat_e7 as f64 / 1e7,
            lon: lon_e7 as f64 / 1e7,
        }
    }
    /// Haversine distance in meters.
    pub fn distance_m(self, other: GeoCoord) -> f64 {
        let dlat = (other.lat - self.lat).to_radians();
        let dlon = (other.lon - self.lon).to_radians();
        let a = (dlat / 2.0).sin().powi(2)
            + self.lat.to_radians().cos() * other.lat.to_radians().cos() * (dlon / 2.0).sin().powi(2);
        2.0 * EARTH_RADIUS_M * a.sqrt().asin()
    }
    /// Initial bearing in degrees [0,360).
    pub fn bearing_deg(self, other: GeoCoord) -> f64 {
        let dlon = (other.lon - self.lon).to_radians();
        let lat1 = self.lat.to_radians();
        let lat2 = other.lat.to_radians();
        let y = dlon.sin() * lat2.cos();
        let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * dlon.cos();
        (y.atan2(x).to_degrees() + 360.0) % 360.0
    }
    /// Destination point given bearing (deg) and distance (m).
    pub fn destination(self, bearing_deg: f64, dist_m: f64) -> GeoCoord {
        let ang = dist_m / EARTH_RADIUS_M;
        let br = bearing_deg.to_radians();
        let lat1 = self.lat.to_radians();
        let lon1 = self.lon.to_radians();
        let lat2 = (lat1.sin() * ang.cos() + lat1.cos() * ang.sin() * br.cos()).asin();
        let lon2 = lon1
            + (br.sin() * ang.sin() * lat1.cos())
                .atan2(ang.cos() - lat1.sin() * lat2.sin());
        // Clamp through constructor-free path (destination is always valid by construction)
        let mut lon = lon2.to_degrees();
        while lon > 180.0 {
            lon -= 360.0;
        }
        while lon < -180.0 {
            lon += 360.0;
        }
        GeoCoord {
            lat: lat2.to_degrees().clamp(-90.0, 90.0),
            lon,
        }
    }
}

/// Turning maneuver feasibility for a vehicle entering a street.
/// `turn_angle_deg`: absolute heading change required (0 = straight).
/// `street_width_cm`: usable width of the target street.
/// Returns true if the vehicle's swept turn fits.
pub fn can_make_turn(class: VehicleClass, turn_angle_deg: f64, street_width_cm: u32) -> bool {
    let angle = turn_angle_deg.abs();
    if angle < 1.0 {
        return true; // straight-through always fits
    }
    let r = class.turning_radius_cm() as f64;
    // Swept width of a turn of `angle` with radius r: r * (1 - cos(angle)) + vehicle width margin.
    // Conservative model: required width = r * (1 - cos(a)) + typical width (200cm).
    let need_cm = r * (1.0 - angle.to_radians().cos()) + 200.0;
    need_cm <= street_width_cm as f64
}

/// Angled parking stall geometry: given stall width and angle, the curb
/// length consumed per stall (cm). Used to check zone capacity claims.
pub fn angled_stall_curb_cm(stall_width_cm: u32, angle_deg: f64) -> f64 {
    let a = angle_deg.to_radians();
    stall_width_cm as f64 / a.sin().max(1e-6)
}

/// Perpendicular stall depth (cm) for a vehicle of given length parked at angle.
pub fn stall_depth_cm(vehicle_len_cm: u32, angle_deg: f64) -> f64 {
    vehicle_len_cm as f64 * angle_deg.to_radians().sin()
}

/// Simple deterministic PRNG (xorshift64*) for property tests.
#[derive(Clone)]
pub struct Rng {
    s: u64,
}
impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { s: seed | 1 }
    }
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.s;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.s = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn range_f64(&mut self, lo: f64, hi: f64) -> f64 {
        let u = (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        lo + u * (hi - lo)
    }
    pub fn coord(&mut self) -> GeoCoord {
        GeoCoord {
            lat: self.range_f64(-89.0, 89.0),
            lon: self.range_f64(-179.0, 179.0),
        }
    }
}

/// Bounding box for fat-thumb selection hit-testing.
#[derive(Clone, Copy, Debug)]
pub struct BBox {
    pub min_lat: f64,
    pub max_lat: f64,
    pub min_lon: f64,
    pub max_lon: f64,
}
impl BBox {
    pub fn around(c: GeoCoord, radius_m: f64) -> Self {
        // Equirectangular approx is fine for festival-scale boxes.
        let dlat = radius_m / 111_320.0;
        let dlon = radius_m / (111_320.0 * c.lat.to_radians().cos().max(0.1));
        Self {
            min_lat: c.lat - dlat,
            max_lat: c.lat + dlat,
            min_lon: c.lon - dlon,
            max_lon: c.lon + dlon,
        }
    }
    pub fn contains(self, c: GeoCoord) -> bool {
        (self.min_lat..=self.max_lat).contains(&c.lat)
            && (self.min_lon..=self.max_lon).contains(&c.lon)
    }
}
