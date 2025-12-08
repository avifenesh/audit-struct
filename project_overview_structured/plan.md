# High-Level Plan

This plan summarizes the main implementation and productization phases described in `project_overview.md`, with a focus on delivering a high-quality product incrementally.

## Phase 1: Core CLI (Weeks 1–6)

Objective:

- Deliver a robust, developer-friendly CLI capable of parsing DWARF and emitting accurate struct layouts.

Key outcomes:

- A single `struct-audit` binary that:
  - Accepts a binary path as input.
  - Prints a detailed layout table for structs (offsets, sizes, padding).
  - Supports JSON output for automation.

Highlights:

- Project scaffolding and dependency setup.
- DWARF abstraction layer (via `gimli` and `object`).
- Basic struct parsing and member resolution.
- Minimal DWARF expression evaluation for member locations.
- "Inspector" view with colorized terminal output.

## Phase 2: Advanced Analysis & Diffing (Weeks 7–10)

Objective:

- Handle edge cases and provide diffing and CI-ready features.

Key outcomes:

- Reliable handling of:
  - DWARF 4 and 5 bitfield representations.
  - Complex member location expressions.
- Diff engine that:
  - Compares two binaries and produces a `DiffReport`.
  - Detects struct renames via member-similarity heuristics.
- CI mode with:
  - `--fail-on-growth` and configuration via `.struct-audit.yaml`.
  - Struct- and repo-level budgets.

## Phase 3: SaaS Platform MVP (Weeks 11–16)

Objective:

- Launch a minimal but functional web platform that stores and visualizes struct layout metrics over time.

Key outcomes:

- Backend API:
  - Ingestion endpoint for CLI JSON reports.
  - Postgres-based schema for repos, commits, structs, and snapshots.
- GitHub App integration:
  - Authentication and repo linking.
  - Status checks posted back to PRs based on budgets.
- Frontend dashboard:
  - Time-series charts of padding and size metrics.
  - Per-struct history and sparkline views.
  - Binary heatmap visualization by packing density (even if initially basic).

## Phase 4: Advanced Capabilities (Post-MVP)

Objective:

- Evolve struct-audit from a passive analyzer into an active optimization assistant.

Candidate features:

- False sharing detection based on atomic types and cache-line locations.
- Automatic layout suggestions (bin-packing based) to reduce padding.
- LTO-aware analysis paths for "final-form" layouts.
- Deeper integrations with profiling and tracing tools for correlation.

## Phase 5: Business & Growth

Objective:

- Turn the technical product into a sustainable business while preserving developer trust.

Key outcomes:

- Production-ready multi-tenant SaaS deployment.
- Self-hosted Enterprise SKU with SSO, audit logs, and compliance.
- Tiered pricing aligned with the value for HFT, embedded, and gaming segments.
- Documentation, examples, and marketing materials that clearly communicate the value proposition.

