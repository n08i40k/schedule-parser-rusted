use self::schema::*;
use crate::AppState;
use crate::app_state::Schedule;
use schedule_parser::parse_xls;
use crate::routes::schedule::schema::CacheStatus;
use crate::routes::schema::{IntoResponseAsError, ResponseError};
use crate::xls_downloader::interface::{FetchError, XLSDownloader};
use actix_web::web::Json;
use actix_web::{patch, web};
use chrono::Utc;

#[utoipa::path(responses(
    (status = OK, body = CacheStatus),
    (status = NOT_ACCEPTABLE, body = ResponseError<ErrorCode>),
))]
#[patch("/update-download-url")]
pub async fn update_download_url(
    data: Json<Request>,
    app_state: web::Data<AppState>,
) -> ServiceResponse {
    if !data.url.starts_with("https://politehnikum-eng.ru/") {
        return ErrorCode::NonWhitelistedHost.into_response();
    }

    let mut downloader = app_state.downloader.lock().unwrap();

    if let Some(url) = &downloader.url {
        if url.eq(&data.url) {
            return Ok(CacheStatus::from(&app_state)).into();
        }
    }

    match downloader.set_url(data.url.clone()).await {
        Ok(fetch_result) => {
            let mut schedule = app_state.schedule.lock().unwrap();

            if schedule.is_some()
                && fetch_result.uploaded_at < schedule.as_ref().unwrap().updated_at
            {
                return ErrorCode::OutdatedSchedule.into_response();
            }

            match downloader.fetch(false).await {
                Ok(download_result) => match parse_xls(&download_result.data.unwrap()) {
                    Ok(data) => {
                        *schedule = Some(Schedule {
                            etag: download_result.etag,
                            fetched_at: download_result.requested_at,
                            updated_at: download_result.uploaded_at,
                            parsed_at: Utc::now(),
                            data,
                        });

                        Ok(CacheStatus::from(schedule.as_ref().unwrap())).into()
                    }
                    Err(error) => {
                        sentry::capture_error(&error);

                        ErrorCode::InvalidSchedule(error).into_response()
                    }
                },
                Err(error) => {
                    if let FetchError::Unknown(error) = &error {
                        sentry::capture_error(&error);
                    }

                    ErrorCode::DownloadFailed(error).into_response()
                }
            }
        }
        Err(error) => {
            if let FetchError::Unknown(error) = &error {
                sentry::capture_error(&error);
            }

            ErrorCode::FetchFailed(error).into_response()
        }
    }
}

mod schema {
    use schedule_parser::schema::ParseError;
    use crate::routes::schedule::schema::CacheStatus;
    use actix_macros::{IntoResponseErrorNamed, StatusCode};
    use derive_more::Display;
    use serde::{Deserialize, Serialize, Serializer};
    use utoipa::ToSchema;
    use crate::xls_downloader::interface::FetchError;

    pub type ServiceResponse = crate::routes::schema::Response<CacheStatus, ErrorCode>;

    #[derive(Serialize, Deserialize, ToSchema)]
    pub struct Request {
        /// Schedule link.
        pub url: String,
    }

    #[derive(Clone, ToSchema, StatusCode, Display, IntoResponseErrorNamed)]
    #[status_code = "actix_web::http::StatusCode::NOT_ACCEPTABLE"]
    #[schema(as = SetDownloadUrl::ErrorCode)]
    pub enum ErrorCode {
        /// Transferred link with host different from politehnikum-eng.ru.
        #[display("URL with unknown host provided. Provide url with 'politehnikum-eng.ru' host.")]
        NonWhitelistedHost,

        /// Failed to retrieve file metadata.
        #[display("Unable to retrieve metadata from the specified URL: {_0}")]
        FetchFailed(FetchError),

        /// Failed to download the file.
        #[display("Unable to retrieve data from the specified URL: {_0}")]
        DownloadFailed(FetchError),

        /// The link leads to an outdated schedule.
        ///
        /// An outdated schedule refers to a schedule that was published earlier
        /// than is currently available.
        #[display("The schedule is older than it already is.")]
        OutdatedSchedule,

        /// Failed to parse the schedule.
        #[display("{_0}")]
        InvalidSchedule(ParseError),
    }

    impl Serialize for ErrorCode {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                ErrorCode::NonWhitelistedHost => serializer.serialize_str("NON_WHITELISTED_HOST"),
                ErrorCode::FetchFailed(_) => serializer.serialize_str("FETCH_FAILED"),
                ErrorCode::DownloadFailed(_) => serializer.serialize_str("DOWNLOAD_FAILED"),
                ErrorCode::OutdatedSchedule => serializer.serialize_str("OUTDATED_SCHEDULE"),
                ErrorCode::InvalidSchedule(_) => serializer.serialize_str("INVALID_SCHEDULE"),
            }
        }
    }
}
