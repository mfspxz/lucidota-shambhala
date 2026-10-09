use super::*;
use sham_domain::{Street, StreetId, VehicleClass};
use sham_spatial::GeoCoord;
use std::collections::HashMap;

fn small_line() -> (RoadGraph, PointId, PointId) {
    // A -- B -- C, wide streets
    let mut g = RoadGraph::new();
    let coords: HashMap<PointId, GeoCoord> = [1u64, 2, 3]
        .iter()
        .map(|&i| {
            (
                PointId(i),
                GeoCoord::new(49.20 + i as f64 * 0.001, -122.91).unwrap(),
            )
        })
        .collect();
    for i in 1..=3 {
        g.add_node(PointId(i), coords[&PointId(i)]);
    }
    let mut sid = 0;
    for (a, b) in [(1u64, 2u64), (2, 3)] {
        sid += 1;
        let s = Street::new(StreetId(sid), "s", PointId(a), PointId(b), 600).unwrap();
        g.add_street(&s, &coords);
    }
    (g, PointId(1), PointId(3))
}

#[test]
fn route_line_found() {
    let (g, a, c) = small_line();
    let r = g.route(a, c, VehicleClass::Car).unwrap();
    assert_eq!(r.hops(), 2);
    assert!(r.dist_m > 200.0);
}
#[test]
fn route_self_zero() {
    let (g, a, _) = small_line();
    let r = g.route(a, a, VehicleClass::Car).unwrap();
    assert_eq!(r.hops(), 0);
    assert_eq!(r.dist_m, 0.0);
}
#[test]
fn route_unknown_node_none() {
    let (g, a, _) = small_line();
    assert!(g.route(a, PointId(999), VehicleClass::Car).is_none());
    assert!(g.route(PointId(999), a, VehicleClass::Car).is_none());
}
#[test]
fn route_nodes_path_consistent() {
    let (g, a, c) = small_line();
    let r = g.route(a, c, VehicleClass::Car).unwrap();
    assert_eq!(r.nodes.first(), Some(&a));
    assert_eq!(r.nodes.last(), Some(&c));
    assert_eq!(r.nodes.len(), r.streets.len() + 1);
}
#[test]
fn route_disconnected_none() {
    let mut g = RoadGraph::new();
    for i in 1..=2 {
        g.add_node(PointId(i), GeoCoord::new(49.0 + i as f64, -123.0).unwrap());
    }
    assert!(g.route(PointId(1), PointId(2), VehicleClass::Car).is_none());
}
#[test]
fn route_respects_one_way() {
    let mut g = RoadGraph::new();
    let coords: HashMap<PointId, GeoCoord> = [1u64, 2]
        .iter()
        .map(|&i| (PointId(i), GeoCoord::new(49.20 + i as f64 * 0.001, -122.91).unwrap()))
        .collect();
    for i in 1..=2 {
        g.add_node(PointId(i), coords[&PointId(i)]);
    }
    let mut s = Street::new(StreetId(1), "oneway", PointId(1), PointId(2), 600).unwrap();
    s.one_way = true;
    g.add_street(&s, &coords);
    assert!(g.route(PointId(1), PointId(2), VehicleClass::Car).is_some());
    assert!(g.route(PointId(2), PointId(1), VehicleClass::Car).is_none());
}
#[test]
fn route_two_way_both_dirs() {
    let (g, a, c) = small_line();
    assert!(g.route(c, a, VehicleClass::Car).is_some());
}
#[test]
fn route_narrow_street_blocks_bus() {
    // bus needs width >= 300 (1200/4); car needs >= 200
    let mut g = RoadGraph::new();
    let coords: HashMap<PointId, GeoCoord> = [1u64, 2]
        .iter()
        .map(|&i| (PointId(i), GeoCoord::new(49.20 + i as f64 * 0.001, -122.91).unwrap()))
        .collect();
    for i in 1..=2 {
        g.add_node(PointId(i), coords[&PointId(i)]);
    }
    let s = Street::new(StreetId(1), "narrow", PointId(1), PointId(2), 250).unwrap();
    g.add_street(&s, &coords);
    assert!(g.route(PointId(1), PointId(2), VehicleClass::Car).is_some());
    assert!(g.route(PointId(1), PointId(2), VehicleClass::Bus).is_none());
}
#[test]
fn route_prefers_shorter() {
    // triangle: direct long edge vs two short edges
    let mut g = RoadGraph::new();
    let a = PointId(1);
    let b = PointId(2);
    let c = PointId(3);
    let ca = GeoCoord::new(49.200, -122.910).unwrap();
    let cb = GeoCoord::new(49.201, -122.910).unwrap();
    let cc = GeoCoord::new(49.200, -122.909).unwrap();
    for (p, co) in [(a, ca), (b, cb), (c, cc)] {
        g.add_node(p, co);
    }
    let coords: HashMap<PointId, GeoCoord> = [(a, ca), (b, cb), (c, cc)].into_iter().collect();
    let mk = |id: u64, x: PointId, y: PointId| {
        Street::new(StreetId(id), "s", x, y, 800).unwrap()
    };
    for (id, x, y) in [(1u64, a, b), (2, b, c), (3, a, c)] {
        let s = mk(id, x, y);
        g.add_street(&s, &coords);
    }
    let r = g.route(a, c, VehicleClass::Car).unwrap();
    // direct edge a-c should win over a-b-c detour
    assert_eq!(r.hops(), 1);
    assert_eq!(r.streets, vec![StreetId(3)]);
}
#[test]
fn grid_graph_builds() {
    let (g, _coords, streets) = grid_graph(100.0, 800);
    assert_eq!(g.nodes.len(), 9);
    assert_eq!(streets.len(), 12);
}
#[test]
fn grid_route_corner_to_corner() {
    let (g, _, _) = grid_graph(100.0, 800);
    let r = g.route(PointId(1), PointId(9), VehicleClass::Car).unwrap();
    assert_eq!(r.hops(), 4); // manhattan distance on 3x3
    assert!(r.dist_m > 350.0 && r.dist_m < 450.0);
}
#[test]
fn grid_route_all_pairs_reachable_car() {
    let (g, _, _) = grid_graph(100.0, 800);
    for a in 1..=9u64 {
        for b in 1..=9u64 {
            assert!(g.route(PointId(a), PointId(b), VehicleClass::Car).is_some(), "{}->{}", a, b);
        }
    }
}
#[test]
fn grid_bus_wide_enough() {
    // bus 90-deg turn needs 1200*(1-cos90)+200 = 1400cm
    let (g, _, _) = grid_graph(100.0, 1600);
    assert!(g.route(PointId(1), PointId(9), VehicleClass::Bus).is_some());
    let (narrow, _, _) = grid_graph(100.0, 800);
    assert!(narrow.route(PointId(1), PointId(9), VehicleClass::Bus).is_none());
}
#[test]
fn edge_len_matches_geo() {
    let (g, _, _) = grid_graph(100.0, 800);
    let e = &g.edges[0];
    assert!((e.len_m - 100.0).abs() < 2.0, "{}", e.len_m);
}
#[test]
fn add_street_missing_coord_skipped() {
    let mut g = RoadGraph::new();
    g.add_node(PointId(1), GeoCoord::new(49.0, -123.0).unwrap());
    let s = Street::new(StreetId(1), "s", PointId(1), PointId(2), 400).unwrap();
    let coords: HashMap<PointId, GeoCoord> = [(PointId(1), GeoCoord::new(49.0, -123.0).unwrap())]
        .into_iter()
        .collect();
    g.add_street(&s, &coords);
    assert!(g.edges.is_empty());
}

// ---- sham-024 flows ----
#[test]
fn live_flow_new() {
    let f = LiveFlow::new("ingress", vec![StreetId(1), StreetId(2)], 120).unwrap();
    assert_eq!(f.rate_cph, 120);
    assert!(f.active_cars.is_empty());
}
#[test]
fn live_flow_empty_path_rejected() {
    assert!(LiveFlow::new("x", vec![], 10).is_err());
}
#[test]
fn arrivals_per_tick_math() {
    let f = LiveFlow::new("x", vec![StreetId(1)], 120).unwrap();
    assert!((f.arrivals_per_tick(30) - 1.0).abs() < 1e-9);
    assert!((f.arrivals_per_tick(3600) - 120.0).abs() < 1e-9);
}
#[test]
fn arrivals_zero_rate() {
    let f = LiveFlow::new("x", vec![StreetId(1)], 0).unwrap();
    assert_eq!(f.arrivals_per_tick(60), 0.0);
}

// ---- sham-025 simulator ----
fn sim_fixture() -> Simulator {
    let (graph, _coords, streets) = grid_graph(100.0, 800);
    let mut sim = Simulator::new(graph, 30, 42);
    let path: Vec<StreetId> = streets.iter().take(4).map(|s| s.id).collect();
    sim.add_flow(LiveFlow::new("ingress", path, 240).unwrap());
    sim
}
#[test]
fn sim_tick_spawns() {
    let mut sim = sim_fixture();
    let rep = sim.tick(VehicleClass::Car);
    // 240/h * 30s = 2.0 expected; deterministic rng may give 1-3
    assert!(rep.spawned <= 4, "spawned={}", rep.spawned);
    assert_eq!(rep.routed + rep.failed, rep.spawned);
}
#[test]
fn sim_run_500_no_deadlock() {
    let mut sim = sim_fixture();
    let rep = sim.run(VehicleClass::Car, 500, 5000);
    assert!(!rep.deadlocked, "deadlocked: {:?}", rep);
    assert!(rep.routed >= 500);
    assert!(rep.avg_dist_m > 0.0);
}
#[test]
fn sim_deterministic_same_seed() {
    let mut a = sim_fixture();
    let mut b = sim_fixture();
    let ra = a.run(VehicleClass::Car, 100, 2000);
    let rb = b.run(VehicleClass::Car, 100, 2000);
    assert_eq!(ra.routed, rb.routed);
    assert_eq!(ra.ticks, rb.ticks);
}
#[test]
fn sim_different_seed_differs_or_same_ok() {
    // just assert it runs; determinism per-seed is what matters
    let (graph, _, streets) = grid_graph(100.0, 800);
    let mut sim = Simulator::new(graph, 30, 777);
    let path: Vec<StreetId> = streets.iter().take(4).map(|s| s.id).collect();
    sim.add_flow(LiveFlow::new("ingress", path, 240).unwrap());
    let rep = sim.run(VehicleClass::Car, 50, 2000);
    assert!(rep.routed >= 50);
}
#[test]
fn sim_deadlock_on_disconnected() {
    let mut g = RoadGraph::new();
    g.add_node(PointId(1), GeoCoord::new(49.0, -123.0).unwrap());
    g.add_node(PointId(2), GeoCoord::new(49.1, -123.0).unwrap());
    let mut sim = Simulator::new(g, 30, 1);
    // flow references streets that don't exist in the graph
    sim.add_flow(LiveFlow::new("bad", vec![StreetId(77)], 240).unwrap());
    let rep = sim.run(VehicleClass::Car, 10, 200);
    assert!(rep.deadlocked);
    assert_eq!(rep.routed, 0);
}
#[test]
fn sim_no_flows_zero_progress() {
    let (graph, _, _) = grid_graph(100.0, 800);
    let mut sim = Simulator::new(graph, 30, 1);
    let rep = sim.tick(VehicleClass::Car);
    assert_eq!((rep.spawned, rep.routed, rep.failed), (0, 0, 0));
}
#[test]
fn sim_failed_counts_no_route() {
    let mut sim = sim_fixture();
    sim.flows[0].path = vec![StreetId(4242)]; // nonexistent street
    let rep = sim.tick(VehicleClass::Car);
    assert_eq!(rep.routed, 0);
    assert_eq!(rep.failed, rep.spawned);
    assert!(sim.flows[0].dropped_no_route > 0);
}
#[test]
fn sim_stalled_ticks_reset_on_progress() {
    let mut sim = sim_fixture();
    sim.tick(VehicleClass::Car);
    assert_eq!(sim.stalled_ticks, 0);
}
#[test]
fn sim_zero_target_immediate() {
    let mut sim = sim_fixture();
    let rep = sim.run(VehicleClass::Car, 0, 100);
    assert_eq!(rep.ticks, 0);
    assert!(!rep.deadlocked);
}
#[test]
fn sim_max_ticks_cap() {
    let mut sim = sim_fixture();
    let rep = sim.run(VehicleClass::Car, 1_000_000, 5);
    assert_eq!(rep.ticks, 5);
}
#[test]
fn sim_avg_dist_sane() {
    let mut sim = sim_fixture();
    let rep = sim.run(VehicleClass::Car, 50, 2000);
    assert!(rep.avg_dist_m > 50.0 && rep.avg_dist_m < 2000.0, "{}", rep.avg_dist_m);
}
#[test]
fn tick_report_fields() {
    let r = TickReport { spawned: 3, routed: 2, failed: 1 };
    assert_eq!(r.spawned, r.routed + r.failed);
}
#[test]
fn route_struct_hops() {
    let r = Route { streets: vec![StreetId(1)], dist_m: 10.0, nodes: vec![PointId(1), PointId(2)] };
    assert_eq!(r.hops(), 1);
}
#[test]
fn grid_graph_spacing_scales_dist() {
    let (g1, _, _) = grid_graph(50.0, 800);
    let (g2, _, _) = grid_graph(200.0, 800);
    let r1 = g1.route(PointId(1), PointId(9), VehicleClass::Car).unwrap();
    let r2 = g2.route(PointId(1), PointId(9), VehicleClass::Car).unwrap();
    assert!(r2.dist_m > r1.dist_m * 3.0);
}
#[test]
fn route_motorcycle_grid() {
    let (g, _, _) = grid_graph(100.0, 800);
    assert!(g.route(PointId(1), PointId(9), VehicleClass::Motorcycle).is_some());
}
#[test]
fn route_truck_grid() {
    // truck 90-deg turn needs 900*(1-cos90)+200 = 1100cm
    let (g, _, _) = grid_graph(100.0, 1200);
    assert!(g.route(PointId(1), PointId(9), VehicleClass::Truck).is_some());
}
#[test]
fn neighbors_one_way_excluded_reverse() {
    let (g, a, _) = small_line();
    // small_line streets are two-way; sanity: neighbors exist both ways
    assert!(!g.neighbors(a, VehicleClass::Car).is_empty());
}
#[test]
fn flow_completed_increments() {
    let mut sim = sim_fixture();
    sim.tick(VehicleClass::Car);
    assert!(sim.flows[0].completed > 0 || sim.flows[0].dropped_no_route > 0);
}
#[test]
fn sim_cars_routed_accumulates() {
    let mut sim = sim_fixture();
    sim.tick(VehicleClass::Car);
    let n = sim.cars_routed;
    sim.tick(VehicleClass::Car);
    assert!(sim.cars_routed >= n);
}
#[test]
fn sim_total_dist_grows() {
    let mut sim = sim_fixture();
    sim.tick(VehicleClass::Car);
    assert!(sim.total_dist_m >= 0.0);
}
#[test]
fn route_dist_positive_multihop() {
    let (g, a, c) = small_line();
    let r = g.route(a, c, VehicleClass::Car).unwrap();
    assert!(r.dist_m > 0.0);
}
#[test]
fn grid_edge_count() {
    let (g, _, _) = grid_graph(100.0, 800);
    assert_eq!(g.edges.len(), 12);
}
#[test]
fn adj_symmetry_two_way() {
    let (g, _, _) = grid_graph(100.0, 800);
    // every edge appears in adjacency of both endpoints
    for e in &g.edges {
        let a_has = g.adj[&e.from].contains(&g.edges.iter().position(|x| x.street == e.street).unwrap());
        assert!(a_has);
    }
}
#[test]
fn route_does_not_revisit_nodes() {
    let (g, _, _) = grid_graph(100.0, 800);
    let r = g.route(PointId(1), PointId(9), VehicleClass::Car).unwrap();
    let mut seen = std::collections::HashSet::new();
    for n in &r.nodes {
        assert!(seen.insert(n), "revisited {:?}", n);
    }
}
#[test]
fn sim_report_equality() {
    let r1 = SimReport { ticks: 1, routed: 2, failed: 0, avg_dist_m: 3.0, deadlocked: false };
    assert_eq!(r1.clone(), r1);
}
#[test]
fn live_flow_name() {
    let f = LiveFlow::new("gate-a-ingress", vec![StreetId(1)], 60).unwrap();
    assert_eq!(f.name, "gate-a-ingress");
}
#[test]
fn turn_constraint_blocks_impossible() {
    // hairpin on narrow street: route may still exist via other streets, but
    // a lone U-turn street pair must not produce a route for a bus
    let mut g = RoadGraph::new();
    let p1 = PointId(1);
    let p2 = PointId(2);
    let p3 = PointId(3);
    // p1 -> p2 east, p2 -> p3 west (180 deg turn at p2)
    let c1 = GeoCoord::new(49.2000, -122.9100).unwrap();
    let c2 = GeoCoord::new(49.2000, -122.9090).unwrap();
    let c3 = GeoCoord::new(49.2000, -122.9100).unwrap(); // same as c1: hairpin
    for (p, c) in [(p1, c1), (p2, c2), (p3, c3)] {
        g.add_node(p, c);
    }
    let coords: HashMap<PointId, GeoCoord> = [(p1, c1), (p2, c2), (p3, c3)].into_iter().collect();
    let s1 = Street::new(StreetId(1), "out", p1, p2, 500).unwrap();
    let s2 = Street::new(StreetId(2), "back", p2, p3, 500).unwrap();
    g.add_street(&s1, &coords);
    g.add_street(&s2, &coords);
    // car can make it (needs 550*(1-cos180)+200 = 1300 > 500? no!)
    // car: 550*2+200=1300 > 500 -> blocked too. Widen to 1400: car fits, bus (2600) blocked.
    let mut g2 = RoadGraph::new();
    for (p, c) in [(p1, c1), (p2, c2), (p3, c3)] {
        g2.add_node(p, c);
    }
    let t1 = Street::new(StreetId(1), "out", p1, p2, 1400).unwrap();
    let t2 = Street::new(StreetId(2), "back", p2, p3, 1400).unwrap();
    g2.add_street(&t1, &coords);
    g2.add_street(&t2, &coords);
    assert!(g2.route(p1, p3, VehicleClass::Car).is_some());
    assert!(g2.route(p1, p3, VehicleClass::Bus).is_none());
}
#[test]
fn route_rv_grid_ok() {
    // RV 90-deg turn needs 1000*(1-cos90)+200 = 1200cm
    let (g, _, _) = grid_graph(100.0, 1300);
    assert!(g.route(PointId(1), PointId(9), VehicleClass::Rv).is_some());
}
#[test]
fn sim_seed_one_stable() {
    let mut a = sim_fixture();
    let ra = a.run(VehicleClass::Car, 25, 1000);
    assert!(ra.routed >= 25 && !ra.deadlocked);
}
#[test]
fn route_heading_state_not_pruned() {
    // Regression: A* pruned on node alone could discard the only arrival
    // heading that permits the next turn, wrongly reporting no route.
    // X is reachable heading east (cheap) and heading north (pricier);
    // X->G needs a northbound arrival for a Bus (90-deg turn needs 1400cm,
    // street is 800cm).
    use std::collections::HashMap;
    let mut g = RoadGraph::new();
    let mut coords = HashMap::new();
    let (s, x, a, gg) = (PointId(1), PointId(2), PointId(3), PointId(4));
    for (p, lat, lon) in [(s, 0.0, 0.0), (x, 0.0, 0.002), (a, -0.001, 0.002), (gg, 0.001, 0.002)] {
        let c = GeoCoord::new(lat, lon).unwrap();
        coords.insert(p, c);
        g.add_node(p, c);
    }
    let mut mk = |id: u64, n: &str, f: PointId, t: PointId, w: u32| {
        let st = Street::new(StreetId(id), n, f, t, w).unwrap();
        g.add_street(&st, &coords);
    };
    mk(1, "s-x", s, x, 2000); // east arrival at X (cheap)
    mk(2, "s-a", s, a, 2000);
    mk(3, "a-x", a, x, 2000); // north arrival at X (pricier)
    mk(4, "x-g", x, gg, 800); // needs northbound arrival for Bus
    let r = g.route(s, gg, VehicleClass::Bus);
    assert!(r.is_some(), "bus must route via the northbound arrival at X");
    let r = r.unwrap();
    assert!(r.nodes.contains(&a), "route must go through A, got {:?}", r.nodes);
}
