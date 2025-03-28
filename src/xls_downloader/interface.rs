use chrono::{DateTime, Utc};

/// Ошибки получения данных XLS
#[derive(PartialEq, Debug)]
pub enum FetchError {
    /// Не установлена ссылка на файл
    NoUrlProvided,

    /// Неизвестная ошибка
    Unknown,

    /// Сервер вернул статус код отличающийся от 200
    BadStatusCode,

    /// Ссылка ведёт на файл другого типа
    BadContentType,

    /// Сервер не вернул ожидаемые заголовки
    BadHeaders,
}

/// Результат получения данных XLS
pub struct FetchOk {
    /// ETag объекта
    pub etag: String,

    /// Дата загрузки файла
    pub uploaded_at: DateTime<Utc>,

    /// Дата получения данных
    pub requested_at: DateTime<Utc>,

    /// Данные файла
    pub data: Option<Vec<u8>>,
}

impl FetchOk {
    /// Результат без контента файла
    pub fn head(etag: String, uploaded_at: DateTime<Utc>) -> Self {
        FetchOk {
            etag,
            uploaded_at,
            requested_at: Utc::now(),
            data: None,
        }
    }

    /// Полный результат
    pub fn get(etag: String, uploaded_at: DateTime<Utc>, data: Vec<u8>) -> Self {
        FetchOk {
            etag,
            uploaded_at,
            requested_at: Utc::now(),
            data: Some(data),
        }
    }
}

pub type FetchResult = Result<FetchOk, FetchError>;

pub trait XLSDownloader {
    /// Получение данных о файле, и, опционально, его контент
    async fn fetch(&self, head: bool) -> FetchResult;

    /// Установка ссылки на файл
    async fn set_url(&mut self, url: String) -> FetchResult;
}
