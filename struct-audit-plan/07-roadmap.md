# Implementation Roadmap

## Executive Summary

The struct-audit implementation spans **16 weeks** across three major phases, culminating in a production-ready CLI and SaaS MVP. This roadmap prioritizes delivering a superior pahole replacement before building differentiated SaaS features.

---

## Timeline Overview

```
Week    1  2  3  4  5  6  7  8  9  10 11 12 13 14 15 16
        ├──────────────────┼───────────────┼───────────────────────┤
Phase 1 │████████████████████              │                       │
        │  Core CLI (6 weeks)              │                       │
        │                   ├──────────────┤                       │
Phase 2 │                   │██████████████│                       │
        │                   │ Advanced CLI │                       │
        │                   │  (4 weeks)   │                       │
        │                                  ├───────────────────────┤
Phase 3 │                                  │███████████████████████│
        │                                  │   SaaS MVP (6 weeks)  │
        └──────────────────────────────────────────────────────────┘
```

---

## Phase 1: Core CLI Development

**Duration**: Weeks 1-6
**Objective**: Build a robust DWARF parser that matches pahole in accuracy but exceeds it in usability.

### Week 1-2: Foundation

| Task | Description | Deliverable |
|------|-------------|-------------|
| 1.1 | Project initialization | Cargo workspace, CI setup |
| 1.2 | Dependency integration | gimli, object, clap, serde |
| 1.3 | Binary loader abstraction | Load ELF/Mach-O/PE files |
| 1.4 | Basic DWARF context | Initialize gimli::Dwarf |

**Exit Criteria**: Can open any binary and access DWARF sections.

### Week 3-4: Struct Extraction

| Task | Description | Deliverable |
|------|-------------|-------------|
| 1.5 | CU iteration | Traverse Compilation Units |
| 1.6 | DIE traversal | Find DW_TAG_structure_type |
| 1.7 | Member extraction | Parse DW_TAG_member children |
| 1.8 | Simple offset handling | Constant DW_AT_data_member_location |
| 1.9 | Type resolution | Follow DW_AT_type references |

**Exit Criteria**: Can extract struct name, size, and member offsets for simple cases.

### Week 5-6: Analysis & Output

| Task | Description | Deliverable |
|------|-------------|-------------|
| 1.10 | Padding detection | Implement gap-finding algorithm |
| 1.11 | Text output | Colorized terminal table |
| 1.12 | JSON output | Machine-readable format |
| 1.13 | Filtering | Regex patterns for include/exclude |
| 1.14 | Basic CLI interface | `inspect` command |

**Exit Criteria**: `struct-audit inspect ./binary` produces accurate, readable output.

### Phase 1 Deliverable

```bash
$ struct-audit inspect ./target/release/myapp

struct my_app::Order (72 bytes, 8 padding, 88.9% density)
┌────────┬──────────────────┬──────────┬────────┐
│ Offset │ Field            │ Size     │ Type   │
├────────┼──────────────────┼──────────┼────────┤
│      0 │ id               │ 8        │ u64    │
│      8 │ timestamp        │ 8        │ i64    │
│     16 │ price            │ 8        │ f64    │
│     24 │ quantity         │ 4        │ u32    │
│     28 │ [PADDING]        │ 4        │ -      │
│     32 │ symbol           │ 32       │ [u8]   │
│     64 │ is_active        │ 1        │ bool   │
│     65 │ [PADDING]        │ 7        │ -      │
└────────┴──────────────────┴──────────┴────────┘
```

---

## Phase 2: Advanced Analysis & Diffing

**Duration**: Weeks 7-10
**Objective**: Handle edge cases, implement comparison, enable CI integration.

### Week 7-8: Edge Cases & Expression Evaluation

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.1 | Expression evaluator | Handle complex DW_AT_data_member_location |
| 2.2 | DWARF 4 bitfields | Implement DW_AT_bit_offset conversion |
| 2.3 | DWARF 5 bitfields | Implement DW_AT_data_bit_offset |
| 2.4 | Version detection | Branch on CU DWARF version |
| 2.5 | Array types | Handle DW_TAG_array_type sizing |
| 2.6 | Nested structs | Recursive type resolution |

**Exit Criteria**: Correctly handles bitfields, C++ virtual inheritance, nested types.

### Week 9-10: Diffing & CI Mode

| Task | Description | Deliverable |
|------|-------------|-------------|
| 2.7 | Diff algorithm | Compare two LayoutReports |
| 2.8 | Identity matching | Match structs by qualified name |
| 2.9 | Change classification | Categorize size/padding/member changes |
| 2.10 | `diff` command | CLI command for comparison |
| 2.11 | Config file parser | .struct-audit.yaml support |
| 2.12 | `check` command | CI mode with budgets |
| 2.13 | Exit codes | Non-zero on threshold violation |
| 2.14 | Cache line analysis | Implement straddling detection |

**Exit Criteria**: Can run in CI, fail on regressions, compare branches.

### Phase 2 Deliverable

```bash
# Compare main vs. feature branch
$ struct-audit diff ./main.bin ./feature.bin

┌─────────────────────────────────────────────────────────────┐
│ struct-audit diff report                                    │
│ Base: main.bin (abc123)                                     │
│ Head: feature.bin (def456)                                  │
├─────────────────────────────────────────────────────────────┤
│ Summary: 2 changed, 1 added, 0 removed                      │
└─────────────────────────────────────────────────────────────┘

⚠️ REGRESSION: my_app::Order
   Size: 64 → 72 bytes (+8)
   Padding: 0 → 8 bytes (+8)
   Reason: Added field 'metadata' without reordering

✅ IMPROVED: my_app::User
   Padding: 12 → 4 bytes (-8)

➕ ADDED: my_app::NewStruct (32 bytes)

# CI mode with budgets
$ struct-audit check ./binary --config .struct-audit.yaml
Error: Budget exceeded for my_app::Order
  - Max size: 64, Actual: 72
  - Max padding: 0, Actual: 8
Exit code: 1
```

---

## Phase 3: SaaS Platform MVP

**Duration**: Weeks 11-16
**Objective**: Launch web dashboard for historical tracking and GitHub integration.

### Week 11-12: API Backend

| Task | Description | Deliverable |
|------|-------------|-------------|
| 3.1 | API server setup | Rust (Axum) or Go (Gin) |
| 3.2 | Authentication | API token generation/validation |
| 3.3 | Report ingestion | POST /api/v1/reports |
| 3.4 | Database schema | PostgreSQL tables |
| 3.5 | Deduplication | Hash-based struct dedup |
| 3.6 | CLI `upload` command | Send reports to SaaS |

**Exit Criteria**: CLI can upload reports to backend, data persisted.

### Week 13-14: GitHub Integration

| Task | Description | Deliverable |
|------|-------------|-------------|
| 3.7 | GitHub App registration | OAuth flow, webhook setup |
| 3.8 | Webhook handler | Process PR events |
| 3.9 | Comparison engine | Compare head vs. base branch |
| 3.10 | Status checks | Post pass/fail to GitHub |
| 3.11 | PR comments | Post detailed diff as comment |
| 3.12 | Branch management | Track default branch, handle renames |

**Exit Criteria**: Opening a PR triggers analysis, status check, and comment.

### Week 15-16: Dashboard & Polish

| Task | Description | Deliverable |
|------|-------------|-------------|
| 3.13 | Frontend setup | Next.js, Tailwind, charts library |
| 3.14 | Auth UI | GitHub OAuth login |
| 3.15 | Repository list | Show connected repos |
| 3.16 | Struct history | Time-series chart per struct |
| 3.17 | Trend dashboard | Aggregate metrics over time |
| 3.18 | Budget configuration | UI for setting limits |
| 3.19 | Documentation | User guide, API docs |
| 3.20 | Landing page | Marketing site |

**Exit Criteria**: Users can log in, see history, configure budgets.

### Phase 3 Deliverable

```
┌─────────────────────────────────────────────────────────────────────────┐
│ struct-audit Dashboard                                         [Logout] │
├─────────────────────────────────────────────────────────────────────────┤
│ owner/my-app                                          Branch: main      │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  Total Binary Padding                     Struct Count                  │
│  ┌────────────────────────────────┐      ┌──────────────────────────┐  │
│  │    ╭──╮                        │      │                          │  │
│  │   ╭╯  ╰─╮    ╭───╮            │      │  ████████████████  142   │  │
│  │  ╭╯     ╰────╯   ╰───         │      │                          │  │
│  │  1.2K                1.4K     │      │  Last 30 days: +3        │  │
│  └────────────────────────────────┘      └──────────────────────────┘  │
│                                                                         │
│  Struct History: my_app::Order                                          │
│  ┌────────────────────────────────────────────────────────────────────┐│
│  │ 80 │                                    ╭────────────────────────  ││
│  │ 72 │                            ╭───────╯                          ││
│  │ 64 │────────────────────────────╯                                  ││
│  │    └───────────────────────────────────────────────────────────────││
│  │     Jan        Feb        Mar        Apr        May        Jun     ││
│  └────────────────────────────────────────────────────────────────────┘│
│  ⚠️ Warning: Crossed 64-byte cache line boundary on Apr 15 (commit xyz) │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Milestone Checklist

### M1: CLI Alpha (Week 6)

- [ ] Parse ELF/Mach-O/PE binaries
- [ ] Extract struct layouts with padding
- [ ] Text and JSON output
- [ ] Filter by regex pattern
- [ ] Basic documentation

### M2: CLI Beta (Week 10)

- [ ] DWARF 4 + 5 bitfield support
- [ ] Complex location expressions
- [ ] Binary diffing
- [ ] CI mode with budgets
- [ ] Config file support
- [ ] Cache line analysis

### M3: SaaS Alpha (Week 14)

- [ ] API backend operational
- [ ] GitHub App integration
- [ ] PR comments and status checks
- [ ] Report storage and retrieval

### M4: SaaS Beta (Week 16)

- [ ] Web dashboard live
- [ ] Struct history visualization
- [ ] Budget configuration UI
- [ ] Documentation complete
- [ ] Landing page launched

---

## Resource Requirements

### Engineering

| Role | Count | Phase |
|------|-------|-------|
| Rust Engineer (CLI) | 1-2 | 1, 2 |
| Backend Engineer | 1 | 3 |
| Frontend Engineer | 1 | 3 |

### Infrastructure (Monthly)

| Service | Cost (MVP) | Purpose |
|---------|------------|---------|
| PostgreSQL (RDS) | $100 | Primary database |
| Redis (ElastiCache) | $50 | Caching, queues |
| Kubernetes (EKS/GKE) | $200 | Application hosting |
| S3/R2 | $20 | Report storage |
| Domain + SSL | $20 | struct-audit.io |
| **Total** | ~$400/mo | |

---

## Risk Mitigation Timeline

| Risk | Detection Point | Mitigation |
|------|-----------------|------------|
| gimli edge cases | Week 3-4 | Community support, workarounds |
| DWARF 4 bitfield complexity | Week 7 | Extra testing, known-bad samples |
| GitHub API rate limits | Week 13 | Caching, webhook-first design |
| Dashboard complexity | Week 15 | MVP scope reduction if needed |

---

## Post-MVP Roadmap (After Week 16)

### Near-Term (Months 4-6)

- GitLab integration
- False sharing detection
- Optimization suggestions
- Email/Slack alerts

### Medium-Term (Months 6-12)

- Self-hosted Enterprise option
- SSO (SAML/OIDC)
- Team management features
- Advanced visualizations (heatmaps)

### Long-Term (Year 2+)

- Go support (DWARF + delve integration)
- Function inlining analysis
- Symbol bloat detection
- AI-powered optimization suggestions

---

*Previous: [Business](./06-business.md) | Next: [Specifications](./08-spec.md)*
