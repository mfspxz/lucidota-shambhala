use super::*;
use sham_domain::VehicleClass;

fn reg() -> Registry {
    Registry::load()
}

// ---- global (7) ----
#[test]
fn total_is_196() {
    assert_eq!(reg().len(), 196);
}
#[test]
fn batch_counts_match_spec() {
    let r = reg();
    for b in [
        Batch::Dispatch,
        Batch::Tracking,
        Batch::Comms,
        Batch::Reporting,
        Batch::Admin,
        Batch::FieldOps,
        Batch::Analytics,
        Batch::Integrations,
    ] {
        assert_eq!(r.batch_count(b), b.expected_count(), "{:?}", b);
    }
}
#[test]
fn integrity_check_passes() {
    assert!(reg().check_integrity().is_ok());
}
#[test]
fn ids_unique() {
    let all = all_features();
    let mut seen = std::collections::HashSet::new();
    for f in &all {
        assert!(seen.insert(f.id.clone()), "dup {}", f.id);
    }
}
#[test]
fn lookup_by_id() {
    let r = reg();
    let f = r.get("ODIN-DIS-002").unwrap();
    assert_eq!(f.name, "assign-car-zone");
    assert!(r.get("ODIN-XXX-001").is_none());
}
#[test]
fn prefixes_match_batches() {
    assert_eq!(Batch::Dispatch.prefix(), "ODIN-DIS");
    assert_eq!(Batch::Integrations.prefix(), "ODIN-INT");
}
#[test]
fn feature_ids_sequential() {
    let all = all_features();
    let dis: Vec<&Feature> = all.iter().filter(|f| f.batch == Batch::Dispatch).collect();
    assert_eq!(dis[0].id, "ODIN-DIS-001");
    assert_eq!(dis[24].id, "ODIN-DIS-025");
    let int: Vec<&Feature> = all.iter().filter(|f| f.batch == Batch::Integrations).collect();
    assert_eq!(int[20].id, "ODIN-INT-021");
}

// ---- dispatch batch (7) ----
#[test]
fn dis_suggest_picks_most_free() {
    let zones = vec![
        (ZoneId(1), "a".into(), 10u32, 9u32, vec![VehicleClass::Car]),
        (ZoneId(2), "b".into(), 10u32, 2u32, vec![VehicleClass::Car]),
    ];
    assert_eq!(suggest_zone(&zones, VehicleClass::Car), Some(ZoneId(2)));
}
#[test]
fn dis_suggest_skips_full() {
    let zones = vec![(ZoneId(1), "a".into(), 10u32, 10u32, vec![VehicleClass::Car])];
    assert_eq!(suggest_zone(&zones, VehicleClass::Car), None);
}
#[test]
fn dis_suggest_skips_wrong_class() {
    let zones = vec![(ZoneId(1), "a".into(), 10u32, 0u32, vec![VehicleClass::Truck])];
    assert_eq!(suggest_zone(&zones, VehicleClass::Car), None);
}
#[test]
fn dis_suggest_empty_none() {
    assert_eq!(suggest_zone(&[], VehicleClass::Car), None);
}
#[test]
fn dis_eta_basic() {
    let e = eta_minutes(5000.0, 30.0).unwrap();
    assert!((e - 10.0).abs() < 1e-9);
}
#[test]
fn dis_eta_rejects_bad_speed() {
    assert!(eta_minutes(100.0, 0.0).is_err());
    assert!(eta_minutes(100.0, -5.0).is_err());
}
#[test]
fn dis_eta_rejects_negative_dist() {
    assert!(eta_minutes(-1.0, 30.0).is_err());
}

// ---- tracking batch (7) ----
#[test]
fn trk_ping_valid() {
    assert!(validate_ping(49.28, -123.12, 1000, 2000).is_ok());
}
#[test]
fn trk_ping_bad_coords() {
    assert!(validate_ping(95.0, 0.0, 1000, 2000).is_err());
    assert!(validate_ping(0.0, 200.0, 1000, 2000).is_err());
}
#[test]
fn trk_ping_future_rejected() {
    assert!(validate_ping(49.0, -123.0, 1_000_000, 1000).is_err());
}
#[test]
fn trk_ping_ancient_rejected() {
    assert!(validate_ping(49.0, -123.0, 0, 8 * 24 * 3600 * 1000 + 1).is_err());
}
#[test]
fn trk_stale_detection() {
    assert!(is_stale(0, 61_000, 60_000));
    assert!(!is_stale(50_000, 61_000, 60_000));
}
#[test]
fn trk_speed_calc() {
    // ~111m in 36s => ~11.1 km/h
    let s = speed_kmh(49.0, -123.0, 0, 49.001, -123.0, 36_000).unwrap();
    assert!((s - 11.1).abs() < 0.6, "s={}", s);
}
#[test]
fn trk_speed_bad_time_none() {
    assert!(speed_kmh(49.0, -123.0, 5000, 49.0, -123.0, 5000).is_none());
    assert!(speed_kmh(49.0, -123.0, 6000, 49.0, -123.0, 5000).is_none());
}

// ---- comms batch (7) ----
#[test]
fn com_count_25() {
    assert_eq!(reg().batch_count(Batch::Comms), 25);
}
#[test]
fn com_has_broadcast() {
    assert!(reg().get("ODIN-COM-001").is_some());
    assert_eq!(reg().get("ODIN-COM-001").unwrap().name, "broadcast");
}
#[test]
fn com_has_offline_queue() {
    let r = reg();
    let f = r.get("ODIN-COM-011").unwrap();
    assert_eq!(f.name, "offline-queue");
}
#[test]
fn com_receipt_feature_last() {
    let r = reg();
    assert_eq!(r.get("ODIN-COM-025").unwrap().name, "comms-receipt");
}
#[test]
fn com_all_descs_nonempty() {
    for f in all_features().iter().filter(|f| f.batch == Batch::Comms) {
        assert!(!f.desc.is_empty(), "{}", f.id);
    }
}
#[test]
fn com_names_unique() {
    let mut s = std::collections::HashSet::new();
    for f in all_features().iter().filter(|f| f.batch == Batch::Comms) {
        assert!(s.insert(f.name), "dup {}", f.name);
    }
}
#[test]
fn com_ack_tracking_present() {
    assert_eq!(reg().get("ODIN-COM-007").unwrap().name, "ack-tracking");
}

// ---- reporting batch (7) ----
#[test]
fn rep_count_25() {
    assert_eq!(reg().batch_count(Batch::Reporting), 25);
}
#[test]
fn rep_incident_report_present() {
    assert_eq!(reg().get("ODIN-REP-002").unwrap().name, "incident-report");
}
#[test]
fn rep_redaction_present() {
    let r = reg();
    let f = r.get("ODIN-REP-022").unwrap();
    assert_eq!(f.name, "redaction");
}
#[test]
fn rep_receipt_feature_last() {
    assert_eq!(reg().get("ODIN-REP-025").unwrap().name, "report-receipt");
}
#[test]
fn rep_export_csv_json() {
    assert_eq!(reg().get("ODIN-REP-006").unwrap().name, "export-csv");
    assert_eq!(reg().get("ODIN-REP-007").unwrap().name, "export-json");
}
#[test]
fn rep_all_have_prefix() {
    for f in all_features().iter().filter(|f| f.batch == Batch::Reporting) {
        assert!(f.id.starts_with("ODIN-REP-"), "{}", f.id);
    }
}
#[test]
fn rep_signoff_present() {
    assert_eq!(reg().get("ODIN-REP-021").unwrap().name, "sign-off");
}

// ---- admin batch (7) ----
#[test]
fn adm_count_25() {
    assert_eq!(reg().batch_count(Batch::Admin), 25);
}
#[test]
fn adm_permission_check() {
    assert_eq!(reg().get("ODIN-ADM-003").unwrap().name, "permission-check");
}
#[test]
fn adm_zone_street_point_flow_define() {
    assert_eq!(reg().get("ODIN-ADM-004").unwrap().name, "zone-define");
    assert_eq!(reg().get("ODIN-ADM-005").unwrap().name, "street-define");
    assert_eq!(reg().get("ODIN-ADM-006").unwrap().name, "point-define");
    assert_eq!(reg().get("ODIN-ADM-007").unwrap().name, "flow-define");
}
#[test]
fn adm_api_key_lifecycle() {
    assert_eq!(reg().get("ODIN-ADM-022").unwrap().name, "api-key-issue");
    assert_eq!(reg().get("ODIN-ADM-023").unwrap().name, "api-key-revoke");
}
#[test]
fn adm_backup_restore() {
    assert_eq!(reg().get("ODIN-ADM-010").unwrap().name, "backup-snapshot");
    assert_eq!(reg().get("ODIN-ADM-011").unwrap().name, "restore");
}
#[test]
fn adm_receipt_last() {
    assert_eq!(reg().get("ODIN-ADM-025").unwrap().name, "admin-receipt");
}
#[test]
fn adm_maintenance_mode() {
    assert_eq!(reg().get("ODIN-ADM-014").unwrap().name, "maintenance-mode");
}

// ---- field-ops batch (7) ----
#[test]
fn fld_count_25() {
    assert_eq!(reg().batch_count(Batch::FieldOps), 25);
}
#[test]
fn fld_checkin_checkout() {
    assert_eq!(reg().get("ODIN-FLD-001").unwrap().name, "check-in");
    assert_eq!(reg().get("ODIN-FLD-002").unwrap().name, "check-out");
}
#[test]
fn fld_urgent_flags() {
    assert_eq!(reg().get("ODIN-FLD-016").unwrap().name, "firstaid-flag");
    assert_eq!(reg().get("ODIN-FLD-017").unwrap().name, "lostchild-flag");
}
#[test]
fn fld_gate_ops() {
    assert_eq!(reg().get("ODIN-FLD-019").unwrap().name, "gate-open");
    assert_eq!(reg().get("ODIN-FLD-020").unwrap().name, "gate-close");
}
#[test]
fn fld_sweep_pair() {
    assert_eq!(reg().get("ODIN-FLD-012").unwrap().name, "sweep-start");
    assert_eq!(reg().get("ODIN-FLD-013").unwrap().name, "sweep-end");
}
#[test]
fn fld_receipt_last() {
    assert_eq!(reg().get("ODIN-FLD-025").unwrap().name, "field-receipt");
}
#[test]
fn fld_cone_barrier_sign() {
    assert_eq!(reg().get("ODIN-FLD-008").unwrap().name, "cone-drop");
    assert_eq!(reg().get("ODIN-FLD-009").unwrap().name, "barrier-move");
    assert_eq!(reg().get("ODIN-FLD-010").unwrap().name, "sign-place");
}

// ---- analytics batch (7) ----
#[test]
fn anl_count_25() {
    assert_eq!(reg().batch_count(Batch::Analytics), 25);
}
#[test]
fn anl_occupancy_curve_first() {
    assert_eq!(reg().get("ODIN-ANL-001").unwrap().name, "occupancy-curve");
}
#[test]
fn anl_rates() {
    assert_eq!(reg().get("ODIN-ANL-003").unwrap().name, "turnover-rate");
    assert_eq!(reg().get("ODIN-ANL-004").unwrap().name, "ingress-rate");
    assert_eq!(reg().get("ODIN-ANL-005").unwrap().name, "egress-rate");
}
#[test]
fn anl_bottleneck() {
    assert_eq!(reg().get("ODIN-ANL-014").unwrap().name, "bottleneck-find");
}
#[test]
fn anl_receipt() {
    assert_eq!(reg().get("ODIN-ANL-021").unwrap().name, "analytics-receipt");
}
#[test]
fn anl_rollups() {
    assert_eq!(reg().get("ODIN-ANL-024").unwrap().name, "rollup-hourly");
    assert_eq!(reg().get("ODIN-ANL-025").unwrap().name, "rollup-daily");
}
#[test]
fn anl_sla_check() {
    assert_eq!(reg().get("ODIN-ANL-016").unwrap().name, "sla-check");
}

// ---- integrations batch (7) ----
#[test]
fn int_count_21() {
    assert_eq!(reg().batch_count(Batch::Integrations), 21);
}
#[test]
fn int_webhooks() {
    assert_eq!(reg().get("ODIN-INT-001").unwrap().name, "webhook-out");
    assert_eq!(reg().get("ODIN-INT-002").unwrap().name, "webhook-in");
}
#[test]
fn int_gis_exports() {
    assert_eq!(reg().get("ODIN-INT-013").unwrap().name, "gis-export");
    assert_eq!(reg().get("ODIN-INT-014").unwrap().name, "kml-export");
    assert_eq!(reg().get("ODIN-INT-015").unwrap().name, "gpx-export");
}
#[test]
fn int_receipt_last() {
    assert_eq!(reg().get("ODIN-INT-021").unwrap().name, "integration-receipt");
}
#[test]
fn int_stubs_are_stubs() {
    for f in all_features().iter().filter(|f| f.batch == Batch::Integrations) {
        if f.name.ends_with("-stub") || f.desc.contains("stub") {
            assert!(f.desc.contains("stub"), "{}", f.id);
        }
    }
}
#[test]
fn int_api_compat() {
    assert_eq!(reg().get("ODIN-INT-016").unwrap().name, "api-v1-compat");
}
#[test]
fn impossible_speed_flag() {
    assert!(is_impossible_speed(250.0));
    assert!(!is_impossible_speed(120.0));
}
