use crate::dirman::workspace::Workspace;
use std::path::PathBuf;

impl Workspace {
    pub fn files(&self) -> Vec<PathBuf> {
        walkdir::WalkDir::new(&self.path)
            .min_depth(1)
            .into_iter()
            .filter(|e| match e {
                Ok(entry) => {
                    !entry.file_name().to_str().unwrap_or(".").starts_with('.')
                        && !entry.path().is_dir()
                }
                Err(_) => false,
            })
            .map(|file| file.unwrap().into_path())
            .collect()
    }
}
