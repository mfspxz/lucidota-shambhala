use super::*;
use sham_domain::ZoneId;

fn coord(lat: f64, lon: f64) -> GeoCoord {
    GeoCoord::new(lat, lon).unwrap()
}
fn touch(lat: f64, lon: f64, t: u64) -> Touch {
    Touch { at: coord(lat, lon), t_ms: t }
}
fn ent(kind: EntityKind, id: u64) -> EntityRef {
    EntityRef { kind, id }
}

// ---- sham-011: gestures ----
#[test]
fn single_tap_after_flush() {
    let mut ui = RtsUi::new();
    assert!(ui.touch(touch(49.2, -122.9, 1000)).is_none());
    let g = ui.flush_tap().unwrap();
    assert!(matches!(g, Gesture::Tap(_)));
}
#[test]
fn double_tap_recognized() {
    let mut ui = RtsUi::new();
    ui.touch(touch(49.2, -122.9, 1000));
    let g = ui.touch(touch(49.20001, -122.9, 1200)).unwrap();
    assert!(matches!(g, Gesture::DoubleTap(_, _)));
}
#[test]
fn double_tap_too_slow_is_two_taps() {
    let mut ui = RtsUi::new();
    ui.touch(touch(49.2, -122.9, 1000));
    assert!(ui.touch(touch(49.2, -122.9, 2000)).is_none()); // > 400ms
    assert!(ui.flush_tap().is_some());
}
#[test]
fn double_tap_drift_breaks() {
    let mut ui = RtsUi::new();
    ui.touch(touch(49.2, -122.9, 1000));
    // 100m away within time window -> not a double tap
    assert!(ui.touch(touch(49.201, -122.9, 1100)).is_none());
}
#[test]
fn drag_gesture() {
    let mut ui = RtsUi::new();
    let g = ui.drag(touch(49.2, -122.9, 0), touch(49.3, -122.8, 500));
    assert!(matches!(g, Gesture::Drag { .. }));
}
#[test]
fn drag_cancels_pending_tap() {
    let mut ui = RtsUi::new();
    ui.touch(touch(49.2, -122.9, 1000));
    ui.drag(touch(49.2, -122.9, 1000), touch(49.3, -122.8, 1100));
    assert!(ui.flush_tap().is_none());
}
#[test]
fn select_drag_picks_inside() {
    let mut ui = RtsUi::new();
    let ents = vec![
        (ent(EntityKind::Car, 1), coord(49.200, -122.900)),
        (ent(EntityKind::Car, 2), coord(49.201, -122.901)),
        (ent(EntityKind::Car, 3), coord(49.500, -122.900)), // far
    ];
    let n = ui.select_drag(coord(49.199, -122.902), coord(49.202, -122.899), &ents);
    assert_eq!(n, 2);
}
#[test]
fn select_drag_empty() {
    let mut ui = RtsUi::new();
    let n = ui.select_drag(coord(49.0, -123.0), coord(49.001, -122.999), &[]);
    assert_eq!(n, 0);
}
#[test]
fn select_logs_receipt() {
    let mut ui = RtsUi::new();
    ui.select_drag(coord(49.0, -123.0), coord(49.001, -122.999), &[]);
    assert_eq!(ui.receipts.len(), 1);
    assert_eq!(ui.receipts[0].op, "select");
}
#[test]
fn paint_accumulates_cells() {
    let mut ui = RtsUi::new();
    let cells = vec![BBox::around(coord(49.2, -122.9), 10.0); 3];
    assert_eq!(ui.paint(cells), 3);
    assert_eq!(ui.painted_cells.len(), 3);
}
#[test]
fn classify_majority() {
    let mut ui = RtsUi::new();
    let ents = vec![
        (ent(EntityKind::Car, 1), coord(49.200, -122.900)),
        (ent(EntityKind::Car, 2), coord(49.200, -122.900)),
        (ent(EntityKind::Zone, 9), coord(49.200, -122.900)),
    ];
    ui.select_drag(coord(49.199, -122.901), coord(49.201, -122.899), &ents);
    let c = ui.classify().unwrap();
    assert_eq!(c.kind, EntityKind::Car);
    assert!((c.confidence - 2.0 / 3.0).abs() < 1e-6);
}
#[test]
fn classify_empty_none() {
    let ui = RtsUi::new();
    assert!(ui.classify().is_none());
}
#[test]
fn classify_unanimous_confidence_one() {
    let mut ui = RtsUi::new();
    let ents = vec![(ent(EntityKind::Point, 1), coord(49.2, -122.9))];
    ui.select_drag(coord(49.19, -122.91), coord(49.21, -122.89), &ents);
    assert_eq!(ui.classify().unwrap().confidence, 1.0);
}

// ---- sham-016: volunteer interaction ----
#[test]
fn tired_mode_bigger_slop() {
    let mut ui = RtsUi::new();
    ui.set_tired(true);
    assert_eq!(ui.slop(), TIRED_SLOP_M);
    ui.set_tired(false);
    assert_eq!(ui.slop(), TOUCH_SLOP_M);
}
#[test]
fn tired_slop_selects_more_forgiving() {
    // entity just outside normal slop but inside tired slop
    let e = (ent(EntityKind::Car, 1), coord(49.2, -122.9));
    let mut ui = RtsUi::new();
    // drag box ending ~15m south of the entity
    let south = coord(49.2, -122.9).destination(180.0, 15.0);
    let n_normal = ui.select_drag(coord(49.199, -122.901), south, &[e.clone()]);
    ui.set_tired(true);
    let n_tired = ui.select_drag(coord(49.199, -122.901), south, &[e]);
    assert!(n_tired >= n_normal);
}
#[test]
fn undo_empty_fails_closed() {
    let mut ui = RtsUi::new();
    assert!(ui.undo().is_err());
}
#[test]
fn undo_roundtrip() {
    let mut ui = RtsUi::new();
    ui.push_undo("park car 5");
    assert_eq!(ui.undo().unwrap(), "park car 5");
    assert!(ui.undo().is_err());
}
#[test]
fn tired_receipt_logged() {
    let mut ui = RtsUi::new();
    let r = ui.set_tired(true);
    assert_eq!(r.op, "tired_mode");
    assert!(r.note.contains("true"));
}
#[test]
fn slop_constants_sane() {
    assert!(TIRED_SLOP_M > TOUCH_SLOP_M);
    assert!(TOUCH_SLOP_M >= 10.0);
}

// ---- sham-023: double-tap angle parking ----
#[test]
fn stall_45_ok() {
    let t = touch(49.2, -122.9, 0);
    let s = place_angled_stall(&t, ZoneId(1), 45.0, 250, 1000.0).unwrap();
    assert_eq!(s.orientation, ParkOrientation::Angled45);
    assert!(s.curb_cm > 250.0); // 45deg consumes more curb than width
}
#[test]
fn stall_60_ok() {
    let t = touch(49.2, -122.9, 0);
    let s = place_angled_stall(&t, ZoneId(1), 60.0, 250, 1000.0).unwrap();
    assert_eq!(s.orientation, ParkOrientation::Angled60);
}
#[test]
fn stall_90_ok() {
    let t = touch(49.2, -122.9, 0);
    let s = place_angled_stall(&t, ZoneId(1), 90.0, 250, 1000.0).unwrap();
    assert_eq!(s.orientation, ParkOrientation::Perpendicular);
    assert!((s.curb_cm - 250.0).abs() < 1.0);
}
#[test]
fn stall_bad_angle_rejected() {
    let t = touch(49.2, -122.9, 0);
    assert!(place_angled_stall(&t, ZoneId(1), 30.0, 250, 1000.0).is_err());
}
#[test]
fn stall_insufficient_curb_rejected() {
    let t = touch(49.2, -122.9, 0);
    // 45deg stall needs ~354cm curb
    assert!(place_angled_stall(&t, ZoneId(1), 45.0, 250, 300.0).is_err());
}
#[test]
fn stall_bad_width_rejected() {
    let t = touch(49.2, -122.9, 0);
    assert!(place_angled_stall(&t, ZoneId(1), 45.0, 150, 1000.0).is_err());
    assert!(place_angled_stall(&t, ZoneId(1), 45.0, 500, 1000.0).is_err());
}
#[test]
fn stall_preserves_zone_and_center() {
    let t = touch(49.2, -122.9, 0);
    let s = place_angled_stall(&t, ZoneId(7), 60.0, 250, 1000.0).unwrap();
    assert_eq!(s.zone, ZoneId(7));
    assert_eq!(s.center, t.at);
}

// ---- sham-029: training mode ----
#[test]
fn training_has_8_steps() {
    assert_eq!(TrainingMode::standard().steps.len(), 8);
}
#[test]
fn training_complete_all() {
    let mut tm = TrainingMode::standard();
    let mut finished = false;
    for (i, id) in ["tap", "drag", "paint", "doubletap", "dispatch", "undo", "tired", "offline"]
        .iter()
        .enumerate()
    {
        finished = tm.complete_step(id, (i * 30_000) as u64).unwrap();
    }
    assert!(finished);
    assert_eq!(tm.progress_pct(), 100.0);
}
#[test]
fn training_unknown_step_rejected() {
    let mut tm = TrainingMode::standard();
    assert!(tm.complete_step("fly", 0).is_err());
}
#[test]
fn training_double_complete_rejected() {
    let mut tm = TrainingMode::standard();
    tm.complete_step("tap", 0).unwrap();
    assert!(tm.complete_step("tap", 1000).is_err());
}
#[test]
fn training_budget_exceeded() {
    let mut tm = TrainingMode::standard();
    assert!(tm.complete_step("tap", 600_001).is_err());
}
#[test]
fn training_progress_partial() {
    let mut tm = TrainingMode::standard();
    tm.complete_step("tap", 0).unwrap();
    tm.complete_step("drag", 1000).unwrap();
    assert!((tm.progress_pct() - 25.0).abs() < 1e-9);
}
#[test]
fn training_remaining_counts_down() {
    let tm = TrainingMode::standard();
    assert_eq!(tm.remaining_ms(0), 600_000);
    assert_eq!(tm.remaining_ms(600_000), 0);
    assert_eq!(tm.remaining_ms(999_999), 0);
}
#[test]
fn training_step_budgets_sum_under_10min() {
    let tm = TrainingMode::standard();
    let sum: u64 = tm.steps.iter().map(|s| s.budget_s).sum();
    assert!(sum <= 600, "sum={}", sum);
}
#[test]
fn entity_ref_hash_usable_in_set() {
    let mut ui = RtsUi::new();
    ui.selection.insert(ent(EntityKind::Flow, 3));
    assert_eq!(ui.selection.len(), 1);
}
#[test]
fn gesture_debug() {
    let g = Gesture::LongPress(touch(49.0, -123.0, 5));
    assert!(format!("{:?}", g).contains("LongPress"));
}
#[test]
fn angled_stall_rejects_near_miss_angle() {
    // 45.9 must not truncate to 45
    let tap = Touch { at: GeoCoord::new(49.2, -122.9).unwrap(), t_ms: 0 };
    assert!(place_angled_stall(&tap, ZoneId(1), 45.9, 250, 1000.0).is_err());
    assert!(place_angled_stall(&tap, ZoneId(1), 45.0, 250, 1000.0).is_ok());
}
