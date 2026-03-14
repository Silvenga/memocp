use crate::file_metadata::FileMetadata;
use crate::templating::replacement::{Replacement, ReplacementError};
use strum::IntoEnumIterator;

pub struct Template {
    template: String,
    replacements: Vec<Replacement>,
}

impl Template {
    pub fn build(template: impl AsRef<str>) -> Self {
        Self {
            template: template.as_ref().to_owned(),
            replacements: Self::get_replacements(template),
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
