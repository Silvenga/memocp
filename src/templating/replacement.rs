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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::Hash;
    use crate::models::FileMetadata;
    use chrono::{Datelike, Local, TimeZone, Utc};

    const TEST_TIMESTAMP: u128 = 1_700_000_000_000_000_000; // 2023-11-14T22:13:20Z

    #[test]
    fn when_template_contains_token_then_can_replace_should_return_true() {
        assert!(Replacement::UtcModifiedYear.can_replace("prefix_{year_utc}_suffix"));
        assert!(Replacement::UtcModifiedMonth.can_replace("prefix_{month_utc}_suffix"));
        assert!(Replacement::UtcModifiedDay.can_replace("prefix_{day_utc}_suffix"));
        assert!(Replacement::LocalModifiedYear.can_replace("prefix_{year_local}_suffix"));
        assert!(Replacement::LocalModifiedMonth.can_replace("prefix_{month_local}_suffix"));
        assert!(Replacement::LocalModifiedDay.can_replace("prefix_{day_local}_suffix"));
    }

    #[test]
    fn when_template_does_not_contain_token_then_can_replace_should_return_false() {
        assert!(!Replacement::UtcModifiedYear.can_replace("no_token_here"));
    }

    #[test]
    fn when_utc_modified_year_replace_called_then_it_should_replace_token_with_correct_value() {
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: TEST_TIMESTAMP, // 2023-11-14T22:13:20Z
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };

        let result = Replacement::UtcModifiedYear
            .replace("year: {year_utc}", &metadata)
            .unwrap();

        assert_eq!(result, "year: 2023");
    }

    #[test]
    fn when_utc_modified_month_replace_called_then_it_should_replace_token_with_correct_value() {
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: TEST_TIMESTAMP, // 2023-11-14T22:13:20Z
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };

        let result = Replacement::UtcModifiedMonth
            .replace("month: {month_utc}", &metadata)
            .unwrap();

        assert_eq!(result, "month: 11");
    }

    #[test]
    fn when_utc_modified_day_replace_called_then_it_should_replace_token_with_correct_value() {
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: TEST_TIMESTAMP, // 2023-11-14T22:13:20Z
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };

        let result = Replacement::UtcModifiedDay
            .replace("day: {day_utc}", &metadata)
            .unwrap();

        assert_eq!(result, "day: 14");
    }

    #[test]
    fn when_local_modified_year_replace_called_then_it_should_replace_token_with_correct_value() {
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: TEST_TIMESTAMP,
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };
        let expected_year = get_date_time(TEST_TIMESTAMP, Local)
            .unwrap()
            .year()
            .to_string();

        let result = Replacement::LocalModifiedYear
            .replace("year: {year_local}", &metadata)
            .unwrap();

        assert_eq!(result, format!("year: {expected_year}"));
    }

    #[test]
    fn when_local_modified_month_replace_called_then_it_should_replace_token_with_correct_value() {
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: TEST_TIMESTAMP,
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };
        let expected_month = get_date_time(TEST_TIMESTAMP, Local)
            .unwrap()
            .month()
            .to_string();

        let result = Replacement::LocalModifiedMonth
            .replace("month: {month_local}", &metadata)
            .unwrap();

        assert_eq!(result, format!("month: {expected_month}"));
    }

    #[test]
    fn when_local_modified_day_replace_called_then_it_should_replace_token_with_correct_value() {
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: TEST_TIMESTAMP,
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };
        let expected_day = get_date_time(TEST_TIMESTAMP, Local)
            .unwrap()
            .day()
            .to_string();

        let result = Replacement::LocalModifiedDay
            .replace("day: {day_local}", &metadata)
            .unwrap();

        assert_eq!(result, format!("day: {expected_day}"));
    }

    #[test]
    fn when_get_date_time_called_with_valid_timestamp_then_it_should_return_date_time() {
        let ns_since_epoch = TEST_TIMESTAMP;

        let result = get_date_time(ns_since_epoch, Utc).unwrap();

        assert_eq!(result, Utc.timestamp_opt(1_700_000_000, 0).unwrap());
    }

    #[test]
    fn when_get_date_time_called_with_invalid_timestamp_then_it_should_return_error() {
        let ns_since_epoch = u128::MAX;

        let result = get_date_time(ns_since_epoch, Utc);

        assert!(result.is_err());
    }
}
