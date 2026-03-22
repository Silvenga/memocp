use crate::models::FileMetadata;
use crate::templating::template::Template;
use std::path::{Path, PathBuf};

pub struct Templater {
    source_path: PathBuf,
    destination_path_tmpl: Template,
}

impl Templater {
    pub fn new(source_path: impl AsRef<Path>, destination_path_tmpl: impl AsRef<str>) -> Self {
        Self {
            source_path: source_path.as_ref().to_path_buf(),
            destination_path_tmpl: Template::build(destination_path_tmpl),
        }
    }

    pub fn render_destination(
        &self,
        file: impl AsRef<Path>,
        metadata: &FileMetadata,
    ) -> anyhow::Result<PathBuf> {
        let mut destination = PathBuf::new();

        let destination_prefix = self.destination_path_tmpl.render(metadata)?;
        destination.push(destination_prefix);

        let relative_path = file.as_ref().strip_prefix(&self.source_path)?;
        destination.push(relative_path);

        Ok(destination)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::Hash;
    use crate::models::FileMetadata;
    use std::path::PathBuf;

    #[test]
    fn when_valid_path_then_render_destination_should_return_correct_path() {
        let source_path = PathBuf::from("/source");
        let dest_tmpl = "dest";
        let templater = Templater::new(&source_path, dest_tmpl);
        let file_path = source_path.join("sub/file.txt");
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: 0,
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };

        let result = templater.render_destination(&file_path, &metadata).unwrap();

        assert_eq!(result, PathBuf::from("dest/sub/file.txt"));
    }

    #[test]
    fn when_template_has_tokens_then_render_destination_should_return_rendered_path() {
        let source_path = PathBuf::from("/source");
        let dest_tmpl = "dest/{year_utc}";
        let templater = Templater::new(&source_path, dest_tmpl);
        let file_path = source_path.join("file.txt");
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: 1_700_000_000_000_000_000, // 2023-11-14T22:13:20Z
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };

        let result = templater.render_destination(&file_path, &metadata).unwrap();

        assert_eq!(result, PathBuf::from("dest/2023/file.txt"));
    }

    #[test]
    fn when_file_outside_source_then_render_destination_should_error() {
        let source_path = PathBuf::from("/source");
        let dest_tmpl = "dest";
        let templater = Templater::new(&source_path, dest_tmpl);
        let file_path = PathBuf::from("/other/file.txt");
        let metadata = FileMetadata {
            file_size_bytes: 0,
            file_modified_time: 0,
            file_created_time: 0,
            file_hash: Hash::empty_hash(),
        };

        let result = templater.render_destination(&file_path, &metadata);

        assert!(result.is_err());
    }
}
