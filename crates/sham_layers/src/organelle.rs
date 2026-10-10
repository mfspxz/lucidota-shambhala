//! sham_layers layer stack as an organelle (Tier A native migration). Map layer management.
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
use crate::{LayerKind, LayerStack};

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

fn parse_kind(s: &str) -> Result<LayerKind, OrganelleError> {
    match s {
        "orthomosaic" => Ok(LayerKind::Orthomosaic),
        "zone" => Ok(LayerKind::Zone),
        "street" => Ok(LayerKind::Street),
        "point" => Ok(LayerKind::Point),
        "flow" => Ok(LayerKind::Flow),
        "heatmap" => Ok(LayerKind::Heatmap),
        "annotation" => Ok(LayerKind::Annotation),
        other => Err(OrganelleError::failed(format!("unknown layer kind '{other}'"))),
    }
}


fn sham_layers_manifest() -> OrganelleManifest {
    OrganelleManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        name: "sham-layers".to_string(),
        codon: "ly".to_string(),
        version: "0.1.0".to_string(),
        description: "map layer stack: add, toggle, opacity, render order (organelle)".to_string(),
        contracts: vec![
            Contract {
                op: "add".to_string(),
                description: "add a layer; returns its index".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["name","kind","z"],"properties":{"name":{"type":"string"},"kind":{"type":"string"},"z":{"type":"integer"}}}),
                mutation_class: Some(MutationClass::Write),
                required_capability: Some("sham.layers.write".to_string()),
            },
            Contract {
                op: "toggle".to_string(),
                description: "toggle layer visibility; returns new state".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["idx"],"properties":{"idx":{"type":"integer"}}}),
                mutation_class: Some(MutationClass::Write),
                required_capability: Some("sham.layers.write".to_string()),
            },
            Contract {
                op: "set_opacity".to_string(),
                description: "set layer opacity 0..1".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["idx","opacity"],"properties":{"idx":{"type":"integer"},"opacity":{"type":"number"}}}),
                mutation_class: Some(MutationClass::Write),
                required_capability: Some("sham.layers.write".to_string()),
            },
            Contract {
                op: "render_order".to_string(),
                description: "layers in render order (bottom to top)".to_string(),
                input_schema: serde_json::json!({"type":"object"}),
                mutation_class: None,
                required_capability: None,
            },
            Contract {
                op: "by_kind".to_string(),
                description: "layers of one kind".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["kind"],"properties":{"kind":{"type":"string"}}}),
                mutation_class: None,
                required_capability: None,
            },
        ],
        authority: Authority {
            mutation_class: MutationClass::Write,
            declared_valence: Valence::Candidate,
            required_capability: Some("sham.layers.write".to_string()),
            granted_capabilities: vec!["sham.layers.write".to_string()],
        },
        transports: vec![Transport::Native, Transport::UnixSocket],
        receipt_policy: ReceiptPolicy {
            content_addressed: true,
            algorithm: RECEIPT_ALGORITHM.to_string(),
            chain: true,
        },
    }
}

pub struct ShamLayersOrg {
    manifest: OrganelleManifest,
    inner: Mutex<LayerStack>,
}

pub fn sham_layers_organelle() -> Arc<dyn Organelle> {
    Arc::new(ShamLayersOrg {
        manifest: sham_layers_manifest(),
            inner: Mutex::new(LayerStack::new()),
    })
}

impl Organelle for ShamLayersOrg {
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
                let mut inner = self.inner.lock().unwrap();
                                let idx = inner.add(req(&input, "name")?, parse_kind(req(&input, "kind")?)?,
                                    input.get("z").and_then(|v| v.as_i64()).unwrap_or(0) as i32);
                                Ok(serde_json::json!({"idx": idx}))
            },
            "toggle" => {
                let mut inner = self.inner.lock().unwrap();
                                let on = inner.toggle(input.get("idx").and_then(|v| v.as_u64()).unwrap_or(0) as usize)
                                    .map_err(|e| OrganelleError::failed(format!("toggle failed: {e}")))?;
                                Ok(serde_json::json!({"visible": on}))
            },
            "set_opacity" => {
                let mut inner = self.inner.lock().unwrap();
                                inner.set_opacity(input.get("idx").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
                                        input.get("opacity").and_then(|v| v.as_f64()).unwrap_or(1.0) as f32)
                                    .map_err(|e| OrganelleError::failed(format!("set_opacity failed: {e}")))?;
                                Ok(serde_json::json!({"ok": true}))
            },
            "render_order" => {
                let inner = self.inner.lock().unwrap();
                                let names: Vec<String> = inner.render_order().iter().map(|l| format!("{l:?}")).collect();
                                Ok(serde_json::json!({"layers": names}))
            },
            "by_kind" => {
                let inner = self.inner.lock().unwrap();
                                let names: Vec<String> = inner.by_kind(parse_kind(req(&input, "kind")?)?)
                                    .iter().map(|l| format!("{l:?}")).collect();
                                Ok(serde_json::json!({"layers": names}))
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
    fn sham_layers_static_manifest_validates() {
        let v: Value = serde_json::from_str(include_str!("../../../organelles/sham-layers.manifest.json")).expect("manifest JSON parses");
        let m = validate_manifest_json(&v).expect("static manifest must validate");
        assert_eq!(m.name, "sham-layers");
        assert_eq!(m.codon, "ly");
    }

    #[test]
    fn sham_layers_rust_manifest_matches_static_json() {
        let from_rust = serde_json::to_value(sham_layers_manifest()).unwrap();
        let from_file: Value = serde_json::from_str(include_str!("../../../organelles/sham-layers.manifest.json")).unwrap();
        assert_eq!(from_rust, from_file, "Rust manifest() and organelles/*.json must agree");
    }

    #[test]
    fn sham_layers_registers_and_serves_through_host_with_receipts() {
        let host = Host::new();
        host.register(sham_layers_organelle()).expect("registration must succeed");

        let ctx = CallContext::new("migration-test", vec!["sham.layers.write".to_string()]);
        let out = host
            .call(&ctx, "ly", "add", serde_json::json!({"name": "zones", "kind": "zone", "z": 1}))
            .expect("add call");
        assert_eq!(out.get("idx").and_then(|v| v.as_u64()), Some(0));
        let out = host
            .call(&ctx, "ly", "render_order", serde_json::json!({}))
            .expect("render_order call");
        assert_eq!(out.get("layers").and_then(|v| v.as_array()).map(|a| a.len()), Some(1));
        let out = host
            .call(&ctx, "ly", "toggle", serde_json::json!({"idx": 0}))
            .expect("toggle call");
        assert!(out.get("visible").and_then(|v| v.as_bool()).is_some());
        let n = host.verify_chain().expect("receipt chain must verify");
        assert!(n >= 2, "register + calls = >=2 receipts, got {n}");
    }
}
