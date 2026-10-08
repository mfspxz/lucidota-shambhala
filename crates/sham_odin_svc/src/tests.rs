use super::*;
use sham_domain::{Car, CarId, VehicleClass, Zone, ZoneId};

fn svc() -> OdinService {
    OdinService::new()
}
fn req(id: &str, args: &[(&str, &str)]) -> OdinRequest {
    OdinRequest {
        feature_id: id.into(),
        args: args.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
        token: "t".into(),
    }
}

#[test]
fn odin_port_is_8090() {
    assert_eq!(ODIN_PORT, 8090);
}
#[test]
fn unknown_feature_rejected() {
    let mut s = svc();
    let out = s.execute(&req("ODIN-XXX-001", &[]));
    assert!(matches!(out, OdinOutcome::Rejected(_)));
    assert_eq!(s.receipts.len(), 1);
    assert_eq!(s.receipts[0].op, "odin_reject");
}
#[test]
fn eta_calc_kernel() {
    let mut s = svc();
    let out = s.execute(&req("ODIN-DIS-009", &[("dist_m", "6000"), ("speed_kmh", "30")]));
    assert!(matches!(out, OdinOutcome::Ok(b) if b.contains("12.00")));
}
#[test]
fn eta_calc_bad_args_rejected() {
    let mut s = svc();
    let out = s.execute(&req("ODIN-DIS-009", &[("dist_m", "x"), ("speed_kmh", "30")]));
    assert!(matches!(out, OdinOutcome::Rejected(_)));
}
#[test]
fn ping_ingest_kernel() {
    let mut s = svc();
    let out = s.execute(&req(
        "ODIN-TRK-001",
        &[("lat", "49.2"), ("lon", "-122.9"), ("ts_ms", "1000")],
    ));
    assert!(matches!(out, OdinOutcome::Ok(_)));
}
#[test]
fn ping_ingest_bad_coords_rejected() {
    let mut s = svc();
    let out = s.execute(&req(
        "ODIN-TRK-001",
        &[("lat", "999"), ("lon", "0"), ("ts_ms", "1000")],
    ));
    assert!(matches!(out, OdinOutcome::Rejected(_)));
}
#[test]
fn assign_car_zone_wired_to_api() {
    let mut s = svc();
    // seed api state directly
    s.api.map.add_zone(Zone::new(ZoneId(5), "z", 10, vec![VehicleClass::Car]).unwrap());
    s.api.map.add_car(Car::new(CarId(6), "P1", VehicleClass::Car).unwrap());
    let out = s.execute(&req("ODIN-DIS-002", &[("car", "6"), ("zone", "5")]));
    assert!(matches!(out, OdinOutcome::Ok(_)));
    assert_eq!(s.api.map.zones[&ZoneId(5)].occupied, 1);
}
#[test]
fn generic_feature_executes() {
    let mut s = svc();
    let out = s.execute(&req("ODIN-COM-001", &[]));
    assert!(matches!(out, OdinOutcome::Ok(b) if b.contains("ODIN-COM-001")));
    assert!(s.receipts.iter().any(|r| r.op == "odin_exec"));
}
