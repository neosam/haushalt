//! Public cross-household report links (Phase 6, PUBREP-01..07).
//!
//! Two distinct surfaces live here:
//!
//! 1. `/api/users/me/reports/...` — authenticated CRUD, scoped to the calling user.
//! 2. `/api/public/reports/{token}` — the unauthenticated read. This is the ONLY endpoint
//!    in the application that serves household data without a JWT, so everything it does
//!    is deliberately narrow: one lookup by token, plain text out, no error detail.
//!
//! The handlers stay thin; all logic lives in `services::public_reports` (D-19's rule from
//! the per-household report, followed here too).

use actix_web::{web, HttpResponse, Result};
use shared::{
    ApiError, ApiSuccess, CreatePublicReportRequest, PublicReportsResponse,
    UpdatePublicReportRequest,
};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::public_reports::{self as service, PublicReportError};

/// Authenticated report management. Registered inside the `/users` scope BEFORE its
/// `/{id}` routes, otherwise "me" would be parsed as a user id.
pub fn configure_user_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/me/reports", web::get().to(list_reports))
        .route("/me/reports", web::post().to(create_report))
        .route("/me/reports/{report_id}", web::get().to(get_report))
        .route("/me/reports/{report_id}", web::put().to(update_report))
        .route("/me/reports/{report_id}", web::delete().to(delete_report))
        .route(
            "/me/reports/{report_id}/token",
            web::post().to(regenerate_token),
        );
}

/// The unauthenticated read.
pub fn configure_public_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/public/reports").route("/{token}", web::get().to(get_public_report)));
}

// ============================================================================
// Shared plumbing
// ============================================================================

fn unauthorized() -> HttpResponse {
    HttpResponse::Unauthorized().json(ApiError {
        error: "unauthorized".to_string(),
        message: "Invalid or missing token".to_string(),
    })
}

fn require_user(
    state: &web::Data<AppState>,
    req: &actix_web::HttpRequest,
) -> std::result::Result<Uuid, HttpResponse> {
    crate::middleware::auth::extract_user_id(req, &state.config.jwt_secret)
        .map_err(|_| unauthorized())
}

/// Map a service error onto HTTP.
///
/// `NotFound` covers both "does not exist" and "belongs to somebody else" — the service
/// deliberately does not distinguish them, so neither does the response.
fn map_error(error: PublicReportError) -> HttpResponse {
    match error {
        PublicReportError::NotFound => HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Report not found".to_string(),
        }),
        PublicReportError::InvalidName => HttpResponse::BadRequest().json(ApiError {
            error: "invalid_name".to_string(),
            message: "Report name must not be empty and at most 100 characters".to_string(),
        }),
        PublicReportError::InvalidLanguage => HttpResponse::BadRequest().json(ApiError {
            error: "invalid_language".to_string(),
            message: "Unsupported language".to_string(),
        }),
        PublicReportError::NotAMemberOfHousehold(household_id) => {
            HttpResponse::Forbidden().json(ApiError {
                error: "forbidden".to_string(),
                message: format!("Not a member of household {household_id}"),
            })
        }
        PublicReportError::TooManyReports => HttpResponse::BadRequest().json(ApiError {
            error: "too_many_reports".to_string(),
            message: "Report limit reached".to_string(),
        }),
        // Details go to the server log only; the body stays generic.
        other => {
            log::error!("Public report error: {other:?}");
            HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to process report".to_string(),
            })
        }
    }
}

fn parse_report_id(raw: &str) -> std::result::Result<Uuid, HttpResponse> {
    Uuid::parse_str(raw).map_err(|_| {
        // A malformed id is answered like a missing one, for the same reason the service
        // hides ownership behind NotFound.
        HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Report not found".to_string(),
        })
    })
}

// ============================================================================
// Authenticated CRUD
// ============================================================================

async fn list_reports(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let user_id = match require_user(&state, &req) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    Ok(match service::list_reports(&state.db, &user_id).await {
        Ok(reports) => HttpResponse::Ok().json(ApiSuccess::new(PublicReportsResponse { reports })),
        Err(e) => map_error(e),
    })
}

async fn get_report(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = match require_user(&state, &req) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };
    let report_id = match parse_report_id(&path.into_inner()) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    Ok(
        match service::get_report(&state.db, &report_id, &user_id).await {
            Ok(report) => HttpResponse::Ok().json(ApiSuccess::new(report)),
            Err(e) => map_error(e),
        },
    )
}

async fn create_report(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    body: web::Json<CreatePublicReportRequest>,
) -> Result<HttpResponse> {
    let user_id = match require_user(&state, &req) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    Ok(
        match service::create_report(&state.db, &user_id, body.into_inner()).await {
            Ok(report) => HttpResponse::Created().json(ApiSuccess::new(report)),
            Err(e) => map_error(e),
        },
    )
}

async fn update_report(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
    body: web::Json<UpdatePublicReportRequest>,
) -> Result<HttpResponse> {
    let user_id = match require_user(&state, &req) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };
    let report_id = match parse_report_id(&path.into_inner()) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    Ok(
        match service::update_report(&state.db, &report_id, &user_id, body.into_inner()).await {
            Ok(report) => HttpResponse::Ok().json(ApiSuccess::new(report)),
            Err(e) => map_error(e),
        },
    )
}

/// PUBREP-05: mint a new token and invalidate every previously shared URL.
async fn regenerate_token(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = match require_user(&state, &req) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };
    let report_id = match parse_report_id(&path.into_inner()) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    Ok(
        match service::regenerate_token(&state.db, &report_id, &user_id).await {
            Ok(report) => HttpResponse::Ok().json(ApiSuccess::new(report)),
            Err(e) => map_error(e),
        },
    )
}

async fn delete_report(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = match require_user(&state, &req) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };
    let report_id = match parse_report_id(&path.into_inner()) {
        Ok(id) => id,
        Err(response) => return Ok(response),
    };

    Ok(
        match service::delete_report(&state.db, &report_id, &user_id).await {
            Ok(()) => HttpResponse::NoContent().finish(),
            Err(e) => map_error(e),
        },
    )
}

// ============================================================================
// The unauthenticated read (PUBREP-02, PUBREP-03, PUBREP-07)
// ============================================================================

/// Answer a public report request.
///
/// D-10: an unknown token, a malformed token and a disabled report all produce the same
/// bare 404, so an unauthenticated caller learns nothing about which tokens exist.
///
/// The rate limiter is recorded ONLY for tokens that resolve to a real report. That is the
/// point, not an oversight: the limiter is an in-memory map keyed by whatever arrives in the
/// URL, so counting unknown tokens would let anyone grow it without bound by requesting
/// random UUIDs. Restricting it to existing reports caps the map at the number of reports
/// that exist, while still throttling the only thing worth throttling — repeated reads of a
/// leaked link.
async fn get_public_report(
    state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let raw_token = path.into_inner();

    let Ok(token) = Uuid::parse_str(&raw_token) else {
        return Ok(not_found_plain());
    };

    let key = token.to_string();
    if !state.public_report_rate_limiter.check(&key) {
        return Ok(HttpResponse::TooManyRequests()
            .insert_header(("Retry-After", "60"))
            .content_type("text/plain; charset=utf-8")
            .body("Too many requests"));
    }

    match service::render_by_token(&state.db, &token, chrono::Utc::now()).await {
        Ok(text) => {
            state.public_report_rate_limiter.record(&key);
            Ok(HttpResponse::Ok()
                .content_type("text/plain; charset=utf-8")
                // PUBREP-07: the link is unlisted, not public — keep it out of search
                // indexes and out of shared caches.
                .insert_header(("X-Robots-Tag", "noindex, nofollow"))
                .insert_header(("Cache-Control", "no-store"))
                // The response is never markup, but a browser that sniffs it into HTML
                // would turn a task title into a script tag.
                .insert_header(("X-Content-Type-Options", "nosniff"))
                .body(text))
        }
        Err(PublicReportError::NotFound) => Ok(not_found_plain()),
        Err(e) => {
            log::error!("Error rendering public report: {e:?}");
            Ok(HttpResponse::InternalServerError()
                .content_type("text/plain; charset=utf-8")
                .body("Failed to generate report"))
        }
    }
}

fn not_found_plain() -> HttpResponse {
    HttpResponse::NotFound()
        .content_type("text/plain; charset=utf-8")
        .insert_header(("X-Robots-Tag", "noindex, nofollow"))
        .body("Report not found")
}

/// The authenticated CRUD surface is covered by `services::public_reports` — these tests
/// exist for the one endpoint the service layer cannot speak for: the unauthenticated read,
/// whose rate limiting, headers and 404 behaviour live in this module.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::middleware::RateLimiter;
    use crate::test_utils::*;
    use actix_web::{test, App};
    use shared::{CreatePublicReportRequest, Role};
    use sqlx::SqlitePool;
    use std::sync::Arc;

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

    /// `max_requests` requests per minute, so a test can drive the limiter to its edge.
    fn app_state(pool: SqlitePool, max_requests: usize) -> web::Data<AppState> {
        web::Data::new(AppState {
            db: pool,
            config: test_config(),
            login_rate_limiter: Arc::new(RateLimiter::new(5, 900)),
            public_report_rate_limiter: Arc::new(RateLimiter::new(max_requests, 60)),
        })
    }

    /// A user with one household, one daily task, and an enabled report over it.
    async fn seeded_report(pool: &SqlitePool) -> shared::PublicReport {
        let user_id = create_test_user(pool, "owner@example.com", Role::Member).await;
        let household_id = create_test_household_with_name(pool, "Kitchen").await;
        create_test_membership(pool, &household_id, &user_id, Role::Member).await;
        create_test_task(pool, &household_id)
            .with_title("Wash the dishes")
            .with_recurrence(shared::RecurrenceType::Daily)
            .build()
            .await;

        service::create_report(
            pool,
            &user_id,
            CreatePublicReportRequest {
                name: "Everything".to_string(),
                language: None,
                include_missed: None,
                separate_undated: None,
                household_ids: Some(vec![household_id]),
            },
        )
        .await
        .unwrap()
    }

    #[actix_web::test]
    async fn test_public_endpoint_serves_plain_text_with_protective_headers() {
        let pool = create_test_pool().await;
        let report = seeded_report(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(app_state(pool, 30))
                .service(web::scope("/api").configure(configure_public_routes)),
        )
        .await;

        let request = test::TestRequest::get()
            .uri(&format!("/api/public/reports/{}", report.token))
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), 200);
        let headers = response.headers().clone();
        assert_eq!(
            headers.get("content-type").unwrap(),
            "text/plain; charset=utf-8"
        );
        assert_eq!(headers.get("x-robots-tag").unwrap(), "noindex, nofollow");
        assert_eq!(headers.get("cache-control").unwrap(), "no-store");
        assert_eq!(headers.get("x-content-type-options").unwrap(), "nosniff");

        let body = test::read_body(response).await;
        let text = String::from_utf8(body.to_vec()).unwrap();
        // The report's own name titles the output, then the household blocks follow.
        assert!(
            text.starts_with("Everything\n==========\n\nDaily report — Kitchen"),
            "got: {text}"
        );
        assert!(text.contains("- Wash the dishes"), "got: {text}");
    }

    /// PUBREP-02: no credential, no token, no report — and the 404 must not hint at why.
    #[actix_web::test]
    async fn test_public_endpoint_404s_for_unknown_and_malformed_tokens() {
        let pool = create_test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(app_state(pool, 30))
                .service(web::scope("/api").configure(configure_public_routes)),
        )
        .await;

        for uri in [
            format!("/api/public/reports/{}", Uuid::new_v4()),
            "/api/public/reports/not-a-uuid".to_string(),
        ] {
            let request = test::TestRequest::get().uri(&uri).to_request();
            let response = test::call_service(&app, request).await;

            assert_eq!(response.status(), 404, "{uri}");
            let body = test::read_body(response).await;
            assert_eq!(&body[..], b"Report not found", "{uri}");
        }
    }

    #[actix_web::test]
    async fn test_public_endpoint_404s_for_a_disabled_report() {
        let pool = create_test_pool().await;
        let report = seeded_report(&pool).await;
        service::update_report(
            &pool,
            &report.id,
            &report.user_id,
            shared::UpdatePublicReportRequest {
                enabled: Some(false),
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let app = test::init_service(
            App::new()
                .app_data(app_state(pool, 30))
                .service(web::scope("/api").configure(configure_public_routes)),
        )
        .await;

        let request = test::TestRequest::get()
            .uri(&format!("/api/public/reports/{}", report.token))
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), 404);
    }

    /// PUBREP-07: the endpoint throttles repeated reads of a leaked link.
    #[actix_web::test]
    async fn test_public_endpoint_rate_limits_repeated_reads() {
        let pool = create_test_pool().await;
        let report = seeded_report(&pool).await;
        let app = test::init_service(
            App::new()
                .app_data(app_state(pool, 2))
                .service(web::scope("/api").configure(configure_public_routes)),
        )
        .await;

        let uri = format!("/api/public/reports/{}", report.token);
        for attempt in 1..=2 {
            let request = test::TestRequest::get().uri(&uri).to_request();
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), 200, "attempt {attempt}");
        }

        let request = test::TestRequest::get().uri(&uri).to_request();
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), 429);
        assert_eq!(response.headers().get("retry-after").unwrap(), "60");
    }

    /// The limiter's map is keyed by whatever arrives in the URL, so unknown tokens must
    /// never create an entry — otherwise anyone could grow it without bound by requesting
    /// random UUIDs.
    #[actix_web::test]
    async fn test_unknown_tokens_do_not_populate_the_rate_limiter() {
        let pool = create_test_pool().await;
        let state = app_state(pool, 30);
        let limiter = state.public_report_rate_limiter.clone();
        let app = test::init_service(
            App::new()
                .app_data(state)
                .service(web::scope("/api").configure(configure_public_routes)),
        )
        .await;

        let stranger = Uuid::new_v4();
        for _ in 0..5 {
            let request = test::TestRequest::get()
                .uri(&format!("/api/public/reports/{stranger}"))
                .to_request();
            let response = test::call_service(&app, request).await;
            assert_eq!(response.status(), 404);
        }

        // Untouched: still the full budget, i.e. nothing was recorded for this key.
        assert_eq!(limiter.remaining(&stranger.to_string()), 30);
    }

    /// The authenticated surface must not be reachable without a JWT.
    #[actix_web::test]
    async fn test_authenticated_routes_require_a_token() {
        let pool = create_test_pool().await;
        let app = test::init_service(
            App::new()
                .app_data(app_state(pool, 30))
                .service(web::scope("/api/users").configure(configure_user_routes)),
        )
        .await;

        let request = test::TestRequest::get()
            .uri("/api/users/me/reports")
            .to_request();
        let response = test::call_service(&app, request).await;

        assert_eq!(response.status(), 401);
    }
}
