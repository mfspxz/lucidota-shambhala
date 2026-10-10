//! sham_offline op-log as an organelle (Tier A native migration). Offline-first op log with merge.
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
use crate::{merge, runbook, Op, OpKind, OpLog};

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

fn parse_kind(v: &Value) -> Result<OpKind, OrganelleError> {
    let t = v.get("type").and_then(|x| x.as_str())
        .ok_or_else(|| OrganelleError::failed("kind.type required"))?;
    let u = |k: &str| v.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    match t {
        "add_zone" => Ok(OpKind::AddZone { id: u("id"), name: s("name"), capacity: u("capacity") as u32 }),
        "add_car" => Ok(OpKind::AddCar { id: u("id"), plate: s("plate") }),
        "park" => Ok(OpKind::Park { car: u("car"), zone: u("zone") }),
        "release" => Ok(OpKind::Release { car: u("car") }),
        "note" => Ok(OpKind::Note { entity: s("entity"), id: u("id"), text: s("text") }),
        other => Err(OrganelleError::failed(format!("unknown op kind '{other}'"))),
    }
}

fn parse_op(v: &Value) -> Result<Op, OrganelleError> {
    Ok(Op {
        lamport: v.get("lamport").and_then(|x| x.as_u64()).unwrap_or(0),
        device: v.get("device").and_then(|x| x.as_str()).unwrap_or("").to_string(),
        kind: parse_kind(&v["kind"])?,
    })
}

fn op_json(o: &Op) -> Value {
    serde_json::json!({"lamport": o.lamport, "device": o.device, "kind": format!("{:?}", o.kind)})
}


fn sham_offline_manifest() -> OrganelleManifest {
    OrganelleManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        name: "sham-offline".to_string(),
        codon: "sb".to_string(),
        version: "0.1.0".to_string(),
        description: "offline-first op log: push, merge, replay; festival-map sync primitives (organelle)".to_string(),
        contracts: vec![
            Contract {
                op: "push".to_string(),
                description: "append an op to this device's log (lamport clock)".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["kind"],"properties":{"kind":{"type":"object"}}}),
                mutation_class: Some(MutationClass::Write),
                required_capability: Some("sham.offline.write".to_string()),
            },
            Contract {
                op: "ops".to_string(),
                description: "all ops in this device's log".to_string(),
                input_schema: serde_json::json!({"type":"object"}),
                mutation_class: None,
                required_capability: None,
            },
            Contract {
                op: "len".to_string(),
                description: "op count".to_string(),
                input_schema: serde_json::json!({"type":"object"}),
                mutation_class: None,
                required_capability: None,
            },
            Contract {
                op: "merge".to_string(),
                description: "merge two op arrays (lamport order, deterministic)".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["a","b"],"properties":{"a":{"type":"array"},"b":{"type":"array"}}}),
                mutation_class: None,
                required_capability: None,
            },
            Contract {
                op: "runbook".to_string(),
                description: "offline sync runbook steps".to_string(),
                input_schema: serde_json::json!({"type":"object"}),
                mutation_class: None,
                required_capability: None,
            },
        ],
        authority: Authority {
            mutation_class: MutationClass::Write,
            declared_valence: Valence::Candidate,
            required_capability: Some("sham.offline.write".to_string()),
            granted_capabilities: vec!["sham.offline.write".to_string()],
        },
        transports: vec![Transport::Native, Transport::UnixSocket],
        receipt_policy: ReceiptPolicy {
            content_addressed: true,
            algorithm: RECEIPT_ALGORITHM.to_string(),
            chain: true,
        },
    }
}

pub struct ShamOfflineOrg {
    manifest: OrganelleManifest,
    inner: Mutex<OpLog>,
}

pub fn sham_offline_organelle() -> Arc<dyn Organelle> {
    Arc::new(ShamOfflineOrg {
        manifest: sham_offline_manifest(),
            inner: Mutex::new(OpLog::new("organelle")),
    })
}

impl Organelle for ShamOfflineOrg {
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
            "push" => {
                let kind = parse_kind(&input["kind"])?;
                                let mut inner = self.inner.lock().unwrap();
                                let op = inner.push(kind);
                                Ok(op_json(op))
            },
            "ops" => {
                let inner = self.inner.lock().unwrap();
                                Ok(serde_json::json!({"ops": inner.ops.iter().map(op_json).collect::<Vec<_>>()}))
            },
            "len" => {
                let inner = self.inner.lock().unwrap();
                                Ok(serde_json::json!({"len": inner.len()}))
            },
            "merge" => {
                let mut la = OpLog::new("a");
                                for v in input["a"].as_array().cloned().unwrap_or_default() {
                                    la.ops.push(parse_op(&v)?);
                                }
                                let mut lb = OpLog::new("b");
                                for v in input["b"].as_array().cloned().unwrap_or_default() {
                                    lb.ops.push(parse_op(&v)?);
                                }
                                let m = merge(&la, &lb);
                                Ok(serde_json::json!({"merged": m.iter().map(op_json).collect::<Vec<_>>()}))
            },
            "runbook" => {
                let steps = runbook();
                                Ok(serde_json::json!({"steps": steps.iter().map(|s| format!("{s:?}")).collect::<Vec<_>>()}))
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
    fn sham_offline_static_manifest_validates() {
        let v: Value = serde_json::from_str(include_str!("../../../organelles/sham-offline.manifest.json")).expect("manifest JSON parses");
        let m = validate_manifest_json(&v).expect("static manifest must validate");
        assert_eq!(m.name, "sham-offline");
        assert_eq!(m.codon, "sb");
    }

    #[test]
    fn sham_offline_rust_manifest_matches_static_json() {
        let from_rust = serde_json::to_value(sham_offline_manifest()).unwrap();
        let from_file: Value = serde_json::from_str(include_str!("../../../organelles/sham-offline.manifest.json")).unwrap();
        assert_eq!(from_rust, from_file, "Rust manifest() and organelles/*.json must agree");
    }

    #[test]
    fn sham_offline_registers_and_serves_through_host_with_receipts() {
        let host = Host::new();
        host.register(sham_offline_organelle()).expect("registration must succeed");

        let ctx = CallContext::new("migration-test", vec!["sham.offline.write".to_string()]);
        let out = host
            .call(&ctx, "sb", "push", serde_json::json!({"kind": {"type": "add_zone", "id": 1, "name": "z1", "capacity": 10}}))
            .expect("push call");
        assert_eq!(out.get("lamport").and_then(|v| v.as_u64()), Some(1));
        let out = host
            .call(&ctx, "sb", "len", serde_json::json!({}))
            .expect("len call");
        assert_eq!(out.get("len").and_then(|v| v.as_u64()), Some(1));
        let out = host
            .call(&ctx, "sb", "merge", serde_json::json!({"a": [{"lamport": 2, "device": "d1", "kind": {"type": "park", "car": 1, "zone": 1}}], "b": [{"lamport": 1, "device": "d2", "kind": {"type": "add_car", "id": 1, "plate": "x"}}]}))
            .expect("merge call");
        assert_eq!(out.get("merged").and_then(|v| v.as_array()).map(|a| a.len()), Some(2));
        let n = host.verify_chain().expect("receipt chain must verify");
        assert!(n >= 2, "register + calls = >=2 receipts, got {n}");
    }
}
