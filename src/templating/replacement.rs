use crate::file_metadata::FileMetadata;
use chrono::{DateTime, Datelike, Local, TimeZone, Utc};
use strum_macros::EnumIter;
use thiserror::Error;

#[derive(EnumIter)]
pub enum Replacement {
    UtcYear,
    UtcMonth,
    UtcDay,
    LocalYear,
    LocalMonth,
    LocalDay,
}

impl Replacement {
    pub fn can_replace(&self, template: impl AsRef<str>) -> bool {
        template.as_ref().contains(self.get_variable_name())
    }

    pub fn replace(
        &self,
        template: impl AsRef<str>,
        metadata: &FileMetadata,
    ) -> Result<String, ReplacementError> {
        let name = self.get_variable_name();
        let value = self.get_value(metadata)?;
        Ok(template.as_ref().replace(name, &value))
    }

    fn get_variable_name(&self) -> &'static str {
        match self {
            Self::UtcYear => "{year_utc}",
            Self::UtcMonth => "{month_utc}",
            Self::UtcDay => "{day_utc}",
            Self::LocalYear => "{year_local}",
            Self::LocalMonth => "{month_local}",
            Self::LocalDay => "{day_local}",
        }
    }

    fn get_value(&self, metadata: &FileMetadata) -> Result<String, ReplacementError> {
        let result = match self {
            Replacement::UtcYear => get_date_time(metadata.file_created_time, Utc)?
                .year()
                .to_string(),
            Replacement::UtcMonth => get_date_time(metadata.file_created_time, Utc)?
                .month()
                .to_string(),
            Replacement::UtcDay => get_date_time(metadata.file_created_time, Utc)?
                .day()
                .to_string(),
            Replacement::LocalYear => get_date_time(metadata.file_created_time, Local)?
                .year()
                .to_string(),
            Replacement::LocalMonth => get_date_time(metadata.file_created_time, Local)?
                .month()
                .to_string(),
            Replacement::LocalDay => get_date_time(metadata.file_created_time, Local)?
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
