//! sham_organelles as standard organelles (Tier A native migration). The 4 declared organelles, re-expressed on the v1 standard.
//!
//! Tier A native migration: implements `organelle_host::Organelle` over the
//! existing crate API. The computation is unchanged; this file only adds the
//! capability contract, manifest, and host registration.

use organelle_host::{CallContext, HostRef, Organelle, OrganelleError};
use organelle_manifest::{
    Authority, Contract, MutationClass, OrganelleManifest, ReceiptPolicy, Transport, Valence,
    MANIFEST_SCHEMA_VERSION, RECEIPT_ALGORITHM,
};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use sham_domain::{VehicleClass, ZoneId};
use sham_odin;
use sham_spatial::GeoCoord;

fn req<'a>(input: &'a Value, key: &str) -> Result<&'a str, OrganelleError> {
    input
        .get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| OrganelleError::failed(format!("missing required input: {key}")))
}

fn opt_str(input: &Value, key: &str) -> Option<String> {
    input.get(key).and_then(|v| v.as_str()).map(|s| s.to_string())
}

fn opt_f64(input: &Value, key: &str) -> Option<f64> {
    input.get(key).and_then(|v| v.as_f64())
}

fn req_f64(input: &Value, key: &str) -> Result<f64, OrganelleError> {
    input.get(key).and_then(|v| v.as_f64())
        .ok_or_else(|| OrganelleError::failed(format!("missing required numeric input: {key}")))
}

fn parse_vehicle_class(s: &str) -> Result<sham_domain::VehicleClass, OrganelleError> {
    use sham_domain::VehicleClass as C;
    match s {
        "car" => Ok(C::Car), "truck" => Ok(C::Truck), "bus" => Ok(C::Bus),
        other => Err(OrganelleError::failed(format!("unknown vehicle class '{other}' (car|truck|bus)"))),
    }
}


fn sham_route_manifest() -> OrganelleManifest {
    OrganelleManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        name: "sham-route".to_string(),
        codon: "sr".to_string(),
        version: "0.1.0".to_string(),
        description: "constraint-checked distance A->B for a vehicle class (organelle)".to_string(),
        contracts: vec![
            Contract {
                op: "route".to_string(),
                description: "distance in meters from A to B (great-circle; class reserved for constraint checks)".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["from_lat","from_lon","to_lat","to_lon"],"properties":{"from_lat":{"type":"number"},"from_lon":{"type":"number"},"to_lat":{"type":"number"},"to_lon":{"type":"number"},"class":{"type":"string"}}}),
                mutation_class: None,
                required_capability: None,
            },
        ],
        authority: Authority {
            mutation_class: MutationClass::ReadOnly,
            declared_valence: Valence::Proven,
            required_capability: None,
            granted_capabilities: vec![],
        },
        transports: vec![Transport::Native, Transport::UnixSocket, Transport::Mcp, Transport::Rest],
        receipt_policy: ReceiptPolicy {
            content_addressed: true,
            algorithm: RECEIPT_ALGORITHM.to_string(),
            chain: true,
        },
    }
}

pub struct ShamRouteOrg {
    manifest: OrganelleManifest,
    _priv: (),
}

pub fn sham_route_organelle() -> Arc<dyn Organelle> {
    Arc::new(ShamRouteOrg {
        manifest: sham_route_manifest(),
            _priv: (),
    })
}

impl Organelle for ShamRouteOrg {
    fn manifest(&self) -> OrganelleManifest {
        self.manifest.clone()
    }

    fn handle(
        &self,
        _ctx: &CallContext,
        _host: &dyn HostRef,
        op: &str,
        input: Value,
    ) -> Result<Value, OrganelleError> {
        match op {
            "route" => {
                let from = GeoCoord::new(req_f64(&input, "from_lat")?, req_f64(&input, "from_lon")?)
                                    .map_err(|e| OrganelleError::failed(format!("bad from coord: {e}")))?;
                                let to = GeoCoord::new(req_f64(&input, "to_lat")?, req_f64(&input, "to_lon")?)
                                    .map_err(|e| OrganelleError::failed(format!("bad to coord: {e}")))?;
                                Ok(serde_json::json!({"dist_m": from.distance_m(to)}))
            },
            _ => Err(OrganelleError::not_found(format!("unknown op {op}"))),
        }
    }
}


fn sham_zone_manifest() -> OrganelleManifest {
    OrganelleManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        name: "sham-zone".to_string(),
        codon: "sz".to_string(),
        version: "0.1.0".to_string(),
        description: "suggest the best zone for a vehicle class (ODIN dispatch) (organelle)".to_string(),
        contracts: vec![
            Contract {
                op: "suggest_zone".to_string(),
                description: "best zone id for a vehicle class given zone capacity/occupancy".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["class","zones"],"properties":{"class":{"type":"string"},"zones":{"type":"array","items":{"type":"object"}}}}),
                mutation_class: None,
                required_capability: None,
            },
        ],
        authority: Authority {
            mutation_class: MutationClass::ReadOnly,
            declared_valence: Valence::Proven,
            required_capability: None,
            granted_capabilities: vec![],
        },
        transports: vec![Transport::Native, Transport::UnixSocket, Transport::Mcp, Transport::Rest],
        receipt_policy: ReceiptPolicy {
            content_addressed: true,
            algorithm: RECEIPT_ALGORITHM.to_string(),
            chain: true,
        },
    }
}

pub struct ShamZoneOrg {
    manifest: OrganelleManifest,
    _priv: (),
}

pub fn sham_zone_organelle() -> Arc<dyn Organelle> {
    Arc::new(ShamZoneOrg {
        manifest: sham_zone_manifest(),
            _priv: (),
    })
}

impl Organelle for ShamZoneOrg {
    fn manifest(&self) -> OrganelleManifest {
        self.manifest.clone()
    }

    fn handle(
        &self,
        _ctx: &CallContext,
        _host: &dyn HostRef,
        op: &str,
        input: Value,
    ) -> Result<Value, OrganelleError> {
        match op {
            "suggest_zone" => {
                let class = parse_vehicle_class(req(&input, "class")?)?;
                                let zones: Vec<(ZoneId, String, u32, u32, Vec<VehicleClass>)> = input["zones"]
                                    .as_array().cloned().unwrap_or_default().into_iter().filter_map(|z| {
                                        Some((ZoneId(z.get("id")?.as_u64()?),
                                              String::new(),
                                              z.get("capacity")?.as_u64()? as u32,
                                              z.get("occupancy")?.as_u64()? as u32,
                                              vec![class]))
                                    }).collect();
                                let zone_id = sham_odin::suggest_zone(&zones, class).map(|z| z.0);
                                Ok(serde_json::json!({"zone_id": zone_id}))
            },
            _ => Err(OrganelleError::not_found(format!("unknown op {op}"))),
        }
    }
}


fn sham_ping_manifest() -> OrganelleManifest {
    OrganelleManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        name: "sham-ping".to_string(),
        codon: "vn".to_string(),
        version: "0.1.0".to_string(),
        description: "validate a GPS ping: bounds + timestamp sanity (ODIN tracking) (organelle)".to_string(),
        contracts: vec![
            Contract {
                op: "validate".to_string(),
                description: "validate a GPS ping; returns valid + reason on failure".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["lat","lon","ts_ms"],"properties":{"lat":{"type":"number"},"lon":{"type":"number"},"ts_ms":{"type":"integer"},"now_ms":{"type":"integer"}}}),
                mutation_class: None,
                required_capability: None,
            },
        ],
        authority: Authority {
            mutation_class: MutationClass::ReadOnly,
            declared_valence: Valence::Proven,
            required_capability: None,
            granted_capabilities: vec![],
        },
        transports: vec![Transport::Native, Transport::UnixSocket, Transport::Mcp, Transport::Rest],
        receipt_policy: ReceiptPolicy {
            content_addressed: true,
            algorithm: RECEIPT_ALGORITHM.to_string(),
            chain: true,
        },
    }
}

pub struct ShamPingOrg {
    manifest: OrganelleManifest,
    _priv: (),
}

pub fn sham_ping_organelle() -> Arc<dyn Organelle> {
    Arc::new(ShamPingOrg {
        manifest: sham_ping_manifest(),
            _priv: (),
    })
}

impl Organelle for ShamPingOrg {
    fn manifest(&self) -> OrganelleManifest {
        self.manifest.clone()
    }

    fn handle(
        &self,
        _ctx: &CallContext,
        _host: &dyn HostRef,
        op: &str,
        input: Value,
    ) -> Result<Value, OrganelleError> {
        match op {
            "validate" => {
                let lat = req_f64(&input, "lat")?;
                                let lon = req_f64(&input, "lon")?;
                                let ts = input.get("ts_ms").and_then(|v| v.as_u64())
                                    .ok_or_else(|| OrganelleError::failed("missing required input: ts_ms"))?;
                                let now = input.get("now_ms").and_then(|v| v.as_u64()).unwrap_or(ts);
                                match sham_odin::validate_ping(lat, lon, ts, now) {
                                    Ok(_) => Ok(serde_json::json!({"valid": true})),
                                    Err(e) => Ok(serde_json::json!({"valid": false, "reason": e})),
                                }
            },
            _ => Err(OrganelleError::not_found(format!("unknown op {op}"))),
        }
    }
}


fn sham_eta_manifest() -> OrganelleManifest {
    OrganelleManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        name: "sham-eta".to_string(),
        codon: "et".to_string(),
        version: "0.1.0".to_string(),
        description: "ETA minutes from distance and speed (ODIN dispatch) (organelle)".to_string(),
        contracts: vec![
            Contract {
                op: "eta".to_string(),
                description: "ETA in minutes from distance (m) and speed (km/h)".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["dist_m","speed_kmh"],"properties":{"dist_m":{"type":"number"},"speed_kmh":{"type":"number"}}}),
                mutation_class: None,
                required_capability: None,
            },
        ],
        authority: Authority {
            mutation_class: MutationClass::ReadOnly,
            declared_valence: Valence::Proven,
            required_capability: None,
            granted_capabilities: vec![],
        },
        transports: vec![Transport::Native, Transport::UnixSocket, Transport::Mcp, Transport::Rest],
        receipt_policy: ReceiptPolicy {
            content_addressed: true,
            algorithm: RECEIPT_ALGORITHM.to_string(),
            chain: true,
        },
    }
}

pub struct ShamEtaOrg {
    manifest: OrganelleManifest,
    _priv: (),
}

pub fn sham_eta_organelle() -> Arc<dyn Organelle> {
    Arc::new(ShamEtaOrg {
        manifest: sham_eta_manifest(),
            _priv: (),
    })
}

impl Organelle for ShamEtaOrg {
    fn manifest(&self) -> OrganelleManifest {
        self.manifest.clone()
    }

    fn handle(
        &self,
        _ctx: &CallContext,
        _host: &dyn HostRef,
        op: &str,
        input: Value,
    ) -> Result<Value, OrganelleError> {
        match op {
            "eta" => {
                let e = sham_odin::eta_minutes(req_f64(&input, "dist_m")?, req_f64(&input, "speed_kmh")?)
                                    .map_err(|e| OrganelleError::failed(format!("eta failed: {e}")))?;
                                Ok(serde_json::json!({"eta_min": e}))
            },
            _ => Err(OrganelleError::not_found(format!("unknown op {op}"))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use organelle_host::Host;
    use organelle_manifest::validate_manifest_json;

    #[test]
    fn sham_route_static_manifest_validates() {
        let v: Value = serde_json::from_str(include_str!("../../../organelles/sham-route.manifest.json")).expect("manifest JSON parses");
        let m = validate_manifest_json(&v).expect("static manifest must validate");
        assert_eq!(m.name, "sham-route");
        assert_eq!(m.codon, "sr");
    }

    #[test]
    fn sham_route_rust_manifest_matches_static_json() {
        let from_rust = serde_json::to_value(sham_route_manifest()).unwrap();
        let from_file: Value = serde_json::from_str(include_str!("../../../organelles/sham-route.manifest.json")).unwrap();
        assert_eq!(from_rust, from_file, "Rust manifest() and organelles/*.json must agree");
    }

    #[test]
    fn sham_route_registers_and_serves_through_host_with_receipts() {
        let host = Host::new();
        host.register(sham_route_organelle()).expect("registration must succeed");

        let ctx = CallContext::new("migration-test", vec![]);
        let out = host
            .call(&ctx, "sr", "route", serde_json::json!({"from_lat": 49.28, "from_lon": -123.12, "to_lat": 49.29, "to_lon": -123.13}))
            .expect("route call");
        assert!(out.get("dist_m").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.0);
        let n = host.verify_chain().expect("receipt chain must verify");
        assert!(n >= 2, "register + calls = >=2 receipts, got {n}");
    }

    #[test]
    fn sham_zone_static_manifest_validates() {
        let v: Value = serde_json::from_str(include_str!("../../../organelles/sham-zone.manifest.json")).expect("manifest JSON parses");
        let m = validate_manifest_json(&v).expect("static manifest must validate");
        assert_eq!(m.name, "sham-zone");
        assert_eq!(m.codon, "sz");
    }

    #[test]
    fn sham_zone_rust_manifest_matches_static_json() {
        let from_rust = serde_json::to_value(sham_zone_manifest()).unwrap();
        let from_file: Value = serde_json::from_str(include_str!("../../../organelles/sham-zone.manifest.json")).unwrap();
        assert_eq!(from_rust, from_file, "Rust manifest() and organelles/*.json must agree");
    }

    #[test]
    fn sham_zone_registers_and_serves_through_host_with_receipts() {
        let host = Host::new();
        host.register(sham_zone_organelle()).expect("registration must succeed");

        let ctx = CallContext::new("migration-test", vec![]);
        let out = host
            .call(&ctx, "sz", "suggest_zone", serde_json::json!({"class": "car", "zones": [{"id": 1, "capacity": 10, "occupancy": 2}, {"id": 2, "capacity": 10, "occupancy": 9}]}))
            .expect("suggest_zone call");
        assert!(out.get("zone_id").is_some());
        let n = host.verify_chain().expect("receipt chain must verify");
        assert!(n >= 2, "register + calls = >=2 receipts, got {n}");
    }

    #[test]
    fn sham_ping_static_manifest_validates() {
        let v: Value = serde_json::from_str(include_str!("../../../organelles/sham-ping.manifest.json")).expect("manifest JSON parses");
        let m = validate_manifest_json(&v).expect("static manifest must validate");
        assert_eq!(m.name, "sham-ping");
        assert_eq!(m.codon, "vn");
    }

    #[test]
    fn sham_ping_rust_manifest_matches_static_json() {
        let from_rust = serde_json::to_value(sham_ping_manifest()).unwrap();
        let from_file: Value = serde_json::from_str(include_str!("../../../organelles/sham-ping.manifest.json")).unwrap();
        assert_eq!(from_rust, from_file, "Rust manifest() and organelles/*.json must agree");
    }

    #[test]
    fn sham_ping_registers_and_serves_through_host_with_receipts() {
        let host = Host::new();
        host.register(sham_ping_organelle()).expect("registration must succeed");

        let ctx = CallContext::new("migration-test", vec![]);
        let out = host
            .call(&ctx, "vn", "validate", serde_json::json!({"lat": 49.28, "lon": -123.12, "ts_ms": 1000}))
            .expect("validate call");
        assert_eq!(out.get("valid").and_then(|v| v.as_bool()), Some(true));
        let out = host
            .call(&ctx, "vn", "validate", serde_json::json!({"lat": 999.0, "lon": -123.12, "ts_ms": 1000}))
            .expect("validate call");
        assert_eq!(out.get("valid").and_then(|v| v.as_bool()), Some(false));
        let n = host.verify_chain().expect("receipt chain must verify");
        assert!(n >= 2, "register + calls = >=2 receipts, got {n}");
    }

    #[test]
    fn sham_eta_static_manifest_validates() {
        let v: Value = serde_json::from_str(include_str!("../../../organelles/sham-eta.manifest.json")).expect("manifest JSON parses");
        let m = validate_manifest_json(&v).expect("static manifest must validate");
        assert_eq!(m.name, "sham-eta");
        assert_eq!(m.codon, "et");
    }

    #[test]
    fn sham_eta_rust_manifest_matches_static_json() {
        let from_rust = serde_json::to_value(sham_eta_manifest()).unwrap();
        let from_file: Value = serde_json::from_str(include_str!("../../../organelles/sham-eta.manifest.json")).unwrap();
        assert_eq!(from_rust, from_file, "Rust manifest() and organelles/*.json must agree");
    }

    #[test]
    fn sham_eta_registers_and_serves_through_host_with_receipts() {
        let host = Host::new();
        host.register(sham_eta_organelle()).expect("registration must succeed");

        let ctx = CallContext::new("migration-test", vec![]);
        let out = host
            .call(&ctx, "et", "eta", serde_json::json!({"dist_m": 1000.0, "speed_kmh": 30.0}))
            .expect("eta call");
        assert!(out.get("eta_min").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.0);
        let n = host.verify_chain().expect("receipt chain must verify");
        assert!(n >= 2, "register + calls = >=2 receipts, got {n}");
    }
}
