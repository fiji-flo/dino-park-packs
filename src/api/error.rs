use crate::error::PacksError;
use crate::rules::error::RuleError;
use actix_web::error::ResponseError;
use actix_web::HttpResponse;
use dino_park_trust::AALevelError;
use dino_park_trust::GroupsTrustError;
use dino_park_trust::TrustError;
use log::warn;
use serde_json::json;
use serde_json::Value;
use std::fmt::Display;

#[derive(Fail, Debug)]
pub enum ApiError {
    #[fail(display = "multipart_error")]
    MultipartError,
    #[fail(display = "bad_api_request")]
    GenericBadRequest(failure::Error),
    #[fail(display = "input_to_long")]
    InputToLong,
    #[fail(display = "{}", _0)]
    PacksError(PacksError),
    #[fail(display = "{}", _0)]
    RuleError(RuleError),
    #[fail(display = "scope_{}", _0)]
    ScopeError(TrustError),
    #[fail(display = "groups_scope_{}", _0)]
    GroupsScopeError(GroupsTrustError),
    #[fail(display = "aal_{}", _0)]
    AALevelError(AALevelError),
    #[fail(display = "invalid_query_parameters.")]
    InvalidQuery,
}

fn to_json_error(e: &impl Display) -> Value {
    json!({ "error": e.to_string() })
}

impl From<PacksError> for ApiError {
    fn from(e: PacksError) -> Self {
        ApiError::PacksError(e)
    }
}

impl From<TrustError> for ApiError {
    fn from(e: TrustError) -> Self {
        ApiError::ScopeError(e)
    }
}

impl From<GroupsTrustError> for ApiError {
    fn from(e: GroupsTrustError) -> Self {
        ApiError::GroupsScopeError(e)
    }
}

impl From<AALevelError> for ApiError {
    fn from(e: AALevelError) -> Self {
        ApiError::AALevelError(e)
    }
}

impl From<failure::Error> for ApiError {
    fn from(e: failure::Error) -> Self {
        let e = match e.downcast::<PacksError>() {
            Ok(e) => return ApiError::PacksError(e),
            Err(e) => e,
        };
        let e = match e.downcast::<RuleError>() {
            Ok(e) => return ApiError::RuleError(e),
            Err(e) => e,
        };
        ApiError::GenericBadRequest(e)
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            Self::GenericBadRequest(ref e) => {
                warn!("{}", e);
                HttpResponse::BadRequest().json(&to_json_error(&'?'))
            }
            Self::PacksError(ref e) => HttpResponse::BadRequest().json(&to_json_error(e)),
            Self::RuleError(ref e) => HttpResponse::Forbidden().json(&to_json_error(e)),
            Self::ScopeError(ref e) => HttpResponse::Forbidden().json(&to_json_error(e)),
            Self::GroupsScopeError(ref e) => HttpResponse::Forbidden().json(&to_json_error(e)),
            Self::AALevelError(ref e) => HttpResponse::Forbidden().json(&to_json_error(e)),
            Self::InputToLong => HttpResponse::BadRequest().json(&to_json_error(self)),
            _ => HttpResponse::InternalServerError().json(""),
        }
    }
}
