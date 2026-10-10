# shambhala — organelle mapping decisions

Wave 4b-1, corrected doctrine: decompose ACTUAL capabilities. The repo
already had an organelle concept (`sham_organelles`, sham-030: 4 declared
organelles with Native/Rest behavioral equivalence) — verified against the
v1 standard and re-expressed on it (codons, capability contracts,
hash-chained receipts, kill semantics), not rebuilt.

## Capability units found

| Unit | Source | Ops | Decision |
|---|---|---|---|
| sham-route | `sham_organelles` (was `sham.route`) + `sham_spatial::GeoCoord` | route | **Bespoke** (`sr`). A→B distance. Considered ref `geo-point` (`gp`): the ref is a point primitive; this unit is the declared routing entry of the organelle set (class-aware contract). Kept as the set's own unit, Proven. |
| sham-zone | `sham_organelles` (was `sham.suggest-zone`) + `sham_odin::suggest_zone` | suggest_zone | **Bespoke** (`sz`). Best-zone dispatch for a vehicle class (ODIN-DIS-010). No ref organelle does dispatch. Proven. |
| sham-ping | `sham_organelles` (was `sham.validate-ping`) + `sham_odin::validate_ping` | validate | **Bespoke** (`vn`). GPS ping validation (bounds + timestamp sanity, ODIN-TRK-001). Domain validation, not a point primitive. Proven. |
| sham-eta | `sham_organelles` (was `sham.eta`) + `sham_odin::eta_minutes` | eta | **Bespoke** (`et`). ETA from distance/speed (ODIN-DIS-009). Proven. |
| sham-astar | `sham_routing::RoadGraph::route` (A*) | route | **Bespoke** (`sa`). Real A* over a street graph with width/one-way/vehicle-class constraints. NOTE: the old `sham.route` description claimed turning-radius/road-width constraints but its implementation was great-circle distance only — the A* in `sham_routing` is the genuine constrained router. Separate unit, Proven (deterministic). |
| sham-offline | `sham_offline::{OpLog, merge, runbook}` | push, ops, len, merge, runbook | **Bespoke** (`so`). Offline-first op log with lamport clock + deterministic merge. Not kv-store: the computation is causal ordering + merge. Candidate (stateful). |
| sham-redteam | `sham_redteam::campaign` | campaign, passes | **Bespoke** (`st`). Executable adversarial campaign with fail-closed verdicts. Included (unlike a timing harness): its output is security verdicts, load-bearing for the repo's attack-the-architecture doctrine. Candidate. |
| sham-gesture | `sham_rts::RtsUi` (touch/flush_tap/drag/paint) | touch, flush_tap, drag, paint | **Bespoke** (`sg`). Touch→gesture state machine. Candidate (stateful recognizer). |
| sham-layers | `sham_layers::LayerStack` | add, toggle, set_opacity, render_order, by_kind | **Bespoke** (`ly`). Map layer stack management. Candidate. |
| sham-odin-svc | `sham_odin_svc::OdinService::execute` | execute, features_in | **Bespoke** (`od`). ODIN feature-request execution with receipted outcomes; unknown features rejected fail-closed (proven in tests). Candidate. |
| sham-vehicles | `sham_vehicles::VehicleRegistry` | add, relate, related, count | **Bespoke** (`vh`). Fleet registry with receipted relations. Candidate. |

## Deliberate exclusions

- `sham_domain`: types + id newtypes + RNG. Type library, not a capability.
- `sham_spatial`: GeoCoord/BBox math. Library consumed by the route
  organelles, not independently exposed (its distance/bearing/destination
  genuinely match ref `geo-point` — noted, no separate organelle needed).
- `sham_api`: HTTP request/response constructors — the REST surface for the
  organelles, not computation.
- `sham_webui`: static file serving helpers — web surface.

## Reference-organelle check (all 12 considered, none adopted)

- `ref-geo-point`: closest near-miss (sham_spatial math). The declared units
  are dispatch/routing/validation workflows, not point primitives — bespoke.
- `ref-kv-store`: the offline op log and layer stack are domain-typed state
  machines, not generic kv.
- `ref-ledger-append`: vehicle relations carry receipts but are a fleet
  registry, not an append log.
- Others (`http-fetch`, `text-transform`, `json-validate`, `hash-seal`,
  `time-oracle`, `scrape-extract`, `model-call`, `notify`, `file-cas`):
  no overlap.
- Verdict: 11 bespoke organelles, 0 adoptions.

## Execution facts

- 5 Proven (pure deterministic: route, zone, ping, eta, astar),
  6 Candidate (stateful or verdict-producing).
- Write units carry `sham.*` capabilities; host fail-closes without them.
- Existing tests untouched and green; new organelle tests prove
  register → call → verify_chain per unit.
- `cargo test --workspace`: all suites green.
