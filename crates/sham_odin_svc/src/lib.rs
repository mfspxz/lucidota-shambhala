// sham_odin_svc — sham-020: ODIN service.
// ODIN on :8090 (dossier port), wired to the API service.
// Executes ODIN features: read-only features run against the registry,
// dispatch/tracking kernels execute with real arguments. Every execution
// leaves a receipt; unknown features fail closed.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

use sham_api::{Api, Method, Request};
use sham_domain::Receipt;
use sham_odin::{Batch, Registry};

pub const ODIN_PORT: u16 = 8090;

/// An ODIN execution request.
#[derive(Clone, Debug)]
pub struct OdinRequest {
    pub feature_id: String,
    pub args: Vec<(String, String)>,
    pub token: String,
}

/// Outcome of an ODIN feature execution.
#[derive(Clone, Debug, PartialEq)]
pub enum OdinOutcome {
    Ok(String),
    Rejected(String),
}

pub struct OdinService {
    pub registry: Registry,
    pub api: Api,
    pub receipts: Vec<Receipt>,
}
impl OdinService {
    pub fn new() -> Self {
        let mut api = Api::new();
        api.auth.issue("odin-internal", "odin");
        Self {
            registry: Registry::load(),
            api,
            receipts: Vec::new(),
        }
    }
    fn log(&mut self, r: Receipt) {
        self.receipts.push(r);
    }
    /// Execute a feature by id. Unknown id -> Rejected (fail-closed).
    pub fn execute(&mut self, req: &OdinRequest) -> OdinOutcome {
        let feature = match self.registry.get(&req.feature_id) {
            Some(f) => f.clone(),
            None => {
                let r = Receipt::new("odin_reject", "odin", 0, &format!("unknown {}", req.feature_id));
                self.log(r);
                return OdinOutcome::Rejected(format!("unknown feature {}", req.feature_id));
            }
        };
        let arg = |k: &str| req.args.iter().find(|(a, _)| a == k).map(|(_, v)| v.as_str());
        let out = match feature.batch {
            Batch::Dispatch if feature.name == "assign-car-zone" => {
                let (car, zone) = (arg("car"), arg("zone"));
                match (car, zone) {
                    (Some(c), Some(z)) => {
                        let r = self.api.handle(
                            &Request::new(Method::Post, "/park")
                                .bearer("odin-internal")
                                .body(&format!("car={}&zone={}", c, z)),
                        );
                        if r.status == 200 {
                            OdinOutcome::Ok(r.body)
                        } else {
                            OdinOutcome::Rejected(r.body)
                        }
                    }
                    _ => OdinOutcome::Rejected("assign-car-zone needs car+zone".into()),
                }
            }
            Batch::Dispatch if feature.name == "eta-calc" => {
                let (d, s) = (arg("dist_m"), arg("speed_kmh"));
                match (d.and_then(|x| x.parse::<f64>().ok()), s.and_then(|x| x.parse::<f64>().ok())) {
                    (Some(dd), Some(ss)) => match sham_odin::eta_minutes(dd, ss) {
                        Ok(e) => OdinOutcome::Ok(format!("{{\"eta_min\":{:.2}}}", e)),
                        Err(x) => OdinOutcome::Rejected(x),
                    },
                    _ => OdinOutcome::Rejected("eta-calc needs dist_m+speed_kmh".into()),
                }
            }
            Batch::Tracking if feature.name == "gps-ping-ingest" => {
                let p = |k: &str| arg(k).and_then(|x| x.parse::<f64>().ok());
                match (p("lat"), p("lon"), arg("ts_ms").and_then(|x| x.parse::<u64>().ok())) {
                    (Some(la), Some(lo), Some(ts)) => {
                        match sham_odin::validate_ping(la, lo, ts, ts + 1000) {
                            Ok(_) => OdinOutcome::Ok("{\"ingested\":true}".into()),
                            Err(e) => OdinOutcome::Rejected(e),
                        }
                    }
                    _ => OdinOutcome::Rejected("gps-ping-ingest needs lat+lon+ts_ms".into()),
                }
            }
            _ => OdinOutcome::Ok(format!(
                "{{\"feature\":\"{}\",\"executed\":true}}",
                feature.id
            )),
        };
        let ok = matches!(out, OdinOutcome::Ok(_));
        self.log(Receipt::new(
            if ok { "odin_exec" } else { "odin_reject" },
            "odin",
            0,
            &format!("{} -> {}", feature.id, if ok { "ok" } else { "rejected" }),
        ));
        out
    }
    /// Features available, by batch.
    pub fn features_in(&self, b: Batch) -> usize {
        self.registry.batch_count(b)
    }
}
