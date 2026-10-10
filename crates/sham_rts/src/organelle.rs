//! sham_rts gesture recognizer as an organelle (Tier A native migration). Touch events in, gestures out.
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
use crate::RtsUi;
use sham_spatial::{BBox, GeoCoord};

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

fn parse_touch(v: &Value) -> Result<crate::Touch, OrganelleError> {
    let lat = v.get("lat").and_then(|x| x.as_f64())
        .ok_or_else(|| OrganelleError::failed("touch.lat required"))?;
    let lon = v.get("lon").and_then(|x| x.as_f64())
        .ok_or_else(|| OrganelleError::failed("touch.lon required"))?;
    let t_ms = v.get("t_ms").and_then(|x| x.as_u64()).unwrap_or(0);
    let at = GeoCoord::new(lat, lon)
        .map_err(|e| OrganelleError::failed(format!("bad touch coord: {e}")))?;
    Ok(crate::Touch { at, t_ms })
}

fn gesture_json(g: &Option<crate::Gesture>) -> Value {
    match g {
        Some(x) => serde_json::json!({"gesture": format!("{x:?}")}),
        None => serde_json::json!({"gesture": null}),
    }
}


fn sham_gesture_manifest() -> OrganelleManifest {
    OrganelleManifest {
        schema_version: MANIFEST_SCHEMA_VERSION.to_string(),
        name: "sham-gesture".to_string(),
        codon: "sg".to_string(),
        version: "0.1.0".to_string(),
        description: "touch gesture recognition: tap, double-tap, drag, long-press, paint (organelle)".to_string(),
        contracts: vec![
            Contract {
                op: "touch".to_string(),
                description: "feed one touch event; returns the recognized gesture if any".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["lat","lon"],"properties":{"lat":{"type":"number"},"lon":{"type":"number"},"t_ms":{"type":"integer"}}}),
                mutation_class: Some(MutationClass::Write),
                required_capability: Some("sham.gesture.write".to_string()),
            },
            Contract {
                op: "flush_tap".to_string(),
                description: "flush a pending tap (timeout expiry)".to_string(),
                input_schema: serde_json::json!({"type":"object"}),
                mutation_class: Some(MutationClass::Write),
                required_capability: Some("sham.gesture.write".to_string()),
            },
            Contract {
                op: "drag".to_string(),
                description: "feed a drag from->to; returns the drag gesture".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["from","to"],"properties":{"from":{"type":"object"},"to":{"type":"object"}}}),
                mutation_class: Some(MutationClass::Write),
                required_capability: Some("sham.gesture.write".to_string()),
            },
            Contract {
                op: "paint".to_string(),
                description: "paint mode: accumulate cells; returns cell count".to_string(),
                input_schema: serde_json::json!({"type":"object","required":["cells"],"properties":{"cells":{"type":"array"}}}),
                mutation_class: Some(MutationClass::Write),
                required_capability: Some("sham.gesture.write".to_string()),
            },
        ],
        authority: Authority {
            mutation_class: MutationClass::Write,
            declared_valence: Valence::Candidate,
            required_capability: Some("sham.gesture.write".to_string()),
            granted_capabilities: vec!["sham.gesture.write".to_string()],
        },
        transports: vec![Transport::Native, Transport::UnixSocket],
        receipt_policy: ReceiptPolicy {
            content_addressed: true,
            algorithm: RECEIPT_ALGORITHM.to_string(),
            chain: true,
        },
    }
}

pub struct ShamGestureOrg {
    manifest: OrganelleManifest,
    inner: Mutex<RtsUi>,
}

pub fn sham_gesture_organelle() -> Arc<dyn Organelle> {
    Arc::new(ShamGestureOrg {
        manifest: sham_gesture_manifest(),
            inner: Mutex::new(RtsUi::new()),
    })
}

impl Organelle for ShamGestureOrg {
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
            "touch" => {
                let t = parse_touch(&input)?;
                                let mut inner = self.inner.lock().unwrap();
                                Ok(gesture_json(&inner.touch(t)))
            },
            "flush_tap" => {
                let mut inner = self.inner.lock().unwrap();
                                Ok(gesture_json(&inner.flush_tap()))
            },
            "drag" => {
                let from = parse_touch(&input["from"])?;
                                let to = parse_touch(&input["to"])?;
                                let mut inner = self.inner.lock().unwrap();
                                let g = inner.drag(from, to);
                                Ok(serde_json::json!({"gesture": format!("{g:?}")}))
            },
            "paint" => {
                let cells: Vec<BBox> = input["cells"].as_array().cloned().unwrap_or_default()
                                    .iter().map(|c| BBox {
                                        min_lat: c.get("min_lat").and_then(|x| x.as_f64()).unwrap_or(0.0),
                                        max_lat: c.get("max_lat").and_then(|x| x.as_f64()).unwrap_or(0.0),
                                        min_lon: c.get("min_lon").and_then(|x| x.as_f64()).unwrap_or(0.0),
                                        max_lon: c.get("max_lon").and_then(|x| x.as_f64()).unwrap_or(0.0),
                                    }).collect();
                                let mut inner = self.inner.lock().unwrap();
                                let n = inner.paint(cells);
                                Ok(serde_json::json!({"cells": n}))
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
    fn sham_gesture_static_manifest_validates() {
        let v: Value = serde_json::from_str(include_str!("../../../organelles/sham-gesture.manifest.json")).expect("manifest JSON parses");
        let m = validate_manifest_json(&v).expect("static manifest must validate");
        assert_eq!(m.name, "sham-gesture");
        assert_eq!(m.codon, "sg");
    }

    #[test]
    fn sham_gesture_rust_manifest_matches_static_json() {
        let from_rust = serde_json::to_value(sham_gesture_manifest()).unwrap();
        let from_file: Value = serde_json::from_str(include_str!("../../../organelles/sham-gesture.manifest.json")).unwrap();
        assert_eq!(from_rust, from_file, "Rust manifest() and organelles/*.json must agree");
    }

    #[test]
    fn sham_gesture_registers_and_serves_through_host_with_receipts() {
        let host = Host::new();
        host.register(sham_gesture_organelle()).expect("registration must succeed");

        let ctx = CallContext::new("migration-test", vec!["sham.gesture.write".to_string()]);
        let out = host
            .call(&ctx, "sg", "touch", serde_json::json!({"lat": 49.28, "lon": -123.12, "t_ms": 100}))
            .expect("touch call");
        assert!(out.get("gesture").is_some());
        let out = host
            .call(&ctx, "sg", "paint", serde_json::json!({"cells": [{"min_lat": 49.28, "max_lat": 49.29, "min_lon": -123.13, "max_lon": -123.12}]}))
            .expect("paint call");
        assert_eq!(out.get("cells").and_then(|v| v.as_u64()), Some(1));
        let n = host.verify_chain().expect("receipt chain must verify");
        assert!(n >= 2, "register + calls = >=2 receipts, got {n}");
    }
}
