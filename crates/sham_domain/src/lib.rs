// sham_domain — sham-001: Shambhala parking/festival domain model.
// cars, streets, points, parking zones, traffic flows as Rust types.
// std-only. Every state change leaves a Receipt.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// Newtype IDs — opaque, non-confusable.
macro_rules! id {
    ($n:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        pub struct $n(pub u64);
        impl fmt::Debug for $n {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($n), self.0)
            }
        }
        impl fmt::Display for $n {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}-{}", stringify!($n), self.0)
            }
        }
    };
}
id!(CarId);
id!(StreetId);
id!(PointId);
id!(ZoneId);
id!(FlowId);
id!(VehicleId);

/// Signed sequence for deterministic IDs in tests.
#[derive(Default)]
pub struct IdGen {
    next: u64,
}
impl IdGen {
    pub fn new(seed: u64) -> Self {
        Self { next: seed }
    }
    pub fn next(&mut self) -> u64 {
        self.next += 1;
        self.next
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Proof row for every mutation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Receipt {
    pub op: String,
    pub entity: String,
    pub entity_id: u64,
    pub at_ms: u64,
    pub note: String,
}
impl Receipt {
    pub fn new(op: &str, entity: &str, entity_id: u64, note: &str) -> Self {
        Self {
            op: op.into(),
            entity: entity.into(),
            entity_id,
            at_ms: now_ms(),
            note: note.into(),
        }
    }
}

/// Compass quadrant of a festival point (volunteer-friendly).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Quadrant {
    North,
    South,
    East,
    West,
    Center,
}

/// A physical point of interest: gate, cone cluster, water station, etc.
#[derive(Clone, Debug, PartialEq)]
pub struct Point {
    pub id: PointId,
    pub label: String,
    pub lat_e7: i32,
    pub lon_e7: i32,
    pub quadrant: Quadrant,
    pub tags: Vec<String>,
}
impl Point {
    pub fn new(id: PointId, label: &str, lat_e7: i32, lon_e7: i32) -> Self {
        Self {
            id,
            label: label.into(),
            lat_e7,
            lon_e7,
            quadrant: Quadrant::Center,
            tags: Vec::new(),
        }
    }
    pub fn tag(mut self, t: &str) -> Self {
        self.tags.push(t.into());
        self
    }
}

/// A street/road segment between two points.
#[derive(Clone, Debug, PartialEq)]
pub struct Street {
    pub id: StreetId,
    pub name: String,
    pub from: PointId,
    pub to: PointId,
    /// Road width in centimeters.
    pub width_cm: u32,
    /// One-way flag; if true, travel allowed only from->to.
    pub one_way: bool,
    /// Max vehicle length allowed in cm (0 = unrestricted).
    pub max_vehicle_len_cm: u32,
}
impl Street {
    pub fn new(
        id: StreetId,
        name: &str,
        from: PointId,
        to: PointId,
        width_cm: u32,
    ) -> Result<Self, String> {
        if from == to {
            return Err("street cannot loop on a single point".into());
        }
        if width_cm < 200 {
            return Err(format!("street width {}cm below minimum 200cm", width_cm));
        }
        Ok(Self {
            id,
            name: name.into(),
            from,
            to,
            width_cm,
            one_way: false,
            max_vehicle_len_cm: 0,
        })
    }
    pub fn permits(&self, len_cm: u32) -> bool {
        self.max_vehicle_len_cm == 0 || len_cm <= self.max_vehicle_len_cm
    }
}

/// Vehicle class for parking rules.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VehicleClass {
    Car,
    Truck,
    Rv,
    Bus,
    Motorcycle,
    Trailer,
}
impl VehicleClass {
    /// Typical length in cm (median-ish), used for constraint checks.
    pub fn typical_len_cm(self) -> u32 {
        match self {
            VehicleClass::Car => 450,
            VehicleClass::Truck => 600,
            VehicleClass::Rv => 750,
            VehicleClass::Bus => 1100,
            VehicleClass::Motorcycle => 220,
            VehicleClass::Trailer => 800,
        }
    }
    /// Typical min turning radius in cm.
    pub fn turning_radius_cm(self) -> u32 {
        match self {
            VehicleClass::Car => 550,
            VehicleClass::Truck => 900,
            VehicleClass::Rv => 1000,
            VehicleClass::Bus => 1200,
            VehicleClass::Motorcycle => 300,
            VehicleClass::Trailer => 1100,
        }
    }
}

/// Parking orientation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ParkOrientation {
    Perpendicular,
    Parallel,
    Angled45,
    Angled60,
}

/// A car present at the festival.
#[derive(Clone, Debug, PartialEq)]
pub struct Car {
    pub id: CarId,
    pub plate: String,
    pub class: VehicleClass,
    pub zone: Option<ZoneId>,
    pub orientation: Option<ParkOrientation>,
    pub arrival_ms: u64,
}
impl Car {
    pub fn new(id: CarId, plate: &str, class: VehicleClass) -> Result<Self, String> {
        if plate.trim().is_empty() {
            return Err("plate cannot be empty".into());
        }
        Ok(Self {
            id,
            plate: plate.trim().to_uppercase(),
            class,
            zone: None,
            orientation: None,
            arrival_ms: now_ms(),
        })
    }
}

/// Zone state machine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoneState {
    Open,
    Filling,
    Full,
    Closed,
    Emergency,
}

/// A parking zone.
#[derive(Clone, Debug)]
pub struct Zone {
    pub id: ZoneId,
    pub name: String,
    pub capacity: u32,
    pub occupied: u32,
    pub state: ZoneState,
    pub allowed: Vec<VehicleClass>,
    pub orientation: ParkOrientation,
}
impl Zone {
    pub fn new(
        id: ZoneId,
        name: &str,
        capacity: u32,
        allowed: Vec<VehicleClass>,
    ) -> Result<Self, String> {
        if capacity == 0 {
            return Err("zone capacity must be > 0".into());
        }
        if allowed.is_empty() {
            return Err("zone must allow at least one vehicle class".into());
        }
        Ok(Self {
            id,
            name: name.into(),
            capacity,
            occupied: 0,
            state: ZoneState::Open,
            allowed,
            orientation: ParkOrientation::Perpendicular,
        })
    }
    pub fn free(&self) -> u32 {
        self.capacity.saturating_sub(self.occupied)
    }
    pub fn admits(&self, class: VehicleClass) -> bool {
        matches!(self.state, ZoneState::Open | ZoneState::Filling) && self.allowed.contains(&class)
    }
    /// Park a car; returns a receipt or a reason.
    pub fn park(&mut self, car: &mut Car) -> Result<Receipt, String> {
        if car.zone == Some(self.id) {
            return Err(format!("car {} already parked in zone {}", car.plate, self.name));
        }        if !self.admits(car.class) {
            return Err(format!(
                "zone {} does not admit {:?} in state {:?}",
                self.name, car.class, self.state
            ));
        }
        if self.free() == 0 {
            self.state = ZoneState::Full;
            return Err(format!("zone {} is full", self.name));
        }
        self.occupied += 1;
        if self.free() == 0 {
            self.state = ZoneState::Full;
        } else if self.state == ZoneState::Open {
            self.state = ZoneState::Filling;
        }
        car.zone = Some(self.id);
        car.orientation = Some(self.orientation);
        Ok(Receipt::new(
            "park",
            "zone",
            self.id.0,
            &format!("car {} -> zone {}", car.plate, self.name),
        ))
    }
    pub fn release(&mut self, car: &mut Car) -> Result<Receipt, String> {
        if car.zone != Some(self.id) {
            return Err("car is not in this zone".into());
        }
        self.occupied = self.occupied.saturating_sub(1);
        if self.state == ZoneState::Full && self.free() > 0 {
            self.state = ZoneState::Filling;
        }
        if self.occupied == 0 {
            self.state = ZoneState::Open;
        }
        car.zone = None;
        car.orientation = None;
        Ok(Receipt::new(
            "release",
            "zone",
            self.id.0,
            &format!("car {} left zone {}", car.plate, self.name),
        ))
    }
    pub fn set_state(&mut self, s: ZoneState) -> Receipt {
        self.state = s;
        Receipt::new("zone_state", "zone", self.id.0, &format!("state -> {:?}", s))
    }
}

/// Traffic flow: directed movement intent along streets.
#[derive(Clone, Debug, PartialEq)]
pub struct Flow {
    pub id: FlowId,
    pub name: String,
    pub path: Vec<StreetId>,
    pub cars_per_hour: u32,
    pub active: bool,
}
impl Flow {
    pub fn new(id: FlowId, name: &str, path: Vec<StreetId>) -> Result<Self, String> {
        if path.is_empty() {
            return Err("flow path cannot be empty".into());
        }
        Ok(Self {
            id,
            name: name.into(),
            path,
            cars_per_hour: 0,
            active: true,
        })
    }
    pub fn set_rate(&mut self, cph: u32) -> Receipt {
        self.cars_per_hour = cph;
        Receipt::new("flow_rate", "flow", self.id.0, &format!("rate -> {}/h", cph))
    }
}

/// In-memory festival map: all entities + receipt log.
#[derive(Default)]
pub struct FestivalMap {
    pub cars: HashMap<CarId, Car>,
    pub streets: HashMap<StreetId, Street>,
    pub points: HashMap<PointId, Point>,
    pub zones: HashMap<ZoneId, Zone>,
    pub flows: HashMap<FlowId, Flow>,
    pub receipts: Vec<Receipt>,
}
impl FestivalMap {
    pub fn new() -> Self {
        Self::default()
    }
    fn log(&mut self, r: Receipt) {
        self.receipts.push(r);
    }
    pub fn add_point(&mut self, p: Point) {
        self.points.insert(p.id, p);
    }
    pub fn add_street(&mut self, s: Street) {
        self.streets.insert(s.id, s);
    }
    pub fn add_zone(&mut self, z: Zone) {
        self.zones.insert(z.id, z);
    }
    pub fn add_car(&mut self, c: Car) {
        self.cars.insert(c.id, c);
    }
    pub fn add_flow(&mut self, f: Flow) {
        self.flows.insert(f.id, f);
    }
    /// Park car in zone with receipt logging. Fails closed on missing entities.
    pub fn park_car(&mut self, car_id: CarId, zone_id: ZoneId) -> Result<Receipt, String> {
        let car = self.cars.get_mut(&car_id).ok_or("car not found")?;
        let zone = self.zones.get_mut(&zone_id).ok_or("zone not found")?;
        let r = zone.park(car)?;
        self.log(r.clone());
        Ok(r)
    }
    pub fn release_car(&mut self, car_id: CarId) -> Result<Receipt, String> {
        let zone_id = self
            .cars
            .get(&car_id)
            .and_then(|c| c.zone)
            .ok_or("car not parked")?;
        let car = self.cars.get_mut(&car_id).ok_or("car not found")?;
        let zone = self.zones.get_mut(&zone_id).ok_or("zone not found")?;
        let r = zone.release(car)?;
        self.log(r.clone());
        Ok(r)
    }
    pub fn occupancy_pct(&self) -> f64 {
        let cap: u32 = self.zones.values().map(|z| z.capacity).sum();
        let occ: u32 = self.zones.values().map(|z| z.occupied).sum();
        if cap == 0 {
            0.0
        } else {
            occ as f64 / cap as f64 * 100.0
        }
    }
}
