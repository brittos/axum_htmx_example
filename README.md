# APP - Bero Admin

A Proof of Concept (PoC) for a To-Do App using Axum, htmx, and SeaORM.

![ScreenShot](./images/Screenshot.png)

# Axum with SeaORM example app

1. Modify the `DATABASE_URL` var in `.env` to point to your chosen database

1. Turn on the appropriate database feature for your chosen db in `app/Cargo.toml` (the `"sqlx-postgres",` line)

1. Execute `cargo run` to start the server

1. Visit [localhost:8000](http://localhost:8000) in browser

Run tests:

```bash
# Run all tests (Unit + Integration)
cargo test -p axum-example-app

# Run specific integration tests
cargo test -p axum-example-app --test auth_tests
cargo test -p axum-example-app --test users_tests
```

Run migration:

```bash
cargo run -p migration -- up
```

Regenerate entity: Auto-generated, do not modify the entity folder.

```bash
sea-orm-cli generate entity --output-dir ./entity/src --lib --entity-format dense --with-serde both
```

## Configuration (Environment Variables)

| Variable | Description | Default |
|----------|-------------|---------|
| `ENABLE_FILE_LOGGING` | Enable rolling file logging in `logs/*.log` | `false` |
| `SQLX_LOGGING` | Enable SQLx query logging | `false` |
| `SQLX_LOG_LEVEL` | Level for SQLx logs (`info`, `debug`, `error`, etc) | `info` |
