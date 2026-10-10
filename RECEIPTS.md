# Shambhala slice — build receipts

## PROVENANCE (R-8, 2026-10-10): honestly-unsigned

This file and any per-run receipts in this repo are **structured build
evidence, not signed provenance**. They record measured test counts, the
crate that ran them, and UTC timestamps. Verification power: reproduction —
run `cargo test --workspace` against the recorded commit and the numbers
should match. There is no signature, no commit-hash binding in the
receipts, and no authority attestation. Nothing here should be read as
"cryptographically proven". (The sham_api `Bearer <redacted>` auth is a live
service control, not receipt provenance; its bounds are documented in
AUDIT-APPLICATION.md, R-2.)

Built 2026-10-06/07 (UTC) by lucidota-subagent, Rust-first, std-only (zero external crates, $0).
Workspace: `~/workspace/shambhala` — 13 crates, single `cargo test --workspace` run.

## Test checkpoint (sham-026): 348 passed, 0 failed, 0 warnings

| Crate | Items | Tests |
|---|---|---|
| sham_domain | sham-001 domain model (cars/streets/zones/flows) | 63 |
| sham_spatial | sham-002 spatial engine (haversine, bearings, turns, property tests) | 36 |
| sham_odin | sham-003–010 ODIN batches (196 features registered, kernels) | 63 |
| sham_routing | sham-013 routing (A* + turn/width constraints), sham-024 flows, sham-025 500-car sim | 48 |
| sham_rts | sham-011 RTS UI state machine, sham-016 tired-user, sham-023 angle parking, sham-029 training | 36 |
| sham_layers | sham-012 layer stack, sham-021 GeoTIFF import, sham-022 CSV/GPX import | 29 |
| sham_vehicles | sham-014 vehicle records, sham-015 street/point graph DB | 21 |
| sham_api | sham-018 REST :8080, OpenAPI, bearer auth | 16 |
| sham_webui | sham-019 web UI :8088, fat-thumb 64px, offline queue, no CDN | 8 |
| sham_odin_svc | sham-020 ODIN :8090 wired to API | 8 |
| sham_offline | sham-017 offline op-log + sync, sham-028 deploy runbook | 12 |
| sham_organelles | sham-030 4 organelles, native/REST equivalence | 4 |
| sham_redteam | sham-027 8-attack campaign, all defeated | 4 |
| **Total** | **sham-001…sham-030 (30/30)** | **348** |

## Key receipts
- ODIN registry: exactly 196 features (25×7 + 21), integrity-checked, ids sequential per batch.
- 500-car ingress simulation: no deadlock, deterministic per seed.
- Turn-constraint calibration: conservative swept-path model `r·(1−cos θ)+200cm` → 90° turns need car 750 / truck 1100 / RV 1200 / bus 1400cm; fixtures use realistic festival widths.
- Offline sync: conflicts recorded, never silently applied; double-logging bug found+fixed.
- A* predecessor-overwrite-on-relax bug found+fixed during build.
- Red-team: 8 attacks (double-park, overflow, GPS spoof, oversized routing, auth bypass, offline double-book, ODIN injection, coord smuggling) — all defeated, zero open findings.
- Ports per dossier: API :8080, UI :8088, ODIN :8090.
- D1 freeze directive (2026-10-07): slice unaffected — zero D1 reads/writes in this build.

## Bugs found and fixed during build (honest log)
1. `can_make_turn` motorcycle test expectation vs model boundary (test fixed to 500cm boundary).
2. A* `came` map set on first discovery instead of on relax (code fixed).
3. Vacuous width check in `neighbors` → class-aware `max(200, turning_radius/4)` (code fixed).
4. TIFF SHORT-inline endianness normalization (code fixed); hand-rolled BE test TIFF.
5. `sync()` double-logging receipts for Park/Release (code fixed).
6. Closure names shadowing `u16`/`u32` types; `ok_or` type mismatch; temp borrow in CSV import (code fixed).
