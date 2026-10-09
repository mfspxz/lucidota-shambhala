# SHAMBHALA — Coding Spree Summary (Agent 1 of 2)

**Date:** 2026-10-09 ~02:00 PDT
**Scope:** GIS/parking logic — `sham_domain`, `sham_spatial`, `sham_routing`, `sham_layers` (+ minor: `sham_rts`, `sham_vehicles`)
**Baseline:** 348 passed / 0 failed → **Final: 353 passed / 0 failed** (+5 new tests)
**Committed:** NO (per instructions; working tree has uncommitted changes)

## Bugs fixed (P1)

### 1. `import_csv` silently discarded every parsed point — sham_layers
**Severity:** HIGH (data loss). The function parsed CSV rows into `pts`, then did `let _ = pts;` — returning only counts. Callers could never retrieve imported coordinates. The GPX importer already used an out-param pattern.
**Fix:** Signature changed to `import_csv(text: &str, pts: &mut Vec<ImportedPoint>)`, mirroring `import_gpx`. Appends (does not clear), so batch imports compose.
**Tests:** Updated 5 existing tests to new signature; added `csv_appends_to_existing` proving points come back with labels/coords and pre-existing vec contents are preserved.

### 2. A* pruned on node alone, could wrongly report "no route" — sham_routing
**Severity:** HIGH (correctness). Turn feasibility depends on *arrival heading*, but the `best`/`came` maps were keyed on `PointId` only. A cheaper arrival with a heading that blocks the next turn would prune a pricier arrival whose heading permits it → valid route reported as nonexistent.
**Fix:** Search state keyed on `(PointId, heading_sector)` — 8 sectors (45°) + sector 8 for "no heading yet" (start). Reconstruction follows sector-keyed predecessors.
**Tests:** Added `route_heading_state_not_pruned` constructing the exact failure geometry (bus, 90° turn needs 1400cm, 800cm street). Passes with fix.
**AI review:** NVIDIA (gpt-oss-20b) via ai-router reviewed the diff; reconstruction logic, predecessor sector tracking, and start-state handling confirmed coherent. No bugs found.

### 3. `place_angled_stall` accepted 45.9° as 45° — sham_rts
**Severity:** MEDIUM. `angle_deg as u32` truncates floats; 45.9 → 45 → wrongly accepted as Angled45.
**Fix:** Epsilon comparison (`(a - 45.0).abs() < 1e-6`) for the three legal angles.
**Tests:** Added `angled_stall_rejects_near_miss_angle`.

## Performance (P2)

### 4. Simulator O(E) street-endpoint scan per car — sham_routing
`Simulator::tick` did `self.graph.edges.iter().find(|e| e.street == ...)` twice per spawned car. With 500-car simulations over many ticks this dominates.
**Fix:** `street_ends: HashMap<StreetId, (PointId, PointId)>` built once in `Simulator::new`; tick does two hash lookups.

### 5. `StreetGraph::shortest_path` format!-sort per BFS expansion — sham_vehicles
Neighbors were collected then sorted via `format!("{:?}")` per pop — O(n log n) string formatting per expansion, just for deterministic order.
**Fix:** Derived `Ord` on `GNode`/`GEdge`, collect into `BTreeSet` (deterministic dedup, no formatting).

## Gaps completed (P3)

### 6. GeoTIFF ModelTiepoint/ModelPixelScale now parsed — sham_layers
`parse_geotiff` claimed to use tag 33922 but hardcoded origin + 5cm GSD. Now reads tag 33922 (6 doubles) into `RasterInfo::tiepoint` and tag 33550 (3 doubles) into `RasterInfo::pixel_scale`. Model coords are projected (not lat/lon), so they're exposed raw; `origin` keeps the festival default with the limitation documented.
**Tests:** `geotiff_tiepoint_and_scale_parsed` (synthetic TIFF with both tags), `geotiff_without_geotags_still_parses`.

### 7. Clippy: fixed own warning (`nh` never-read initial assignment in route()); pre-existing warnings left untouched.

### 8. Cosmetic: fixed `}        if` brace collapse in `Zone::park` (sham_domain).

## Audit notes (reviewed, intentionally not changed)

- **`FestivalMap::add_*` silent overwrite on duplicate ids:** last-write-wins without a receipt. Left as-is — `sham_offline::sync` relies on idempotent replay and the redteam A6 expectation assumes it. Flagging as a design consideration, not a bug.
- **ODIN service:** only 3 of 196 features execute real kernels; the rest return canned `{"executed":true}`. Out of scope for this pass (would be a large feature build, not a bug fix).
- **`sham_api::parse_form`:** no URL percent-decoding; POST /zones hardcodes `VehicleClass::Car`. Minor; left for a follow-up (outside GIS/parking core).
- **A* optimality:** 8-sector heading keying is a heuristic — two headings in one sector can still differ on borderline turns. Correctness (finding valid routes) is fixed; strict optimality under turn constraints would need continuous-heading state. Documented in code.

## AI usage
- **NVIDIA via ai-router** (`router-call.py`, free lanes): correctness review of the A* sector-keying diff. Verdict from model: reconstruction coherent, no bugs found.
- **DeepSeek-R1:** requested via free lanes; router failover served NVIDIA gpt-oss-20b instead (R1 not currently on free lanes). No api.deepseek.com used ($0 held).

## Files changed (uncommitted)
- `crates/sham_layers/src/lib.rs` — import_csv out-param; TIFF tiepoint/scale
- `crates/sham_layers/src/tests.rs` — 6 tests updated + 2 new
- `crates/sham_routing/src/lib.rs` — sector-keyed A*; street_ends index; clippy fix
- `crates/sham_routing/src/tests.rs` — heading regression test
- `crates/sham_rts/src/lib.rs` — epsilon angle validation
- `crates/sham_rts/src/tests.rs` — near-miss angle test
- `crates/sham_domain/src/lib.rs` — formatting
- `crates/sham_vehicles/src/lib.rs` — Ord derives; BTreeSet neighbors

## Recommended follow-ups (for Agent 2 or next wave)
1. `parse_form` URL-decoding + zone class param in sham_api.
2. Wire more ODIN kernels in sham_odin_svc (zone-suggest, speed-estimate are 10-line wins).
3. `cargo clippy` pre-existing warnings (len/is_empty, new_without_default, etc.).
4. Fuzz `import_gpx` edge cases (unclosed `<wpt` tag path).
