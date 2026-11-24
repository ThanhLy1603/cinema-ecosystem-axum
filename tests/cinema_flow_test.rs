mod common;

use axum::http::{Method, StatusCode};
use chrono::{Duration, Utc};
use common::{send, test_app, JsonResponse, RequestOptions, TestApp};
use serde_json::{json, Value};

const IGNORE_REASON: &str = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL";

fn database_url() -> String {
   std::env::var("DATABASE_URL").expect("DATABASE_URL")
}

fn redis_url() -> String {
   std::env::var("REDIS_URL").unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string())
}

fn app() -> TestApp {
   test_app(&database_url(), &redis_url())
}

fn unique_suffix() -> String {
   uuid::Uuid::new_v4().simple().to_string()[..10].to_string()
}

struct Actor {
   user_id: i64,
   token: String,
}

async fn create_actor(app: &TestApp, role_code: &str, cinema_ids: &[i64]) -> Actor {
   let email: String = format!("cinema-{}@example.com", unique_suffix());
   let user_id: i64 = sqlx::query_scalar::<_, i64>(
      "INSERT INTO users (email, full_name, email_verified) VALUES ($1, 'Người Kiểm Thử', TRUE) RETURNING id",
   )
   .bind(&email)
   .fetch_one(&app.app_state.pg_pool)
   .await
   .unwrap();
   sqlx::query("INSERT INTO user_roles (user_id, role_id) SELECT $1, id FROM roles WHERE code = $2")
      .bind(user_id)
      .bind(role_code)
      .execute(&app.app_state.pg_pool)
      .await
      .unwrap();
   for cinema_id in cinema_ids {
      sqlx::query("INSERT INTO user_branch_assignments (user_id, cinema_id, is_primary) VALUES ($1, $2, FALSE)")
         .bind(user_id)
         .bind(cinema_id)
         .execute(&app.app_state.pg_pool)
         .await
         .unwrap();
   }
   let token: String = app
      .app_state
      .jwt_service
      .generate_access_token(user_id, &email, &[role_code.to_string()], Utc::now(), 900)
      .unwrap();
   Actor { user_id, token }
}

async fn call(app: &TestApp, method: Method, uri: &str, token: Option<&str>, body: Option<Value>) -> JsonResponse {
   let mut request_options: RequestOptions = RequestOptions::new(method, uri);
   if let Some(token) = token {
      request_options = request_options.bearer(token);
   }
   if let Some(body) = body {
      request_options = request_options.json(body);
   }
   send(app.router.clone(), request_options).await
}

fn cinema_body(name: &str, facility_ids: &[i64]) -> Value {
   json!({
      "name": name, "city": "Thành phố Thử Nghiệm", "address": "1 Đường Kiểm Thử", "phone": "0901234567",
      "latitude": 10.5, "longitude": 106.25, "status": "ACTIVE", "facilityIds": facility_ids
   })
}

async fn seed_showtime(app: &TestApp, hall_id: i64) -> (i64, i64) {
   let slug: String = format!("phim-{}", unique_suffix());
   let movie_id: i64 = sqlx::query_scalar::<_, i64>(
      "INSERT INTO movies (title, duration_minutes, release_date, rated, slug) VALUES ($1, 100, CURRENT_DATE, 'P', $1) RETURNING id",
   )
   .bind(&slug)
   .fetch_one(&app.app_state.pg_pool)
   .await
   .unwrap();
   let start_time = Utc::now() + Duration::days(30);
   let showtime_id: i64 = sqlx::query_scalar::<_, i64>(
      "INSERT INTO showtimes (movie_id, hall_id, start_time, end_time, base_price) VALUES ($1, $2, $3, $4, 80000) RETURNING id",
   )
   .bind(movie_id)
   .bind(hall_id)
   .bind(start_time)
   .bind(start_time + Duration::minutes(100))
   .fetch_one(&app.app_state.pg_pool)
   .await
   .unwrap();
   (movie_id, showtime_id)
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn admin_endpoints_reject_anonymous_customers_and_malformed_ids() {
   let _ = IGNORE_REASON;
   let app: TestApp = app();
   let customer: Actor = create_actor(&app, "ROLE_CUSTOMER", &[]).await;
   let anonymous: JsonResponse = call(&app, Method::GET, "/api/v1/admin/cinemas", None, None).await;
   assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
   let forbidden: JsonResponse = call(&app, Method::GET, "/api/v1/admin/cinemas", Some(&customer.token), None).await;
   assert_eq!(forbidden.status, StatusCode::FORBIDDEN);
   assert_eq!(forbidden.body["errorCode"], "ACCESS_DENIED");
   let admin: Actor = create_actor(&app, "ROLE_ADMIN", &[]).await;
   let bad_id: JsonResponse = call(&app, Method::GET, "/api/v1/admin/cinemas/abc", Some(&admin.token), None).await;
   assert_eq!(bad_id.status, StatusCode::BAD_REQUEST);
   assert_eq!(bad_id.body["errorCode"], "INVALID_PARAMETER");
   assert_eq!(bad_id.body["message"], "Tham số 'cinemaId' không hợp lệ");
   let bad_flag: JsonResponse =
      call(&app, Method::GET, "/api/v1/admin/cinemas?deleted=maybe", Some(&admin.token), None).await;
   assert_eq!(bad_flag.body["message"], "Tham số 'deleted' không hợp lệ");
   let bad_status: JsonResponse =
      call(&app, Method::GET, "/api/v1/admin/cinemas?status=GONE", Some(&admin.token), None).await;
   assert_eq!(bad_status.body["message"], "Tham số 'status' không hợp lệ");
   let validation: JsonResponse =
      call(&app, Method::POST, "/api/v1/admin/cinemas", Some(&admin.token), Some(json!({}))).await;
   assert_eq!(validation.status, StatusCode::BAD_REQUEST);
   assert_eq!(validation.body["data"]["name"], "Vui lòng nhập tên rạp");
   let missing: JsonResponse =
      call(&app, Method::GET, "/api/v1/admin/cinemas/999999999", Some(&admin.token), None).await;
   assert_eq!(missing.status, StatusCode::NOT_FOUND);
   assert_eq!(missing.body["errorCode"], "CINEMA_NOT_FOUND");
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn admin_manages_facilities_cinemas_halls_and_seat_maps_end_to_end() {
   let app: TestApp = app();
   let admin: Actor = create_actor(&app, "ROLE_ADMIN", &[]).await;
   let token: &str = &admin.token;
   let suffix: String = unique_suffix();

   let facility_name: String = format!("Phòng chờ {suffix}");
   let created_facility: JsonResponse = call(
      &app,
      Method::POST,
      "/api/v1/admin/cinema-facilities",
      Some(token),
      Some(
         json!({ "name": facility_name, "iconUrl": "https://cdn.example.com/lounge.svg", "description": "Khu nghỉ" }),
      ),
   )
   .await;
   assert_eq!(created_facility.status, StatusCode::CREATED);
   assert_eq!(created_facility.body["message"], "Thêm tiện ích thành công");
   let facility_id: i64 = created_facility.body["data"]["id"].as_i64().unwrap();
   let duplicate_facility: JsonResponse = call(
      &app,
      Method::POST,
      "/api/v1/admin/cinema-facilities",
      Some(token),
      Some(json!({ "name": facility_name.to_uppercase() })),
   )
   .await;
   assert_eq!(duplicate_facility.status, StatusCode::CONFLICT);
   assert_eq!(duplicate_facility.body["errorCode"], "FACILITY_NAME_EXISTS");

   let cinema_name: String = format!("Rạp Thử Nghiệm {suffix}");
   let invalid_facility: JsonResponse = call(
      &app,
      Method::POST,
      "/api/v1/admin/cinemas",
      Some(token),
      Some(cinema_body(&cinema_name, &[facility_id, 987654321])),
   )
   .await;
   assert_eq!(invalid_facility.body["errorCode"], "INVALID_FACILITY");
   let mut lone_latitude: Value = cinema_body(&cinema_name, &[]);
   lone_latitude["longitude"] = Value::Null;
   let lone_response: JsonResponse =
      call(&app, Method::POST, "/api/v1/admin/cinemas", Some(token), Some(lone_latitude)).await;
   assert_eq!(lone_response.body["errorCode"], "INVALID_COORDINATES");

   let created_cinema: JsonResponse =
      call(&app, Method::POST, "/api/v1/admin/cinemas", Some(token), Some(cinema_body(&cinema_name, &[facility_id])))
         .await;
   assert_eq!(created_cinema.status, StatusCode::CREATED, "{}", created_cinema.body);
   let cinema_id: i64 = created_cinema.body["data"]["id"].as_i64().unwrap();
   let slug: String = created_cinema.body["data"]["slug"].as_str().unwrap().to_string();
   assert_eq!(slug, format!("rap-thu-nghiem-{suffix}"));
   assert_eq!(created_cinema.body["data"]["facilities"][0]["name"], facility_name);
   assert_eq!(created_cinema.body["data"]["halls"], json!([]));
   assert_eq!(created_cinema.body["data"]["latitude"], 10.5);

   let same_name: JsonResponse =
      call(&app, Method::POST, "/api/v1/admin/cinemas", Some(token), Some(cinema_body(&cinema_name, &[]))).await;
   assert_eq!(same_name.body["data"]["slug"], format!("{slug}-2"));
   let second_cinema_id: i64 = same_name.body["data"]["id"].as_i64().unwrap();

   let public_uri: String = format!("/api/v1/cinemas/public/{slug}");
   let public_before: JsonResponse = call(&app, Method::GET, &public_uri, None, None).await;
   assert_eq!(public_before.status, StatusCode::OK);
   assert_eq!(public_before.body["data"]["halls"], json!([]));

   let hall_uri: String = format!("/api/v1/admin/cinemas/{cinema_id}/halls");
   let invalid_hall: JsonResponse = call(&app, Method::POST, &hall_uri, Some(token), Some(json!({}))).await;
   assert_eq!(invalid_hall.body["data"]["screenType"], "Vui lòng chọn loại màn hình");
   let created_hall: JsonResponse = call(
      &app,
      Method::POST,
      &hall_uri,
      Some(token),
      Some(json!({ "name": "Phòng 1", "screenType": "IMAX", "status": "ACTIVE" })),
   )
   .await;
   assert_eq!(created_hall.status, StatusCode::CREATED);
   assert_eq!(created_hall.body["data"]["screenType"], "IMAX");
   assert_eq!(created_hall.body["data"]["totalSeats"], 0);
   let hall_id: i64 = created_hall.body["data"]["id"].as_i64().unwrap();
   let duplicate_hall: JsonResponse = call(
      &app,
      Method::POST,
      &hall_uri,
      Some(token),
      Some(json!({ "name": "PHÒNG 1", "screenType": "2D", "status": "ACTIVE" })),
   )
   .await;
   assert_eq!(duplicate_hall.body["errorCode"], "HALL_NAME_EXISTS");

   let public_after: JsonResponse = call(&app, Method::GET, &public_uri, None, None).await;
   assert_eq!(public_after.body["data"]["halls"][0]["name"], "Phòng 1", "cache công khai phải được làm mới");

   let seat_map_uri: String = format!("/api/v1/admin/halls/{hall_id}/seat-map");
   let generate_uri: String = format!("/api/v1/admin/halls/{hall_id}/generate-seats");
   let generated: JsonResponse = call(
      &app,
      Method::POST,
      &generate_uri,
      Some(token),
      Some(json!({ "rowCount": 3, "columnCount": 6, "seatTypeCode": "STANDARD" })),
   )
   .await;
   assert_eq!(generated.status, StatusCode::CREATED, "{}", generated.body);
   assert_eq!(generated.body["data"]["rows"], json!(["A", "B", "C"]));
   assert_eq!(generated.body["data"]["columns"], 6);
   assert_eq!(generated.body["data"]["seats"].as_array().unwrap().len(), 18);
   assert_eq!(generated.body["data"]["seats"][0]["type"], "STANDARD");
   let version: i64 = generated.body["data"]["version"].as_i64().unwrap();
   let regenerate: JsonResponse = call(
      &app,
      Method::POST,
      &generate_uri,
      Some(token),
      Some(json!({ "rowCount": 1, "columnCount": 1, "seatTypeCode": "STANDARD" })),
   )
   .await;
   assert_eq!(regenerate.body["errorCode"], "HALL_ALREADY_HAS_SEATS");

   let original_seat_ids: Vec<i64> =
      generated.body["data"]["seats"].as_array().unwrap().iter().map(|seat| seat["id"].as_i64().unwrap()).collect();
   let new_layout: Vec<Value> = vec![
      json!({ "row": "A", "number": 1, "seatTypeCode": "VIP", "status": "ACTIVE" }),
      json!({ "row": "A", "number": 2, "seatTypeCode": "SWEETBOX", "status": "ACTIVE" }),
      json!({ "row": "A", "number": 4, "seatTypeCode": "STANDARD", "status": "MAINTENANCE" }),
      json!({ "row": "B", "number": 1, "seatTypeCode": "STANDARD", "status": "ACTIVE" }),
   ];
   let stale: JsonResponse =
      call(&app, Method::PUT, &seat_map_uri, Some(token), Some(json!({ "version": version - 1, "seats": new_layout })))
         .await;
   assert_eq!(stale.status, StatusCode::CONFLICT);
   assert_eq!(stale.body["errorCode"], "SEAT_MAP_CONFLICT");
   let overlapping: JsonResponse = call(
      &app,
      Method::PUT,
      &seat_map_uri,
      Some(token),
      Some(json!({ "version": version, "seats": [
         { "row": "A", "number": 1, "seatTypeCode": "SWEETBOX", "status": "ACTIVE" },
         { "row": "A", "number": 2, "seatTypeCode": "STANDARD", "status": "ACTIVE" }
      ] })),
   )
   .await;
   assert_eq!(overlapping.body["errorCode"], "INVALID_SEAT_MAP");
   assert_eq!(overlapping.body["message"], "Ghế A2 chồng lên ghế A1");
   let saved: JsonResponse =
      call(&app, Method::PUT, &seat_map_uri, Some(token), Some(json!({ "version": version, "seats": new_layout })))
         .await;
   assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
   assert_eq!(saved.body["data"]["version"], version + 1);
   assert_eq!(saved.body["data"]["columns"], 4);
   assert_eq!(saved.body["data"]["rows"], json!(["A", "B"]));
   let saved_seats: &Vec<Value> = saved.body["data"]["seats"].as_array().unwrap();
   assert_eq!(saved_seats.len(), 4);
   assert_eq!(saved_seats[1]["span"], 2);
   assert_eq!(saved_seats[1]["surcharge"], 40000.0);
   assert_eq!(saved_seats[2]["status"], "MAINTENANCE");
   assert!(
      original_seat_ids.contains(&saved_seats[0]["id"].as_i64().unwrap()),
      "ghế giữ nguyên nhãn phải giữ nguyên id"
   );
   let soft_deleted: i64 = sqlx::query_scalar::<_, i64>("SELECT count(*) FROM seats WHERE hall_id = $1 AND is_deleted")
      .bind(hall_id)
      .fetch_one(&app.app_state.pg_pool)
      .await
      .unwrap();
   assert_eq!(soft_deleted, 14);
   let hall_row: (i32,) = sqlx::query_as("SELECT total_seats FROM halls WHERE id = $1")
      .bind(hall_id)
      .fetch_one(&app.app_state.pg_pool)
      .await
      .unwrap();
   assert_eq!(hall_row.0, 4);

   let public_layout: JsonResponse =
      call(&app, Method::GET, &format!("/api/v1/halls/{hall_id}/seat-layout"), None, None).await;
   assert_eq!(public_layout.status, StatusCode::OK);
   assert_eq!(public_layout.body["data"]["seats"].as_array().unwrap().len(), 4);
   let seat_types: JsonResponse = call(&app, Method::GET, "/api/v1/seat-types", None, None).await;
   assert_eq!(seat_types.body["data"][2]["code"], "SWEETBOX");
   assert_eq!(seat_types.body["data"][2]["span"], 2);

   let unlocked: JsonResponse = call(&app, Method::GET, &format!("{seat_map_uri}/lock"), Some(token), None).await;
   assert_eq!(unlocked.body["data"]["locked"], false);
   let (movie_id, showtime_id) = seed_showtime(&app, hall_id).await;
   let locked: JsonResponse = call(&app, Method::GET, &format!("{seat_map_uri}/lock"), Some(token), None).await;
   assert_eq!(locked.body["data"]["locked"], true);
   assert_eq!(locked.body["data"]["upcomingShowtimeCount"], 1);
   assert_eq!(locked.body["data"]["bookedShowtimeCount"], 0);
   let blocked_save: JsonResponse =
      call(&app, Method::PUT, &seat_map_uri, Some(token), Some(json!({ "version": version + 1, "seats": [] }))).await;
   assert_eq!(blocked_save.status, StatusCode::CONFLICT);
   assert_eq!(blocked_save.body["errorCode"], "SEAT_MAP_LOCKED");
   let blocked_hall_delete: JsonResponse =
      call(&app, Method::DELETE, &format!("/api/v1/admin/halls/{hall_id}"), Some(token), None).await;
   assert_eq!(blocked_hall_delete.body["errorCode"], "HALL_HAS_UPCOMING_SHOWTIMES");
   let blocked_cinema_delete: JsonResponse =
      call(&app, Method::DELETE, &format!("/api/v1/admin/cinemas/{cinema_id}"), Some(token), None).await;
   assert_eq!(blocked_cinema_delete.body["errorCode"], "CINEMA_HAS_UPCOMING_SHOWTIMES");

   sqlx::query("DELETE FROM showtimes WHERE id = $1").bind(showtime_id).execute(&app.app_state.pg_pool).await.unwrap();
   sqlx::query("DELETE FROM movies WHERE id = $1").bind(movie_id).execute(&app.app_state.pg_pool).await.unwrap();

   let updated_hall: JsonResponse = call(
      &app,
      Method::PUT,
      &format!("/api/v1/admin/halls/{hall_id}"),
      Some(token),
      Some(json!({ "name": "Phòng 1", "screenType": "4DX", "status": "MAINTENANCE" })),
   )
   .await;
   assert_eq!(updated_hall.body["data"]["screenType"], "4DX");
   let hidden_layout: JsonResponse =
      call(&app, Method::GET, &format!("/api/v1/halls/{hall_id}/seat-layout"), None, None).await;
   assert_eq!(hidden_layout.status, StatusCode::NOT_FOUND, "phòng bảo trì không được lộ ra công khai");

   let deleted_hall: JsonResponse =
      call(&app, Method::DELETE, &format!("/api/v1/admin/halls/{hall_id}"), Some(token), None).await;
   assert_eq!(deleted_hall.status, StatusCode::OK);
   assert_eq!(deleted_hall.body["message"], "Xóa phòng chiếu thành công");
   let deleted_halls: JsonResponse =
      call(&app, Method::GET, &format!("{hall_uri}?deleted=true"), Some(token), None).await;
   assert_eq!(deleted_halls.body["data"][0]["deleted"], true);
   let restored_hall: JsonResponse =
      call(&app, Method::POST, &format!("/api/v1/admin/halls/{hall_id}/restore"), Some(token), None).await;
   assert_eq!(restored_hall.body["data"]["deleted"], false);

   let updated_cinema: JsonResponse = call(
      &app,
      Method::PUT,
      &format!("/api/v1/admin/cinemas/{cinema_id}"),
      Some(token),
      Some(json!({
         "name": cinema_name, "city": "Thành phố Thử Nghiệm", "address": "2 Đường Mới", "status": "INACTIVE",
         "facilityIds": []
      })),
   )
   .await;
   assert_eq!(updated_cinema.status, StatusCode::OK, "{}", updated_cinema.body);
   assert_eq!(updated_cinema.body["data"]["facilities"], json!([]));
   assert_eq!(updated_cinema.body["data"]["latitude"], Value::Null);
   assert_eq!(updated_cinema.body["data"]["slug"], slug);
   assert_eq!(call(&app, Method::GET, &public_uri, None, None).await.status, StatusCode::NOT_FOUND);

   let search: JsonResponse = call(
      &app,
      Method::GET,
      &format!("/api/v1/admin/cinemas?search=th%E1%BB%AD%20nghi%E1%BB%87m%20{suffix}&size=1&page=0"),
      Some(token),
      None,
   )
   .await;
   assert_eq!(search.body["data"]["totalElements"], 2);
   assert_eq!(search.body["data"]["totalPages"], 2);
   assert_eq!(search.body["data"]["size"], 1);
   assert_eq!(search.body["data"]["content"][0]["hallCount"], 1);
   let only_inactive: JsonResponse =
      call(&app, Method::GET, &format!("/api/v1/admin/cinemas?search={suffix}&status=INACTIVE"), Some(token), None)
         .await;
   assert_eq!(only_inactive.body["data"]["totalElements"], 1);

   for deleted_cinema_id in [cinema_id, second_cinema_id] {
      let deleted: JsonResponse =
         call(&app, Method::DELETE, &format!("/api/v1/admin/cinemas/{deleted_cinema_id}"), Some(token), None).await;
      assert_eq!(deleted.status, StatusCode::OK, "{}", deleted.body);
   }
   let in_trash: JsonResponse =
      call(&app, Method::GET, &format!("/api/v1/admin/cinemas?search={suffix}&deleted=true"), Some(token), None).await;
   assert_eq!(in_trash.body["data"]["totalElements"], 2);
   let recreated: JsonResponse =
      call(&app, Method::POST, "/api/v1/admin/cinemas", Some(token), Some(cinema_body(&cinema_name, &[]))).await;
   assert_eq!(recreated.body["data"]["slug"], slug, "slug của rạp đã xóa được dùng lại");
   let restored: JsonResponse =
      call(&app, Method::POST, &format!("/api/v1/admin/cinemas/{cinema_id}/restore"), Some(token), None).await;
   assert_eq!(restored.status, StatusCode::OK, "{}", restored.body);
   assert_eq!(restored.body["data"]["slug"], format!("{slug}-2"), "slug trùng thì được cấp lại khi khôi phục");
   assert_eq!(restored.body["data"]["halls"][0]["name"], "Phòng 1");

   let deleted_facility: JsonResponse =
      call(&app, Method::DELETE, &format!("/api/v1/admin/cinema-facilities/{facility_id}"), Some(token), None).await;
   assert_eq!(deleted_facility.status, StatusCode::OK);
   let restored_facility: JsonResponse =
      call(&app, Method::POST, &format!("/api/v1/admin/cinema-facilities/{facility_id}/restore"), Some(token), None)
         .await;
   assert_eq!(restored_facility.body["data"]["deleted"], false);
}

#[tokio::test]
#[ignore = "cần DATABASE_URL (PostgreSQL đã chạy migration của Spring) và REDIS_URL"]
async fn manager_is_limited_to_assigned_branches_and_cannot_use_admin_only_actions() {
   let app: TestApp = app();
   let admin: Actor = create_actor(&app, "ROLE_ADMIN", &[]).await;
   let suffix: String = unique_suffix();
   let mut cinema_ids: Vec<i64> = Vec::new();
   for index in 0..2 {
      let created: JsonResponse = call(
         &app,
         Method::POST,
         "/api/v1/admin/cinemas",
         Some(&admin.token),
         Some(cinema_body(&format!("Chi nhánh {index} {suffix}"), &[])),
      )
      .await;
      cinema_ids.push(created.body["data"]["id"].as_i64().unwrap());
   }
   let manager: Actor = create_actor(&app, "ROLE_MANAGER", &[cinema_ids[0]]).await;
   let token: &str = &manager.token;
   let _ = manager.user_id;

   let own: JsonResponse =
      call(&app, Method::GET, &format!("/api/v1/admin/cinemas/{}", cinema_ids[0]), Some(token), None).await;
   assert_eq!(own.status, StatusCode::OK);
   let other: JsonResponse =
      call(&app, Method::GET, &format!("/api/v1/admin/cinemas/{}", cinema_ids[1]), Some(token), None).await;
   assert_eq!(other.status, StatusCode::FORBIDDEN);
   assert_eq!(other.body["errorCode"], "BRANCH_ACCESS_DENIED");

   let search: JsonResponse =
      call(&app, Method::GET, &format!("/api/v1/admin/cinemas?search={suffix}"), Some(token), None).await;
   assert_eq!(search.body["data"]["totalElements"], 1);
   assert_eq!(search.body["data"]["content"][0]["id"], cinema_ids[0]);

   let options: JsonResponse = call(&app, Method::GET, "/api/v1/admin/cinema-options", Some(token), None).await;
   let option_ids: Vec<i64> =
      options.body["data"].as_array().unwrap().iter().map(|option| option["id"].as_i64().unwrap()).collect();
   assert_eq!(option_ids, vec![cinema_ids[0]]);

   let create: JsonResponse =
      call(&app, Method::POST, "/api/v1/admin/cinemas", Some(token), Some(cinema_body("Rạp mới", &[]))).await;
   assert_eq!(create.status, StatusCode::FORBIDDEN);
   let trash: JsonResponse = call(&app, Method::GET, "/api/v1/admin/cinemas?deleted=true", Some(token), None).await;
   assert_eq!(trash.status, StatusCode::FORBIDDEN);
   let delete: JsonResponse =
      call(&app, Method::DELETE, &format!("/api/v1/admin/cinemas/{}", cinema_ids[0]), Some(token), None).await;
   assert_eq!(delete.status, StatusCode::FORBIDDEN);
   let facility: JsonResponse =
      call(&app, Method::POST, "/api/v1/admin/cinema-facilities", Some(token), Some(json!({ "name": "X" }))).await;
   assert_eq!(facility.status, StatusCode::FORBIDDEN);

   let mut keep_status: Value = cinema_body(&format!("Chi nhánh 0 {suffix}"), &[]);
   keep_status["address"] = json!("Địa chỉ mới");
   let edited: JsonResponse = call(
      &app,
      Method::PUT,
      &format!("/api/v1/admin/cinemas/{}", cinema_ids[0]),
      Some(token),
      Some(keep_status.clone()),
   )
   .await;
   assert_eq!(edited.status, StatusCode::OK, "{}", edited.body);
   keep_status["status"] = json!("INACTIVE");
   let flipped: JsonResponse =
      call(&app, Method::PUT, &format!("/api/v1/admin/cinemas/{}", cinema_ids[0]), Some(token), Some(keep_status))
         .await;
   assert_eq!(flipped.status, StatusCode::FORBIDDEN);
   assert_eq!(flipped.body["errorCode"], "ADMIN_ONLY");

   let hall_other: JsonResponse = call(
      &app,
      Method::POST,
      &format!("/api/v1/admin/cinemas/{}/halls", cinema_ids[1]),
      Some(token),
      Some(json!({ "name": "P", "screenType": "2D", "status": "ACTIVE" })),
   )
   .await;
   assert_eq!(hall_other.body["errorCode"], "BRANCH_ACCESS_DENIED");

   for cinema_id in cinema_ids {
      call(&app, Method::DELETE, &format!("/api/v1/admin/cinemas/{cinema_id}"), Some(&admin.token), None).await;
   }
}
