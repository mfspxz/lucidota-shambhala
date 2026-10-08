use super::*;

fn festival_ops(device: &str) -> OpLog {
    let mut log = OpLog::new(device);
    log.push(OpKind::AddZone { id: 1, name: "north".into(), capacity: 100 });
    log.push(OpKind::AddCar { id: 10, plate: "BC1".into() });
    log.push(OpKind::Park { car: 10, zone: 1 });
    log.push(OpKind::Release { car: 10 });
    log
}

#[test]
fn oplog_appends_with_clock() {
    let log = festival_ops("tablet-1");
    assert_eq!(log.len(), 4);
    assert_eq!(log.ops[0].lamport, 1);
    assert_eq!(log.ops[3].lamport, 4);
}
#[test]
fn merge_orders_causally() {
    let a = festival_ops("tablet-1");
    let b = festival_ops("tablet-2");
    let m = merge(&a, &b);
    assert_eq!(m.len(), 8);
    for w in m.windows(2) {
        assert!(w[0].lamport < w[1].lamport
            || (w[0].lamport == w[1].lamport && w[0].device <= w[1].device));
    }
}
#[test]
fn full_offline_run_syncs_clean() {
    // sham-017: full festival run with zero network, then sync
    let log = festival_ops("tablet-1");
    let mut map = FestivalMap::new();
    let rep = sync(&mut map, &log.ops);
    assert_eq!(rep.applied, 4);
    assert!(rep.conflicts.is_empty());
    assert_eq!(map.zones[&ZoneId(1)].occupied, 0); // parked then released
    assert_eq!(map.receipts.len(), 4);
}
#[test]
fn sync_conflict_double_park() {
    let mut log = OpLog::new("t1");
    log.push(OpKind::AddZone { id: 1, name: "z".into(), capacity: 10 });
    log.push(OpKind::AddCar { id: 10, plate: "A".into() });
    log.push(OpKind::Park { car: 10, zone: 1 });
    log.push(OpKind::Park { car: 10, zone: 1 }); // conflict: already parked
    let mut map = FestivalMap::new();
    let rep = sync(&mut map, &log.ops);
    assert_eq!(rep.applied, 3);
    assert_eq!(rep.conflicts.len(), 1);
    assert!(rep.conflicts[0].reason.contains("already parked"));
}
#[test]
fn sync_conflict_unknown_car() {
    let mut log = OpLog::new("t1");
    log.push(OpKind::Park { car: 999, zone: 1 });
    let mut map = FestivalMap::new();
    let rep = sync(&mut map, &log.ops);
    assert_eq!(rep.applied, 0);
    assert_eq!(rep.conflicts.len(), 1);
}
#[test]
fn sync_note_always_applies() {
    let mut log = OpLog::new("t1");
    log.push(OpKind::Note { entity: "zone".into(), id: 1, text: "cone down".into() });
    let mut map = FestivalMap::new();
    let rep = sync(&mut map, &log.ops);
    assert_eq!(rep.applied, 1);
}
#[test]
fn two_device_merge_then_sync() {
    let mut a = OpLog::new("tablet-1");
    a.push(OpKind::AddZone { id: 1, name: "z".into(), capacity: 2 });
    a.push(OpKind::AddCar { id: 10, plate: "A".into() });
    let mut b = OpLog::new("tablet-2");
    b.push(OpKind::AddCar { id: 11, plate: "B".into() });
    b.push(OpKind::Park { car: 11, zone: 1 });
    let m = merge(&a, &b);
    let mut map = FestivalMap::new();
    let rep = sync(&mut map, &m);
    assert!(rep.conflicts.is_empty(), "{:?}", rep.conflicts);
    assert_eq!(map.zones[&ZoneId(1)].occupied, 1);
}
#[test]
fn runbook_valid() {
    assert!(check_runbook(&runbook()).is_ok());
}
#[test]
fn runbook_step_count() {
    assert_eq!(runbook().len(), 10);
}
#[test]
fn runbook_broken_numbering_rejected() {
    let mut rb = runbook();
    rb[3].n = 99;
    assert!(check_runbook(&rb).is_err());
}
#[test]
fn runbook_single_binary_first() {
    let rb = runbook();
    assert!(rb[0].command.contains("cargo build --release"));
    assert!(rb[0].verify.contains("shambhala"));
}
#[test]
fn oplog_device_tagged() {
    let log = OpLog::new("radio-3");
    assert_eq!(log.device, "radio-3");
    assert_eq!(log.clock, 0);
}
