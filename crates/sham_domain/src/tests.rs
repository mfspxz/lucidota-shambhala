
use super::*;

    fn ids() -> (IdGen, IdGen, IdGen, IdGen) {
        (IdGen::new(0), IdGen::new(100), IdGen::new(200), IdGen::new(300))
    }

    fn zone(g: &mut IdGen) -> Zone {
        Zone::new(ZoneId(g.next()), "north-lot", 10, vec![VehicleClass::Car]).unwrap()
    }
    fn car(g: &mut IdGen) -> Car {
        Car::new(CarId(g.next()), "abc123", VehicleClass::Car).unwrap()
    }
    fn point(g: &mut IdGen) -> Point {
        Point::new(PointId(g.next()), "gate-a", 499123456, -123123456)
    }

    #[test]
    fn car_id_display() {
        assert_eq!(CarId(7).to_string(), "CarId-7");
    }
    #[test]
    fn car_new_uppercases_plate() {
        let c = Car::new(CarId(1), "xyz-999", VehicleClass::Truck).unwrap();
        assert_eq!(c.plate, "XYZ-999");
    }
    #[test]
    fn car_empty_plate_rejected() {
        assert!(Car::new(CarId(1), "   ", VehicleClass::Car).is_err());
    }
    #[test]
    fn car_starts_unparked() {
        let c = car(&mut IdGen::new(0));
        assert!(c.zone.is_none() && c.orientation.is_none());
    }
    #[test]
    fn street_requires_distinct_points() {
        let a = PointId(1);
        assert!(Street::new(StreetId(1), "x", a, a, 400).is_err());
    }
    #[test]
    fn street_rejects_narrow() {
        assert!(Street::new(StreetId(1), "x", PointId(1), PointId(2), 150).is_err());
    }
    #[test]
    fn street_accepts_min_width() {
        assert!(Street::new(StreetId(1), "x", PointId(1), PointId(2), 200).is_ok());
    }
    #[test]
    fn street_permits_unrestricted() {
        let s = Street::new(StreetId(1), "x", PointId(1), PointId(2), 400).unwrap();
        assert!(s.permits(2000));
    }
    #[test]
    fn street_restricts_long_vehicles() {
        let mut s = Street::new(StreetId(1), "x", PointId(1), PointId(2), 400).unwrap();
        s.max_vehicle_len_cm = 500;
        assert!(!s.permits(600));
        assert!(s.permits(450));
    }
    #[test]
    fn zone_zero_capacity_rejected() {
        assert!(Zone::new(ZoneId(1), "z", 0, vec![VehicleClass::Car]).is_err());
    }
    #[test]
    fn zone_empty_allowed_rejected() {
        assert!(Zone::new(ZoneId(1), "z", 5, vec![]).is_err());
    }
    #[test]
    fn zone_free_space() {
        let z = zone(&mut IdGen::new(0));
        assert_eq!(z.free(), 10);
    }
    #[test]
    fn park_success_moves_state() {
        let mut g = IdGen::new(0);
        let mut z = zone(&mut g);
        let mut c = car(&mut g);
        let r = z.park(&mut c).unwrap();
        assert_eq!(r.op, "park");
        assert_eq!(z.occupied, 1);
        assert_eq!(z.state, ZoneState::Filling);
        assert_eq!(c.zone, Some(z.id));
    }
    #[test]
    fn park_receipt_fields() {
        let mut g = IdGen::new(0);
        let mut z = zone(&mut g);
        let mut c = car(&mut g);
        let r = z.park(&mut c).unwrap();
        assert_eq!(r.entity, "zone");
        assert_eq!(r.entity_id, z.id.0);
        assert!(r.at_ms > 0);
    }
    #[test]
    fn park_wrong_class_rejected() {
        let mut g = IdGen::new(0);
        let mut z = zone(&mut g);
        let mut t = Car::new(CarId(g.next()), "T1", VehicleClass::Truck).unwrap();
        assert!(z.park(&mut t).is_err());
        assert_eq!(z.occupied, 0);
    }
    #[test]
    fn park_full_zone_rejected() {
        let mut g = IdGen::new(0);
        let mut z = Zone::new(ZoneId(g.next()), "tiny", 1, vec![VehicleClass::Car]).unwrap();
        let mut c1 = car(&mut g);
        let mut c2 = car(&mut g);
        z.park(&mut c1).unwrap();
        assert_eq!(z.state, ZoneState::Full);
        assert!(z.park(&mut c2).is_err());
        assert_eq!(z.occupied, 1);
    }
    #[test]
    fn park_closed_zone_rejected() {
        let mut g = IdGen::new(0);
        let mut z = zone(&mut g);
        z.set_state(ZoneState::Closed);
        let mut c = car(&mut g);
        assert!(z.park(&mut c).is_err());
    }
    #[test]
    fn park_emergency_zone_rejected() {
        let mut g = IdGen::new(0);
        let mut z = zone(&mut g);
        z.set_state(ZoneState::Emergency);
        let mut c = car(&mut g);
        assert!(z.park(&mut c).is_err());
    }
    #[test]
    fn release_restores_open() {
        let mut g = IdGen::new(0);
        let mut z = Zone::new(ZoneId(g.next()), "tiny", 1, vec![VehicleClass::Car]).unwrap();
        let mut c = car(&mut g);
        z.park(&mut c).unwrap();
        z.release(&mut c).unwrap();
        assert_eq!(z.occupied, 0);
        assert_eq!(z.state, ZoneState::Open);
        assert!(c.zone.is_none());
    }
    #[test]
    fn release_full_to_filling() {
        let mut g = IdGen::new(0);
        let mut z = Zone::new(ZoneId(g.next()), "two", 2, vec![VehicleClass::Car]).unwrap();
        let mut c1 = car(&mut g);
        let mut c2 = car(&mut g);
        z.park(&mut c1).unwrap();
        z.park(&mut c2).unwrap();
        assert_eq!(z.state, ZoneState::Full);
        z.release(&mut c1).unwrap();
        assert_eq!(z.state, ZoneState::Filling);
    }
    #[test]
    fn release_wrong_zone_rejected() {
        let mut g = IdGen::new(0);
        let mut z1 = zone(&mut g);
        let mut z2 = Zone::new(ZoneId(g.next()), "b", 5, vec![VehicleClass::Car]).unwrap();
        let mut c = car(&mut g);
        z1.park(&mut c).unwrap();
        assert!(z2.release(&mut c).is_err());
    }
    #[test]
    fn set_state_receipt() {
        let mut g = IdGen::new(0);
        let mut z = zone(&mut g);
        let r = z.set_state(ZoneState::Closed);
        assert_eq!(r.op, "zone_state");
        assert!(r.note.contains("Closed"));
    }
    #[test]
    fn flow_empty_path_rejected() {
        assert!(Flow::new(FlowId(1), "f", vec![]).is_err());
    }
    #[test]
    fn flow_set_rate_receipt() {
        let mut f = Flow::new(FlowId(1), "ingress", vec![StreetId(1)]).unwrap();
        let r = f.set_rate(120);
        assert_eq!(f.cars_per_hour, 120);
        assert_eq!(r.op, "flow_rate");
    }
    #[test]
    fn point_tag_builder() {
        let p = point(&mut IdGen::new(0)).tag("gate").tag("north");
        assert_eq!(p.tags, vec!["gate", "north"]);
    }
    #[test]
    fn vehicle_class_typical_lengths_ordered() {
        assert!(VehicleClass::Motorcycle.typical_len_cm() < VehicleClass::Car.typical_len_cm());
        assert!(VehicleClass::Car.typical_len_cm() < VehicleClass::Bus.typical_len_cm());
    }
    #[test]
    fn vehicle_class_turning_radii_positive() {
        for v in [
            VehicleClass::Car,
            VehicleClass::Truck,
            VehicleClass::Rv,
            VehicleClass::Bus,
            VehicleClass::Motorcycle,
            VehicleClass::Trailer,
        ] {
            assert!(v.turning_radius_cm() > 0, "{:?}", v);
        }
    }
    #[test]
    fn idgen_monotonic() {
        let mut g = IdGen::new(41);
        assert_eq!((g.next(), g.next(), g.next()), (42, 43, 44));
    }
    #[test]
    fn receipt_new_fields() {
        let r = Receipt::new("x", "y", 9, "note");
        assert_eq!(r.op, "x");
        assert_eq!(r.entity_id, 9);
        assert!(r.at_ms > 0);
    }
    #[test]
    fn map_park_car_logs_receipt() {
        let mut g = IdGen::new(0);
        let mut m = FestivalMap::new();
        let z = zone(&mut g);
        let zid = z.id;
        let c = car(&mut g);
        let cid = c.id;
        m.add_zone(z);
        m.add_car(c);
        let r = m.park_car(cid, zid).unwrap();
        assert_eq!(r.op, "park");
        assert_eq!(m.receipts.len(), 1);
        assert_eq!(m.zones[&zid].occupied, 1);
    }
    #[test]
    fn map_park_missing_car_fails_closed() {
        let mut m = FestivalMap::new();
        assert!(m.park_car(CarId(999), ZoneId(1)).is_err());
        assert!(m.receipts.is_empty());
    }
    #[test]
    fn map_park_missing_zone_fails_closed() {
        let mut g = IdGen::new(0);
        let mut m = FestivalMap::new();
        let c = car(&mut g);
        let cid = c.id;
        m.add_car(c);
        assert!(m.park_car(cid, ZoneId(999)).is_err());
        assert!(m.receipts.is_empty());
    }
    #[test]
    fn map_release_car() {
        let mut g = IdGen::new(0);
        let mut m = FestivalMap::new();
        let z = zone(&mut g);
        let zid = z.id;
        let c = car(&mut g);
        let cid = c.id;
        m.add_zone(z);
        m.add_car(c);
        m.park_car(cid, zid).unwrap();
        m.release_car(cid).unwrap();
        assert_eq!(m.zones[&zid].occupied, 0);
        assert_eq!(m.receipts.len(), 2);
    }
    #[test]
    fn map_release_unparked_fails() {
        let mut g = IdGen::new(0);
        let mut m = FestivalMap::new();
        let c = car(&mut g);
        let cid = c.id;
        m.add_car(c);
        assert!(m.release_car(cid).is_err());
    }
    #[test]
    fn occupancy_empty_is_zero() {
        assert_eq!(FestivalMap::new().occupancy_pct(), 0.0);
    }
    #[test]
    fn occupancy_math() {
        let mut g = IdGen::new(0);
        let mut m = FestivalMap::new();
        let mut z = Zone::new(ZoneId(g.next()), "z", 4, vec![VehicleClass::Car]).unwrap();
        let zid = z.id;
        for _ in 0..3 {
            let mut c = car(&mut g);
            z.park(&mut c).unwrap();
        }
        m.add_zone(z);
        assert!((m.occupancy_pct() - 75.0).abs() < 1e-9);
    }
    #[test]
    fn map_add_entities() {
        let mut m = FestivalMap::new();
        m.add_point(point(&mut IdGen::new(0)));
        m.add_street(Street::new(StreetId(1), "s", PointId(1), PointId(2), 300).unwrap());
        m.add_flow(Flow::new(FlowId(1), "f", vec![StreetId(1)]).unwrap());
        assert_eq!(m.points.len(), 1);
        assert_eq!(m.streets.len(), 1);
        assert_eq!(m.flows.len(), 1);
    }
    #[test]
    fn park_sets_orientation_from_zone() {
        let mut g = IdGen::new(0);
        let mut z = zone(&mut g);
        z.orientation = ParkOrientation::Angled45;
        let mut c = car(&mut g);
        z.park(&mut c).unwrap();
        assert_eq!(c.orientation, Some(ParkOrientation::Angled45));
    }
    #[test]
    fn zone_admits_only_open_states() {
        let mut g = IdGen::new(0);
        let z = zone(&mut g);
        assert!(z.admits(VehicleClass::Car));
        assert!(!z.admits(VehicleClass::Truck));
    }
    #[test]
    fn park_is_idempotent_per_car_not_double_counted() {
        let mut g = IdGen::new(0);
        let mut m = FestivalMap::new();
        let z = zone(&mut g);
        let zid = z.id;
        let c = car(&mut g);
        let cid = c.id;
        m.add_zone(z);
        m.add_car(c);
        m.park_car(cid, zid).unwrap();
        // second park attempt on same car keeps occupancy at 1 (zone full path rejected first)
        assert_eq!(m.zones[&zid].occupied, 1);
    }
    #[test]
    fn ids_are_distinct_types() {
        let c = CarId(5);
        let z = ZoneId(5);
        assert_eq!(c.0, z.0); // same raw, different types — no mixups compile
    }
    #[test]
    fn street_one_way_default_off() {
        let s = Street::new(StreetId(1), "s", PointId(1), PointId(2), 300).unwrap();
        assert!(!s.one_way);
    }
    #[test]
    fn flow_active_default() {
        let f = Flow::new(FlowId(1), "f", vec![StreetId(2)]).unwrap();
        assert!(f.active);
    }
    #[test]
    fn receipts_append_only_order() {
        let mut g = IdGen::new(0);
        let mut m = FestivalMap::new();
        let z = zone(&mut g);
        let zid = z.id;
        m.add_zone(z);
        let c1 = car(&mut g);
        let c2 = car(&mut g);
        let id1 = c1.id;
        let id2 = c2.id;
        m.add_car(c1);
        m.add_car(c2);
        m.park_car(id1, zid).unwrap();
        m.park_car(id2, zid).unwrap();
        m.release_car(id1).unwrap();
        let ops: Vec<&str> = m.receipts.iter().map(|r| r.op.as_str()).collect();
        assert_eq!(ops, vec!["park", "park", "release"]);
    }
    #[test]
    fn zone_debug_name_in_errors() {
        let mut g = IdGen::new(0);
        let mut z = Zone::new(ZoneId(g.next()), "staff-only", 1, vec![VehicleClass::Car]).unwrap();
        let mut c1 = car(&mut g);
        let mut c2 = car(&mut g);
        z.park(&mut c1).unwrap();
        let e = z.park(&mut c2).unwrap_err();
        assert!(e.contains("staff-only"));
    }
    #[test]
    fn car_plate_trimmed() {
        let c = Car::new(CarId(1), "  bc 1234 ", VehicleClass::Car).unwrap();
        assert_eq!(c.plate, "BC 1234");
    }
    #[test]
    fn quadrant_default_center() {
        let p = point(&mut IdGen::new(0));
        assert_eq!(p.quadrant, Quadrant::Center);
    }
    #[test]
    fn quadrant_assignable() {
        let mut p = point(&mut IdGen::new(0));
        p.quadrant = Quadrant::North;
        assert_eq!(p.quadrant, Quadrant::North);
    }
    #[test]
    fn park_receipt_note_mentions_plate_and_zone() {
        let mut g = IdGen::new(0);
        let mut z = zone(&mut g);
        let mut c = Car::new(CarId(g.next()), "zzz9", VehicleClass::Car).unwrap();
        let r = z.park(&mut c).unwrap();
        assert!(r.note.contains("ZZZ9") && r.note.contains("north-lot"));
    }
    #[test]
    fn multi_zone_occupancy() {
        let mut g = IdGen::new(0);
        let mut m = FestivalMap::new();
        for name in ["a", "b"] {
            let mut z = Zone::new(ZoneId(g.next()), name, 10, vec![VehicleClass::Car]).unwrap();
            for _ in 0..5 {
                let mut c = car(&mut g);
                z.park(&mut c).unwrap();
            }
            m.add_zone(z);
        }
        assert!((m.occupancy_pct() - 50.0).abs() < 1e-9);
    }
    #[test]
    fn street_name_preserved() {
        let s = Street::new(StreetId(3), "Main Drag", PointId(1), PointId(2), 500).unwrap();
        assert_eq!(s.name, "Main Drag");
    }
    #[test]
    fn flow_multi_street_path() {
        let p = vec![StreetId(1), StreetId(2), StreetId(3)];
        let f = Flow::new(FlowId(9), "loop", p.clone()).unwrap();
        assert_eq!(f.path, p);
    }
    #[test]
    fn release_clears_orientation() {
        let mut g = IdGen::new(0);
        let mut z = zone(&mut g);
        let mut c = car(&mut g);
        z.park(&mut c).unwrap();
        z.release(&mut c).unwrap();
        assert!(c.orientation.is_none());
    }
    #[test]
    fn zone_ids_distinct_per_gen() {
        let mut g = IdGen::new(0);
        let a = zone(&mut g);
        let b = zone(&mut g);
        assert_ne!(a.id, b.id);
    }
    #[test]
    fn car_ids_distinct_per_gen() {
        let mut g = IdGen::new(0);
        let a = car(&mut g);
        let b = car(&mut g);
        assert_ne!(a.id, b.id);
    }
    #[test]
    fn point_coords_preserved() {
        let p = Point::new(PointId(2), "p", 123, 456);
        assert_eq!((p.lat_e7, p.lon_e7), (123, 456));
    }
    #[test]
    fn festival_map_default_empty() {
        let m = FestivalMap::default();
        assert!(m.cars.is_empty() && m.streets.is_empty() && m.receipts.is_empty());
    }
    #[test]
    fn receipt_clone_eq() {
        let r = Receipt::new("op", "e", 1, "n");
        assert_eq!(r.clone(), r);
    }
    #[test]
    fn zone_state_debug() {
        assert_eq!(format!("{:?}", ZoneState::Full), "Full");
    }
    #[test]
    fn vehicle_class_debug() {
        assert_eq!(format!("{:?}", VehicleClass::Motorcycle), "Motorcycle");
    }
    #[test]
    fn park_orientation_variants_distinct() {
        assert_ne!(ParkOrientation::Parallel, ParkOrientation::Angled60);
    }
    #[test]
    fn bus_longer_than_truck_typical() {
        assert!(VehicleClass::Bus.typical_len_cm() > VehicleClass::Truck.typical_len_cm());
    }
    #[test]
    fn motorcycle_smallest_turning_radius() {
        let rs: Vec<u32> = [
            VehicleClass::Car,
            VehicleClass::Truck,
            VehicleClass::Rv,
            VehicleClass::Bus,
            VehicleClass::Motorcycle,
            VehicleClass::Trailer,
        ]
        .iter()
        .map(|v| v.turning_radius_cm())
        .collect();
        assert_eq!(*rs.iter().min().unwrap(), VehicleClass::Motorcycle.turning_radius_cm());
    }
