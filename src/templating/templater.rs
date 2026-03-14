use crate::file_metadata::FileMetadata;
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
