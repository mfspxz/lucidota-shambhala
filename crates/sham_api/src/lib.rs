// sham_api — sham-018: API service.
// REST API on :8080 (dossier port), OpenAPI document generation, bearer auth.
// Handler-level: Request/Response types + router; the binary binds the socket
// (no listener in tests — no daemons doctrine).
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

use sham_domain::{Car, CarId, FestivalMap, VehicleClass, Zone, ZoneId};
use std::collections::HashMap;

pub const API_PORT: u16 = 8080;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Clone, Debug)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}
impl Request {
    pub fn new(method: Method, path: &str) -> Self {
        Self {
            method,
            path: path.into(),
            headers: HashMap::new(),
            body: String::new(),
        }
    }
    pub fn bearer(mut self, token: &str) -> Self {
        self.headers.insert("authorization".into(), format!("Bearer {}", token));
        self
    }
    pub fn body(mut self, b: &str) -> Self {
        self.body = b.into();
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Response {
    pub status: u16,
    pub body: String,
}
impl Response {
    pub fn ok(body: &str) -> Self {
        Self { status: 200, body: body.into() }
    }
    pub fn created(body: &str) -> Self {
        Self { status: 201, body: body.into() }
    }
    pub fn bad(msg: &str) -> Self {
        Self { status: 400, body: format!("{{\"error\":\"{}\"}}", msg) }
    }
    pub fn unauthorized() -> Self {
        Self { status: 401, body: "{\"error\":\"unauthorized\"}".into() }
    }
    pub fn not_found() -> Self {
        Self { status: 404, body: "{\"error\":\"not found\"}".into() }
    }
}

/// Minimal auth: token -> role. Fail-closed on unknown/missing token.
#[derive(Default)]
pub struct Auth {
    tokens: HashMap<String, String>, // token -> role
}
impl Auth {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn issue(&mut self, token: &str, role: &str) {
        self.tokens.insert(token.into(), role.into());
    }
    pub fn check(&self, req: &Request) -> Result<String, Response> {
        let h = req.headers.get("authorization").map(|s| s.as_str()).unwrap_or("");
        let tok = h.strip_prefix("Bearer ").unwrap_or("");
        self.tokens.get(tok).cloned().ok_or_else(Response::unauthorized)
    }
}

/// The API application: routes requests against a FestivalMap.
pub struct Api {
    pub map: FestivalMap,
    pub auth: Auth,
    pub next_id: u64,
}
impl Api {
    pub fn new() -> Self {
        Self {
            map: FestivalMap::new(),
            auth: Auth::new(),
            next_id: 1000,
        }
    }
    fn nid(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
    pub fn handle(&mut self, req: &Request) -> Response {
        // auth gate: everything except /health needs a token
        if req.path != "/health" {
            if let Err(r) = self.auth.check(req) {
                return r;
            }
        }
        let segs: Vec<&str> = req.path.split('/').filter(|s| !s.is_empty()).collect();
        match (req.method.clone(), segs.as_slice()) {
            (Method::Get, ["health"]) => Response::ok("{\"status\":\"ok\"}"),
            (Method::Get, ["openapi.json"]) => Response::ok(&openapi_json()),
            (Method::Get, ["zones"]) => {
                let z: Vec<String> = self
                    .map
                    .zones
                    .values()
                    .map(|z| {
                        format!(
                            "{{\"id\":{},\"name\":\"{}\",\"capacity\":{},\"occupied\":{}}}",
                            z.id.0, z.name, z.capacity, z.occupied
                        )
                    })
                    .collect();
                Response::ok(&format!("[{}]", z.join(",")))
            }
            (Method::Post, ["zones"]) => {
                // body: name,capacity (minimal form encoding: "name=X&capacity=N")
                let p = parse_form(&req.body);
                let name = p.get("name").cloned().unwrap_or_default();
                let cap: u32 = p.get("capacity").and_then(|c| c.parse().ok()).unwrap_or(0);
                let zid = ZoneId(self.nid());
                match Zone::new(zid, &name, cap, vec![VehicleClass::Car]) {
                    Ok(z) => {
                        self.map.add_zone(z);
                        Response::created(&format!("{{\"id\":{}}}", zid.0))
                    }
                    Err(e) => Response::bad(&e),
                }
            }
            (Method::Post, ["cars"]) => {
                let p = parse_form(&req.body);
                let plate = p.get("plate").cloned().unwrap_or_default();
                let cid = CarId(self.nid());
                match Car::new(cid, &plate, VehicleClass::Car) {
                    Ok(c) => {
                        self.map.add_car(c);
                        Response::created(&format!("{{\"id\":{}}}", cid.0))
                    }
                    Err(e) => Response::bad(&e),
                }
            }
            (Method::Post, ["park"]) => {
                let p = parse_form(&req.body);
                let car: u64 = p.get("car").and_then(|c| c.parse().ok()).unwrap_or(0);
                let zone: u64 = p.get("zone").and_then(|c| c.parse().ok()).unwrap_or(0);
                match self.map.park_car(CarId(car), ZoneId(zone)) {
                    Ok(r) => Response::ok(&format!("{{\"receipt\":\"{}\"}}", r.op)),
                    Err(e) => Response::bad(&e),
                }
            }
            (Method::Post, ["release"]) => {
                let p = parse_form(&req.body);
                let car: u64 = p.get("car").and_then(|c| c.parse().ok()).unwrap_or(0);
                match self.map.release_car(CarId(car)) {
                    Ok(r) => Response::ok(&format!("{{\"receipt\":\"{}\"}}", r.op)),
                    Err(e) => Response::bad(&e),
                }
            }
            (Method::Get, ["occupancy"]) => {
                Response::ok(&format!("{{\"occupancy_pct\":{:.2}}}", self.map.occupancy_pct()))
            }
            (Method::Get, ["receipts"]) => {
                let r: Vec<String> = self
                    .map
                    .receipts
                    .iter()
                    .map(|x| format!("{{\"op\":\"{}\",\"entity\":\"{}\"}}", x.op, x.entity))
                    .collect();
                Response::ok(&format!("[{}]", r.join(",")))
            }
            _ => Response::not_found(),
        }
    }
}

fn parse_form(body: &str) -> HashMap<String, String> {
    body.split('&')
        .filter_map(|kv| {
            let mut it = kv.splitn(2, '=');
            Some((it.next()?.trim().to_string(), it.next().unwrap_or("").trim().to_string()))
        })
        .collect()
}

/// Generated OpenAPI 3.0 document for the routes above.
pub fn openapi_json() -> String {
    let paths = [
        ("/health", "get", "Health check"),
        ("/openapi.json", "get", "This document"),
        ("/zones", "get", "List zones"),
        ("/zones", "post", "Create zone"),
        ("/cars", "post", "Register car"),
        ("/park", "post", "Park car in zone"),
        ("/release", "post", "Release car"),
        ("/occupancy", "get", "Occupancy percent"),
        ("/receipts", "get", "Receipt log"),
    ];
    let mut p = String::new();
    for (path, m, desc) in paths {
        p.push_str(&format!(
            "\"{}\":{{\"{}\":{{\"summary\":\"{}\",\"security\":[{{\"bearerAuth\":[]}}]}}}},",
            path, m, desc
        ));
    }
    format!(
        "{{\"openapi\":\"3.0.0\",\"info\":{{\"title\":\"Shambhala API\",\"version\":\"0.1.0\"}},\"paths\":{{{}}}}}",
        p.trim_end_matches(',')
    )
}
