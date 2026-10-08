use super::*;

#[test]
fn organelle_set_has_four() {
    assert_eq!(organelle_set().len(), 4);
}
#[test]
fn all_manifests_valid() {
    for o in organelle_set() {
        assert!(o.check_manifest().is_ok(), "{}", o.manifest.name);
        assert!(o.manifest.transports.contains(&Transport::Native));
    }
}
#[test]
fn native_rest_equivalence() {
    // behavioral equivalence: same input -> same output across transports
    let cases: Vec<(&str, Vec<(&str, &str)>)> = vec![
        ("sham.route", vec![("from_lat", "49.2"), ("from_lon", "-122.9"), ("to_lat", "49.21"), ("to_lon", "-122.91")]),
        ("sham.suggest-zone", vec![("class", "Car"), ("zones", "1:10:2,2:10:9")]),
        ("sham.validate-ping", vec![("lat", "49.2"), ("lon", "-122.9"), ("ts_ms", "1000")]),
        ("sham.eta", vec![("dist_m", "6000"), ("speed_kmh", "30")]),
    ];
    for o in organelle_set() {
        let (_, pairs) = cases.iter().find(|(n, _)| *n == o.manifest.name).unwrap();
        let a = args(pairs);
        assert_eq!(o.invoke_native(&a), o.invoke_rest(&a), "{}", o.manifest.name);
    }
}
#[test]
fn suggest_zone_organelle_picks_roomiest() {
    let o = organelle_set().into_iter().find(|o| o.manifest.name == "sham.suggest-zone").unwrap();
    let out = o.invoke_native(&args(&[("class", "Car"), ("zones", "1:10:9,2:10:0")])).unwrap();
    assert!(out.contains("\"zone_id\":2"), "{}", out);
    let none = o.invoke_native(&args(&[("class", "Car"), ("zones", "1:10:10")])).unwrap();
    assert!(none.contains("null"), "{}", none);
}
