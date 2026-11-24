# cinema-ecosystem-axum

Backend Rust (Axum 0.8, sqlx, redis) dùng chung PostgreSQL và Redis với backend Spring. Contract JSON, mã lỗi và thông báo tiếng Việt giữ nguyên như Spring.

## Chạy

```bash
cp .env.example .env
openssl rand -hex 32
cargo run
```

Dán chuỗi vừa tạo vào `JWT_SECRET` trong `.env`. Thiếu hoặc ngắn hơn 32 byte thì server từ chối khởi động. Nếu dùng chung DB với Spring thì đặt cùng `JWT_SECRET` để hai bên chấp nhận token của nhau.

Mặc định lắng nghe cổng 8081 (Spring dùng 8080). Biến môi trường trùng tên với Spring: `DB_HOST`, `DB_PORT`, `DB_NAME`, `DB_USER`, `DB_PASSWORD`, `REDIS_HOST`, `REDIS_PORT`, `MAIL_HOST`, `MAIL_PORT`, `MAIL_USERNAME`, `MAIL_PASSWORD`, `MAIL_FROM`. Có thể đặt thẳng `DATABASE_URL` hoặc `REDIS_URL`.

| Biến | Mặc định | Ý nghĩa |
|---|---|---|
| `JWT_SECRET` | bắt buộc | Khóa ký HS256, tối thiểu 32 byte |
| `JWT_ACCESS_TOKEN_EXPIRATION_SECONDS` | `900` | Thời hạn access token (15 phút) |
| `JWT_REFRESH_TOKEN_EXPIRATION_SECONDS` | `604800` | Thời hạn refresh token (7 ngày) |
| `COOKIE_SECURE` | `false` | Thêm cờ `Secure` cho cookie. Đặt `true` khi chạy HTTPS |
| `FRONTEND_BASE_URL` | `http://localhost:5173` | Gốc liên kết trong email đặt lại mật khẩu và xác thực email |
| `GOOGLE_CLIENT_ID` | client ID mặc định của Spring | Client ID dùng để kiểm tra `aud` của Google ID token. Để trống thì `/auth/google` trả `GOOGLE_LOGIN_DISABLED` |
| `MAIL_STARTTLS` | `false` | Bật STARTTLS khi gửi qua SMTP thật (Gmail, cổng 587) |

Email gửi qua SMTP. Mặc định trỏ tới `localhost:1025`, khớp với dịch vụ `mailhog` trong `docker-compose.yml` (giao diện xem thư ở `http://localhost:8025`). Lỗi gửi email chỉ được ghi log, không làm hỏng request.

## Endpoint

Phân hệ `auth`, đường dẫn và hành vi như Spring:

| Phương thức | Đường dẫn | Điều kiện |
|---|---|---|
| POST | `/api/v1/auth/register` | công khai |
| POST | `/api/v1/auth/login` | công khai, đặt cookie `access_token` và `refresh_token` |
| POST | `/api/v1/auth/google` | công khai, nhận `{ idToken }` của Google, đặt cookie như đăng nhập thường |
| POST | `/api/v1/auth/refresh-token` | công khai, đọc cookie `refresh_token`, xoay vòng token |
| POST | `/api/v1/auth/forgot-password` | công khai, tối đa 3 yêu cầu mỗi giờ cho mỗi email |
| POST | `/api/v1/auth/reset-password` | công khai |
| POST | `/api/v1/auth/verify-email` | công khai |
| POST | `/api/v1/auth/resend-verification` | đăng nhập, tối đa 3 lần mỗi giờ |
| POST | `/api/v1/auth/logout` | đăng nhập, thu hồi họ refresh token hiện tại |
| POST | `/api/v1/auth/logout-all` | đăng nhập, thu hồi mọi thiết bị |
| GET, PUT | `/api/v1/users/me` | đăng nhập, hồ sơ cá nhân |
| PUT | `/api/v1/users/me/password` | đăng nhập, đổi mật khẩu và cấp phiên mới |

Phân hệ `cinema`, đường dẫn và hành vi như Spring. Cột quyền: P là công khai, M là `ROLE_MANAGER` hoặc `ROLE_ADMIN` (manager chỉ thấy chi nhánh được gán), A là chỉ `ROLE_ADMIN`.

| Phương thức | Đường dẫn | Quyền |
|---|---|---|
| GET | `/api/v1/cinemas/public?city=`, `/cinemas/public/cities`, `/cinemas/public/{slug}` | P |
| GET | `/api/v1/halls/{hallId}/seat-layout`, `/seat-types` | P |
| GET, POST | `/api/v1/admin/cinemas` (GET có `search`, `city`, `status`, `deleted`, `page`, `size`) | M, POST và `deleted=true`: A |
| GET, PUT | `/api/v1/admin/cinemas/{cinemaId}` | M, đổi `status`: A |
| DELETE, POST | `/api/v1/admin/cinemas/{cinemaId}`, `/admin/cinemas/{cinemaId}/restore` | A |
| GET | `/api/v1/admin/cinema-options` | M |
| GET, POST | `/api/v1/admin/cinema-facilities` | GET: M, ghi và `deleted=true`: A |
| PUT, DELETE, POST | `/api/v1/admin/cinema-facilities/{facilityId}`, `.../restore` | A |
| GET, POST | `/api/v1/admin/cinemas/{cinemaId}/halls` | M |
| PUT, DELETE, POST | `/api/v1/admin/halls/{hallId}`, `/admin/halls/{hallId}/restore` | M |
| GET, PUT | `/api/v1/admin/halls/{hallId}/seat-map` | M |
| GET | `/api/v1/admin/halls/{hallId}/seat-map/lock` | M |
| POST | `/api/v1/admin/halls/{hallId}/generate-seats` | M |

Danh mục rạp công khai được cache trong Redis với khóa `cinema:public:{phiên bản}:{tên}` (1 giờ), cùng khóa và cùng định dạng JSON với Spring nên hai bên đọc và làm mới cache của nhau. Phạm vi chi nhánh của nhân viên cache ở `branch-access:{userId}` (5 phút), cũng dùng chung khóa với Spring.

Khác: `GET /api/v1/health`.

Quy tắc bảo mật: token chỉ nằm trong cookie HttpOnly SameSite=Strict, không bao giờ trong body. Request đổi dữ liệu xác thực bằng cookie phải kèm header `X-Requested-With`, thiếu thì cookie bị bỏ qua. `Authorization: Bearer` vẫn được chấp nhận và ưu tiên hơn cookie. Sai mật khẩu 5 lần thì khóa tài khoản 15 phút. Dùng lại refresh token đã xoay vòng quá 10 giây thì thu hồi cả họ token.

Đăng nhập Google: Axum tải khóa công khai của Google (cache 1 giờ), kiểm tra chữ ký RS256, `iss`, `aud`, hạn dùng và `email_verified`, rồi tạo hoặc liên kết tài khoản như Spring (email đã có mà chưa xác thực thì từ chối với `ACCOUNT_EXISTS_UNVERIFIED`). Origin của Vue phải được khai báo trong Google Cloud Console thì nút Google mới đăng nhập được.

Chưa port từ Spring: quản lý người dùng và nhân viên của admin (`/admin/users`, `/admin/staff`), nhật ký thao tác. Phạm vi chi nhánh đã có cho cinema (`BranchScope` trong module `auth`), chưa có phần gán chi nhánh cho nhân viên.

Dùng trong module khác: `use crate::modules::auth::AuthenticatedUser;` làm tham số handler để bắt buộc đăng nhập, rồi `authenticated_user.require_any_role(&["ROLE_ADMIN"])?` để kiểm tra vai trò.

## Cấu trúc

```text
src/
├── main.rs, lib.rs, app.rs, config.rs, state.rs
├── common/            lỗi, ApiResponse, extractor, kiểm tra dữ liệu, giới hạn tần suất
├── infrastructure/    kết nối hạ tầng dùng chung: database, redis, mailer
└── modules/<tên>/     mỗi module nghiệp vụ, giống module của Spring
```

Mỗi vai trò trong module tương ứng một thư mục của Spring: `handler` (controller), `dto/request`, `dto/response`, `entity`, `enums`, `model`, `repository`, `security`, `service`. Ví dụ module `auth`:

```text
modules/auth/
├── mod.rs              router() và các thứ module khác được dùng (AuthenticatedUser, ...)
├── handler/            auth_handler, profile_handler
├── dto/request/        mỗi request một file, kèm kiểm tra dữ liệu và test
├── dto/response/       user_response, login_response, user_profile_response, ...
├── entity/             bản ghi ánh xạ bảng: user_record, refresh_token_record, ...
├── enums/              user_status, user_token_type, gender
├── model/              auth_session
├── repository/         mỗi bảng một file: user_repository, refresh_token_repository, ...
├── security/           jwt_service, password_service, session_cookies, token_hasher, authenticated_user
└── service/            auth_service, user_profile_service, password_reset_service, ...
```

Quy ước khi thêm module: module nhỏ mà mỗi vai trò chỉ có một file thì để phẳng như `health` (`dto.rs`, `handler.rs`, `repository.rs`, `service.rs`). Khi một vai trò có từ hai file trở lên thì tách thành thư mục như `auth` và `cinema`. Module khác chỉ dùng những gì `mod.rs` của module đó `pub use`, không chui vào thư mục bên trong.

## Kiểm thử

```bash
cargo test
cargo clippy --all-targets
cargo fmt --check
```

`cargo test` chạy unit test và test HTTP không cần DB. Test luồng đầy đủ (đăng ký, đăng nhập, refresh, khóa tài khoản, đặt lại mật khẩu, giới hạn tần suất, dọn token, quản lý rạp, phòng, tiện ích và sơ đồ ghế) cần PostgreSQL đã nạp migration của Spring và Redis:

```bash
DATABASE_URL=postgres://user:pass@localhost:5432/cinema_test \
REDIS_URL=redis://localhost:6379 \
cargo test -- --ignored
```

Nên dùng DB và Redis riêng cho test (ví dụ container tạm), vì test tạo người dùng thật. Mỗi test dùng email ngẫu nhiên nên chạy song song được.

## Lưu ý

Lược đồ DB do Flyway của Spring quản lý. Dự án này không chạy migration, không dùng `sqlx migrate`.
