# Business & Go-To-Market

## Market Landscape

- Problem space:
  - Memory layout is a primary driver of latency and memory usage in performance-critical domains.
  - Struct layout regressions are currently invisible in most teams’ workflows.
- Existing tools:
  - `pahole`:
    - Mature, widely used in kernel development.
    - Linux-centric, C-based, with known issues parsing modern C++ and handling large binaries.
    - No persistence, history, or collaboration.
  - IDE tools (Visual Studio, VS Code extensions):
    - Provide localized, developer-only visualizations.
    - Do not connect to CI or team-wide policy.
  - `ddbug` and similar CLIs:
    - Strong technically but optimized for one-off debugging, not long-term metrics.

Opportunity:

- No existing platform provides:
  - Continuous tracking of struct layouts over time.
  - CI-native regression gating with budgets.
  - Visualization and collaboration around memory layout metrics.

## Segments & Value Propositions

- High-Frequency Trading (HFT) / FinTech:
  - Pain:
    - Latency spikes translate directly into financial loss.
  - Value:
    - "Performance insurance" against accidental struct regressions.
    - Guarantees that key structs (Order, Tick, etc.) remain within cache-line budgets.
  - Requirements:
    - Self-hosted / on-prem.
    - Strong IP and security posture.

- Embedded / IoT:
  - Pain:
    - Strict flash/RAM constraints drive BOM cost.
  - Value:
    - "BOM optimization" via reduced memory usage.
    - Potential to step down microcontroller classes and save per-unit cost at scale.

- AAA Game Development:
  - Pain:
    - Frame-time budget is tight; ECS and data layouts heavily impact performance.
  - Value:
    - Frame-rate stability by preventing layout regressions in hot-path components.

## Business Model

- Open Core SaaS:
  - CLI is free and open-source:
    - Developer adoption driver.
    - Usable without any SaaS dependency.
  - SaaS adds:
    - History, dashboards, alerts.
    - Team-level features and integrations.

### Pricing Tiers (Conceptual)

- Community (Free):
  - CLI usage.
  - Public repo support.
  - Short history window.
- Pro (Per-user, per-month):
  - Private repos.
  - Unlimited history.
  - CI gating and alerts.
- Enterprise (Custom):
  - Self-hosted deployment.
  - SSO (SAML/Okta).
  - Audit logs, priority support, security commitments.

## Adoption Strategy

- Trojan Horse CLI:
  - Release a better `pahole`:
    - Simpler installation (e.g., `cargo install struct-audit`).
    - Safer and faster on large binaries.
    - Better UX (colorized tables, intuitive flags).
  - Once widely used locally, CI integration and SaaS become a natural next step.

- CI & DevEx:
  - Provide first-class support for:
    - GitHub Actions, GitLab CI, and other major CI providers.
  - Offer copy-pasteable templates and example workflows.

- Monetize the Manager:
  - Use SaaS to surface:
    - Technical-debt reduction over time.
    - Performance stability metrics.
  - Give engineering managers a clear narrative:
    - "We stopped regressions before they shipped."

## Risks & Mitigations

- IP Sensitivity:
  - HFT and similar customers avoid sending code-level data off-prem.
  - Mitigation:
    - Hash struct names before upload (salted hashing mode).
    - Self-hosted Enterprise deployment.

- Compiler & Build Variability:
  - Different flags and toolchains produce different layouts.
  - Mitigation:
    - Tag reports by build flavor (OS, compiler, flags).
    - Compare like-with-like in the SaaS.

- Trust in Analysis:
  - Incorrect layouts destroy confidence.
  - Mitigation:
    - Aggressive test coverage using known-good fixtures.
    - Clear transparency about what is and isn’t supported.

