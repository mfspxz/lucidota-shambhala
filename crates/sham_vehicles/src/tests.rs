use super::*;

fn veh(id: u64, plate: &str) -> Vehicle {
    Vehicle::new(VehicleId(id), plate, "Civic", 2019, "blue", VehicleClass::Car).unwrap()
}

// ---- sham-014 (11) ----
#[test]
fn vehicle_new_ok() {
    let v = veh(1, "abc 123");
    assert_eq!(v.plate, "ABC 123");
    assert_eq!(v.model, "Civic");
}
#[test]
fn vehicle_empty_plate_rejected() {
    assert!(Vehicle::new(VehicleId(1), " ", "Civic", 2019, "blue", VehicleClass::Car).is_err());
}
#[test]
fn vehicle_empty_model_rejected() {
    assert!(Vehicle::new(VehicleId(1), "A1", " ", 2019, "blue", VehicleClass::Car).is_err());
}
#[test]
fn vehicle_year_range() {
    assert!(Vehicle::new(VehicleId(1), "A1", "M", 1979, "c", VehicleClass::Car).is_err());
    assert!(Vehicle::new(VehicleId(1), "A1", "M", 2028, "c", VehicleClass::Car).is_err());
    assert!(Vehicle::new(VehicleId(1), "A1", "M", 2026, "c", VehicleClass::Car).is_ok());
}
#[test]
fn vehicle_add_note_receipt() {
    let mut v = veh(1, "A1");
    let r = v.add_note("dent on rear bumper");
    assert_eq!(r.op, "note");
    assert_eq!(v.notes.len(), 1);
}
#[test]
fn relate_convoy() {
    let mut reg = VehicleRegistry::new();
    reg.add(veh(1, "A1"));
    reg.add(veh(2, "B2"));
    let r = reg.relate(VehicleId(1), VehicleRel::ConvoyWith, VehicleId(2)).unwrap();
    assert_eq!(r.op, "relate");
    assert_eq!(reg.related(VehicleId(1), VehicleRel::ConvoyWith), vec![VehicleId(2)]);
}
#[test]
fn relate_self_rejected() {
    let mut reg = VehicleRegistry::new();
    reg.add(veh(1, "A1"));
    assert!(reg.relate(VehicleId(1), VehicleRel::ConvoyWith, VehicleId(1)).is_err());
}
#[test]
fn relate_unknown_rejected() {
    let mut reg = VehicleRegistry::new();
    reg.add(veh(1, "A1"));
    assert!(reg.relate(VehicleId(1), VehicleRel::TowedBy, VehicleId(99)).is_err());
}
#[test]
fn find_by_plate_case_insensitive() {
    let mut reg = VehicleRegistry::new();
    reg.add(veh(1, "zz 9"));
    assert!(reg.find_by_plate("ZZ 9").is_some());
    assert!(reg.find_by_plate("nope").is_none());
}
#[test]
fn relate_directed() {
    let mut reg = VehicleRegistry::new();
    reg.add(veh(1, "A1"));
    reg.add(veh(2, "B2"));
    reg.relate(VehicleId(1), VehicleRel::TowedBy, VehicleId(2)).unwrap();
    assert!(reg.related(VehicleId(2), VehicleRel::TowedBy).is_empty());
}
#[test]
fn owner_of_chain() {
    let mut reg = VehicleRegistry::new();
    reg.add(veh(1, "TRUCK1"));
    reg.add(veh(2, "TRL9"));
    reg.relate(VehicleId(1), VehicleRel::OwnerOf, VehicleId(2)).unwrap();
    assert_eq!(reg.related(VehicleId(1), VehicleRel::OwnerOf), vec![VehicleId(2)]);
}

// ---- sham-015 (10) ----
fn sample_graph() -> StreetGraph {
    // street 1 connects points 10,11; street 2 connects 11,12; street1 feeds street2
    let mut g = StreetGraph::new();
    let s1 = GNode::Street(StreetId(1));
    let s2 = GNode::Street(StreetId(2));
    let p10 = GNode::Point(10);
    let p11 = GNode::Point(11);
    let p12 = GNode::Point(12);
    g.add_edge(s1, GEdge::Connects, p10).unwrap();
    g.add_edge(s1, GEdge::Connects, p11).unwrap();
    g.add_edge(s2, GEdge::Connects, p11).unwrap();
    g.add_edge(s2, GEdge::Connects, p12).unwrap();
    g.add_edge(s1, GEdge::FeedsInto, s2).unwrap();
    g.add_edge(p10, GEdge::AdjacentTo, p11).unwrap();
    g
}
#[test]
fn graph_counts() {
    let g = sample_graph();
    assert_eq!(g.node_count(), 5);
    assert_eq!(g.edge_count(), 6);
}
#[test]
fn query_connects() {
    let g = sample_graph();
    let pts = g.query(GNode::Street(StreetId(1)), GEdge::Connects);
    assert_eq!(pts.len(), 2);
    assert!(pts.contains(&GNode::Point(10)));
}
#[test]
fn query_empty_for_wrong_rel() {
    let g = sample_graph();
    assert!(g.query(GNode::Point(12), GEdge::FeedsInto).is_empty());
}
#[test]
fn shortest_path_direct() {
    let g = sample_graph();
    let p = g.shortest_path(GNode::Street(StreetId(1)), GNode::Street(StreetId(2))).unwrap();
    assert_eq!(p.len(), 2);
}
#[test]
fn shortest_path_multi_hop() {
    let g = sample_graph();
    let p = g.shortest_path(GNode::Point(10), GNode::Point(12)).unwrap();
    assert!(p.len() >= 3);
    assert_eq!(p.first(), Some(&GNode::Point(10)));
    assert_eq!(p.last(), Some(&GNode::Point(12)));
}
#[test]
fn shortest_path_self() {
    let g = sample_graph();
    assert_eq!(
        g.shortest_path(GNode::Point(10), GNode::Point(10)),
        Some(vec![GNode::Point(10)])
    );
}
#[test]
fn shortest_path_disconnected_none() {
    let mut g = StreetGraph::new();
    g.add_node(GNode::Point(1));
    g.add_node(GNode::Point(2));
    assert!(g.shortest_path(GNode::Point(1), GNode::Point(2)).is_none());
}
#[test]
fn self_edge_rejected() {
    let mut g = StreetGraph::new();
    assert!(g.add_edge(GNode::Point(1), GEdge::AdjacentTo, GNode::Point(1)).is_err());
}
#[test]
fn feeds_into_query() {
    let g = sample_graph();
    let next = g.query(GNode::Street(StreetId(1)), GEdge::FeedsInto);
    assert_eq!(next, vec![GNode::Street(StreetId(2))]);
}
#[test]
fn add_edge_auto_adds_nodes() {
    let mut g = StreetGraph::new();
    g.add_edge(GNode::Point(5), GEdge::PartOf, GNode::Street(StreetId(9))).unwrap();
    assert_eq!(g.node_count(), 2);
}
