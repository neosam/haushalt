use actix_web::{web, HttpResponse, Result};
use shared::{ApiError, ApiSuccess, DailyReportResponse};
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{households as household_service, report as report_service};
use crate::services::report::ReportError;

/// D-16: exactly one endpoint, `GET /api/households/{household_id}/report`.
/// D-18: strictly today/yesterday — no `?date=` parameter, so there is no date input
/// to validate (T-04-C).
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/report").route("", web::get().to(get_report)));
}

/// D-19: thin handler — extract the user, authorize, call one service function, map the
/// `Result` to HTTP. No report logic lives here.
async fn get_report(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<Uuid>,
) -> Result<HttpResponse> {
    let user_id = match crate::middleware::auth::extract_user_id(&req, &state.config.jwt_secret) {
        Ok(id) => id,
        Err(_) => {
            return Ok(HttpResponse::Unauthorized().json(ApiError {
                error: "unauthorized".to_string(),
                message: "Invalid or missing token".to_string(),
            }));
        }
    };

    let household_id = path.into_inner();

    // T-04-A: deny by default at the HTTP boundary, exactly like every sibling handler.
    // The service re-checks membership too (defense in depth).
    if !household_service::is_member(&state.db, &household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Not a member of this household".to_string(),
        }));
    }

    // `Utc::now()` is injected here, at the edge, precisely so the service stays testable
    // with a pinned date (D-04/D-12).
    match report_service::generate_daily_report(
        &state.db,
        &household_id,
        &user_id,
        chrono::Utc::now(),
    )
    .await
    {
        // D-17: the body carries only the generated text.
        Ok(report_text) => Ok(HttpResponse::Ok().json(ApiSuccess::new(DailyReportResponse {
            report_text,
        }))),
        Err(ReportError::NotAMember) => Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Not a member of this household".to_string(),
        })),
        Err(ReportError::HouseholdNotFound) => Ok(HttpResponse::NotFound().json(ApiError {
            error: "not_found".to_string(),
            message: "Household not found".to_string(),
        })),
        // T-04-B: details go to the server log only; the body stays generic.
        Err(e) => {
            log::error!("Error generating daily report: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to generate report".to_string(),
            }))
        }
    }
}
