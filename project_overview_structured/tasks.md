# Tasks Backlog

This backlog expands the roadmap into concrete tasks that can be tracked and executed by the team.

## Phase 1 – Core CLI (Weeks 1–6)

- [ ] Initialize Rust workspace and crate structure.
- [ ] Add core dependencies (`gimli`, `object`, `clap`, `serde`, `memmap2`, `comfy-table`).
- [ ] Implement a `StructProvider` abstraction over ELF/Mach-O/PE.
- [ ] Implement binary loading and DWARF context initialization.
- [ ] Implement iteration over Compilation Units and struct-like DIEs.
- [ ] Implement type resolution for member sizes and names.
- [ ] Implement simple `DW_AT_data_member_location` handling (constant offsets).
- [ ] Implement padding detection between members and tail padding.
- [ ] Implement cache-line analysis with configurable line size.
- [ ] Implement "Inspector" CLI command and colorized table output.
- [ ] Implement JSON output schema for individual builds.
- [ ] Add basic logging and error reporting.
- [ ] Create initial README and usage examples.

## Phase 2 – Advanced Analysis & Diffing (Weeks 7–10)

- [ ] Implement DWARF expression evaluator for complex `DW_AT_data_member_location`.
- [ ] Implement bitfield handling for DWARF 4 (`DW_AT_bit_offset`) and DWARF 5 (`DW_AT_data_bit_offset`).
- [ ] Add automated tests for tricky DWARF constructs and malformed inputs.
- [ ] Implement struct diffing:
  - [ ] Matching by fully-qualified name.
  - [ ] Heuristic rename detection via member overlap.
  - [ ] Per-member added/removed/changed reporting.
- [ ] Implement CI mode (`--ci`, `--fail-on-growth`).
- [ ] Design and implement `.struct-audit.yaml` config schema.
- [ ] Support per-struct budgets and global padding/size budgets.
- [ ] Provide machine-readable diff JSON for CI consumption.
- [ ] Ship example CI configs (GitHub Actions, GitLab CI, etc.).

## Phase 3 – SaaS Platform MVP (Weeks 11–16)

- [ ] Design API contract between CLI and SaaS (finalize JSON schema).
- [ ] Implement ingestion API (`POST /api/reports`) in Rust (Axum) or Go (Gin).
- [ ] Design and migrate Postgres schema:
  - [ ] Repositories.
  - [ ] Commits / Builds.
  - [ ] Structs.
  - [ ] Snapshots.
- [ ] Implement layout-hash deduplication to avoid storing identical layouts repeatedly.
- [ ] Implement GitHub App for auth and repo linking.
- [ ] Implement status check integration for PRs.
- [ ] Build a minimal dashboard (Next.js or similar):
  - [ ] Repository overview page.
  - [ ] Per-struct history and sparkline.
  - [ ] Padding and size trend charts.
  - [ ] Basic binary heatmap by packing density.
- [ ] Implement configuration UI for budgets and alerts.

## Phase 4 – Advanced Capabilities

- [ ] Prototype false sharing detection for atomic types:
  - [ ] Detect atomics sharing a cache line.
  - [ ] Surface high-risk patterns in reports.
- [ ] Prototype automatic layout suggestions:
  - [ ] Implement bin-packing heuristic based on member sizes and alignments.
  - [ ] Provide "suggested order" alongside current order.
- [ ] Integrate LTO-specific analysis paths:
  - [ ] Detect LTO builds.
  - [ ] Compare pre- and post-LTO layouts for sanity check.

## Business & GTM Tasks

- [ ] Finalize pricing tiers (Community, Pro, Enterprise) and feature mapping.
- [ ] Draft security and privacy docs (hashing mode, data retention, encryption).
- [ ] Prepare HFT-focused and embedded-focused value-proposition one-pagers.
- [ ] Collect case-study–style benchmarks highlighting real-world wins (RAM saved, latency reductions).
- [ ] Produce onboarding guides for CLI, CI integration, and SaaS setup.
- [ ] Define KPIs for product usage (e.g., number of repos, daily reports, prevented regressions).

