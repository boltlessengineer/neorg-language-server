use crate::dirman::workspace::Workspace;
use std::path::PathBuf;

impl Workspace {
    pub fn files(&self) -> Vec<PathBuf> {
        walkdir::WalkDir::new(&self.path)
            .min_depth(1)
            .into_iter()
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let path = entry.path();
                if path.is_dir()
                    || path.file_name()?.to_string_lossy().starts_with('.')
                    || !path.extension().is_some_and(|ext| ext == "norg")
                {
                    return None;
                }
                Some(entry)
            })
            .map(|file| file.into_path())
            .collect()
    }
}
