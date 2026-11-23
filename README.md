# cinema-ecosystem-axum

Khung backend Rust (Axum 0.8, sqlx, redis) dùng chung PostgreSQL và Redis với backend Spring.

## Chạy

```bash
cp .env.example .env
cargo run
```

Mặc định lắng nghe cổng 8081 (Spring dùng 8080). Biến môi trường trùng tên với Spring: `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USER`, `DB_PASSWORD`, `REDIS_HOST`, `REDIS_PORT`. Có thể đặt thẳng `DATABASE_URL` hoặc `REDIS_URL`.

## Kiểm thử

```bash
cargo test
DATABASE_URL=postgres://... cargo test --test api -- --ignored
cargo clippy --all-targets
```

## Endpoint hiện có

- `GET /api/v1/health`
- `GET /api/v1/cinemas/public?city=`
- `GET /api/v1/cinemas/public/cities`

## Lưu ý

Lược đồ DB do Flyway của Spring quản lý. Dự án này không chạy migration, không dùng `sqlx migrate`.
