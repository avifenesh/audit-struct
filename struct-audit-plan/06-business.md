# Business Model & Go-to-Market Strategy

## Executive Summary

struct-audit follows an **Open Core** business model: a free, open-source CLI drives adoption, while a commercial SaaS platform monetizes team collaboration and historical tracking features.

---

## Business Model: Open Core

### The Two Products

| Component | Pricing | Purpose |
|-----------|---------|---------|
| **CLI Tool** | Free (MIT/Apache) | Adoption driver, brand awareness |
| **SaaS Platform** | Subscription | Revenue generation |

### Why Open Core Works

1. **Low Barrier to Entry**: Developers try the CLI risk-free
2. **Network Effects**: Open source creates community, contributions, trust
3. **Natural Upgrade Path**: Teams outgrow local tools, need collaboration
4. **Competitive Moat**: Community + integrations hard to replicate

### Precedents

| Company | Open Source | Paid Product | Outcome |
|---------|-------------|--------------|---------|
| GitLab | GitLab CE | GitLab EE/SaaS | $14B market cap |
| HashiCorp | Terraform, Vault | Terraform Cloud | $15B acquisition |
| Elastic | Elasticsearch | Elastic Cloud | $8B+ market cap |
| Sentry | Sentry SDK | Sentry SaaS | $3B+ valuation |
| Codecov | Coverage Tools | Codecov SaaS | Acquired by Sentry |

---

## Pricing Strategy

### Tier Structure

| Tier | Price | Target |
|------|-------|--------|
| **Community** | Free | OSS projects, hobbyists, evaluation |
| **Pro** | $29/user/month | Startups, small teams |
| **Team** | $49/user/month | Mid-market, growing companies |
| **Enterprise** | Custom ($100k+/year) | Large orgs, regulated industries |

### Feature Matrix

| Feature | Community | Pro | Team | Enterprise |
|---------|-----------|-----|------|------------|
| CLI Tool | ✓ | ✓ | ✓ | ✓ |
| Public Repo Support | ✓ | ✓ | ✓ | ✓ |
| Private Repo Support | - | ✓ | ✓ | ✓ |
| History Retention | 14 days | 1 year | Unlimited | Unlimited |
| CI/CD Blocking | - | ✓ | ✓ | ✓ |
| PR Comments | - | ✓ | ✓ | ✓ |
| Custom Budgets | - | 10 | Unlimited | Unlimited |
| Team Dashboard | - | - | ✓ | ✓ |
| Multi-Repo Views | - | - | ✓ | ✓ |
| Slack/Email Alerts | - | - | ✓ | ✓ |
| SSO (SAML/OIDC) | - | - | - | ✓ |
| Self-Hosted Option | - | - | - | ✓ |
| Audit Logs | - | - | - | ✓ |
| Name Hashing Mode | - | - | - | ✓ |
| Dedicated Support | - | - | - | ✓ |
| SLA Guarantee | - | - | - | ✓ |

### Pricing Rationale

**Pro ($29/user)**: Comparable to Codecov Pro ($10-29), Sentry Team (~$26), linear pricing scales with value.

**Team ($49/user)**: Premium for collaboration features. Teams of 5-20 engineers where struct efficiency matters.

**Enterprise (Custom)**: Accounts for:
- Self-hosting infrastructure support
- Security reviews and compliance docs
- Custom integrations
- High-touch support

---

## Go-to-Market Strategy

### Phase 1: "Win the Developer" (Months 1-6)

**Objective**: Establish the CLI as the modern pahole replacement.

**Tactics**:

| Activity | Goal | Metric |
|----------|------|--------|
| Launch on Hacker News | Initial awareness | 500+ points |
| Reddit (r/rust, r/cpp, r/programming) | Community seeding | 1000 subscribers |
| Publish benchmark vs. pahole | Credibility | Blog shares |
| Present at RustConf/CppCon | Reach core audience | Attendee engagement |
| YouTube demo videos | SEO + education | 10k views |

**Content Strategy**:
- "Memory Layout: The Hidden Performance Tax" (thought leadership)
- "From 128 to 64 bytes: Optimizing Order structs for HFT" (case study)
- "Why Your Rust Structs Are Bigger Than You Think" (educational)

**Target Metrics**:
- 5,000 GitHub stars
- 10,000 weekly CLI downloads
- 2,000 Discord/community members

### Phase 2: "Embed in Workflow" (Months 6-12)

**Objective**: Drive CI/CD adoption, collect usage data.

**Tactics**:

| Activity | Goal |
|----------|------|
| GitHub Action (marketplace listing) | One-click CI integration |
| GitLab CI template | Expand platform reach |
| CircleCI Orb | Enterprise CI presence |
| "Getting Started" guides per platform | Reduce friction |
| Free tier for open source | Capture data, build trust |

**Target Metrics**:
- 500 organizations using CI integration
- 100 active SaaS users (free tier)
- 5 case study partners

### Phase 3: "Monetize the Manager" (Months 12-18)

**Objective**: Convert free users to paid, land enterprise deals.

**Tactics**:

| Activity | Goal |
|----------|------|
| Implement Team dashboard | Unlock Team tier |
| Sales outreach to known HFT firms | Land enterprise |
| Partner with consulting firms | Channel sales |
| SOC 2 Type II certification | Enterprise requirement |
| Customer success program | Reduce churn |

**Target Metrics**:
- $500k ARR
- 5 Enterprise customers
- <5% monthly churn (Pro tier)

---

## Customer Segments Deep Dive

### Segment 1: High-Frequency Trading (HFT)

**Profile**:
- Companies: Jane Street, Citadel, Two Sigma, Jump Trading
- Team size: 50-500 engineers
- Languages: C++, Rust
- Pain: Nanosecond latency matters

**Value Proposition**: "Prevent cache-miss regressions that cost you money"

**Buying Process**:
- Engineer discovers tool → Internal champion
- Security/compliance review (critical)
- POC with non-critical codebase
- Enterprise contract negotiation

**Requirements**:
- Self-hosted deployment (no data leaves premises)
- Name hashing for IP protection
- SOC 2 / ISO 27001 compliance
- Air-gapped environment support

**Deal Size**: $100k-500k/year

**Sales Approach**: Direct enterprise sales, long cycle (6-12 months)

### Segment 2: Embedded/IoT

**Profile**:
- Companies: Tesla, Rivian, John Deere, medical device makers
- Team size: 20-200 firmware engineers
- Languages: C, C++, Rust
- Pain: RAM/Flash are expensive, constrained

**Value Proposition**: "Reduce memory footprint → use cheaper hardware → save millions in BOM"

**Buying Process**:
- Cost reduction initiative from management
- Engineering evaluates tools
- Procurement/vendor onboarding

**Requirements**:
- Cross-compilation support (ARM, RISC-V)
- Integration with embedded CI (Jenkins, custom)
- Firmware binary analysis

**Deal Size**: $30k-100k/year

**Sales Approach**: Technical content marketing, conference presence (Embedded World)

### Segment 3: Game Development

**Profile**:
- Companies: Epic, Valve, EA, Ubisoft, indie studios
- Team size: 10-500 engine/systems engineers
- Languages: C++, Rust (emerging)
- Pain: Frame rate stability, memory pressure

**Value Proposition**: "Keep your hot path data structures cache-optimal"

**Buying Process**:
- Technical Director / Engine Lead evaluates
- Studio-wide license negotiation

**Requirements**:
- Console-specific binary support (PS5, Xbox, Switch)
- Unreal/Unity integration guidance
- Game engine struct patterns

**Deal Size**: $20k-50k/year (per studio)

**Sales Approach**: GDC presence, Unreal/Unity partner program, dev advocacy

### Segment 4: Infrastructure / Cloud Native

**Profile**:
- Companies: Cloudflare, Fastly, Datadog, databases
- Team size: 50-500 systems engineers
- Languages: Rust, C, Go
- Pain: Scale amplifies inefficiencies

**Value Proposition**: "Optimize memory at scale → reduce fleet costs"

**Buying Process**:
- Platform team identifies need
- Self-serve trial → team adoption
- Enterprise upgrade for compliance

**Deal Size**: $20k-50k/year

**Sales Approach**: Product-led growth, developer relations

---

## Competitive Positioning

### Messaging Framework

**For**: Systems engineers building performance-critical software

**Who**: Need visibility into memory layout and struct efficiency

**struct-audit is**: A memory layout intelligence platform

**That**: Tracks, visualizes, and enforces struct efficiency over time

**Unlike**: pahole (outdated, local-only) or IDE plugins (no CI integration)

**Our product**: Provides continuous performance assurance through historical tracking and CI gating

### Key Differentiators

1. **Historical Tracking**: No competitor offers time-series struct metrics
2. **CI-Native**: Built for automation, not just inspection
3. **Modern Foundation**: Rust + gimli vs. aging C libraries
4. **Team Collaboration**: Shared dashboards, PR comments, alerts

### Competitive Response Playbook

| Competitor Move | Our Response |
|-----------------|--------------|
| pahole adds CI support | Highlight our superior UX, cross-platform, SaaS features |
| VS Code extension improves | Emphasize outer-loop (CI) vs. inner-loop (editor) |
| New entrant appears | Focus on our depth (time-series, collaboration, enterprise) |

---

## Risk Analysis

### Technical Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| DWARF spec changes | Low | Medium | Active monitoring, rapid adaptation |
| New binary formats | Low | Medium | Modular architecture |
| gimli crate abandonment | Very Low | High | Contribute/fork if needed |

### Market Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Market too niche | Medium | High | Expand to related metrics (binary bloat, etc.) |
| Enterprise sales cycles too long | Medium | Medium | Strong self-serve motion |
| Competition from IDE vendors | Low | Medium | Focus on CI/historical differentiation |

### Business Risks

| Risk | Probability | Impact | Mitigation |
|------|-------------|--------|------------|
| Customer concentration | Medium | High | Diversify across segments |
| Pricing too high/low | Medium | Medium | A/B testing, customer feedback |
| Churn from free users | Medium | Low | Improve activation, feature gating |

---

## Financial Projections (3-Year)

### Revenue Model

| Year | ARR Target | Key Drivers |
|------|------------|-------------|
| Year 1 | $200k | Early adopters, 2-3 enterprise deals |
| Year 2 | $1M | Growth in Team tier, 10 enterprise deals |
| Year 3 | $3M | Market leadership, expansion revenue |

### Unit Economics (Targets)

| Metric | Target | Rationale |
|--------|--------|-----------|
| CAC (Pro) | $500 | Content marketing, self-serve |
| CAC (Enterprise) | $25k | Direct sales, long cycle |
| LTV (Pro) | $3,000 | 3-year retention, $29×36 |
| LTV (Enterprise) | $300k | 3-year retention, $100k×3 |
| LTV:CAC | 6:1+ | Healthy SaaS benchmark |
| Gross Margin | 80%+ | SaaS standard |
| Net Revenue Retention | 110%+ | Seat expansion |

---

## Success Metrics

### North Star Metric

**Weekly Active Organizations (WAO)**: Number of organizations that uploaded at least one report in the past 7 days.

### Supporting Metrics

| Category | Metric | Year 1 Target |
|----------|--------|---------------|
| Acquisition | GitHub stars | 5,000 |
| Acquisition | CLI weekly downloads | 10,000 |
| Activation | Orgs completing CI setup | 500 |
| Retention | Monthly active orgs | 200 |
| Revenue | ARR | $200k |
| Revenue | Paying customers | 50 |

---

*Previous: [Algorithms](./05-algorithms.md) | Next: [Roadmap](./07-roadmap.md)*
