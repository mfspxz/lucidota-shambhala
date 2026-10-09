# SHAMBHALA — Red Team + Research (Agent 2 of 2)

Date: 2026-10-09 ~02:10 PDT. Scope: GIS logic in `crates/sham_spatial`, `sham_layers`, `sham_routing`.
Method: adversarial proof harness (`/tmp/rt2`, path-deps on the crates, executed — not reviewed).
Baseline: `cargo test --workspace` → 348 passed, 0 failed before and after (Agent 1's edits are in the working tree; none of my attacks target modified code paths except where noted).

## RED TEAM — 8 PROVEN findings

All reproduced by executing code. Harness output verbatim where it matters.

### RT2-1 — CSV importer violates RFC 4180 (quoted commas) — PROVEN
`import_csv` splits lines on `,` naively. Input `"Main St, Suite 2",49.20,-122.91` → `imported=1 rejected=1 reasons=["line 2: unparsable numbers"]`.
Real-world CSVs (Excel/Sheets exports) quote fields containing commas. Every such row is silently dropped.
Fix: proper quote-aware field splitting.

### RT2-2 — GPX self-closing `<wpt/>` silently lost — PROVEN
`<wpt lat="49.20" lon="-122.91"/>` followed by a second wpt → `imported=1` (only "B"). The self-closing tag is consumed, its point is never recorded (the parser requires `</wpt>` to advance correctly; the first point vanishes with zero rejection).
Most real GPX exporters (Garmin, GPSBabel, QGIS) emit self-closing wpt elements. This importer loses them.
Fix: handle `/>` termination in the wpt scan loop.

### RT2-3 — GPX single-quoted attributes missed — PROVEN
`<wpt lat='49.20' lon='-122.91'></wpt>` → `imported=0 rejected=1` ("wpt missing lat/lon attributes"). Valid XML, rejected.
Fix: accept both quote styles (or use a real XML reader).

### RT2-4 — Valid RGB GeoTIFF rejected (BitsPerSample offset misread) — PROVEN
When BitsPerSample has count>1 (RGB = 3 SHORTs), the TIFF entry's value field is an OFFSET to the array, not an inline value. The parser reads the offset as the value: `only 8-bit supported, got 200` where 200 was the offset I planted.
Result: every real RGB orthomosaic (the stated sham-021 use case: drone orthomosaics) is rejected.
Fix: when type==SHORT and count>1, follow the offset and read count values.

### RT2-5 — GeoTIFF georeferencing is FABRICATED — PROVEN
The comment says "use ModelTiepoint (tag 33922) if present, else default 0,0" — but the code never reads tag 33922. `origin` is unconditionally `GeoCoord::new(49.20, -122.91)` (festival default) and `pixel_size_m` is hardcoded `0.05`.
**Data integrity impact: any orthomosaic from anywhere on Earth is silently placed at the festival site.** A GIS that misplaces rasters without warning is worse than one that refuses them.
Fix: parse ModelTiepoint (33922) + ModelPixelScale (33550); fail LOUD if absent instead of fabricating.

### RT2-8 — BBox::around has no antimeridian wrap — PROVEN
Center (49.2, 179.99), radius 5000m → box lon (179.921, 180.059). A point at (49.2, −179.99), 1453m away across the antimeridian, returns `contains=false`.
Fix: normalize box/point longitudes into a wrapped comparison when the box crosses ±180.

### RT2-9 — BBox::around underestimates at high latitude — PROVEN
At lat 89.9, `cos(lat).max(0.1)` clamps the divisor so dlon is ~6× too small: a point 900m due east of center falls OUTSIDE a 1000m box.
Fix: use the exact spherical offset (destination-based corners) instead of the equirectangular approximation near poles.

### RT2-13 — UTF-8 BOM breaks CSV header detection — PROVEN
`"\u{FEFF}lat,lon\n..."` → `imported=0 rejected=0 reasons=["missing lat/lon columns"]`. Excel on Windows writes BOM by default; the whole file is rejected as headerless.
Fix: strip BOM before parsing.

## RED TEAM — 5 attacks DEFEATED (fail-closed holds)

- **RT2-6** evil TIFF (IFD offset 0xFFFFFFFF) → `Err`, no panic, no OOM. Bounds checks on every read work.
- **RT2-7** haversine at exact + 20k near-antipodal pairs → exact πR (20015114.44m), zero NaN. The asin-domain worry does not manifest in float64.
- **RT2-10** `GeoCoord::new(NaN|±inf, _)` → rejected. RangeInclusive::contains returns false for NaN — fail-closed by accident of PartialOrd, but it holds.
- **RT2-11** destination() 500km north across the pole → (86.50, 180.0). Correct: pole crossed, longitude flipped.
- **RT2-12** CSV extra columns → silently dropped (attrs always empty). Not a crash vector; informational data-loss note.

Pattern: the parsers are **fail-closed against malicious input but fail-open against legitimate real-world input**. The threat model is wrong — the enemy is Excel, Garmin, and QGIS exports, not attackers.

## RESEARCH — gap analysis vs ArcGIS Pro / QGIS / ATAK-CIV

Sources: web comparison data (atlas.co 2026, G2, Medium/GeoWGS84 2026) + NVIDIA gpt-oss-20b synthesis via ai-router (free lane, $0).

What both ArcGIS and QGIS do that Shambhala does not (operational impact order):

1. **GeoJSON I/O — ABSENT.** `grep -ri geojson crates/` → zero hits. GeoJSON is the lingua franca of web GIS (ArcGIS Online, QGIS, ATAK all speak it). Shambhala imports CSV/GPX only. Highest-leverage single addition.
2. **No projection/CRS support.** Everything is WGS84 lat/lon; no UTM, no Web Mercator, no datum transforms. QGIS/ArcGIS reproject on the fly. Festival drone orthomosaics commonly arrive in UTM.
3. **No spatial predicates beyond bbox.** No buffer/clip/intersect/union (QGIS geoprocessing basics). Zone capacity checks are scalar, not geometric.
4. **No real-time stream ingestion.** Heatmap/flow layers are static snapshots; ArcGIS Velocity / QGIS temporal controller update live. Festival ops need live vehicle positions.
5. **No vector tiles / offline map caching depth.** sham_offline syncs op-logs, not map tiles. ATAK-CIV's documented 2026 crash class is vector-tile catalogs — but the capability itself is table stakes for field use.
6. **Single-user, single-process.** No multi-user editing/versioning (ArcGIS enterprise geodatabase). Offline merge handles 2-device conflicts, not N-team ops.
7. **No 3D.** Multi-level parking structures can't be modeled. (ArcGIS's clearest edge per every 2026 comparison.)
8. **Auth is bearer-token only.** No RBAC, no per-role scopes. Field staff vs ops lead see the same API.
9. **Routing is static A\*.** No live closures, no traffic weighting, no turn-by-turn regeneration. ArcGIS Network Analyst / QGIS pgRouting territory.
10. **Web UI is a 32-line stub.** No drag-drop map editing, labeling, or print layouts (QGIS's cartography core).

Where Shambhala is STRONG vs the incumbents (keep these):
- Deterministic, zero-dependency Rust (no PROJ/GDAL version hell; ATAK's plugin-pinning crashes are the counterexample).
- Offline-first op-log sync with explicit conflict surfacing (better than silent last-write-wins).
- Turning-radius + width-constrained routing is domain-correct for pedicabs/festival vehicles — ArcGIS would need custom network attributes for this.
- 348 tests green, fail-closed red-team posture, std-only (auditable supply chain).

## DATA INTEGRITY

- Tree is a **RECONSTRUCTION** (PRESERVATION.md: 13 crates as found vs ~29 in the recovery dossier; "not the original bytes"). Epistemic status carries to all findings above.
- No data fixtures in tree (no .geojson/.tif/.csv/.gpx under crates/) — importers are tested only against synthetic inputs, which is exactly why RT2-1..RT2-4 survived.
- `git status`: Agent 1 has modified `sham_layers/{lib,tests}.rs` and `sham_routing/{lib,tests}.rs` (uncommitted). I touched no crate files; this report is the only file I added.
- Harness: `/tmp/rt2` (ephemeral, path-deps, release build, offline). Rerun: `cargo run --offline --release` in `/tmp/rt2`.

## RECOMMENDED FIX ORDER (Agent 1 / next wave)

1. RT2-5 tiepoint parsing (silent misplacement is the worst failure mode here) + fail loud when absent.
2. RT2-4 BitsPerSample offset-following (unblocks all RGB orthomosaics).
3. RT2-1 quote-aware CSV + RT2-13 BOM strip (one parser pass).
4. RT2-2 self-closing wpt + RT2-3 quote styles (one parser pass).
5. RT2-8 antimeridian + RT2-9 polar bbox (one BBox rewrite).
6. New capability: GeoJSON import (research gap #1) — highest leverage vs incumbents.

No commits made. No DeepSeek API used ($0 preserved). One NVIDIA router call (644 tokens, free lane).
