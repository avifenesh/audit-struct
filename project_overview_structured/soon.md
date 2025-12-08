# Near-Term Focus ("Soon")

This document captures what should happen next to move from feasibility/specification into a concrete, high-quality product.

## Immediate Technical Next Steps

- Finalize the CLI MVP scope for Phase 1:
  - Decide which languages and platforms are in-scope for the very first release (e.g., Rust + Linux/Mac).
  - Lock the JSON schema version for the initial CLI output.
- Build a minimal but real prototype:
  - Run struct-audit on at least one non-trivial open-source project.
  - Validate performance characteristics on large binaries.
  - Capture any DWARF edge cases not yet covered in the design.
- Establish testing strategy:
  - Golden-layout fixtures for well-known binaries.
  - Regression tests around padding calculations and bitfield handling.

## Immediate Product & UX Steps

- Refine CLI ergonomics:
  - Subcommand structure (`inspect`, `diff`, `ci`).
  - Default filters and sensible defaults for cache-line size.
  - Output formatting that is usable in both local terminals and CI logs.
- Decide on the minimal initial SaaS surface:
  - Which pages and charts are essential for the first pilot customer?
  - What can be deferred to later iterations without harming adoption?
- Prepare messaging and docs:
  - Short "Why struct-audit?" explainer.
  - Quickstart guide for CLI and GitHub Actions integration.

## Immediate Business & GTM Steps

- Validate demand with target users:
  - Short interviews with engineers in HFT, embedded, and gaming.
  - Validate specific pain points and willingness to adopt a CI gating tool.
- Define pilot program:
  - Criteria for pilot repos and teams.
  - Support and feedback channels.
  - Success metrics for the pilot (e.g., number of regressions caught).
- Align on pricing direction:
  - Decide whether to launch with formal pricing or start with free pilots.
  - Outline how and when to introduce Pro/Enterprise tiers.

## Open Questions to Resolve Soon

- How aggressively should the tool enforce budgets by default?
  - Conservative mode (warn only) vs. strict mode (fail on regression).
- Which advanced features (false sharing detection, layout suggestions) should be started in parallel vs. strictly after MVP?
- How much of the initial engineering effort should go into self-hosted deployment versus SaaS reliability and UX?

Clarifying these points early will keep the execution path focused and ensure the project converges on a high-level, production-grade product rather than a perpetual feasibility study.

