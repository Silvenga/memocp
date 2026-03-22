use crate::models::FileMetadata;
use crate::templating::replacement::{Replacement, ReplacementError};
use strum::IntoEnumIterator;

pub struct Template {
    template: String,
    replacements: Vec<Replacement>,
}

impl Template {
    pub fn build(template: impl AsRef<str>) -> Self {
        let template = template.as_ref().to_owned();
        let replacements = Self::get_replacements(&template);

        tracing::trace!(
            "Built template with {} replacements active: {:?}.",
            replacements.len(),
            replacements
        );

        Self {
            template,
            replacements,
        }
    }

    pub fn render(&self, metadata: &FileMetadata) -> Result<String, ReplacementError> {
        let mut result = self.template.clone();
        for replacement in &self.replacements {
            result = replacement.replace(&result, metadata)?;
        }
        Ok(result)
    }

    fn get_replacements(template: impl AsRef<str>) -> Vec<Replacement> {
        let mut replacements = Vec::default();
        let template = template.as_ref();
        for replacement in Replacement::iter() {
            if replacement.can_replace(template) {
                replacements.push(replacement);
            }
        }
        replacements.shrink_to_fit();
        replacements
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::Hash;
    use crate::models::FileMetadata;

    #[test]
    fn when_template_has_no_tokens_then_render_should_return_original_string() {
        let template = Template::build("path/to/file.txt");
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: 0,
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };

        let result = template.render(&metadata).unwrap();

        assert_eq!(result, "path/to/file.txt");
    }

    #[test]
    fn when_template_has_one_token_then_render_should_replace_token() {
        let template = Template::build("path/{year_utc}/file.txt");
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: 1_700_000_000_000_000_000, // 2023-11-14T22:13:20Z
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };

        let result = template.render(&metadata).unwrap();

        assert_eq!(result, "path/2023/file.txt");
    }

    #[test]
    fn when_template_has_multiple_tokens_then_render_should_replace_all_tokens() {
        let template = Template::build("path/{year_utc}/{year_utc}/file.txt");
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: 1_700_000_000_000_000_000, // 2023-11-14T22:13:20Z
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };

        let result = template.render(&metadata).unwrap();

        assert_eq!(result, "path/2023/2023/file.txt");
    }

    #[test]
    fn when_template_has_different_tokens_then_render_should_replace_all_different_tokens() {
        let template = Template::build("path/{year_utc}/{month_utc}/{day_utc}/file.txt");
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: 1_700_000_000_000_000_000, // 2023-11-14T22:13:20Z
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };

        let result = template.render(&metadata).unwrap();

        assert_eq!(result, "path/2023/11/14/file.txt");
    }

    #[test]
    fn when_file_modified_time_is_invalid_then_render_should_return_error() {
        let template = Template::build("path/{year_utc}/file.txt");
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: u128::MAX,
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };

        let result = template.render(&metadata);

        assert!(result.is_err());
    }
}
