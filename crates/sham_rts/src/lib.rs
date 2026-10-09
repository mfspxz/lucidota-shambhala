// sham_rts — sham-011 (RTS selection/painting UI), sham-016 (volunteer
// interaction model), sham-023 (double-tap angle parking), sham-029
// (volunteer training mode).
// Headless gesture->intent->command state machine: the testable core of the
// fat-thumb volunteer UI. Pixel rendering is a thin layer over this.
#![forbid(unsafe_code)]

#[cfg(test)]
mod tests;

use sham_domain::{ParkOrientation, Receipt, ZoneId};
use sham_spatial::{BBox, GeoCoord};
use std::collections::HashSet;

/// Fat-thumb touch slop radius in meters (sham-016: forgiving targets).
pub const TOUCH_SLOP_M: f64 = 12.0;
/// Tired-user protocol: bigger slop after long shifts (sham-016).
pub const TIRED_SLOP_M: f64 = 25.0;
/// Double-tap max interval (ms) and max drift (m).
pub const DOUBLE_TAP_MS: u64 = 400;
pub const DOUBLE_TAP_DRIFT_M: f64 = 15.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Touch {
    pub at: GeoCoord,
    pub t_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Gesture {
    Tap(Touch),
    DoubleTap(Touch, Touch),
    Drag { from: Touch, to: Touch },
    LongPress(Touch),
    Paint(Vec<Touch>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EntityKind {
    Car,
    Zone,
    Street,
    Point,
    Flow,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct EntityRef {
    pub kind: EntityKind,
    pub id: u64,
}

/// Classified selection with volunteer-friendly confidence.
#[derive(Clone, Debug, PartialEq)]
pub struct Classification {
    pub kind: EntityKind,
    pub confidence: f32, // 0..1
}

/// UI state machine.
#[derive(Default)]
pub struct RtsUi {
    pub selection: HashSet<EntityRef>,
    pub painted_cells: Vec<BBox>,
    pub tired_mode: bool,
    pub pending_tap: Option<Touch>,
    pub undo_stack: Vec<String>,
    pub receipts: Vec<Receipt>,
}
impl RtsUi {
    pub fn new() -> Self {
        Self::default()
    }
    fn slop(&self) -> f64 {
        if self.tired_mode { TIRED_SLOP_M } else { TOUCH_SLOP_M }
    }
    fn log(&mut self, r: Receipt) {
        self.receipts.push(r);
    }
    /// Feed a raw touch; returns any recognized gesture.
    pub fn touch(&mut self, t: Touch) -> Option<Gesture> {
        if let Some(prev) = self.pending_tap {
            let dt = t.t_ms.saturating_sub(prev.t_ms);
            if dt <= DOUBLE_TAP_MS && prev.at.distance_m(t.at) <= DOUBLE_TAP_DRIFT_M {
                self.pending_tap = None;
                return Some(Gesture::DoubleTap(prev, t));
            }
        }
        self.pending_tap = Some(t);
        None
    }
    /// Flush a pending single tap (called when tap timeout expires).
    pub fn flush_tap(&mut self) -> Option<Gesture> {
        self.pending_tap.take().map(Gesture::Tap)
    }
    pub fn drag(&mut self, from: Touch, to: Touch) -> Gesture {
        self.pending_tap = None;
        Gesture::Drag { from, to }
    }
    /// Select entities whose coords fall in the drag box (fat-thumb: expand by slop).
    pub fn select_drag(
        &mut self,
        from: GeoCoord,
        to: GeoCoord,
        entities: &[(EntityRef, GeoCoord)],
    ) -> usize {
        let (mut min_lat, mut max_lat) = (from.lat.min(to.lat), from.lat.max(to.lat));
        let (mut min_lon, mut max_lon) = (from.lon.min(to.lon), from.lon.max(to.lon));
        // expand by slop in degrees (approx)
        let pad = self.slop() / 111_320.0;
        min_lat -= pad;
        max_lat += pad;
        min_lon -= pad;
        max_lon += pad;
        self.selection.clear();
        for (e, c) in entities {
            if (min_lat..=max_lat).contains(&c.lat) && (min_lon..=max_lon).contains(&c.lon) {
                self.selection.insert(e.clone());
            }
        }
        self.log(Receipt::new(
            "select",
            "ui",
            0,
            &format!("{} selected", self.selection.len()),
        ));
        self.selection.len()
    }
    /// Paint mode: accumulate cells; returns cell count.
    pub fn paint(&mut self, cells: Vec<BBox>) -> usize {
        let n = cells.len();
        self.painted_cells.extend(cells);
        self.log(Receipt::new("paint", "ui", 0, &format!("{} cells", n)));
        n
    }
    /// Classify current selection: majority kind wins; confidence = share.
    pub fn classify(&self) -> Option<Classification> {
        if self.selection.is_empty() {
            return None;
        }
        let mut counts: std::collections::HashMap<EntityKind, usize> =
            std::collections::HashMap::new();
        for e in &self.selection {
            *counts.entry(e.kind).or_default() += 1;
        }
        let (&kind, &n) = counts.iter().max_by_key(|(_, &c)| c).unwrap();
        Some(Classification {
            kind,
            confidence: n as f32 / self.selection.len() as f32,
        })
    }
    /// Tired-user protocol (sham-016): destructive ops need explicit confirm.
    pub fn set_tired(&mut self, tired: bool) -> Receipt {
        self.tired_mode = tired;
        let r = Receipt::new("tired_mode", "ui", 0, &format!("tired={}", tired));
        self.log(r.clone());
        r
    }
    /// Undo last op; fails closed when stack empty.
    pub fn undo(&mut self) -> Result<String, String> {
        self.undo_stack.pop().ok_or_else(|| "nothing to undo".to_string())
    }
    pub fn push_undo(&mut self, label: &str) {
        self.undo_stack.push(label.into());
    }
}

// ---------------- sham-023: double-tap angle parking ----------------

/// Result of placing an angled stall via double-tap.
#[derive(Clone, Debug, PartialEq)]
pub struct StallPlacement {
    pub zone: ZoneId,
    pub center: GeoCoord,
    pub angle_deg: f64,
    pub curb_cm: f64,
    pub orientation: ParkOrientation,
}

/// Place an angled parking stall from a double-tap gesture.
/// Constraint-checked: angle in {45,60,90}, stall fits zone curb budget.
pub fn place_angled_stall(
    tap: &Touch,
    zone: ZoneId,
    angle_deg: f64,
    stall_width_cm: u32,
    curb_available_cm: f64,
) -> Result<StallPlacement, String> {
    let orientation = if (angle_deg - 45.0).abs() < 1e-6 {
        ParkOrientation::Angled45
    } else if (angle_deg - 60.0).abs() < 1e-6 {
        ParkOrientation::Angled60
    } else if (angle_deg - 90.0).abs() < 1e-6 {
        ParkOrientation::Perpendicular
    } else {
        return Err(format!(
            "angle {} not supported; use 45, 60, or 90",
            angle_deg
        ));
    };
    if stall_width_cm < 200 || stall_width_cm > 400 {
        return Err(format!("stall width {}cm out of range 200-400", stall_width_cm));
    }
    let curb = sham_spatial::angled_stall_curb_cm(stall_width_cm, angle_deg);
    if curb > curb_available_cm {
        return Err(format!(
            "stall needs {:.0}cm curb, only {:.0} available",
            curb, curb_available_cm
        ));
    }
    Ok(StallPlacement {
        zone,
        center: tap.at,
        angle_deg,
        curb_cm: curb,
        orientation,
    })
}

// ---------------- sham-029: volunteer training mode ----------------

/// One tutorial step.
#[derive(Clone, Debug, PartialEq)]
pub struct TutorialStep {
    pub id: &'static str,
    pub title: &'static str,
    pub instruction: &'static str,
    /// max seconds before the step is marked "needs help"
    pub budget_s: u64,
}

/// Guided tutorial: 10-minute onboarding (sham-029).
pub struct TrainingMode {
    pub steps: Vec<TutorialStep>,
    pub done: Vec<&'static str>,
    pub started_ms: u64,
    pub budget_ms: u64,
}
impl TrainingMode {
    pub fn standard() -> Self {
        Self {
            steps: vec![
                TutorialStep { id: "tap", title: "Tap to select", instruction: "Tap a car on the map to select it.", budget_s: 60 },
                TutorialStep { id: "drag", title: "Drag to multi-select", instruction: "Drag a box around several cars.", budget_s: 90 },
                TutorialStep { id: "paint", title: "Paint a zone", instruction: "Paint over the zone to mark it full.", budget_s: 90 },
                TutorialStep { id: "doubletap", title: "Double-tap parking", instruction: "Double-tap to place an angled stall.", budget_s: 90 },
                TutorialStep { id: "dispatch", title: "Dispatch a car", instruction: "Assign the selected car to a zone.", budget_s: 120 },
                TutorialStep { id: "undo", title: "Undo mistakes", instruction: "Shake-tap undo to revert the last action.", budget_s: 60 },
                TutorialStep { id: "tired", title: "Tired mode", instruction: "Enable tired mode for bigger touch targets.", budget_s: 60 },
                TutorialStep { id: "offline", title: "Go offline", instruction: "Work a full scenario with no network.", budget_s: 30 },
            ],
            done: Vec::new(),
            started_ms: 0,
            budget_ms: 600_000, // 10 minutes
        }
    }
    pub fn complete_step(&mut self, id: &'static str, now_ms: u64) -> Result<bool, String> {
        if !self.steps.iter().any(|s| s.id == id) {
            return Err(format!("unknown step {}", id));
        }
        if self.done.contains(&id) {
            return Err(format!("step {} already done", id));
        }
        if now_ms.saturating_sub(self.started_ms) > self.budget_ms {
            return Err("training budget (10 min) exceeded".into());
        }
        self.done.push(id);
        Ok(self.done.len() == self.steps.len())
    }
    pub fn progress_pct(&self) -> f64 {
        self.done.len() as f64 / self.steps.len() as f64 * 100.0
    }
    pub fn remaining_ms(&self, now_ms: u64) -> u64 {
        self.budget_ms.saturating_sub(now_ms.saturating_sub(self.started_ms))
    }
}
