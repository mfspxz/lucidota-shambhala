//! sham_vehicles registry as an organelle (Tier A native migration). Fleet registry with relations.
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
use sham_domain::VehicleClass;
use crate::{Vehicle, VehicleId, VehicleRegistry, VehicleRel};

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

fn parse_class(s: &str) -> Result<VehicleClass, OrganelleError> {
    match s {
        "car" => Ok(VehicleClass::Car), "truck" => Ok(VehicleClass::Truck), "bus" => Ok(VehicleClass::Bus),
        other => Err(OrganelleError::failed(format!("unknown class '{other}'"))),
    }
}

fn parse_rel(s: &str) -> Result<VehicleRel, OrganelleError> {
    match s {
        "convoy_with" => Ok(VehicleRel::ConvoyWith),
        "towed_by" => Ok(VehicleRel::TowedBy),
        "owner_of" => Ok(VehicleRel::OwnerOf),
        "blocked_by" => Ok(VehicleRel::BlockedBy),
        other => Err(OrganelleError::failed(format!("unknown relation '{other}'"))),
    }
}


fn sham_vehicles_manifest() -> OrganelleManifest {
    OrganelleManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        name: "sham-vehicles".to_string(),
        codon: "vh".to_string(),
        version: "0.1.0".to_string(),
        description: "fleet registry: vehicles, notes, and vehicle relations with receipts (organelle)".to_string(),
        contracts: vec![
            Contract {
                op: "add".to_string(),
                description: "register a vehicle".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["id","plate","model","year","colour","class"],"properties":{"id":{"type":"integer"},"plate":{"type":"string"},"model":{"type":"string"},"year":{"type":"integer"},"colour":{"type":"string"},"class":{"type":"string"}}}),
                mutation_class: Some(MutationClass::Write),
                required_capability: Some("sham.vehicles.write".to_string()),
            },
            Contract {
                op: "relate".to_string(),
                description: "assert a relation between two vehicles (receipted)".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["a","rel","b"],"properties":{"a":{"type":"integer"},"rel":{"type":"string"},"b":{"type":"integer"}}}),
                mutation_class: Some(MutationClass::Write),
                required_capability: Some("sham.vehicles.write".to_string()),
            },
            Contract {
                op: "related".to_string(),
                description: "vehicles related to one vehicle by relation".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["a","rel"],"properties":{"a":{"type":"integer"},"rel":{"type":"string"}}}),
                mutation_class: None,
                required_capability: None,
            },
            Contract {
                op: "count".to_string(),
                description: "registered vehicle count".to_string(),
                input_schema: serde_json::json!({"type":"object"}),
                mutation_class: None,
                required_capability: None,
            },
        ],
        authority: Authority {
            mutation_class: MutationClass::Write,
            declared_valence: Valence::Candidate,
            required_capability: Some("sham.vehicles.write".to_string()),
            granted_capabilities: vec!["sham.vehicles.write".to_string()],
        },
        transports: vec![Transport::Native, Transport::UnixSocket],
        receipt_policy: ReceiptPolicy {
            content_addressed: true,
            algorithm: RECEIPT_ALGORITHM.to_string(),
            chain: true,
        },
    }
}

pub struct ShamVehiclesOrg {
    manifest: OrganelleManifest,
    inner: Mutex<VehicleRegistry>,
}

pub fn sham_vehicles_organelle() -> Arc<dyn Organelle> {
    Arc::new(ShamVehiclesOrg {
        manifest: sham_vehicles_manifest(),
            inner: Mutex::new(VehicleRegistry::new()),
    })
}

impl Organelle for ShamVehiclesOrg {
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
            "add" => {
                let v = Vehicle::new(
                                    VehicleId(input.get("id").and_then(|x| x.as_u64()).unwrap_or(0)),
                                    req(&input, "plate")?, req(&input, "model")?,
                                    input.get("year").and_then(|x| x.as_u64()).unwrap_or(2020) as u16,
                                    req(&input, "colour")?, parse_class(req(&input, "class")?)?,
                                ).map_err(|e| OrganelleError::failed(format!("bad vehicle: {e}")))?;
                                let mut inner = self.inner.lock().unwrap();
                                inner.add(v);
                                Ok(serde_json::json!({"ok": true}))
            },
            "relate" => {
                let mut inner = self.inner.lock().unwrap();
                                let r = inner.relate(
                                    VehicleId(input.get("a").and_then(|x| x.as_u64()).unwrap_or(0)),
                                    parse_rel(req(&input, "rel")?)?,
                                    VehicleId(input.get("b").and_then(|x| x.as_u64()).unwrap_or(0)),
                                ).map_err(|e| OrganelleError::failed(format!("relate failed: {e}")))?;
                                Ok(serde_json::json!({"receipt": format!("{r:?}")}))
            },
            "related" => {
                let inner = self.inner.lock().unwrap();
                                let ids: Vec<u64> = inner.related(
                                    VehicleId(input.get("a").and_then(|x| x.as_u64()).unwrap_or(0)),
                                    parse_rel(req(&input, "rel")?)?,
                                ).iter().map(|v| v.0).collect();
                                Ok(serde_json::json!({"related": ids}))
            },
            "count" => {
                let inner = self.inner.lock().unwrap();
                                Ok(serde_json::json!({"count": inner.vehicles.len()}))
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
    fn sham_vehicles_static_manifest_validates() {
        let v: Value = serde_json::from_str(include_str!("../../../organelles/sham-vehicles.manifest.json")).expect("manifest JSON parses");
        let m = validate_manifest_json(&v).expect("static manifest must validate");
        assert_eq!(m.name, "sham-vehicles");
        assert_eq!(m.codon, "vh");
    }

    #[test]
    fn sham_vehicles_rust_manifest_matches_static_json() {
        let from_rust = serde_json::to_value(sham_vehicles_manifest()).unwrap();
        let from_file: Value = serde_json::from_str(include_str!("../../../organelles/sham-vehicles.manifest.json")).unwrap();
        assert_eq!(from_rust, from_file, "Rust manifest() and organelles/*.json must agree");
    }

    #[test]
    fn sham_vehicles_registers_and_serves_through_host_with_receipts() {
        let host = Host::new();
        host.register(sham_vehicles_organelle()).expect("registration must succeed");

        let ctx = CallContext::new("migration-test", vec!["sham.vehicles.write".to_string()]);
        let out = host
            .call(&ctx, "vh", "add", serde_json::json!({"id": 1, "plate": "ABC123", "model": "sprinter", "year": 2022, "colour": "white", "class": "truck"}))
            .expect("add call");
        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
        let out = host
            .call(&ctx, "vh", "add", serde_json::json!({"id": 2, "plate": "XYZ789", "model": "transit", "year": 2021, "colour": "blue", "class": "truck"}))
            .expect("add call");
        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(true));
        let out = host
            .call(&ctx, "vh", "relate", serde_json::json!({"a": 1, "rel": "convoy_with", "b": 2}))
            .expect("relate call");
        assert!(out.get("receipt").is_some());
        let out = host
            .call(&ctx, "vh", "related", serde_json::json!({"a": 1, "rel": "convoy_with"}))
            .expect("related call");
        assert_eq!(out.get("related").and_then(|v| v.as_array()).map(|a| a.len()), Some(1));
        let n = host.verify_chain().expect("receipt chain must verify");
        assert!(n >= 2, "register + calls = >=2 receipts, got {n}");
    }
}
