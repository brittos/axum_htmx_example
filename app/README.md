# APP - Bero Admin

Backend API built with **Axum + Askama + HTMX + SeaORM**.

## Stack

| Technology | Version | Description |
|-----------|---------|-------------|
| Axum | 0.8 | Async web framework |
| Askama | 0.15 | Template engine |
| SeaORM | 2.0.0-rc.20 | Async ORM |
| HTMX | 2.0.8 | Frontend interactivity |
| Argon2 | 0.5.3 | Password hashing |
| Tracing | 0.1 | Structured logging |

## Project Structure

```
app/
├── src/
│   ├── config/          # Configuration (DB, logging)
│   ├── handlers/        # Controllers/Handlers
│   ├── models/          # View models
│   ├── repository/      # Mutations and queries
│   ├── service/         # Business logic + audit
│   ├── utils/           # Utilities (password hash)
│   ├── routes.rs        # Route definitions
│   ├── state.rs         # Application state
│   └── lib.rs           # Entry point
├── templates/           # Askama templates
│   ├── admin/           # Admin templates
│   ├── app/             # App templates
│   └── error/           # Error pages
└── static/              # Static files (embedded in release)
    ├── style.css        # CSS styles
    ├── htmx.min.js      # HTMX library
    ├── sse.js           # Server-Sent Events
    └── favicon.ico      # Site icon
```

## Static Files

Static files are served differently based on the build mode:

| Mode | Behavior |
|------|----------|
| **Debug** (`cargo run`) | Served from filesystem (`static/`) - allows hot reload |
| **Release** (`cargo build --release`) | Embedded in binary via `rust-embed` + `axum-embed` |

## Endpoints

### Admin - Authentication

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/admin/login` | Login page (visual) |

### Admin - Dashboard

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/admin` | Main dashboard |
| GET | `/admin/settings` | Settings |

### Admin - User Management

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/admin/users/management` | User list with RBAC |
| GET | `/admin/users/partial` | HTMX partial for table (supports `?page=N`) |
| GET | `/admin/users/create` | Creation form |
| POST | `/admin/users` | Create user |
| GET | `/admin/users/{id}/edit` | Edit form |
| PUT | `/admin/users/{id}` | Update user |
| DELETE | `/admin/users/{id}` | Delete user |

### Admin - RBAC

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/admin/rbac/partial` | RBAC matrix partial |
| PATCH | `/admin/rbac/toggle` | Permission toggle |

### App

| Method | Route | Description |
|--------|-------|-------------|
| GET | `/app` | App dashboard |
| GET | `/app/posts` | Post list |
| GET | `/app/settings` | App settings |

## Running

### Development

```bash
# Run migrations
cargo run -p migration

# Start server
cargo run -p axum-example-app
```

The server starts at `http://localhost:8000`.

### Generating Entities

```bash
sea-orm-cli generate entity \
  --output-dir ./entity/src \
  --lib \
  --entity-format dense \
  --with-serde both
```

## Implemented Features

- [x] Dashboard with statistics
- [x] User CRUD
- [x] Pagination (10 users/page)
- [x] RBAC matrix with permission toggles
- [x] Password hashing with Argon2
- [x] Audit logs (create, update, delete)
- [x] Login template (visual)
- [x] Structured logging with tracing

## Security

> ⚠️ **Warning**: This is a development version.

For production, implement:

- [ ] Authentication (JWT or sessions)
- [ ] CSRF tokens in forms
- [ ] Rate limiting
- [ ] Authorization middleware

## Logs

Logs are saved to `logs/app.log` with daily rotation.

```
2024-12-22T03:00:00 INFO ✅ Server running successfully
2024-12-22T03:00:00 INFO 🚀 listening on 127.0.0.1:8000
```

## Audit Logs

CRUD actions are recorded in the `audit_logs` table:

| Field | Type | Description |
|-------|------|-------------|
| id | UUID | Log ID |
| action | String | "create", "update", "delete" |
| entity_type | String | "user", "role", etc. |
| entity_id | UUID? | ID of the affected entity |
| user_id | UUID? | Who performed the action |
| details | Text? | JSON with additional details |
| ip_address | String? | Client IP |
| created_at | Timestamp | Action date/time |
