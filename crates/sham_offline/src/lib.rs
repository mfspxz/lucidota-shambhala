// sham_offline — sham-017 (offline operation), sham-028 (deployment runbook).
// Full festival run with zero network: an append-only op log with causal
// ordering (lamport clock + device id tiebreak). Sync replays the log onto a
// server map with conflict detection (same car parked twice -> conflict row,
// never silent overwrite). Deployment runbook as checked data.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

use sham_domain::{Car, CarId, FestivalMap, Receipt, VehicleClass, Zone, ZoneId};

/// One offline operation.
#[derive(Clone, Debug, PartialEq)]
pub struct Op {
    pub lamport: u64,
    pub device: String,
    pub kind: OpKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OpKind {
    AddZone { id: u64, name: String, capacity: u32 },
    AddCar { id: u64, plate: String },
    Park { car: u64, zone: u64 },
    Release { car: u64 },
    Note { entity: String, id: u64, text: String },
}

#[derive(Default)]
pub struct OpLog {
    pub device: String,
    pub clock: u64,
    pub ops: Vec<Op>,
}
impl OpLog {
    pub fn new(device: &str) -> Self {
        Self { device: device.into(), clock: 0, ops: Vec::new() }
    }
    pub fn push(&mut self, kind: OpKind) -> &Op {
        self.clock += 1;
        self.ops.push(Op {
            lamport: self.clock,
            device: self.device.clone(),
            kind,
        });
        self.ops.last().unwrap()
    }
    pub fn len(&self) -> usize {
        self.ops.len()
    }
}

/// Merge two logs into causal order (lamport, then device).
pub fn merge(a: &OpLog, b: &OpLog) -> Vec<Op> {
    let mut all: Vec<Op> = a.ops.iter().chain(b.ops.iter()).cloned().collect();
    all.sort_by(|x, y| {
        x.lamport
            .cmp(&y.lamport)
            .then_with(|| x.device.cmp(&y.device))
    });
    all
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Conflict {
    pub op_index: usize,
    pub reason: String,
}

#[derive(Default)]
pub struct SyncReport {
    pub applied: usize,
    pub conflicts: Vec<Conflict>,
    pub receipts: Vec<Receipt>,
}

/// Replay a merged op log onto a FestivalMap. Conflicts are recorded, never
/// silently applied.
pub fn sync(map: &mut FestivalMap, ops: &[Op]) -> SyncReport {
    let mut rep = SyncReport::default();
    for (i, op) in ops.iter().enumerate() {
        let res: Result<Receipt, String> = match &op.kind {
            OpKind::AddZone { id, name, capacity } => {
                match Zone::new(ZoneId(*id), name, *capacity, vec![VehicleClass::Car]) {
                    Ok(z) => {
                        map.add_zone(z);
                        Ok(Receipt::new("sync_add_zone", "zone", *id, &op.device))
                    }
                    Err(e) => Err(e),
                }
            }
            OpKind::AddCar { id, plate } => match Car::new(CarId(*id), plate, VehicleClass::Car) {
                Ok(c) => {
                    map.add_car(c);
                    Ok(Receipt::new("sync_add_car", "car", *id, &op.device))
                }
                Err(e) => Err(e),
            },
            OpKind::Park { car, zone } => map.park_car(CarId(*car), ZoneId(*zone)),
            OpKind::Release { car } => map.release_car(CarId(*car)),
            OpKind::Note { entity, id, text } => {
                Ok(Receipt::new("sync_note", entity, *id, text))
            }
        };
        match res {
            Ok(r) => {
                rep.applied += 1;
                // Park/Release already logged inside FestivalMap; others need it here.
                if !matches!(op.kind, OpKind::Park { .. } | OpKind::Release { .. }) {
                    map.receipts.push(r.clone());
                }
                rep.receipts.push(r);
            }
            Err(e) => rep.conflicts.push(Conflict { op_index: i, reason: e }),
        }
    }
    rep
}

// ---------------- sham-028: deployment runbook ----------------

#[derive(Clone, Debug, PartialEq)]
pub struct RunbookStep {
    pub n: u32,
    pub title: &'static str,
    pub command: &'static str,
    pub verify: &'static str,
}

/// Bare-metal deploy runbook: single binary where possible (sham-028).
pub fn runbook() -> Vec<RunbookStep> {
    vec![
        RunbookStep { n: 1, title: "Build release binary", command: "cargo build --release", verify: "target/release/shambhala exists" },
        RunbookStep { n: 2, title: "Run full test suite", command: "cargo test --workspace", verify: "348 passed, 0 failed" },
        RunbookStep { n: 3, title: "Copy binary to field box", command: "scp target/release/shambhala field:/opt/shambhala/", verify: "sha256 matches build log" },
        RunbookStep { n: 4, title: "Install systemd unit (no daemon auto-start)", command: "cp shambhala.service /etc/systemd/system/", verify: "systemctl cat shambhala shows unit" },
        RunbookStep { n: 5, title: "Open firewall for :8080 :8088 :8090", command: "ufw allow 8080,8088,8090/tcp", verify: "ufw status shows rules" },
        RunbookStep { n: 6, title: "Seed map data", command: "shambhala seed --map festival.map", verify: "/health returns ok, zones listed" },
        RunbookStep { n: 7, title: "Smoke test API", command: "curl localhost:8080/health", verify: "{\"status\":\"ok\"}" },
        RunbookStep { n: 8, title: "Smoke test ODIN", command: "curl localhost:8090/health", verify: "{\"status\":\"ok\"}" },
        RunbookStep { n: 9, title: "Verify offline bundle on tablets", command: "check /offline bundle hash", verify: "bundle hash matches release" },
        RunbookStep { n: 10, title: "Go-live checklist sign-off", command: "lead signs checklist", verify: "signed checklist in receipts" },
    ]
}

/// Validate the runbook: steps sequential, all fields non-empty.
pub fn check_runbook(steps: &[RunbookStep]) -> Result<(), String> {
    for (i, s) in steps.iter().enumerate() {
        if s.n != i as u32 + 1 {
            return Err(format!("step numbering broken at {}", s.n));
        }
        if s.title.is_empty() || s.command.is_empty() || s.verify.is_empty() {
            return Err(format!("step {} has empty field", s.n));
        }
    }
    Ok(())
}
