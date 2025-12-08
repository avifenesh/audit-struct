# Phases & Milestones

This document focuses on the temporal structure of the project: what happens when, and what "done" looks like for each phase.

## Phase 1 – Core CLI (Target: Weeks 1–6)

Scope:

- Implement the minimal powerful CLI that can be adopted by individual developers without any SaaS dependency.

Milestones:

- Week 1–2:
  - Workspace setup.
  - Basic binary loading and DWARF context creation.
- Week 3–4:
  - Struct discovery and member resolution.
  - Padding and cache-line analysis.
- Week 5–6:
  - Inspector table view.
  - JSON output.
  - Initial docs and examples.

Exit criteria:

- CLI can be pointed at a real binary and produce a credible layout report.
- At least one external engineer (not the primary author) can use it successfully.

## Phase 2 – Advanced Analysis & Diffing (Target: Weeks 7–10)

Scope:

- Make the CLI robust for complex codebases and ready for CI usage.

Milestones:

- Week 7–8:
  - DWARF expression evaluator.
  - Bitfield support for DWARF 4 and 5.
- Week 9–10:
  - Diff engine.
  - CI mode and config file support.
  - Example CI pipelines published.

Exit criteria:

- CLI can diff two large, realistic binaries and surface layout regressions.
- CI pipelines can use struct-audit to fail builds on configured budget violations.

## Phase 3 – SaaS Platform MVP (Target: Weeks 11–16)

Scope:

- Build the first version of the hosted platform that adds real value beyond the CLI.

Milestones:

- Week 11–12:
  - Backend ingestion API and DB schema.
  - CLI integration with API.
- Week 13–14:
  - Basic dashboard UI (repo list, build list, struct tables).
  - Simple historical charts.
- Week 15–16:
  - GitHub App integration and PR status checks.
  - Budget configuration UI.

Exit criteria:

- At least one pilot repository using the full CLI+SaaS loop in CI.
- Product can demonstrate "when did this struct regress?" for real projects.

## Phase 4 – Advanced Features & Hardening

Scope:

- Turn the MVP into a production-ready system for demanding customers.

Milestones:

- False sharing detection beta.
- Layout suggestion beta.
- LTO-aware analysis validation.
- Performance and scalability benchmarking under heavy CI load.

Exit criteria:

- System remains stable and performant under concurrent CI usage.
- Advanced analyses produce actionable and trusted results for expert users.

## Phase 5 – Business & Enterprise

Scope:

- Operationalize the business aspects while preserving a strong developer-centric ethos.

Milestones:

- Pricing page and tier definitions.
- Self-hosted deployment option (Helm chart / Docker Compose).
- SSO, audit logs, and compliance basics (e.g., SOC2 trajectory).

Exit criteria:

- At least a handful of paying customers across target segments (HFT, embedded, games).
- Clear, repeatable onboarding and support processes.

