use self::schema::*;
use crate::AppState;
use crate::app_state::Schedule;
use crate::parser::parse_xls;
use crate::routes::schedule::schema::CacheStatus;
use crate::routes::schema::{IntoResponseAsError, ResponseError};
use crate::xls_downloader::interface::XLSDownloader;
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
                Ok(download_result) => match parse_xls(download_result.data.as_ref().unwrap()) {
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
                    Err(error) => ErrorCode::InvalidSchedule(error).into_response(),
                },
                Err(error) => {
                    eprintln!("Unknown url provided {}", data.url);
                    eprintln!("{:?}", error);

                    ErrorCode::DownloadFailed.into_response()
                }
            }
        }
        Err(error) => {
            eprintln!("Unknown url provided {}", data.url);
            eprintln!("{:?}", error);

            ErrorCode::FetchFailed.into_response()
        }
    }
}

mod schema {
    use crate::parser::schema::ParseError;
    use crate::routes::schedule::schema::CacheStatus;
    use actix_macros::{IntoResponseErrorNamed, StatusCode};
    use derive_more::Display;
    use serde::{Deserialize, Serialize, Serializer};
    use utoipa::ToSchema;

    pub type ServiceResponse = crate::routes::schema::Response<CacheStatus, ErrorCode>;

    #[derive(Serialize, Deserialize, ToSchema)]
    pub struct Request {
        /// Ссылка на расписание
        pub url: String,
    }

    #[derive(Clone, ToSchema, StatusCode, Display, IntoResponseErrorNamed)]
    #[status_code = "actix_web::http::StatusCode::NOT_ACCEPTABLE"]
    #[schema(as = SetDownloadUrl::ErrorCode)]
    pub enum ErrorCode {
        /// Передана ссылка с хостом отличающимся от politehnikum-eng.ru
        #[display("URL with unknown host provided. Provide url with politehnikum-eng.ru host.")]
        NonWhitelistedHost,

        /// Не удалось получить мета-данные файла
        #[display("Unable to retrieve metadata from the specified URL.")]
        FetchFailed,

        /// Не удалось скачать файл
        #[display("Unable to retrieve data from the specified URL.")]
        DownloadFailed,

        /// Ссылка ведёт на устаревшее расписание
        ///
        /// Под устаревшим расписанием подразумевается расписание, которое было опубликовано раньше, чем уже имеется на данный момент
        #[display("The schedule is older than it already is.")]
        OutdatedSchedule,

        /// Не удалось преобразовать расписание
        #[display("{}", "_0.display()")]
        InvalidSchedule(ParseError),
    }

    impl Serialize for ErrorCode {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                ErrorCode::NonWhitelistedHost => serializer.serialize_str("NON_WHITELISTED_HOST"),
                ErrorCode::FetchFailed => serializer.serialize_str("FETCH_FAILED"),
                ErrorCode::DownloadFailed => serializer.serialize_str("DOWNLOAD_FAILED"),
                ErrorCode::OutdatedSchedule => serializer.serialize_str("OUTDATED_SCHEDULE"),
                ErrorCode::InvalidSchedule(_) => serializer.serialize_str("INVALID_SCHEDULE"),
            }
        }
    }
}
