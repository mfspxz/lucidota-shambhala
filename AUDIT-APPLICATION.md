# Audit Application — shambhala

- **Repo:** `~/workspace/shambhala`
- **Date:** 2026-10-10
- **Auditor:** brutal-audit rollout slice 7/8 (parent orchestrator)
- **Workspace:** 13 crates — `sham_domain`, `sham_spatial`, `sham_odin`, `sham_routing`, `sham_rts`, `sham_layers`, `sham_vehicles`, `sham_api`, `sham_webui`, `sham_odin_svc`, `sham_offline`, `sham_organelles`, `sham_redteam`
- **Basis:** `RECEIPTS.md`, `SPREE-REDTEAM-A2.md`, repo state recorded at the 2026-10-07 close-shit commit, and the 2026-10-10 `RECEIPTS.md` PROVENANCE edit.

---

## Disposition table

| R# | Finding | Disposition | Verdict |
|----|---------|-------------|---------|
| R-1 | Fix-verification claims instance-only until re-attack signs off; claims and defeats in ONE book; no PROVEN/FIXED without re-attack result | Policy stated in this document; no PROVEN/FIXED is asserted for RT attack class scope | UNKNOWN for class scope; PROVEN for exact instance inputs |
| R-2 | Honest bounds on every guardrail; freeze hardening past bound | Bounds stated for `sham_api` Bearer auth and RT2-10 accidental `PartialOrd` rejection; dedicated bounds doc queued | UNKNOWN for repo-level bounds doc; PROVEN for bounds stated here |
| R-3 | Hash-chained log needs out-of-band head copy | N/A — no hash-chained log exists | N/A |
| R-4 | Triage backlogs; standing HIGHs get wont-fix with complexity budget or fix | No backlog doc exists; no standing HIGHs identified; wont-fix list empty; bounds doc queued as follow-up | UNKNOWN for backlog doc existence; PROVEN for no standing HIGHs in this pass |
| R-5 | Signature/denylist/strip games best-effort with FIXED complexity budget; redirect to structural answers | N/A | N/A |
| R-6 | No detector without named consumer + enforcement gate | N/A as deployed control; `sham_redteam` is a test harness, consumer = SPREE report, gate = recorded test run | N/A |
| R-7 | Separate property tests from change-detection tests; never rewrite property test expectation without adversarial sign-off | Policy documented; full-suite labeling unknown | UNKNOWN for labeling; PROVEN for policy statement |
| R-8 | No unsigned provenance that invites misreading | `RECEIPTS.md` PROVENANCE section added 2026-10-10; honestly-unsigned, verification power stated plainly | PROVEN |
| R-9 | Claim logs: frozen schema, SUPERSEDED-BY-BYPASS status | N/A — no claim log exists | N/A |
| R-10 | Record-keeping in same motion as event | `RECEIPTS.md` written same motion as build (2026-10-06/07) | PROVEN |
| R-11 | Explicit UNKNOWN list; no “probably” as PROVEN | UNKNOWN list present in this document | PROVEN |
| R-12 | No unsourced oral footnotes | `RECEIPTS.md` and SPREE report cite measured counts and specific attack IDs | PROVEN |

---

## R-1: Instance-only defeat claims

- `SPREE-REDTEAM-A2.md` reports **“RED TEAM — 5 attacks DEFEATED (fail-closed holds)”**.
- These defeats are **instance-only**: verified for the exact inputs listed in the report.
- No dedicated re-attack wave with new variants has signed off.
- Therefore this application asserts **no PROVEN/FIXED** for class-level defense against the five attack families.
- Claims and defeats remain in **one book** — the SPREE report. This document does not re-litigate them.

**Verdict:** UNKNOWN for class scope; PROVEN for exact instance inputs as recorded in `SPREE-REDTEAM-A2.md`.

---

## R-2: Guardrail bounds; freeze hardening past bound

- **`sham_api` Bearer `<redacted>` auth**
  - Catches: unauthenticated callers when the token is required.
  - Cannot catch: caller dishonesty after authentication. It authenticates the caller only; it says nothing about caller intent.
  - Bound: no claim beyond caller identity.
- **`sham_redteam` RT2-10 — `GeoCoord::new(NaN|inf)` rejection**
  - Holds **“by accident of PartialOrd”** — not by explicit numeric-safety design.
  - Bound: do not assert this as a designed invariant; it is an accidental fail-closed property.
- **Freeze:** no hardening past these bounds until a dedicated bounds doc is committed.
- **Queued follow-up:** dedicated bounds document for `sham_api` auth and `sham_redteam` gates.

**Verdict:** UNKNOWN for repo-level bound documentation; PROVEN for the bound statements made in this section.

---

## R-3: Hash-chained log tamper-evidence

- N/A — no hash-chained log exists in the workspace.
- No out-of-band head copy requirement applies.
- No tamper-evident claim is made.

**Verdict:** N/A.

---

## R-4: Backlog triage; standing HIGHs

- No backlog document exists in the repo.
- No standing HIGHs were identified in this pass.
- Wont-fix list is empty.
- Queued follow-up: dedicated bounds doc for R-2. This is not a standing HIGH.

**Verdict:** UNKNOWN for backlog doc existence; PROVEN that no standing HIGHs were found in this pass.

---

## R-5: Signature/denylist/strip games

- N/A — no signature, denylist, or strip-game machinery is under audit in this application.

**Verdict:** N/A.

---

## R-6: Detector consumer and enforcement gate

- `sham_redteam` is a **test harness**, not a deployed runtime detector.
- Named consumer: `SPREE-REDTEAM-A2.md` / audit pass.
- Enforcement gate: recorded test run through `cargo test --workspace`.
- N/A as a runtime control.

**Verdict:** N/A.

---

## R-7: Property tests vs change-detection tests

- Test count recorded: **353/353** per the 2026-10-07 commit.
- Full-suite labeling of property tests vs change-detection tests is **UNKNOWN**.
- Policy adopted:
  - `sham_redteam` fail-closed tests are **property tests**.
  - Their expectations must **never be rewritten to match behavior changes** without adversarial sign-off.
- No property-test expectation rewrite is authorized by this application.

**Verdict:** UNKNOWN for full-suite labeling; PROVEN for policy statement.

---

## R-8: Provenance

- `RECEIPTS.md` gained a **PROVENANCE section** today, 2026-10-10.
- The receipts are declared **honestly-unsigned**.
- Verification power is stated plainly:
  - Reproduction via `cargo test --workspace` against the recorded commit.
  - No signature.
  - No commit-hash binding in the receipts.
  - The Bearer `<redacted>` auth is a live service control, not receipt provenance.
- Conformant with R-8.

**Verdict:** PROVEN.

---

## R-9: Claim logs

- N/A — no claim log exists in the workspace.

**Verdict:** N/A.

---

## R-10: Record-keeping timing

- `RECEIPTS.md` was written in the same motion as the build (2026-10-06/07).
- No archival-only backfill is claimed.

**Verdict:** PROVEN.

---

## R-11: Explicit UNKNOWN list

- This application contains an explicit UNKNOWN list.
- No “probably” is used as PROVEN.
- See the UNKNOWN list below.

**Verdict:** PROVEN.

---

## R-12: Sourced evidence

- `RECEIPTS.md` cites measured test counts: **348 tests** at checkpoint `sham-026`, **30/30 items built**.
- `SPREE-REDTEAM-A2.md` cites specific attack IDs, including RT2-10.
- No unsourced oral footnotes are used.

**Verdict:** PROVEN.

---

## Changes made today

- `RECEIPTS.md`: added PROVENANCE section (R-8).
  - Declares receipts honestly-unsigned.
  - States verification power plainly: reproduction via `cargo test --workspace` against recorded commit; no signature; no commit-hash binding in receipts; Bearer `<redacted>` auth is a live service control, not receipt provenance.
- No other files changed in this pass.

---

## Commits

- Commit for `RECEIPTS.md` PROVENANCE edit + AUDIT-APPLICATION.md: `909f371`
- Commit for 2026-10-07 close-shit test counts (353/353): `bdc433f` (pre-existing, recorded-not-rerun)
- Commit for `RECEIPTS.md` build receipt: **TBD**

No commit hashes are asserted by this application. Verify independently.

---

## Test counts

- **353/353 tests** per the 2026-10-07 close-shit commit — **NOT re-run in this pass**.
  - Label: **recorded-not-rerun**.
- `RECEIPTS.md` table records **348 tests** at checkpoint `sham-026` (2026-10-06/07).
- No current `cargo test --workspace` run is claimed by this application.

---

## UNKNOWN list

- **Class scope of the 5 defeated attacks:** exact-input defeats are recorded in `SPREE-REDTEAM-A2.md`; whether the underlying guards hold for variant/fuzz classes is UNKNOWN until a re-attack wave signs off.
- **Bearer `<redacted>` key management/rotation state:** UNKNOWN — no evidence reviewed in this pass.
- **Whether the tree has moved since the SPREE report:** UNKNOWN — this application does not assert a commit hash for the SPREE state.
- **Property vs change-detection labeling of the full 353-test suite:** UNKNOWN.

---

## Wont-fix / backlog triage

- **Standing HIGHs:** none.
- **Wont-fix list:** none.
- **Queued follow-up (not wont-fix):** dedicated bounds doc for `sham_api` auth and `sham_redteam` gates (R-2).
