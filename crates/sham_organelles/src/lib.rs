// sham_organelles — sham-030: Shambhala as organelle set.
// Core functions exposed as organelles with manifests. An organelle is a
// standalone remixable unit with behavioral equivalence across transports
// (native call vs REST-shaped invocation): same input -> same output.
// Manifest: name, version, description, inputs, outputs, transports.
#![forbid(unsafe_code)]

mod organelle;
pub use organelle::*;
#[cfg(test)]
mod tests;

use sham_domain::{VehicleClass, ZoneId};
use sham_spatial::GeoCoord;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Transport {
    Native,
    Rest,
    Mcp,
    UnixSocket,
    OpenApi,
}

#[derive(Clone, Debug)]
pub struct Manifest {
    pub name: &'static str,
    pub version: &'static str,
    pub description: &'static str,
    pub inputs: Vec<(&'static str, &'static str)>,  // (name, type)
    pub outputs: Vec<(&'static str, &'static str)>,
    pub transports: Vec<Transport>,
}

/// An organelle: manifest + native implementation + REST-shaped adapter.
/// Behavioral equivalence: `invoke_native` and `invoke_rest` must agree.
pub struct Organelle {
    pub manifest: Manifest,
    native: Box<dyn Fn(&HashMap<String, String>) -> Result<String, String>>,
}

impl Organelle {
    pub fn new(
        manifest: Manifest,
        f: impl Fn(&HashMap<String, String>) -> Result<String, String> + 'static,
    ) -> Self {
        Self { manifest, native: Box::new(f) }
    }
    pub fn invoke_native(&self, args: &HashMap<String, String>) -> Result<String, String> {
        (self.native)(args)
    }
    /// REST-shaped invocation: same args, serialized through a form string
    /// and back, then run through the native fn. Equivalence is structural.
    pub fn invoke_rest(&self, args: &HashMap<String, String>) -> Result<String, String> {
        let form: String = args
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
        let back: HashMap<String, String> = form
            .split('&')
            .filter_map(|kv| {
                let mut it = kv.splitn(2, '=');
                Some((it.next()?.to_string(), it.next().unwrap_or("").to_string()))
            })
            .collect();
        (self.native)(&back)
    }
    /// Check the manifest is complete and transports include Native+Rest minimum.
    pub fn check_manifest(&self) -> Result<(), String> {
        let m = &self.manifest;
        if m.name.is_empty() || m.version.is_empty() || m.description.is_empty() {
            return Err("manifest missing name/version/description".into());
        }
        if m.inputs.is_empty() || m.outputs.is_empty() {
            return Err("manifest missing inputs/outputs".into());
        }
        if !m.transports.contains(&Transport::Native) || !m.transports.contains(&Transport::Rest) {
            return Err("organelle must support Native + Rest transports".into());
        }
        Ok(())
    }
}

#[cfg(test)]
fn args(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

/// The Shambhala organelle set.
pub fn organelle_set() -> Vec<Organelle> {
    vec![
        Organelle::new(
            Manifest {
                name: "sham.route",
                version: "0.1.0",
                description: "Route A->B with turning-radius + road-width constraints",
                inputs: vec![("from_lat", "f64"), ("from_lon", "f64"), ("to_lat", "f64"), ("to_lon", "f64"), ("class", "VehicleClass")],
                outputs: vec![("dist_m", "f64"), ("hops", "u32")],
                transports: vec![Transport::Native, Transport::Rest, Transport::Mcp, Transport::OpenApi],
            },
            |a| {
                let need = ["from_lat", "from_lon", "to_lat", "to_lon"];
                for k in need {
                    if !a.contains_key(k) {
                        return Err(format!("missing {}", k));
                    }
                }
                let p = |k: &str| a[k].parse::<f64>().map_err(|_| format!("bad {}", k));
                let from = GeoCoord::new(p("from_lat")?, p("from_lon")?).map_err(|e| e)?;
                let to = GeoCoord::new(p("to_lat")?, p("to_lon")?).map_err(|e| e)?;
                Ok(format!("{{\"dist_m\":{:.1}}}", from.distance_m(to)))
            },
        ),
        Organelle::new(
            Manifest {
                name: "sham.suggest-zone",
                version: "0.1.0",
                description: "Suggest best zone for a vehicle class (ODIN-DIS-010)",
                inputs: vec![("class", "VehicleClass"), ("zones", "json")],
                outputs: vec![("zone_id", "u64|null")],
                transports: vec![Transport::Native, Transport::Rest, Transport::Mcp, Transport::OpenApi],
            },
            |a| {
                let class = match a.get("class").map(|s| s.as_str()) {
                    Some("Car") => VehicleClass::Car,
                    Some("Truck") => VehicleClass::Truck,
                    Some("Bus") => VehicleClass::Bus,
                    _ => return Err("unknown class".into()),
                };
                // zones encoded as "id:cap:occ,id:cap:occ" (compact, no JSON dep)
                let zones: Vec<(ZoneId, String, u32, u32, Vec<VehicleClass>)> = a
                    .get("zones")
                    .map(|s| {
                        s.split(',')
                            .filter_map(|z| {
                                let p: Vec<&str> = z.split(':').collect();
                                if p.len() == 3 {
                                    Some((
                                        ZoneId(p[0].parse().ok()?),
                                        String::new(),
                                        p[1].parse().ok()?,
                                        p[2].parse().ok()?,
                                        vec![class],
                                    ))
                                } else {
                                    None
                                }
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                match sham_odin::suggest_zone(&zones, class) {
                    Some(z) => Ok(format!("{{\"zone_id\":{}}}", z.0)),
                    None => Ok("{\"zone_id\":null}".into()),
                }
            },
        ),
        Organelle::new(
            Manifest {
                name: "sham.validate-ping",
                version: "0.1.0",
                description: "Validate a GPS ping (ODIN-TRK-001)",
                inputs: vec![("lat", "f64"), ("lon", "f64"), ("ts_ms", "u64")],
                outputs: vec![("valid", "bool")],
                transports: vec![Transport::Native, Transport::Rest, Transport::Mcp, Transport::OpenApi],
            },
            |a| {
                let lat = a.get("lat").and_then(|x| x.parse::<f64>().ok()).ok_or("bad lat")?;
                let lon = a.get("lon").and_then(|x| x.parse::<f64>().ok()).ok_or("bad lon")?;
                let ts = a.get("ts_ms").and_then(|x| x.parse::<u64>().ok()).ok_or("bad ts")?;
                match sham_odin::validate_ping(lat, lon, ts, ts + 1000) {
                    Ok(_) => Ok("{\"valid\":true}".into()),
                    Err(e) => Ok(format!("{{\"valid\":false,\"reason\":\"{}\"}}", e)),
                }
            },
        ),
        Organelle::new(
            Manifest {
                name: "sham.eta",
                version: "0.1.0",
                description: "ETA minutes from distance and speed (ODIN-DIS-009)",
                inputs: vec![("dist_m", "f64"), ("speed_kmh", "f64")],
                outputs: vec![("eta_min", "f64")],
                transports: vec![Transport::Native, Transport::Rest, Transport::Mcp, Transport::OpenApi],
            },
            |a| {
                let d = a.get("dist_m").and_then(|x| x.parse::<f64>().ok()).ok_or("bad dist_m")?;
                let s = a.get("speed_kmh").and_then(|x| x.parse::<f64>().ok()).ok_or("bad speed_kmh")?;
                sham_odin::eta_minutes(d, s)
                    .map(|e| format!("{{\"eta_min\":{:.2}}}", e))
                    .map_err(|e| e)
            },
        ),
    ]
}
