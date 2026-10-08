
use super::*;
    use sham_domain::VehicleClass;

    fn vancouver() -> GeoCoord {
        GeoCoord::new(49.2827, -123.1207).unwrap()
    }
    fn newwest() -> GeoCoord {
        GeoCoord::new(49.2057, -122.9110).unwrap()
    }

    #[test]
    fn coord_validation_lat() {
        assert!(GeoCoord::new(91.0, 0.0).is_err());
        assert!(GeoCoord::new(-91.0, 0.0).is_err());
        assert!(GeoCoord::new(90.0, 0.0).is_ok());
    }
    #[test]
    fn coord_validation_lon() {
        assert!(GeoCoord::new(0.0, 181.0).is_err());
        assert!(GeoCoord::new(0.0, -180.0).is_ok());
    }
    #[test]
    fn e7_roundtrip() {
        let c = vancouver();
        let (la, lo) = c.to_e7();
        let back = GeoCoord::from_e7(la, lo);
        assert!((back.lat - c.lat).abs() < 1e-6);
        assert!((back.lon - c.lon).abs() < 1e-6);
    }
    #[test]
    fn distance_van_to_newwest() {
        let d = vancouver().distance_m(newwest());
        assert!((d - 16500.0).abs() < 1500.0, "d={}", d);
    }
    #[test]
    fn distance_self_zero() {
        let c = vancouver();
        assert_eq!(c.distance_m(c), 0.0);
    }
    #[test]
    fn distance_symmetric() {
        let a = vancouver();
        let b = newwest();
        assert!((a.distance_m(b) - b.distance_m(a)).abs() < 1e-9);
    }
    #[test]
    fn bearing_north() {
        let a = GeoCoord::new(49.0, -123.0).unwrap();
        let b = GeoCoord::new(50.0, -123.0).unwrap();
        let br = a.bearing_deg(b);
        assert!(br < 1.0 || br > 359.0, "br={}", br);
    }
    #[test]
    fn bearing_east() {
        let a = GeoCoord::new(49.0, -123.0).unwrap();
        let b = GeoCoord::new(49.0, -122.0).unwrap();
        let br = a.bearing_deg(b);
        assert!((br - 90.0).abs() < 2.0, "br={}", br);
    }
    #[test]
    fn destination_roundtrip() {
        let a = vancouver();
        let b = a.destination(45.0, 1000.0);
        let d = a.distance_m(b);
        assert!((d - 1000.0).abs() < 1.0, "d={}", d);
    }
    #[test]
    fn destination_zero_stays() {
        let a = vancouver();
        let b = a.destination(123.0, 0.0);
        assert!(a.distance_m(b) < 1e-6);
    }
    #[test]
    fn turn_straight_always_fits() {
        assert!(can_make_turn(VehicleClass::Bus, 0.0, 200));
    }
    #[test]
    fn turn_motorcycle_tight() {
        // need = 300*(1-cos90) + 200 = 500cm; boundary is honest
        assert!(can_make_turn(VehicleClass::Motorcycle, 90.0, 500));
        assert!(!can_make_turn(VehicleClass::Motorcycle, 90.0, 499));
    }
    #[test]
    fn turn_bus_needs_width() {
        assert!(!can_make_turn(VehicleClass::Bus, 90.0, 400));
        assert!(can_make_turn(VehicleClass::Bus, 90.0, 1600));
    }
    #[test]
    fn turn_monotone_in_width() {
        for w in [300u32, 600, 900, 1200, 1500] {
            if can_make_turn(VehicleClass::Truck, 90.0, w) {
                assert!(can_make_turn(VehicleClass::Truck, 90.0, w + 100));
            }
        }
    }
    #[test]
    fn turn_small_angle_easier() {
        assert!(can_make_turn(VehicleClass::Truck, 10.0, 300));
    }
    #[test]
    fn angled_stall_45_vs_90() {
        let w = 250;
        assert!(angled_stall_curb_cm(w, 45.0) > angled_stall_curb_cm(w, 90.0));
    }
    #[test]
    fn stall_depth_90_full_length() {
        assert!((stall_depth_cm(450, 90.0) - 450.0).abs() < 1.0);
    }
    #[test]
    fn stall_depth_45_shorter() {
        assert!(stall_depth_cm(450, 45.0) < stall_depth_cm(450, 90.0));
    }
    #[test]
    fn bbox_contains_center() {
        let c = vancouver();
        let b = BBox::around(c, 50.0);
        assert!(b.contains(c));
    }
    #[test]
    fn bbox_excludes_far() {
        let b = BBox::around(vancouver(), 50.0);
        assert!(!b.contains(newwest()));
    }
    #[test]
    fn rng_deterministic() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(7);
        for _ in 0..10 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }
    #[test]
    fn rng_different_seeds_differ() {
        let mut a = Rng::new(7);
        let mut b = Rng::new(8);
        assert_ne!(a.next_u64(), b.next_u64());
    }
    #[test]
    fn rng_range_in_bounds() {
        let mut r = Rng::new(3);
        for _ in 0..100 {
            let x = r.range_f64(-5.0, 5.0);
            assert!((-5.0..=5.0).contains(&x));
        }
    }

    // ---- property tests (seeded, deterministic) ----
    #[test]
    fn prop_distance_nonneg() {
        let mut r = Rng::new(1001);
        for _ in 0..200 {
            let (a, b) = (r.coord(), r.coord());
            assert!(a.distance_m(b) >= 0.0);
        }
    }
    #[test]
    fn prop_distance_triangle_inequality() {
        let mut r = Rng::new(1002);
        for _ in 0..200 {
            let (a, b, c) = (r.coord(), r.coord(), r.coord());
            assert!(a.distance_m(c) <= a.distance_m(b) + b.distance_m(c) + 1e-6);
        }
    }
    #[test]
    fn prop_bearing_in_range() {
        let mut r = Rng::new(1003);
        for _ in 0..200 {
            let (a, b) = (r.coord(), r.coord());
            let br = a.bearing_deg(b);
            assert!((0.0..360.0).contains(&br), "br={}", br);
        }
    }
    #[test]
    fn prop_destination_distance() {
        let mut r = Rng::new(1004);
        for _ in 0..200 {
            let a = r.coord();
            let d = r.range_f64(1.0, 50_000.0);
            let b = a.destination(r.range_f64(0.0, 360.0), d);
            let got = a.distance_m(b);
            assert!((got - d).abs() < d * 0.001 + 0.5, "want {} got {}", d, got);
        }
    }
    #[test]
    fn prop_e7_roundtrip() {
        let mut r = Rng::new(1005);
        for _ in 0..200 {
            let a = r.coord();
            let (la, lo) = a.to_e7();
            let b = GeoCoord::from_e7(la, lo);
            assert!(a.distance_m(b) < 1.0);
        }
    }
    #[test]
    fn prop_turn_monotone_angle() {
        let mut r = Rng::new(1006);
        for _ in 0..100 {
            let w = 800u32;
            let a1 = r.range_f64(5.0, 60.0);
            let a2 = r.range_f64(60.0, 170.0);
            // if a sharp turn fits, the gentler one fits too
            if can_make_turn(VehicleClass::Truck, a2, w) {
                assert!(can_make_turn(VehicleClass::Truck, a1, w));
            }
        }
    }
    #[test]
    fn prop_stall_depth_monotone() {
        let mut r = Rng::new(1007);
        for _ in 0..100 {
            let len = r.range_f64(300.0, 1200.0) as u32;
            assert!(stall_depth_cm(len, 30.0) <= stall_depth_cm(len, 90.0));
        }
    }
    #[test]
    fn prop_bbox_radius() {
        let mut r = Rng::new(1008);
        for _ in 0..100 {
            let c = r.coord();
            let b = BBox::around(c, 25.0);
            let inside = c.destination(r.range_f64(0.0, 360.0), r.range_f64(0.0, 20.0));
            assert!(b.contains(inside));
        }
    }
    #[test]
    fn prop_coord_new_valid() {
        let mut r = Rng::new(1009);
        for _ in 0..100 {
            let c = r.coord();
            assert!(GeoCoord::new(c.lat, c.lon).is_ok());
        }
    }
    #[test]
    fn prop_distance_zero_iff_same() {
        let mut r = Rng::new(1010);
        for _ in 0..100 {
            let a = r.coord();
            assert!(a.distance_m(a) < 1e-9);
            let b = r.coord();
            if (a.lat - b.lat).abs() > 1e-4 || (a.lon - b.lon).abs() > 1e-4 {
                assert!(a.distance_m(b) > 0.0);
            }
        }
    }
    #[test]
    fn destination_antimeridian_safe() {
        let a = GeoCoord::new(0.0, 179.9).unwrap();
        let b = a.destination(90.0, 50_000.0);
        assert!((-180.0..=180.0).contains(&b.lon));
    }
    #[test]
    fn earth_radius_positive() {
        assert!(EARTH_RADIUS_M > 6.0e6);
    }
    #[test]
    fn vehicle_class_reexport_usable() {
        assert_eq!(VehicleClass::Car.turning_radius_cm(), 550);
    }
