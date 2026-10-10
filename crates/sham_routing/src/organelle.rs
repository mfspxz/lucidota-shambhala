//! sham_routing A* as an organelle (Tier A native migration). Real street-graph pathfinding.
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
use sham_domain::{PointId, Street, StreetId, VehicleClass};
use crate::RoadGraph;
use sham_spatial::GeoCoord;
use std::collections::HashMap;

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


fn sham_astar_manifest() -> OrganelleManifest {
    OrganelleManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        name: "sham-astar".to_string(),
        codon: "sa".to_string(),
        version: "0.1.0".to_string(),
        description: "A* route over a street graph with width/one-way/vehicle-class constraints (organelle)".to_string(),
        contracts: vec![
            Contract {
                op: "route".to_string(),
                description: "A* path from->to over the supplied street graph".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["nodes","streets","from","to","class"],"properties":{"nodes":{"type":"array"},"streets":{"type":"array"},"from":{"type":"integer"},"to":{"type":"integer"},"class":{"type":"string"}}}),
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

pub struct ShamAstarOrg {
    manifest: OrganelleManifest,
    _priv: (),
}

pub fn sham_astar_organelle() -> Arc<dyn Organelle> {
    Arc::new(ShamAstarOrg {
        manifest: sham_astar_manifest(),
            _priv: (),
    })
}

impl Organelle for ShamAstarOrg {
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
                let mut g = RoadGraph::new();
                                let mut coords: HashMap<PointId, GeoCoord> = HashMap::new();
                                for n in input["nodes"].as_array().cloned().unwrap_or_default() {
                                    let id = PointId(n.get("id").and_then(|v| v.as_u64()).unwrap_or(0));
                                    let c = GeoCoord::new(n.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0),
                                                          n.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0))
                                        .map_err(|e| OrganelleError::failed(format!("bad node coord: {e}")))?;
                                    g.add_node(id, c);
                                    coords.insert(id, c);
                                }
                                for s in input["streets"].as_array().cloned().unwrap_or_default() {
                                    let st = Street {
                                        id: StreetId(s.get("id").and_then(|v| v.as_u64()).unwrap_or(0)),
                                        name: s.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                                        from: PointId(s.get("from").and_then(|v| v.as_u64()).unwrap_or(0)),
                                        to: PointId(s.get("to").and_then(|v| v.as_u64()).unwrap_or(0)),
                                        width_cm: s.get("width_cm").and_then(|v| v.as_u64()).unwrap_or(300) as u32,
                                        one_way: s.get("one_way").and_then(|v| v.as_bool()).unwrap_or(false),
                                        max_vehicle_len_cm: s.get("max_vehicle_len_cm").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                    };
                                    g.add_street(&st, &coords);
                                }
                                let class = parse_vehicle_class(req(&input, "class")?)?;
                                let from = PointId(input.get("from").and_then(|v| v.as_u64()).unwrap_or(0));
                                let to = PointId(input.get("to").and_then(|v| v.as_u64()).unwrap_or(0));
                                match g.route(from, to, class) {
                                    Some(r) => Ok(serde_json::json!({
                                        "streets": r.streets.iter().map(|s| s.0).collect::<Vec<_>>(),
                                        "nodes": r.nodes.iter().map(|n| n.0).collect::<Vec<_>>(),
                                        "dist_m": r.dist_m, "hops": r.hops(),
                                    })),
                                    None => Err(OrganelleError::not_found("no route between from and to")),
                                }
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
    fn sham_astar_static_manifest_validates() {
        let v: Value = serde_json::from_str(include_str!("../../../organelles/sham-astar.manifest.json")).expect("manifest JSON parses");
        let m = validate_manifest_json(&v).expect("static manifest must validate");
        assert_eq!(m.name, "sham-astar");
        assert_eq!(m.codon, "sa");
    }

    #[test]
    fn sham_astar_rust_manifest_matches_static_json() {
        let from_rust = serde_json::to_value(sham_astar_manifest()).unwrap();
        let from_file: Value = serde_json::from_str(include_str!("../../../organelles/sham-astar.manifest.json")).unwrap();
        assert_eq!(from_rust, from_file, "Rust manifest() and organelles/*.json must agree");
    }

    #[test]
    fn sham_astar_registers_and_serves_through_host_with_receipts() {
        let host = Host::new();
        host.register(sham_astar_organelle()).expect("registration must succeed");

        let ctx = CallContext::new("migration-test", vec![]);
        // triangle graph: 1->2->3 and direct 1->3
        let graph = serde_json::json!({
            "nodes": [{"id": 1, "lat": 49.28, "lon": -123.12},
                      {"id": 2, "lat": 49.281, "lon": -123.121},
                      {"id": 3, "lat": 49.282, "lon": -123.122}],
            "streets": [{"id": 10, "name": "a", "from": 1, "to": 2, "width_cm": 500},
                        {"id": 11, "name": "b", "from": 2, "to": 3, "width_cm": 500},
                        {"id": 12, "name": "c", "from": 1, "to": 3, "width_cm": 500}],
            "from": 1, "to": 3, "class": "car"});
        let out = host.call(&ctx, "sa", "route", graph).expect("route call");
        assert!(out.get("dist_m").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.0);
        assert!(out.get("hops").and_then(|v| v.as_u64()).unwrap_or(0) >= 1);
        assert!(host.call(&ctx, "sa", "route",
            serde_json::json!({"nodes": [], "streets": [], "from": 9, "to": 10, "class": "car"})).is_err());
        let n = host.verify_chain().expect("receipt chain must verify");
        assert!(n >= 2, "register + calls = >=2 receipts, got {n}");
    }
}
