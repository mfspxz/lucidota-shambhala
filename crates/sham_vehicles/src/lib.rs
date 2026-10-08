// sham_vehicles — sham-014 (vehicle records), sham-015 (street/point
// relationship graph DB).
// Vehicle: model/year/colour/notes + relationship graph (owner, convoy,
// towed-by...). Graph DB of streets/points with tested queries.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

use sham_domain::{Receipt, StreetId, VehicleClass, VehicleId};
use std::collections::{HashMap, HashSet, VecDeque};

/// A vehicle record (sham-014).
#[derive(Clone, Debug, PartialEq)]
pub struct Vehicle {
    pub id: VehicleId,
    pub plate: String,
    pub model: String,
    pub year: u16,
    pub colour: String,
    pub class: VehicleClass,
    pub notes: Vec<String>,
}
impl Vehicle {
    pub fn new(
        id: VehicleId,
        plate: &str,
        model: &str,
        year: u16,
        colour: &str,
        class: VehicleClass,
    ) -> Result<Self, String> {
        if plate.trim().is_empty() {
            return Err("plate required".into());
        }
        if model.trim().is_empty() {
            return Err("model required".into());
        }
        if !(1980..=2027).contains(&year) {
            return Err(format!("year {} out of range", year));
        }
        Ok(Self {
            id,
            plate: plate.trim().to_uppercase(),
            model: model.trim().into(),
            year,
            colour: colour.trim().into(),
            class,
            notes: Vec::new(),
        })
    }
    pub fn add_note(&mut self, note: &str) -> Receipt {
        self.notes.push(note.into());
        Receipt::new("note", "vehicle", self.id.0, note)
    }
}

/// Relationship between vehicles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum VehicleRel {
    ConvoyWith,
    TowedBy,
    OwnerOf, // a -> b means a owns b (e.g. truck owns trailer)
    BlockedBy,
}

#[derive(Default)]
pub struct VehicleRegistry {
    pub vehicles: HashMap<VehicleId, Vehicle>,
    pub rels: HashSet<(VehicleId, VehicleRel, VehicleId)>,
    pub receipts: Vec<Receipt>,
}
impl VehicleRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add(&mut self, v: Vehicle) {
        self.vehicles.insert(v.id, v);
    }
    /// Fail-closed: both vehicles must exist; no self-relationship.
    pub fn relate(&mut self, a: VehicleId, rel: VehicleRel, b: VehicleId) -> Result<Receipt, String> {
        if a == b {
            return Err("vehicle cannot relate to itself".into());
        }
        if !self.vehicles.contains_key(&a) || !self.vehicles.contains_key(&b) {
            return Err("unknown vehicle".into());
        }
        self.rels.insert((a, rel, b));
        let r = Receipt::new("relate", "vehicle", a.0, &format!("{:?} -> {:?}", rel, b.0));
        self.receipts.push(r.clone());
        Ok(r)
    }
    pub fn related(&self, a: VehicleId, rel: VehicleRel) -> Vec<VehicleId> {
        self.rels
            .iter()
            .filter(|(x, r, _)| *x == a && *r == rel)
            .map(|(_, _, b)| *b)
            .collect()
    }
    pub fn find_by_plate(&self, plate: &str) -> Option<&Vehicle> {
        let p = plate.trim().to_uppercase();
        self.vehicles.values().find(|v| v.plate == p)
    }
}

// ---------------- sham-015: street/point graph DB ----------------

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GNode {
    Street(StreetId),
    Point(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GEdge {
    Connects,   // street connects two points
    AdjacentTo, // point adjacent to point (via street)
    PartOf,     // point part of street corridor
    FeedsInto,  // street feeds into street
}

#[derive(Default)]
pub struct StreetGraph {
    nodes: HashSet<GNode>,
    edges: HashSet<(GNode, GEdge, GNode)>,
}
impl StreetGraph {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_node(&mut self, n: GNode) {
        self.nodes.insert(n);
    }
    pub fn add_edge(&mut self, a: GNode, rel: GEdge, b: GNode) -> Result<(), String> {
        if a == b {
            return Err("no self edges".into());
        }
        self.nodes.insert(a);
        self.nodes.insert(b);
        self.edges.insert((a, rel, b));
        Ok(())
    }
    /// Outgoing neighbors of a node via a relation.
    pub fn query(&self, a: GNode, rel: GEdge) -> Vec<GNode> {
        self.edges
            .iter()
            .filter(|(x, r, _)| *x == a && *r == rel)
            .map(|(_, _, b)| *b)
            .collect()
    }
    /// BFS shortest path (in edges) between two nodes, any relation.
    pub fn shortest_path(&self, from: GNode, to: GNode) -> Option<Vec<GNode>> {
        if from == to {
            return Some(vec![from]);
        }
        let mut prev: HashMap<GNode, GNode> = HashMap::new();
        let mut seen = HashSet::new();
        let mut q = VecDeque::new();
        seen.insert(from);
        q.push_back(from);
        while let Some(cur) = q.pop_front() {
            let mut nbrs: Vec<GNode> = self
                .edges
                .iter()
                .filter(|(x, _, _)| *x == cur)
                .map(|(_, _, b)| *b)
                .chain(self.edges.iter().filter(|(_, _, b)| *b == cur).map(|(x, _, _)| *x))
                .collect();
            nbrs.sort_by_key(|n| format!("{:?}", n));
            nbrs.dedup();
            for nb in nbrs {
                if seen.insert(nb) {
                    prev.insert(nb, cur);
                    if nb == to {
                        let mut path = vec![to];
                        let mut c = to;
                        while c != from {
                            c = prev[&c];
                            path.push(c);
                        }
                        path.reverse();
                        return Some(path);
                    }
                    q.push_back(nb);
                }
            }
        }
        None
    }
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}
