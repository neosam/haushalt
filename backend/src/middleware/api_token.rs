//! Authentication middleware for programmatic API tokens.
//!
//! Wrapped around the `/households` scope, this is the single enforcement point that lets an
//! external system reach the household API with a token instead of a JWT — without touching
//! any of the dozens of existing handlers.
//!
//! For a request carrying `Authorization: Bearer hht_...` it:
//! 1. validates the token → its creator, household and write permission;
//! 2. rejects it unless the household in the URL is the token's household;
//! 3. rejects a write method (anything but GET/HEAD/OPTIONS) on a read-only token;
//! 4. mints a short-lived access token for the creator and REWRITES the Authorization
//!    header, so every downstream `extract_user_id`, `is_member` and role check runs exactly
//!    as it does for that logged-in user — no more, no less.
//!
//! Any request that is not one of our tokens (a JWT, or nothing) passes straight through
//! untouched, so the JWT path is entirely unaffected.

use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header::{HeaderValue, AUTHORIZATION};
use actix_web::http::Method;
use actix_web::middleware::Next;
use actix_web::{web, Error, HttpResponse};
use uuid::Uuid;

use shared::ApiError;

use crate::models::AppState;
use crate::services::{api_tokens, auth as auth_service};

/// The minted access token is decoded by the handler in the same request, so it only has to
/// outlive one call. Kept short so a rewritten header is never useful if it somehow escapes.
const MINTED_JWT_MINUTES: i64 = 5;

/// The `from_fn` middleware. See the module docs for the full contract.
pub async fn api_token_auth(
    mut req: ServiceRequest,
    next: Next<impl MessageBody + 'static>,
) -> Result<ServiceResponse<BoxBody>, Error> {
    // Not one of our tokens → normal JWT (or anonymous) request, left completely untouched.
    let Some(secret) = bearer_api_token(&req) else {
        return Ok(next.call(req).await?.map_into_boxed_body());
    };

    let Some(state) = req.app_data::<web::Data<AppState>>().cloned() else {
        return Ok(req.into_response(internal_error()));
    };

    let auth = match api_tokens::authenticate(&state.db, &secret).await {
        Ok(auth) => auth,
        Err(_) => return Ok(req.into_response(unauthorized())),
    };

    // A token is bound to one household; it may only reach that household's endpoints, never
    // account-level ones like listing or creating households.
    let Some(path_household) = household_id_from_path(req.path()) else {
        return Ok(req.into_response(forbidden(
            "API tokens can only access household-scoped endpoints",
        )));
    };
    if path_household != auth.household_id {
        return Ok(req.into_response(forbidden("Token is not valid for this household")));
    }

    // Read-only tokens may issue only safe (read) requests.
    if is_write_method(req.method()) && !auth.can_write {
        return Ok(req.into_response(forbidden("Token is read-only")));
    }

    // Hand the request to the existing handlers AS the token's creator.
    let jwt = match auth_service::create_access_token(
        &auth.user_id,
        &state.config.jwt_secret,
        MINTED_JWT_MINUTES,
    ) {
        Ok(jwt) => jwt,
        Err(_) => return Ok(req.into_response(internal_error())),
    };
    let Ok(header_value) = HeaderValue::from_str(&format!("Bearer {jwt}")) else {
        return Ok(req.into_response(internal_error()));
    };
    req.headers_mut().insert(AUTHORIZATION, header_value);

    Ok(next.call(req).await?.map_into_boxed_body())
}

/// The bearer secret, but only if it is one of ours (has the API-token prefix). Anything
/// else — a JWT, a malformed header, no header — returns `None` and is left for the JWT path.
fn bearer_api_token(req: &ServiceRequest) -> Option<String> {
    let value = req.headers().get(AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    token
        .starts_with(api_tokens::API_TOKEN_PREFIX)
        .then(|| token.to_string())
}

/// GET/HEAD/OPTIONS are reads; everything else (POST/PUT/PATCH/DELETE) is a write.
fn is_write_method(method: &Method) -> bool {
    !matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}

/// The household id in a path like `/api/households/{id}/tasks`, i.e. the segment right after
/// `households`. `None` for `/households` (list/create) — those are not household-scoped.
fn household_id_from_path(path: &str) -> Option<Uuid> {
    let mut segments = path.split('/').filter(|s| !s.is_empty());
    while let Some(segment) = segments.next() {
        if segment == "households" {
            return segments.next().and_then(|id| Uuid::parse_str(id).ok());
        }
    }
    None
}

fn unauthorized() -> HttpResponse {
    HttpResponse::Unauthorized().json(ApiError {
        error: "unauthorized".to_string(),
        message: "Invalid or missing token".to_string(),
    })
}

fn forbidden(message: &str) -> HttpResponse {
    HttpResponse::Forbidden().json(ApiError {
        error: "forbidden".to_string(),
        message: message.to_string(),
    })
}

fn internal_error() -> HttpResponse {
    HttpResponse::InternalServerError().json(ApiError {
        error: "internal_error".to_string(),
        message: "Failed to process request".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::models::AppState;
    use crate::test_utils::*;
    use actix_web::middleware::from_fn;
    use actix_web::{test as actix_test, App, HttpRequest};
    use shared::{CreateApiTokenRequest, Role};
    use sqlx::SqlitePool;

    // ---- pure helpers ----

    #[test]
    fn test_household_id_from_path() {
        let id = Uuid::new_v4();
        assert_eq!(
            household_id_from_path(&format!("/api/households/{id}/tasks")),
            Some(id)
        );
        assert_eq!(
            household_id_from_path(&format!("/api/households/{id}")),
            Some(id)
        );
        // No id → not household-scoped.
        assert_eq!(household_id_from_path("/api/households"), None);
        // Not a UUID in the id position.
        assert_eq!(household_id_from_path("/api/households/not-a-uuid"), None);
        // Unrelated path.
        assert_eq!(household_id_from_path("/api/users/me"), None);
    }

    #[test]
    fn test_is_write_method() {
        assert!(!is_write_method(&Method::GET));
        assert!(!is_write_method(&Method::HEAD));
        assert!(!is_write_method(&Method::OPTIONS));
        assert!(is_write_method(&Method::POST));
        assert!(is_write_method(&Method::PUT));
        assert!(is_write_method(&Method::DELETE));
        assert!(is_write_method(&Method::PATCH));
    }

    // ---- end-to-end middleware behaviour ----

    fn test_config() -> Config {
        Config {
            host: "127.0.0.1".to_string(),
            port: 8080,
            database_url: "sqlite::memory:".to_string(),
            jwt_secret: "test-secret".to_string(),
            access_token_expiration_minutes: 15,
            refresh_token_expiration_days: 30,
            static_files_path: None,
            cors_origins: vec![],
            legal_dir: None,
        }
    }

    fn app_state(pool: SqlitePool) -> web::Data<AppState> {
        web::Data::new(AppState {
            db: pool,
            config: test_config(),
            login_rate_limiter: std::sync::Arc::new(crate::middleware::RateLimiter::new(5, 900)),
            public_report_rate_limiter: std::sync::Arc::new(crate::middleware::RateLimiter::new(
                30, 60,
            )),
        })
    }

    /// A minimal stand-in for a real handler: it resolves the caller exactly as every real
    /// handler does (`extract_user_id` + `is_member`) and echoes the resolved user id, so a
    /// test can prove the middleware handed the request over as the token's creator.
    async fn ping(
        state: web::Data<AppState>,
        req: HttpRequest,
        path: web::Path<String>,
    ) -> HttpResponse {
        let Ok(user_id) = crate::middleware::auth::extract_user_id(&req, &state.config.jwt_secret)
        else {
            return HttpResponse::Unauthorized().finish();
        };
        let household_id = Uuid::parse_str(&path.into_inner()).unwrap();
        if !crate::services::households::is_member(&state.db, &household_id, &user_id)
            .await
            .unwrap_or(false)
        {
            return HttpResponse::Forbidden().finish();
        }
        HttpResponse::Ok().body(user_id.to_string())
    }

    /// Mount the middleware-wrapped `/households` scope with a couple of `ping` routes. Each
    /// test inits its own service from this so the service type never has to be named.
    fn mount(cfg: &mut web::ServiceConfig) {
        cfg.service(
            web::scope("/households")
                .wrap(from_fn(api_token_auth))
                .route("", web::get().to(ping)) // non-household-scoped (list)
                .route("/{id}/ping", web::get().to(ping))
                .route("/{id}/ping", web::post().to(ping)),
        );
    }

    /// Household + a member, plus a token for that member. Returns (household, member, secret).
    async fn seeded_token(pool: &SqlitePool, can_write: bool) -> (Uuid, Uuid, String) {
        let household_id = create_test_household(pool).await;
        let user_id = create_test_user(pool, "member@test.com", Role::Member).await;
        create_test_membership(pool, &household_id, &user_id, Role::Member).await;
        let created = api_tokens::create_token(
            pool,
            &user_id,
            CreateApiTokenRequest {
                household_id,
                name: "external".to_string(),
                can_write: Some(can_write),
            },
        )
        .await
        .unwrap();
        (household_id, user_id, created.secret)
    }

    #[actix_web::test]
    async fn test_read_token_reads_its_household_as_the_creator() {
        let pool = create_test_pool().await;
        let (household_id, user_id, secret) = seeded_token(&pool, false).await;
        let app = actix_test::init_service(App::new().app_data(app_state(pool)).configure(mount)).await;

        let request = actix_test::TestRequest::get()
            .uri(&format!("/households/{household_id}/ping"))
            .insert_header(("Authorization", format!("Bearer {secret}")))
            .to_request();
        let response = actix_test::call_service(&app, request).await;

        assert_eq!(response.status(), 200);
        let body = actix_test::read_body(response).await;
        // The request ran as the token's creator.
        assert_eq!(String::from_utf8(body.to_vec()).unwrap(), user_id.to_string());
    }

    #[actix_web::test]
    async fn test_read_only_token_cannot_write() {
        let pool = create_test_pool().await;
        let (household_id, _user_id, secret) = seeded_token(&pool, false).await;
        let app = actix_test::init_service(App::new().app_data(app_state(pool)).configure(mount)).await;

        let request = actix_test::TestRequest::post()
            .uri(&format!("/households/{household_id}/ping"))
            .insert_header(("Authorization", format!("Bearer {secret}")))
            .to_request();
        let response = actix_test::call_service(&app, request).await;

        assert_eq!(response.status(), 403);
    }

    #[actix_web::test]
    async fn test_write_token_can_write() {
        let pool = create_test_pool().await;
        let (household_id, _user_id, secret) = seeded_token(&pool, true).await;
        let app = actix_test::init_service(App::new().app_data(app_state(pool)).configure(mount)).await;

        let request = actix_test::TestRequest::post()
            .uri(&format!("/households/{household_id}/ping"))
            .insert_header(("Authorization", format!("Bearer {secret}")))
            .to_request();
        let response = actix_test::call_service(&app, request).await;

        assert_eq!(response.status(), 200);
    }

    #[actix_web::test]
    async fn test_token_cannot_reach_a_foreign_household() {
        let pool = create_test_pool().await;
        let (_household_id, _user_id, secret) = seeded_token(&pool, true).await;
        let foreign = create_test_household_with_name(&pool, "Foreign").await;
        let app = actix_test::init_service(App::new().app_data(app_state(pool)).configure(mount)).await;

        let request = actix_test::TestRequest::get()
            .uri(&format!("/households/{foreign}/ping"))
            .insert_header(("Authorization", format!("Bearer {secret}")))
            .to_request();
        let response = actix_test::call_service(&app, request).await;

        assert_eq!(response.status(), 403);
    }

    #[actix_web::test]
    async fn test_token_cannot_reach_non_household_scoped_endpoint() {
        let pool = create_test_pool().await;
        let (_household_id, _user_id, secret) = seeded_token(&pool, true).await;
        let app = actix_test::init_service(App::new().app_data(app_state(pool)).configure(mount)).await;

        // GET /households (the list) has no household id in the path.
        let request = actix_test::TestRequest::get()
            .uri("/households")
            .insert_header(("Authorization", format!("Bearer {secret}")))
            .to_request();
        let response = actix_test::call_service(&app, request).await;

        assert_eq!(response.status(), 403);
    }

    #[actix_web::test]
    async fn test_unknown_and_disabled_tokens_are_unauthorized() {
        let pool = create_test_pool().await;
        let (household_id, user_id, secret) = seeded_token(&pool, true).await;

        // Disable it.
        api_tokens::update_token(
            &pool,
            &api_tokens::list_tokens(&pool, &user_id).await.unwrap()[0].id,
            &user_id,
            shared::UpdateApiTokenRequest {
                enabled: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let app = actix_test::init_service(App::new().app_data(app_state(pool)).configure(mount)).await;

        for bearer in [secret.as_str(), "hht_bogus"] {
            let request = actix_test::TestRequest::get()
                .uri(&format!("/households/{household_id}/ping"))
                .insert_header(("Authorization", format!("Bearer {bearer}")))
                .to_request();
            let response = actix_test::call_service(&app, request).await;
            assert_eq!(response.status(), 401, "bearer: {bearer}");
        }
    }

    #[actix_web::test]
    async fn test_jwt_requests_pass_through_untouched() {
        let pool = create_test_pool().await;
        let household_id = create_test_household(&pool).await;
        let user_id = create_test_user(&pool, "jwt@test.com", Role::Member).await;
        create_test_membership(&pool, &household_id, &user_id, Role::Member).await;
        let jwt = auth_service::create_access_token(&user_id, "test-secret", 15).unwrap();
        let app = actix_test::init_service(App::new().app_data(app_state(pool)).configure(mount)).await;

        // A normal JWT still works, and can write, since the middleware ignores it entirely.
        let request = actix_test::TestRequest::post()
            .uri(&format!("/households/{household_id}/ping"))
            .insert_header(("Authorization", format!("Bearer {jwt}")))
            .to_request();
        let response = actix_test::call_service(&app, request).await;

        assert_eq!(response.status(), 200);
        let body = actix_test::read_body(response).await;
        assert_eq!(String::from_utf8(body.to_vec()).unwrap(), user_id.to_string());
    }
}
