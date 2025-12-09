# Future SaaS Vision (Archived)

> **Note**: This document contains deferred plans for the SaaS platform. Do not implement until CLI v0.2.0 is stable and has real users.

---

## Prerequisites Before Starting SaaS

1. CLI v0.2.0 (diff + CI mode) is stable
2. CLI is being used on real projects (not just your own)
3. You feel recurring pain from lack of historical tracking
4. You're willing to take on ongoing maintenance cost of hosted infrastructure

---

## SaaS Platform Overview

### Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         SaaS Platform                                    │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                        Load Balancer                             │   │
│  └────────────────────────────┬────────────────────────────────────┘   │
│                               │                                         │
│          ┌────────────────────┼────────────────────┐                   │
│          │                    │                    │                    │
│          ▼                    ▼                    ▼                    │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐             │
│  │  API Server  │    │  API Server  │    │  API Server  │             │
│  │   (Axum)     │    │   (Axum)     │    │   (Axum)     │             │
│  └──────┬───────┘    └──────┬───────┘    └──────┬───────┘             │
│         │                   │                   │                       │
│         └───────────────────┼───────────────────┘                       │
│                             │                                           │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                        Data Layer                                │   │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │   │
│  │  │  PostgreSQL  │  │    Redis     │  │     S3       │           │   │
│  │  │  (Primary)   │  │   (Cache)    │  │  (Reports)   │           │   │
│  │  └──────────────┘  └──────────────┘  └──────────────┘           │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
│  ┌─────────────────────────────────────────────────────────────────┐   │
│  │                      Frontend (Next.js)                          │   │
│  └─────────────────────────────────────────────────────────────────┘   │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Technology Stack

| Component | Technology |
|-----------|------------|
| Backend | Rust (Axum) |
| Database | PostgreSQL |
| Cache | Redis |
| Frontend | Next.js + Tailwind |
| Auth | GitHub/GitLab OAuth |
| Hosting | Render / Vercel |

---

## Database Schema (Draft)

```sql
CREATE TABLE organizations (
    id UUID PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    slug VARCHAR(255) UNIQUE NOT NULL,
    plan VARCHAR(50) DEFAULT 'community',
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE repositories (
    id UUID PRIMARY KEY,
    organization_id UUID REFERENCES organizations(id),
    provider VARCHAR(50) NOT NULL,
    provider_id VARCHAR(255) NOT NULL,
    name VARCHAR(255) NOT NULL,
    full_name VARCHAR(512) NOT NULL,
    config JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(provider, provider_id)
);

CREATE TABLE commits (
    id UUID PRIMARY KEY,
    repository_id UUID REFERENCES repositories(id) ON DELETE CASCADE,
    sha VARCHAR(40) NOT NULL,
    branch VARCHAR(255),
    author VARCHAR(255),
    message TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(repository_id, sha)
);

CREATE TABLE struct_layouts (
    id UUID PRIMARY KEY,
    content_hash VARCHAR(64) UNIQUE NOT NULL,
    name VARCHAR(512) NOT NULL,
    size BIGINT NOT NULL,
    alignment BIGINT NOT NULL,
    padding_bytes BIGINT NOT NULL,
    padding_percent DECIMAL(5,2) NOT NULL,
    cache_lines INTEGER NOT NULL,
    density DECIMAL(5,4) NOT NULL,
    layout JSONB NOT NULL,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

CREATE TABLE commit_structs (
    commit_id UUID REFERENCES commits(id) ON DELETE CASCADE,
    struct_layout_id UUID REFERENCES struct_layouts(id),
    PRIMARY KEY (commit_id, struct_layout_id)
);

CREATE TABLE budgets (
    id UUID PRIMARY KEY,
    repository_id UUID REFERENCES repositories(id) ON DELETE CASCADE,
    pattern VARCHAR(512) NOT NULL,
    max_size BIGINT,
    max_padding_percent DECIMAL(5,2),
    max_cache_lines INTEGER,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

---

## API Endpoints (Draft)

### Authentication
- `POST /auth/github` - GitHub OAuth callback
- `POST /auth/gitlab` - GitLab OAuth callback
- `DELETE /auth/logout` - Logout

### Reports
- `POST /api/v1/reports` - Upload new report
- `GET /api/v1/reports/:id` - Get report by ID

### Repositories
- `GET /api/v1/repos` - List connected repos
- `GET /api/v1/repos/:id/structs` - List structs
- `GET /api/v1/repos/:id/structs/:name/history` - Struct history

### Budgets
- `GET /api/v1/repos/:id/budgets` - List budgets
- `POST /api/v1/repos/:id/budgets` - Create budget
- `PUT /api/v1/repos/:id/budgets/:bid` - Update budget
- `DELETE /api/v1/repos/:id/budgets/:bid` - Delete budget

### Webhooks
- `POST /webhooks/github` - Handle GitHub events
- `POST /webhooks/gitlab` - Handle GitLab events

---

## Pricing Tiers (Draft)

| Tier | Price | Features |
|------|-------|----------|
| Community | Free | Public repos, 14-day history, 1 user |
| Pro | $29/user/mo | Private repos, unlimited history, CI blocking |
| Team | $49/user/mo | Slack integration, custom budgets, priority support |
| Enterprise | Custom | Self-hosted, SSO, audit logs, SLA |

---

## GitHub Integration

### GitHub App Permissions
- Read repository contents
- Read/write checks
- Read/write pull requests (comments)
- Webhook events: push, pull_request

### PR Comment Format

```markdown
## struct-audit Report

### Layout Changes Detected

| Struct | Size Change | Padding | Cache Lines |
|--------|-------------|---------|-------------|
| `Order` | 64 → 72 (+12.5%) | 6 → 14 | 1 → 2 |

### Warning
`Order` now spans **2 cache lines** (was 1).

[View full report](https://app.struct-audit.io/reports/abc123)
```

---

## Dashboard Features (Draft)

### Overview Page
- Health score (0-100)
- Total padding trend chart
- Top 10 worst offenders
- Recent regressions

### Struct History Page
- Size over time sparkline
- Commit-by-commit changes
- Blame view

### Budget Configuration
- Visual budget editor
- Sync to `.struct-audit.yaml`

---

## Security Considerations

### IP Protection Mode
For HFT customers who won't upload struct names:

```rust
// Hashing mode in CLI
// Original: "trading::AlphaStrategy"
// Hashed:   "struct_9a8b7c6d"

// Local mapping file (.struct-audit-mapping.json)
{
    "struct_9a8b7c6d": "trading::AlphaStrategy"
}
```

### Self-Hosted Option
- Docker Compose / Helm chart
- Air-gapped deployment
- No data leaves customer network

---

## Infrastructure Costs (Estimated)

| Service | Monthly Cost |
|---------|--------------|
| Render (API) | $25-100 |
| Render (DB) | $20-50 |
| Vercel | $20 |
| Cloudflare | Free-$20 |
| **Total** | $65-190 |

---

## Milestones (When Ready)

1. **API Alpha**: Report upload working
2. **GitHub Integration**: PR comments posting
3. **Dashboard Beta**: Basic UI functional
4. **GA Launch**: Production ready

---

## Decision: When to Start SaaS

Answer these questions:
1. Is CLI v0.2.0 stable and feature-complete?
2. Do you have users other than yourself?
3. Have multiple people asked for historical tracking?
4. Are you prepared for ongoing ops burden?

If all answers are "yes", revisit this document and begin implementation.
