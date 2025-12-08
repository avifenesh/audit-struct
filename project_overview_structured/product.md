# Product: Struct-Audit Platform

## Overview

Struct-audit is a two-part product:

- A local CLI agent for developers.
- A centralized SaaS (or self-hosted) platform for teams.

Together they turn memory layout into a visible, enforceable part of the development workflow.

## Personas

- Systems engineer / backend engineer:
  - Needs to understand why performance regressed after a code change.
  - Uses CLI to inspect specific structs and compare binaries.
- Tech lead / performance engineer:
  - Owns latency and memory budgets for critical services.
  - Uses SaaS dashboard and CI gating to enforce policies.
- Engineering manager / VP Eng:
  - Wants confidence that performance regressions are caught early.
  - Looks at historical trends, risk hotspots, and alerts.

## CLI Agent

Primary responsibilities:

- Ingest a compiled binary (ELF, Mach-O, PE) with DWARF debug info.
- Reconstruct struct/class layouts:
  - Member names, types, offsets, and sizes.
  - Explicit and implicit padding regions.
  - Cache-line crossings and density metrics.
- Provide multiple output modes:
  - Human-readable colorized tables for local exploration.
  - JSON for machine consumption (CI, SaaS upload, custom tooling).
- Core features:
  - `inspect` view for single-binary layout analysis.
  - `diff` view comparing two binaries (e.g., `main` vs `feature`).
  - Filtering by namespace/module, regex on struct names, or size thresholds.
  - CI mode with `--fail-on-growth` and struct-specific budgets.

## SaaS / Self-Hosted Service

Primary responsibilities:

- Accept JSON reports from CI pipelines.
- Store build metadata and layout metrics over time.
- Provide dashboards for:
  - Per-struct history (size, padding, cache behavior).
  - Repository-level metrics (total padding, largest regressions).
  - Heatmaps and visualizations of packing density.
- Integrate with GitHub/GitLab:
  - Status checks that block merges on violated budgets.
  - PR comments summarizing layout changes and risk.

Key features:

- Repository and commit explorer.
- Configurable layout budgets and alert thresholds.
- Time-series charts for memory and padding metrics.
- Multi-tenant SaaS plus self-hosted Enterprise option.

## Differentiation

Compared to incumbents:

- `pahole`:
  - Local-only, Linux-centric, and suffering from parsing issues and memory problems on large binaries.
  - No history, collaboration, or CI-native gating.
- IDE extensions:
  - Great for inner-loop visualization, but no enforcement in CI and no shared team view.
- `ddbug` and similar Rust CLIs:
  - Strong technical foundations but positioned as point tools, not platforms.

Struct-audit:

- Combines superior local CLI usability with a purpose-built SaaS layer.
- Treats layout metrics as time-series data with budgets and policy.
- Targets multi-platform CI and enterprise workflows from day one.

## Product Boundaries (v1)

Included:

- CLI layout inspection and diffing for Rust, C, C++ (Go as a stretch).
- CI integration with JSON output, budgets, and fail-on-regression behavior.
- Minimal but functional SaaS UI for history, comparison, and gating configuration.

Out of scope (future):

- Automated source-level refactoring or layout rewriting.
- Full-fledged performance profiler or tracing tool.
- Arbitrary language support beyond DWARF-based toolchains.

