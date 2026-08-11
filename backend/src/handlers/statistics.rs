use actix_web::{web, HttpResponse, Result};
use chrono::NaiveDate;
use shared::ApiError;
use uuid::Uuid;

use crate::models::AppState;
use crate::services::{
    household_settings as settings_service, households as household_service,
    statistics as statistics_service,
};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/statistics")
            .route("/weekly", web::get().to(get_weekly_statistics))
            .route("/weekly/calculate", web::post().to(calculate_weekly_statistics))
            .route("/weekly/available", web::get().to(list_available_weeks))
            .route(
                "/weekly/recalculate",
                web::post().to(recalculate_weekly_statistics),
            )
            .route("/monthly", web::get().to(get_monthly_statistics))
            .route("/monthly/calculate", web::post().to(calculate_monthly_statistics))
            .route("/monthly/available", web::get().to(list_available_months))
            .route(
                "/monthly/recalculate",
                web::post().to(recalculate_monthly_statistics),
            ),
    );
}

#[derive(Debug, serde::Deserialize)]
pub struct WeeklyStatsQuery {
    pub week_start: Option<String>,
}

#[derive(Debug, serde::Deserialize)]
pub struct MonthlyStatsQuery {
    pub month: Option<String>,
}

/// Query for recalculating a whole range of periods
#[derive(Debug, serde::Deserialize)]
pub struct RecalculateQuery {
    pub from: String,
    pub to: String,
}

/// Resolve the caller and make sure they belong to the household.
/// Returns the error response to send back when they don't.
async fn require_member(
    state: &AppState,
    req: &actix_web::HttpRequest,
    household_id: &Uuid,
) -> Result<(), HttpResponse> {
    let user_id = crate::middleware::auth::extract_user_id(req, &state.config.jwt_secret)
        .map_err(|_| {
            HttpResponse::Unauthorized().json(ApiError {
                error: "unauthorized".to_string(),
                message: "Invalid or missing token".to_string(),
            })
        })?;

    if !household_service::is_member(&state.db, household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Err(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Not a member of this household".to_string(),
        }));
    }

    Ok(())
}

/// Parse the from/to pair of a recalculation request
fn parse_range(query: &RecalculateQuery) -> Result<(NaiveDate, NaiveDate), HttpResponse> {
    let parse = |value: &str| {
        NaiveDate::parse_from_str(value, "%Y-%m-%d").map_err(|_| {
            HttpResponse::BadRequest().json(ApiError {
                error: "invalid_date".to_string(),
                message: "Invalid date format. Use YYYY-MM-DD".to_string(),
            })
        })
    };

    Ok((parse(&query.from)?, parse(&query.to)?))
}

/// Map a statistics service error onto an HTTP response
fn range_error_response(error: statistics_service::StatisticsError) -> HttpResponse {
    use statistics_service::StatisticsError;

    match error {
        StatisticsError::InvalidRange => HttpResponse::BadRequest().json(ApiError {
            error: "invalid_range".to_string(),
            message: "End date must not be before start date".to_string(),
        }),
        StatisticsError::RangeTooLarge(periods) => HttpResponse::BadRequest().json(ApiError {
            error: "range_too_large".to_string(),
            message: format!(
                "Range covers {} periods, at most {} are allowed",
                periods,
                statistics_service::MAX_RECALCULATION_PERIODS
            ),
        }),
        other => {
            log::error!("Error recalculating statistics: {:?}", other);
            HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to recalculate statistics".to_string(),
            })
        }
    }
}

/// Build the response body for a completed recalculation
fn recalculation_response(periods: Vec<NaiveDate>) -> HttpResponse {
    HttpResponse::Ok().json(shared::ApiSuccess::new(
        shared::RecalculateStatisticsResponse {
            periods_calculated: periods.len(),
            first_period: periods.first().copied(),
            last_period: periods.last().copied(),
        },
    ))
}

async fn recalculate_weekly_statistics(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<RecalculateQuery>,
) -> Result<HttpResponse> {
    let household_id = path.into_inner();

    if let Err(response) = require_member(&state, &req, &household_id).await {
        return Ok(response);
    }

    let (from, to) = match parse_range(&query) {
        Ok(range) => range,
        Err(response) => return Ok(response),
    };

    let settings = settings_service::get_or_create_settings(&state.db, &household_id)
        .await
        .map_err(|e| {
            log::error!("Error getting settings: {:?}", e);
            actix_web::error::ErrorInternalServerError("Failed to get settings")
        })?;

    match statistics_service::recalculate_weekly_range(
        &state.db,
        &household_id,
        from,
        to,
        settings.week_start_day,
    )
    .await
    {
        Ok(periods) => Ok(recalculation_response(periods)),
        Err(e) => Ok(range_error_response(e)),
    }
}

async fn recalculate_monthly_statistics(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<RecalculateQuery>,
) -> Result<HttpResponse> {
    let household_id = path.into_inner();

    if let Err(response) = require_member(&state, &req, &household_id).await {
        return Ok(response);
    }

    let (from, to) = match parse_range(&query) {
        Ok(range) => range,
        Err(response) => return Ok(response),
    };

    match statistics_service::recalculate_monthly_range(&state.db, &household_id, from, to).await {
        Ok(periods) => Ok(recalculation_response(periods)),
        Err(e) => Ok(range_error_response(e)),
    }
}

async fn get_weekly_statistics(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<WeeklyStatsQuery>,
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

    // Verify membership
    if !household_service::is_member(&state.db, &household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Not a member of this household".to_string(),
        }));
    }

    // Parse week_start from query or use current week
    let week_start = match &query.week_start {
        Some(date_str) => match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => {
                return Ok(HttpResponse::BadRequest().json(ApiError {
                    error: "invalid_date".to_string(),
                    message: "Invalid date format. Use YYYY-MM-DD".to_string(),
                }));
            }
        },
        None => {
            // Get current week start based on household settings
            let settings = settings_service::get_or_create_settings(&state.db, &household_id)
                .await
                .map_err(|e| {
                    log::error!("Error getting settings: {:?}", e);
                    actix_web::error::ErrorInternalServerError("Failed to get settings")
                })?;
            let today = chrono::Local::now().date_naive();
            statistics_service::get_week_start(today, settings.week_start_day)
        }
    };

    match statistics_service::get_weekly_statistics(&state.db, &household_id, week_start).await {
        Ok(stats) => Ok(HttpResponse::Ok().json(shared::ApiSuccess::new(stats))),
        Err(e) => {
            log::error!("Error getting weekly statistics: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to get statistics".to_string(),
            }))
        }
    }
}

async fn calculate_weekly_statistics(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<WeeklyStatsQuery>,
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

    // Verify membership and role (at least admin to trigger calculation)
    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if role.is_none() {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Not a member of this household".to_string(),
        }));
    }

    // Parse week_start from query or use current week
    let week_start = match &query.week_start {
        Some(date_str) => match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => {
                return Ok(HttpResponse::BadRequest().json(ApiError {
                    error: "invalid_date".to_string(),
                    message: "Invalid date format. Use YYYY-MM-DD".to_string(),
                }));
            }
        },
        None => {
            let settings = settings_service::get_or_create_settings(&state.db, &household_id)
                .await
                .map_err(|e| {
                    log::error!("Error getting settings: {:?}", e);
                    actix_web::error::ErrorInternalServerError("Failed to get settings")
                })?;
            let today = chrono::Local::now().date_naive();
            statistics_service::get_week_start(today, settings.week_start_day)
        }
    };

    match statistics_service::calculate_weekly_statistics(&state.db, &household_id, week_start)
        .await
    {
        Ok(()) => {
            // Return the newly calculated statistics
            match statistics_service::get_weekly_statistics(&state.db, &household_id, week_start)
                .await
            {
                Ok(stats) => Ok(HttpResponse::Ok().json(shared::ApiSuccess::new(stats))),
                Err(e) => {
                    log::error!("Error getting weekly statistics: {:?}", e);
                    Ok(HttpResponse::InternalServerError().json(ApiError {
                        error: "internal_error".to_string(),
                        message: "Failed to get statistics".to_string(),
                    }))
                }
            }
        }
        Err(e) => {
            log::error!("Error calculating weekly statistics: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to calculate statistics".to_string(),
            }))
        }
    }
}

async fn list_available_weeks(
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

    if !household_service::is_member(&state.db, &household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Not a member of this household".to_string(),
        }));
    }

    match statistics_service::list_available_weeks(&state.db, &household_id).await {
        Ok(weeks) => Ok(HttpResponse::Ok().json(shared::ApiSuccess::new(weeks))),
        Err(e) => {
            log::error!("Error listing available weeks: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list available weeks".to_string(),
            }))
        }
    }
}

async fn get_monthly_statistics(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<MonthlyStatsQuery>,
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

    if !household_service::is_member(&state.db, &household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Not a member of this household".to_string(),
        }));
    }

    // Parse month from query or use current month
    let month = match &query.month {
        Some(date_str) => match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => {
                return Ok(HttpResponse::BadRequest().json(ApiError {
                    error: "invalid_date".to_string(),
                    message: "Invalid date format. Use YYYY-MM-DD".to_string(),
                }));
            }
        },
        None => chrono::Local::now().date_naive(),
    };

    match statistics_service::get_monthly_statistics(&state.db, &household_id, month).await {
        Ok(stats) => Ok(HttpResponse::Ok().json(shared::ApiSuccess::new(stats))),
        Err(e) => {
            log::error!("Error getting monthly statistics: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to get statistics".to_string(),
            }))
        }
    }
}

async fn calculate_monthly_statistics(
    state: web::Data<AppState>,
    req: actix_web::HttpRequest,
    path: web::Path<Uuid>,
    query: web::Query<MonthlyStatsQuery>,
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

    let role = household_service::get_member_role(&state.db, &household_id, &user_id).await;
    if role.is_none() {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Not a member of this household".to_string(),
        }));
    }

    let month = match &query.month {
        Some(date_str) => match NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            Ok(date) => date,
            Err(_) => {
                return Ok(HttpResponse::BadRequest().json(ApiError {
                    error: "invalid_date".to_string(),
                    message: "Invalid date format. Use YYYY-MM-DD".to_string(),
                }));
            }
        },
        None => chrono::Local::now().date_naive(),
    };

    match statistics_service::calculate_monthly_statistics(&state.db, &household_id, month).await {
        Ok(()) => {
            match statistics_service::get_monthly_statistics(&state.db, &household_id, month).await
            {
                Ok(stats) => Ok(HttpResponse::Ok().json(shared::ApiSuccess::new(stats))),
                Err(e) => {
                    log::error!("Error getting monthly statistics: {:?}", e);
                    Ok(HttpResponse::InternalServerError().json(ApiError {
                        error: "internal_error".to_string(),
                        message: "Failed to get statistics".to_string(),
                    }))
                }
            }
        }
        Err(e) => {
            log::error!("Error calculating monthly statistics: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to calculate statistics".to_string(),
            }))
        }
    }
}

async fn list_available_months(
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

    if !household_service::is_member(&state.db, &household_id, &user_id)
        .await
        .unwrap_or(false)
    {
        return Ok(HttpResponse::Forbidden().json(ApiError {
            error: "forbidden".to_string(),
            message: "Not a member of this household".to_string(),
        }));
    }

    match statistics_service::list_available_months(&state.db, &household_id).await {
        Ok(months) => Ok(HttpResponse::Ok().json(shared::ApiSuccess::new(months))),
        Err(e) => {
            log::error!("Error listing available months: {:?}", e);
            Ok(HttpResponse::InternalServerError().json(ApiError {
                error: "internal_error".to_string(),
                message: "Failed to list available months".to_string(),
            }))
        }
    }
}
