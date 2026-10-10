//! sham_odin_svc as an organelle (Tier A native migration). ODIN feature request execution.
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
use crate::{OdinOutcome, OdinRequest, OdinService};


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


fn sham_odin_svc_manifest() -> OrganelleManifest {
    OrganelleManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        name: "sham-odin-svc".to_string(),
        codon: "od".to_string(),
        version: "0.1.0".to_string(),
        description: "ODIN dispatch service: execute feature requests with receipted outcomes (organelle)".to_string(),
        contracts: vec![
            Contract {
                op: "execute".to_string(),
                description: "execute an ODIN feature request; unknown features are rejected (fail-closed)".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["feature_id"],"properties":{"feature_id":{"type":"string"},"args":{"type":"object"},"token":{"type":"string"}}}),
                mutation_class: Some(MutationClass::Write),
                required_capability: Some("sham.odin.execute".to_string()),
            },
            Contract {
                op: "features_in".to_string(),
                description: "count features matching a batch (service inventory probe)".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["batch"],"properties":{"batch":{"type":"string","enum":["dispatch","tracking","comms","reporting","admin","field_ops","analytics","integrations"]}}}),
                mutation_class: None,
                required_capability: None,
            },
        ],
        authority: Authority {
            mutation_class: MutationClass::Write,
            declared_valence: Valence::Candidate,
            required_capability: Some("sham.odin.execute".to_string()),
            granted_capabilities: vec!["sham.odin.execute".to_string()],
        },
        transports: vec![Transport::Native, Transport::UnixSocket],
        receipt_policy: ReceiptPolicy {
            content_addressed: true,
            algorithm: RECEIPT_ALGORITHM.to_string(),
            chain: true,
        },
    }
}

pub struct ShamOdinSvcOrg {
    manifest: OrganelleManifest,
    inner: Mutex<OdinService>,
}

pub fn sham_odin_svc_organelle() -> Arc<dyn Organelle> {
    Arc::new(ShamOdinSvcOrg {
        manifest: sham_odin_svc_manifest(),
            inner: Mutex::new(OdinService::new()),
    })
}

impl Organelle for ShamOdinSvcOrg {
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
            "execute" => {
                let args: Vec<(String, String)> = input.get("args").and_then(|v| v.as_object())
                                    .map(|m| m.iter().map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string())).collect())
                                    .unwrap_or_default();
                                let reqo = OdinRequest {
                                    feature_id: req(&input, "feature_id")?.to_string(),
                                    args,
                                    token: opt_str(&input, "token").unwrap_or_default(),
                                };
                                let mut inner = self.inner.lock().unwrap();
                                match inner.execute(&reqo) {
                                    OdinOutcome::Ok(s) => Ok(serde_json::json!({"ok": true, "result": s})),
                                    OdinOutcome::Rejected(s) => Ok(serde_json::json!({"ok": false, "rejected": s})),
                                }
            },
            "features_in" => {
                let b = match req(&input, "batch")? {
                                    "dispatch" => sham_odin::Batch::Dispatch,
                                    "tracking" => sham_odin::Batch::Tracking,
                                    "comms" => sham_odin::Batch::Comms,
                                    "reporting" => sham_odin::Batch::Reporting,
                                    "admin" => sham_odin::Batch::Admin,
                                    "field_ops" => sham_odin::Batch::FieldOps,
                                    "analytics" => sham_odin::Batch::Analytics,
                                    "integrations" => sham_odin::Batch::Integrations,
                                    other => return Err(OrganelleError::failed(format!("unknown batch '{other}'"))),
                                };
                                let inner = self.inner.lock().unwrap();
                                let n = inner.features_in(b);
                                Ok(serde_json::json!({"features": n}))
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
    fn sham_odin_svc_static_manifest_validates() {
        let v: Value = serde_json::from_str(include_str!("../../../organelles/sham-odin-svc.manifest.json")).expect("manifest JSON parses");
        let m = validate_manifest_json(&v).expect("static manifest must validate");
        assert_eq!(m.name, "sham-odin-svc");
        assert_eq!(m.codon, "od");
    }

    #[test]
    fn sham_odin_svc_rust_manifest_matches_static_json() {
        let from_rust = serde_json::to_value(sham_odin_svc_manifest()).unwrap();
        let from_file: Value = serde_json::from_str(include_str!("../../../organelles/sham-odin-svc.manifest.json")).unwrap();
        assert_eq!(from_rust, from_file, "Rust manifest() and organelles/*.json must agree");
    }

    #[test]
    fn sham_odin_svc_registers_and_serves_through_host_with_receipts() {
        let host = Host::new();
        host.register(sham_odin_svc_organelle()).expect("registration must succeed");

        let ctx = CallContext::new("migration-test", vec!["sham.odin.execute".to_string()]);
        // unknown feature must be rejected fail-closed, not panic
        let out = host
            .call(&ctx, "od", "execute", serde_json::json!({"feature_id": "nope"}))
            .expect("execute call");
        assert_eq!(out.get("ok").and_then(|v| v.as_bool()), Some(false));
        let n = host.verify_chain().expect("receipt chain must verify");
        assert!(n >= 2, "register + calls = >=2 receipts, got {n}");
    }
}
