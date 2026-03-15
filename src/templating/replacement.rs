use crate::models::FileMetadata;
use chrono::{DateTime, Datelike, Local, TimeZone, Utc};
use strum_macros::EnumIter;
use thiserror::Error;

#[derive(EnumIter, Debug)]
pub enum Replacement {
    UtcModifiedYear,
    UtcModifiedMonth,
    UtcModifiedDay,
    LocalModifiedYear,
    LocalModifiedMonth,
    LocalModifiedDay,
}

impl Replacement {
    pub fn can_replace(&self, template: impl AsRef<str>) -> bool {
        template.as_ref().contains(self.get_token())
    }

    pub fn replace(
        &self,
        template: impl AsRef<str>,
        metadata: &FileMetadata,
    ) -> Result<String, ReplacementError> {
        let token = self.get_token();
        let value = self.get_value(metadata)?;
        Ok(template.as_ref().replace(token, &value))
    }

    fn get_token(&self) -> &'static str {
        match self {
            Self::UtcModifiedYear => "{year_utc}",
            Self::UtcModifiedMonth => "{month_utc}",
            Self::UtcModifiedDay => "{day_utc}",
            Self::LocalModifiedYear => "{year_local}",
            Self::LocalModifiedMonth => "{month_local}",
            Self::LocalModifiedDay => "{day_local}",
        }
    }

    fn get_value(&self, metadata: &FileMetadata) -> Result<String, ReplacementError> {
        let result = match self {
            Replacement::UtcModifiedYear => get_date_time(metadata.file_modified_time, Utc)?
                .year()
                .to_string(),
            Replacement::UtcModifiedMonth => get_date_time(metadata.file_modified_time, Utc)?
                .month()
                .to_string(),
            Replacement::UtcModifiedDay => get_date_time(metadata.file_modified_time, Utc)?
                .day()
                .to_string(),
            Replacement::LocalModifiedYear => get_date_time(metadata.file_modified_time, Local)?
                .year()
                .to_string(),
            Replacement::LocalModifiedMonth => get_date_time(metadata.file_modified_time, Local)?
                .month()
                .to_string(),
            Replacement::LocalModifiedDay => get_date_time(metadata.file_modified_time, Local)?
                .day()
                .to_string(),
        };
        Ok(result)
    }
}

#[derive(Debug, Error)]
pub enum ReplacementError {
    #[error("Failed to convert the file creation date to a date.")]
    InvalidDate,
}

fn get_date_time<T: TimeZone>(
    ns_since_epoch: u128,
    tz: T,
) -> Result<DateTime<T>, ReplacementError> {
    let secs = (ns_since_epoch / 1_000_000_000) as i64;
    let nsec = (ns_since_epoch % 1_000_000_000) as u32;

    match tz.timestamp_opt(secs, nsec).latest() {
        None => Err(ReplacementError::InvalidDate),
        Some(date_time) => Ok(date_time),
    }
}
