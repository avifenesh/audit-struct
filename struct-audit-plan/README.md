# struct-audit: Product Plan & Technical Specification

> **Memory Layout Intelligence for Systems Programming**

This directory contains the comprehensive planning documentation for struct-audit—a platform for analyzing, tracking, and enforcing memory layout efficiency in compiled binaries.

---

## Quick Navigation

| Document | Description | Audience |
|----------|-------------|----------|
| [01-vision.md](./01-vision.md) | Strategic imperative, problem statement, why this matters | Everyone |
| [02-market.md](./02-market.md) | Competitive landscape, target segments, positioning | Product, Business |
| [03-tech.md](./03-tech.md) | DWARF format, gimli, parsing foundations | Engineers |
| [04-architecture.md](./04-architecture.md) | System design, CLI + SaaS components | Engineers, Architects |
| [05-algorithms.md](./05-algorithms.md) | Padding detection, cache analysis, diffing | Engineers |
| [06-business.md](./06-business.md) | Pricing, GTM strategy, unit economics | Business, Product |
| [07-roadmap.md](./07-roadmap.md) | 16-week implementation timeline | Everyone |
| [08-spec.md](./08-spec.md) | JSON schemas, API specs, CLI interface | Engineers |
| [09-tasks.md](./09-tasks.md) | Detailed task breakdown with estimates | Engineers, PM |

---

## Executive Summary

### The Problem

Modern CPUs are bottlenecked by memory access, not computation. A single cache miss costs 100-300 CPU cycles. Yet the **memory layout** of data structures—the physical arrangement that determines cache efficiency—remains invisible to developers.

When a struct has unnecessary padding (invisible bytes inserted for alignment), it:
- Wastes RAM
- Reduces cache efficiency
- Increases latency

This is **silent technical debt** that passes all tests and code reviews.

### The Solution

**struct-audit** makes memory layout a first-class metric:

```
┌─────────────────────────────────────────────────────────┐
│                    struct-audit                          │
├─────────────────────────────────────────────────────────┤
│                                                         │
│  1. PARSE    Binary → DWARF → Struct Layouts            │
│                                                         │
│  2. ANALYZE  Detect padding, cache inefficiency         │
│                                                         │
│  3. TRACK    Historical metrics over commits            │
│                                                         │
│  4. GATE     Fail CI on regressions                     │
│                                                         │
└─────────────────────────────────────────────────────────┘
```

### Target Market

| Segment | Pain Point | Value |
|---------|------------|-------|
| **HFT/FinTech** | Nanosecond latency | Cache miss = money lost |
| **Embedded/IoT** | RAM constraints | Smaller structs = cheaper chips |
| **Gaming** | Frame budgets | Better cache = stable FPS |
| **Infrastructure** | Scale | Efficiency × millions |

### Business Model

**Open Core**:
- **Free CLI**: Superior pahole replacement, drives adoption
- **Paid SaaS**: Historical tracking, CI integration, team features

---

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Developer Workflow                           │
└───────────────────────────────┬─────────────────────────────────────┘
                                │
              ┌─────────────────┼─────────────────┐
              ▼                 ▼                 ▼
       ┌────────────┐    ┌────────────┐    ┌────────────┐
       │   Local    │    │     CI     │    │ Dashboard  │
       │   Debug    │    │  Pipeline  │    │   (SaaS)   │
       └─────┬──────┘    └─────┬──────┘    └─────┬──────┘
             │                 │                 │
             └─────────────────┼─────────────────┘
                               ▼
                    ┌─────────────────────┐
                    │   struct-audit CLI  │
                    │  ┌───────────────┐  │
                    │  │ gimli (DWARF) │  │
                    │  │ object (ELF)  │  │
                    │  └───────────────┘  │
                    └──────────┬──────────┘
                               │ JSON Report
                               ▼
                    ┌─────────────────────┐
                    │  struct-audit SaaS  │
                    │  ┌───────────────┐  │
                    │  │ History DB    │  │
                    │  │ GitHub App    │  │
                    │  │ Dashboard     │  │
                    │  └───────────────┘  │
                    └─────────────────────┘
```

---

## Roadmap Summary

### Phase 1: Core CLI (Weeks 1-6)
- DWARF parsing with gimli
- Struct extraction and padding detection
- Text/JSON output
- `inspect` command

### Phase 2: Advanced CLI (Weeks 7-10)
- Bitfield support (DWARF 4 + 5)
- Expression evaluation
- Binary diffing
- CI mode with budgets

### Phase 3: SaaS MVP (Weeks 11-16)
- API backend
- GitHub integration (PR comments, status checks)
- Web dashboard
- Historical tracking

---

## Key Technical Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Language** | Rust | Memory safety, gimli ecosystem, performance |
| **DWARF Parser** | gimli | Zero-copy, lazy, well-maintained |
| **Binary Abstraction** | object | Unified ELF/Mach-O/PE handling |
| **Backend** | Rust (Axum) | Type safety, performance |
| **Frontend** | Next.js | Modern React, SSR, good DX |
| **Database** | PostgreSQL | Reliable, time-series extensions |

---

## Success Metrics

### Year 1 Targets

| Metric | Target |
|--------|--------|
| GitHub Stars | 5,000 |
| Weekly CLI Downloads | 10,000 |
| Active Organizations | 500 |
| Paying Customers | 50 |
| ARR | $200k |

---

## Getting Started (For Contributors)

Once implementation begins:

```bash
# Clone the repository
git clone https://github.com/struct-audit/struct-audit
cd struct-audit

# Build the CLI
cargo build --release

# Run on a binary
./target/release/struct-audit inspect ./path/to/binary

# Run tests
cargo test
```

---

## Document Changelog

| Date | Version | Changes |
|------|---------|---------|
| 2024-06-15 | 1.0.0 | Initial comprehensive plan |

---

## Contact

- **Project Lead**: TBD
- **Technical Questions**: TBD
- **Business Inquiries**: TBD

---

*This plan is a living document. As implementation progresses, details may be refined based on learnings and feedback.*
