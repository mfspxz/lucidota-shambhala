// sham_redteam — sham-027: Shambhala red-team.
// Adversarial campaign against the built crates. Each attack must be
// defeated fail-closed. Findings are recorded as (attack, verdict, fix).
#![forbid(unsafe_code)]

mod organelle;
pub use organelle::*;
#[cfg(test)]
mod tests;

use sham_api::{Api, Method, Request};
use sham_domain::{Car, CarId, FestivalMap, VehicleClass, Zone, ZoneId};
use sham_offline::{OpKind, OpLog, merge, sync};
use sham_odin::{is_stale, validate_ping};
use sham_routing::grid_graph;
use sham_spatial::{GeoCoord, can_make_turn};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    Defeated, // attack failed against the system (good)
    Finding,  // attack succeeded -> must fix before close-out
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub attack: &'static str,
    pub verdict: Verdict,
    pub detail: String,
}

/// Campaign: run all attacks, return findings. Red-team passes iff no open findings.
pub fn campaign() -> Vec<Finding> {
    let mut out = Vec::new();

    // A1: double-park race — same car parked twice must not double-count.
    {
        let mut m = FestivalMap::new();
        m.add_zone(Zone::new(ZoneId(1), "z", 10, vec![VehicleClass::Car]).unwrap());
        m.add_car(Car::new(CarId(1), "A1", VehicleClass::Car).unwrap());
        let r1 = m.park_car(CarId(1), ZoneId(1));
        let r2 = m.park_car(CarId(1), ZoneId(1));
        let defeated = r1.is_ok() && r2.is_err() && m.zones[&ZoneId(1)].occupied == 1;
        out.push(Finding {
            attack: "A1 double-park race",
            verdict: if defeated { Verdict::Defeated } else { Verdict::Finding },
            detail: format!("occupied={}", m.zones[&ZoneId(1)].occupied),
        });
    }

    // A2: zone overflow — parking past capacity must fail closed.
    {
        let mut m = FestivalMap::new();
        m.add_zone(Zone::new(ZoneId(1), "tiny", 1, vec![VehicleClass::Car]).unwrap());
        m.add_car(Car::new(CarId(1), "A1", VehicleClass::Car).unwrap());
        m.add_car(Car::new(CarId(2), "B2", VehicleClass::Car).unwrap());
        let _ = m.park_car(CarId(1), ZoneId(1));
        let r2 = m.park_car(CarId(2), ZoneId(1));
        let defeated = r2.is_err() && m.zones[&ZoneId(1)].occupied == 1;
        out.push(Finding {
            attack: "A2 zone overflow",
            verdict: if defeated { Verdict::Defeated } else { Verdict::Finding },
            detail: format!("second park rejected: {}", r2.is_err()),
        });
    }

    // A3: GPS spoof — impossible coordinates / future timestamps rejected.
    {
        let bad1 = validate_ping(91.0, 0.0, 1000, 2000).is_err();
        let bad2 = validate_ping(49.0, -123.0, 9_999_999_999, 1000).is_err();
        let stale = is_stale(0, 10_000_000, 60_000);
        let defeated = bad1 && bad2 && stale;
        out.push(Finding {
            attack: "A3 GPS spoof",
            verdict: if defeated { Verdict::Defeated } else { Verdict::Finding },
            detail: format!("bad-coord={} future-ts={} stale={}", bad1, bad2, stale),
        });
    }

    // A4: oversized vehicle routed onto narrow street must get no route.
    {
        let (g, _, _) = grid_graph(100.0, 800);
        let no_route = g.route(sham_domain::PointId(1), sham_domain::PointId(9), VehicleClass::Bus).is_none();
        // and a bus must never be told it can hairpin on a 500cm street
        let no_hairpin = !can_make_turn(VehicleClass::Bus, 170.0, 500);
        let defeated = no_route && no_hairpin;
        out.push(Finding {
            attack: "A4 oversized vehicle routing",
            verdict: if defeated { Verdict::Defeated } else { Verdict::Finding },
            detail: format!("no_route={} no_hairpin={}", no_route, no_hairpin),
        });
    }

    // A5: API auth bypass — no token / wrong token must not reach handlers.
    {
        let mut api = Api::new();
        api.auth.issue("real", "lead");
        let r1 = api.handle(&Request::new(Method::Get, "/zones"));
        let r2 = api.handle(&Request::new(Method::Get, "/zones").bearer("fake"));
        let r3 = api.handle(&Request::new(Method::Post, "/zones").body("name=x&capacity=5"));
        let defeated = r1.status == 401 && r2.status == 401 && r3.status == 401;
        out.push(Finding {
            attack: "A5 API auth bypass",
            verdict: if defeated { Verdict::Defeated } else { Verdict::Finding },
            detail: format!("statuses={}/{}/{}", r1.status, r2.status, r3.status),
        });
    }

    // A6: offline log tamper — conflicting ops from two devices surface as
    // conflicts, never silent overwrite.
    {
        let mut a = OpLog::new("t1");
        a.push(OpKind::AddZone { id: 1, name: "z".into(), capacity: 1 });
        a.push(OpKind::AddCar { id: 10, plate: "A".into() });
        a.push(OpKind::Park { car: 10, zone: 1 });
        let mut b = OpLog::new("t2");
        b.push(OpKind::AddCar { id: 11, plate: "B".into() });
        b.push(OpKind::Park { car: 11, zone: 1 }); // conflicts: zone full
        let m = merge(&a, &b);
        let mut map = FestivalMap::new();
        let rep = sync(&mut map, &m);
        let defeated = rep.conflicts.len() == 1 && map.zones[&ZoneId(1)].occupied == 1;
        out.push(Finding {
            attack: "A6 offline double-book",
            verdict: if defeated { Verdict::Defeated } else { Verdict::Finding },
            detail: format!("conflicts={}", rep.conflicts.len()),
        });
    }

    // A7: ODIN unknown feature must fail closed, never execute.
    {
        let reg = sham_odin::Registry::load();
        let defeated = reg.get("ODIN-ROOT-007").is_none() && reg.len() == 196;
        out.push(Finding {
            attack: "A7 ODIN feature injection",
            verdict: if defeated { Verdict::Defeated } else { Verdict::Finding },
            detail: format!("registry_size={}", reg.len()),
        });
    }

    // A8: coordinate smuggling — GeoCoord rejects out-of-range at construction.
    {
        let bad = GeoCoord::new(90.1, 0.0).is_err() && GeoCoord::new(0.0, 180.1).is_err();
        out.push(Finding {
            attack: "A8 coordinate smuggling",
            verdict: if bad { Verdict::Defeated } else { Verdict::Finding },
            detail: "constructor validation".into(),
        });
    }

    out
}

/// Red-team passes iff every attack was defeated (no open findings).
pub fn campaign_passes(findings: &[Finding]) -> bool {
    findings.iter().all(|f| f.verdict == Verdict::Defeated)
}
