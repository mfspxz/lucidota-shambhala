// sham_routing — sham-013 (routing engine), sham-024 (traffic flows),
// sham-025 (festival scenario simulator).
// A* over the street graph with turning-radius + road-width constraints.
// Flows are first-class map objects. Deterministic 500-car ingress simulator.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

use sham_domain::{PointId, Street, StreetId, VehicleClass};
use sham_spatial::GeoCoord;
use std::collections::{BinaryHeap, HashMap};

/// A node in the routable graph: a point with a coordinate.
#[derive(Clone, Debug)]
pub struct Node {
    pub point: PointId,
    pub coord: GeoCoord,
}

/// Weighted edge with constraint data.
#[derive(Clone, Debug)]
pub struct Edge {
    pub street: StreetId,
    pub from: PointId,
    pub to: PointId,
    pub len_m: f64,
    pub width_cm: u32,
    pub one_way: bool,
}

#[derive(Default)]
pub struct RoadGraph {
    pub nodes: HashMap<PointId, Node>,
    pub edges: Vec<Edge>,
    pub adj: HashMap<PointId, Vec<usize>>, // node -> edge indices
}
impl RoadGraph {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_node(&mut self, point: PointId, coord: GeoCoord) {
        self.nodes.insert(point, Node { point, coord });
    }
    pub fn add_street(&mut self, s: &Street, coords: &HashMap<PointId, GeoCoord>) {
        let (Some(&a), Some(&b)) = (coords.get(&s.from), coords.get(&s.to)) else {
            return;
        };
        let idx = self.edges.len();
        self.edges.push(Edge {
            street: s.id,
            from: s.from,
            to: s.to,
            len_m: a.distance_m(b),
            width_cm: s.width_cm,
            one_way: s.one_way,
        });
        self.adj.entry(s.from).or_default().push(idx);
        if !s.one_way {
            self.adj.entry(s.to).or_default().push(idx);
        }
    }
    fn neighbors(&self, n: PointId, class: VehicleClass) -> Vec<(PointId, usize, f64)> {
        let mut out = Vec::new();
        for &ei in self.adj.get(&n).map(|v| v.as_slice()).unwrap_or(&[]) {
            let e = &self.edges[ei];
            // Width constraint: street must be wide enough for the vehicle's
            // swept path. Heuristic: min width = max(200cm, turning_radius/4).
            let min_w = (class.turning_radius_cm() / 4).max(200);
            if e.width_cm < min_w {
                continue;
            }
            let next = if e.from == n { e.to } else { e.from };
            if e.one_way && e.from != n {
                continue;
            }
            out.push((next, ei, e.len_m));
        }
        out
    }
    /// A* from `from` to `to` for a vehicle class.
    /// Turning-radius constraint: penalize sharp heading changes; reject
    /// turns the vehicle cannot physically make on the target street.
    pub fn route(
        &self,
        from: PointId,
        to: PointId,
        class: VehicleClass,
    ) -> Option<Route> {
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return None;
        }
        if from == to {
            return Some(Route { streets: vec![], dist_m: 0.0, nodes: vec![from] });
        }
        #[derive(Clone)]
        struct State {
            f: f64,
            g: f64,
            node: PointId,
            heading: Option<f64>,
        }
        impl PartialEq for State {
            fn eq(&self, other: &Self) -> bool {
                self.f == other.f
            }
        }
        impl Eq for State {}
        impl PartialOrd for State {
            fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
                other.f.partial_cmp(&self.f)
            }
        }
        impl Ord for State {
            fn cmp(&self, other: &Self) -> std::cmp::Ordering {
                self.partial_cmp(other).unwrap()
            }
        }
        let h = |n: PointId| {
            self.nodes[&n].coord.distance_m(self.nodes[&to].coord)
        };
        let mut open = BinaryHeap::new();
        open.push(State { f: h(from), g: 0.0, node: from, heading: None });
        let mut best: HashMap<PointId, f64> = HashMap::new();
        let mut came: HashMap<PointId, (PointId, usize)> = HashMap::new();
        while let Some(s) = open.pop() {
            if s.node == to {
                // reconstruct
                let mut streets = Vec::new();
                let mut nodes = vec![to];
                let mut cur = to;
                while let Some(&(p, ei)) = came.get(&cur) {
                    streets.push(self.edges[ei].street);
                    nodes.push(p);
                    cur = p;
                }
                streets.reverse();
                nodes.reverse();
                return Some(Route { streets, dist_m: s.g, nodes });
            }
            if best.get(&s.node).is_some_and(|&b| b <= s.g) {
                continue;
            }
            best.insert(s.node, s.g);
            for (next, ei, len) in self.neighbors(s.node, class) {
                // turning constraint: heading change vs street width
                let e = &self.edges[ei];
                let mut cost = len;
                if let Some(hd) = s.heading {
                    let nb = self.nodes[&s.node].coord.bearing_deg(self.nodes[&next].coord);
                    let mut turn = (nb - hd).abs();
                    if turn > 180.0 {
                        turn = 360.0 - turn;
                    }
                    if !sham_spatial::can_make_turn(class, turn, e.width_cm) {
                        continue; // physically impossible turn
                    }
                    cost += turn / 90.0 * 20.0; // soft penalty for sharp turns
                }
                let g2 = s.g + cost;
                let nb = self.nodes[&s.node].coord.bearing_deg(self.nodes[&next].coord);
                // only relax if better; keep predecessor in sync with the relaxation
                if best.get(&next).is_none_or(|&b| g2 < b) {
                    came.insert(next, (s.node, ei));
                    open.push(State { f: g2 + h(next), g: g2, node: next, heading: Some(nb) });
                }
            }
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub streets: Vec<StreetId>,
    pub dist_m: f64,
    pub nodes: Vec<PointId>,
}
impl Route {
    pub fn hops(&self) -> usize {
        self.streets.len()
    }
}

// ---------------- sham-024: flows as first-class objects ----------------

/// A traffic flow with live state, usable directly by the simulator and UI.
#[derive(Clone, Debug)]
pub struct LiveFlow {
    pub name: String,
    pub path: Vec<StreetId>,
    pub rate_cph: u32,
    /// cars currently on the flow (car ids)
    pub active_cars: Vec<u64>,
    pub completed: u32,
    pub dropped_no_route: u32,
}
impl LiveFlow {
    pub fn new(name: &str, path: Vec<StreetId>, rate_cph: u32) -> Result<Self, String> {
        if path.is_empty() {
            return Err("flow path empty".into());
        }
        Ok(Self {
            name: name.into(),
            path,
            rate_cph,
            active_cars: Vec::new(),
            completed: 0,
            dropped_no_route: 0,
        })
    }
    /// Cars expected to enter per simulation tick of `tick_s` seconds.
    pub fn arrivals_per_tick(&self, tick_s: u64) -> f64 {
        self.rate_cph as f64 * tick_s as f64 / 3600.0
    }
}

// ---------------- sham-025: festival scenario simulator ----------------

/// Deterministic ingress simulator: N cars arrive at gates, route to zones.
pub struct Simulator {
    pub graph: RoadGraph,
    pub flows: Vec<LiveFlow>,
    pub tick_s: u64,
    pub rng_state: u64,
    pub cars_routed: u32,
    pub cars_failed: u32,
    pub total_dist_m: f64,
    /// deadlock detector: ticks with zero progress while cars remain
    pub stalled_ticks: u32,
}
impl Simulator {
    pub fn new(graph: RoadGraph, tick_s: u64, seed: u64) -> Self {
        Self {
            graph,
            flows: Vec::new(),
            tick_s,
            rng_state: seed | 1,
            cars_routed: 0,
            cars_failed: 0,
            total_dist_m: 0.0,
            stalled_ticks: 0,
        }
    }
    fn rand(&mut self) -> u64 {
        let mut x = self.rng_state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.rng_state = x;
        x.wrapping_mul(0x2545F4914F6CDD1D)
    }
    pub fn add_flow(&mut self, f: LiveFlow) {
        self.flows.push(f);
    }
    /// Run one tick: spawn cars per flow rate, route each from flow entry
    /// (first street's `from`) to flow exit (last street's `to`).
    pub fn tick(&mut self, class: VehicleClass) -> TickReport {
        let mut spawned = 0u32;
        let mut routed = 0u32;
        let mut failed = 0u32;
        for fi in 0..self.flows.len() {
            let want = self.flows[fi].arrivals_per_tick(self.tick_s);
            // deterministic fractional accumulation via rng
            let n = want as u32 + ((self.rand() % 1000) as f64 / 1000.0 < want.fract()) as u32;
            for _ in 0..n {
                spawned += 1;
                let path = &self.flows[fi].path;
                let (Some(first), Some(last)) = (path.first(), path.last()) else {
                    failed += 1;
                    continue;
                };
                let from = self.graph.edges.iter().find(|e| e.street == *first).map(|e| e.from);
                let to = self.graph.edges.iter().find(|e| e.street == *last).map(|e| e.to);
                let (Some(f), Some(t)) = (from, to) else {
                    failed += 1;
                    self.flows[fi].dropped_no_route += 1;
                    continue;
                };
                match self.graph.route(f, t, class) {
                    Some(r) => {
                        routed += 1;
                        self.total_dist_m += r.dist_m;
                        self.flows[fi].completed += 1;
                    }
                    None => {
                        failed += 1;
                        self.flows[fi].dropped_no_route += 1;
                    }
                }
            }
        }
        self.cars_routed += routed;
        self.cars_failed += failed;
        if spawned > 0 && routed == 0 {
            self.stalled_ticks += 1;
        } else {
            self.stalled_ticks = 0;
        }
        TickReport { spawned, routed, failed }
    }
    /// Run until `target` cars routed or `max_ticks` hit. Returns deadlock verdict.
    pub fn run(&mut self, class: VehicleClass, target: u32, max_ticks: u32) -> SimReport {
        let mut ticks = 0u32;
        while self.cars_routed < target && ticks < max_ticks {
            self.tick(class);
            ticks += 1;
            if self.stalled_ticks > 10 {
                break; // deadlock: no progress for 10 ticks
            }
        }
        SimReport {
            ticks,
            routed: self.cars_routed,
            failed: self.cars_failed,
            avg_dist_m: if self.cars_routed > 0 {
                self.total_dist_m / self.cars_routed as f64
            } else {
                0.0
            },
            deadlocked: self.cars_routed < target,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TickReport {
    pub spawned: u32,
    pub routed: u32,
    pub failed: u32,
}
#[derive(Clone, Debug, PartialEq)]
pub struct SimReport {
    pub ticks: u32,
    pub routed: u32,
    pub failed: u32,
    pub avg_dist_m: f64,
    pub deadlocked: bool,
}

/// Build a small grid test graph: 3x3 points, streets between neighbors.
pub fn grid_graph(spacing_m: f64, width_cm: u32) -> (RoadGraph, HashMap<PointId, GeoCoord>, Vec<Street>) {
    let origin = GeoCoord::new(49.20, -122.91).unwrap();
    let mut graph = RoadGraph::new();
    let mut coords = HashMap::new();
    let mut streets = Vec::new();
    let mut id = 0u64;
    let mut pid = [[PointId(0); 3]; 3];
    for r in 0..3 {
        for c in 0..3 {
            id += 1;
            let p = PointId(id);
            pid[r][c] = p;
            let coord = origin.destination(0.0, r as f64 * spacing_m)
                .destination(90.0, c as f64 * spacing_m);
            coords.insert(p, coord);
            graph.add_node(p, coord);
        }
    }
    let mut sid = 0u64;
    let mut link = |a: PointId, b: PointId, name: String| {
        sid += 1;
        let s = Street::new(StreetId(sid), &name, a, b, width_cm).unwrap();
        graph.add_street(&s, &coords);
        streets.push(s);
    };
    for r in 0..3 {
        for c in 0..3 {
            if c < 2 {
                link(pid[r][c], pid[r][c + 1], format!("e-{}-{}", r, c));
            }
            if r < 2 {
                link(pid[r][c], pid[r + 1][c], format!("s-{}-{}", r, c));
            }
        }
    }
    (graph, coords, streets)
}
