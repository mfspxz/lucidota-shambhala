use super::*;

#[test]
fn campaign_runs_8_attacks() {
    assert_eq!(campaign().len(), 8);
}
#[test]
fn campaign_all_defeated() {
    let f = campaign();
    for x in &f {
        assert_eq!(x.verdict, Verdict::Defeated, "OPEN FINDING: {}", x.attack);
    }
    assert!(campaign_passes(&f));
}
#[test]
fn campaign_passes_false_with_open_finding() {
    let mut f = campaign();
    f[0].verdict = Verdict::Finding;
    assert!(!campaign_passes(&f));
}
#[test]
fn findings_carry_details() {
    for x in campaign() {
        assert!(!x.attack.is_empty() && !x.detail.is_empty());
    }
}
