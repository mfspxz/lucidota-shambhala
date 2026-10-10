//! sham_redteam campaign as an organelle (Tier A native migration). Executable adversarial findings.
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
use crate::campaign;

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


fn sham_redteam_manifest() -> OrganelleManifest {
    OrganelleManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        name: "sham-redteam".to_string(),
        codon: "st".to_string(),
        version: "0.1.0".to_string(),
        description: "adversarial campaign: executable attacks with fail-closed verdicts (organelle)".to_string(),
        contracts: vec![
            Contract {
                op: "campaign".to_string(),
                description: "run all attacks in order; summarize held / pinned / regressions".to_string(),
                input_schema: serde_json::json!({"type":"object"}),
                mutation_class: None,
                required_capability: None,
            },
            Contract {
                op: "passes".to_string(),
                description: "true iff the campaign has no open findings".to_string(),
                input_schema: serde_json::json!({"type":"object"}),
                mutation_class: None,
                required_capability: None,
            },
        ],
        authority: Authority {
            mutation_class: MutationClass::ReadOnly,
            declared_valence: Valence::Candidate,
            required_capability: None,
            granted_capabilities: vec![],
        },
        transports: vec![Transport::Native, Transport::UnixSocket],
        receipt_policy: ReceiptPolicy {
            content_addressed: true,
            algorithm: RECEIPT_ALGORITHM.to_string(),
            chain: true,
        },
    }
}

pub struct ShamRedteamOrg {
    manifest: OrganelleManifest,
    _priv: (),
}

pub fn sham_redteam_organelle() -> Arc<dyn Organelle> {
    Arc::new(ShamRedteamOrg {
        manifest: sham_redteam_manifest(),
            _priv: (),
    })
}

impl Organelle for ShamRedteamOrg {
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
            "campaign" => {
                let findings = campaign();
                                let items: Vec<Value> = findings.iter().map(|f| {
                                    serde_json::json!({"attack": f.attack, "verdict": format!("{:?}", f.verdict), "detail": f.detail})
                                }).collect();
                                Ok(serde_json::json!({"findings": items, "count": items.len()}))
            },
            "passes" => {
                let findings = campaign();
                                let open = findings.iter().filter(|f| format!("{:?}", f.verdict).contains("Open")).count();
                                Ok(serde_json::json!({"passes": open == 0, "open_findings": open}))
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
    fn sham_redteam_static_manifest_validates() {
        let v: Value = serde_json::from_str(include_str!("../../../organelles/sham-redteam.manifest.json")).expect("manifest JSON parses");
        let m = validate_manifest_json(&v).expect("static manifest must validate");
        assert_eq!(m.name, "sham-redteam");
        assert_eq!(m.codon, "st");
    }

    #[test]
    fn sham_redteam_rust_manifest_matches_static_json() {
        let from_rust = serde_json::to_value(sham_redteam_manifest()).unwrap();
        let from_file: Value = serde_json::from_str(include_str!("../../../organelles/sham-redteam.manifest.json")).unwrap();
        assert_eq!(from_rust, from_file, "Rust manifest() and organelles/*.json must agree");
    }

    #[test]
    fn sham_redteam_registers_and_serves_through_host_with_receipts() {
        let host = Host::new();
        host.register(sham_redteam_organelle()).expect("registration must succeed");

        let ctx = CallContext::new("migration-test", vec![]);
        let out = host
            .call(&ctx, "st", "campaign", serde_json::json!({}))
            .expect("campaign call");
        assert!(out.get("count").and_then(|v| v.as_u64()).is_some());
        let out = host
            .call(&ctx, "st", "passes", serde_json::json!({}))
            .expect("passes call");
        assert!(out.get("passes").and_then(|v| v.as_bool()).is_some());
        let n = host.verify_chain().expect("receipt chain must verify");
        assert!(n >= 2, "register + calls = >=2 receipts, got {n}");
    }
}
