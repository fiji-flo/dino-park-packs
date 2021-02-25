use crate::api::error::ApiError;
use crate::db::operations;
use crate::db::Pool;
use crate::user::User;
use actix_multipart::Multipart;
use actix_web::dev::HttpServiceFactory;
use actix_web::web;
use actix_web::web::Bytes;
use actix_web::HttpResponse;
use cis_client::AsyncCisClientTrait;
use cis_profile::schema::Profile;
use futures::StreamExt;
use futures::TryFutureExt;
use futures::TryStreamExt;
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Serialize)]
pub struct UpdatedProfiles {
    updated: usize,
}

#[derive(Serialize)]
pub struct NotificationStatus {
    expire_first: usize,
    expire_second: usize,
}

async fn update_user<T: AsyncCisClientTrait>(
    pool: web::Data<Pool>,
    profile: web::Json<Profile>,
    cis_client: web::Data<T>,
) -> Result<HttpResponse, ApiError> {
    operations::users::update_user_cache(&pool, &profile, Arc::clone(&*cis_client)).await?;
    Ok(HttpResponse::Ok().json(""))
}

async fn delete_user(
    pool: web::Data<Pool>,
    user_uuid: web::Path<Uuid>,
) -> Result<HttpResponse, ApiError> {
    let user = User {
        user_uuid: user_uuid.into_inner(),
    };
    operations::users::delete_user(&pool, &user)
        .map(|_| HttpResponse::Ok().json(""))
        .map_err(Into::into)
}

async fn expire_all<T: AsyncCisClientTrait>(
    pool: web::Data<Pool>,
    cis_client: web::Data<T>,
) -> Result<HttpResponse, ApiError> {
    operations::expirations::expire_requests(&pool)?;
    operations::expirations::expire_invitations(&pool)?;
    operations::expirations::expire_memberships(&pool, Arc::clone(&*cis_client)).await?;
    Ok(HttpResponse::Ok().json(""))
}

async fn expiration_notifications(pool: web::Data<Pool>) -> Result<HttpResponse, ApiError> {
    let expire_first = operations::expirations::expiration_notification(&pool, true)?;
    let expire_second = operations::expirations::expiration_notification(&pool, false)?;
    Ok(HttpResponse::Ok().json(&NotificationStatus {
        expire_first,
        expire_second,
    }))
}

async fn requests_notifications(pool: web::Data<Pool>) -> Result<HttpResponse, ApiError> {
    operations::requests::pending_requests_notification(&pool)?;
    Ok(HttpResponse::Ok().json(""))
}

async fn all_notifications(pool: web::Data<Pool>) -> Result<HttpResponse, ApiError> {
    operations::requests::pending_requests_notification(&pool)?;
    operations::expirations::expiration_notification(&pool, true)?;
    operations::expirations::expiration_notification(&pool, false)?;
    Ok(HttpResponse::Ok().json(""))
}

async fn anonymous_notifications(pool: web::Data<Pool>) -> Result<HttpResponse, ApiError> {
    operations::members::notify_anonymous_members(&pool)?;
    Ok(HttpResponse::Ok().json(""))
}

async fn bulk_update_users(
    pool: web::Data<Pool>,
    mut multipart: Multipart,
) -> Result<HttpResponse, ApiError> {
    let mut updated = 0;
    while let Some(item) = multipart.next().await {
        let field = item.map_err(|_| ApiError::MultipartError)?;
        let buf = field
            .try_fold(
                Vec::<u8>::new(),
                |mut acc: Vec<u8>, bytes: Bytes| async move {
                    acc.extend(bytes.into_iter());
                    Ok(acc)
                },
            )
            .map_err(|_| ApiError::MultipartError)
            .await?;
        let profiles =
            serde_json::from_slice::<Vec<Profile>>(&buf).map_err(|_| ApiError::MultipartError)?;
        updated += operations::users::batch_update_user_cache(&pool, profiles)?;
    }
    Ok(HttpResponse::Ok().json(&UpdatedProfiles { updated }))
}

pub fn internal_app<T: AsyncCisClientTrait + 'static>() -> impl HttpServiceFactory {
    web::scope("/internal")
        .app_data(web::JsonConfig::default().limit(1_048_576))
        .service(web::resource("/update/bulk").route(web::post().to(bulk_update_users)))
        .service(web::resource("/update/user").route(web::post().to(update_user::<T>)))
        .service(web::resource("/delete/{user_uuid}").route(web::delete().to(delete_user)))
        .service(web::resource("/expire/all").route(web::post().to(expire_all::<T>)))
        .service(
            web::resource("/notify/expiration").route(web::post().to(expiration_notifications)),
        )
        .service(web::resource("/notify/requests").route(web::post().to(requests_notifications)))
        .service(web::resource("/notify/all").route(web::post().to(all_notifications)))
        .service(web::resource("/notify/anonymous").route(web::post().to(anonymous_notifications)))
}
