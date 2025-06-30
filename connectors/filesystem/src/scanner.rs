use crate::models::{FilesystemFile, FilesystemPermissions, FilesystemSource};
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};
use walkdir::WalkDir;

pub struct FilesystemScanner {
    source: FilesystemSource,
}

impl FilesystemScanner {
    pub fn new(source: FilesystemSource) -> Self {
        Self { source }
    }

    pub async fn scan_directory(&self) -> Result<Vec<FilesystemFile>> {
        info!("Starting filesystem scan for source: {}", self.source.name);
        let mut files = Vec::new();

        if !self.source.base_path.exists() {
            return Err(anyhow::anyhow!(
                "Base path does not exist: {}",
                self.source.base_path.display()
            ));
        }

        if !self.source.base_path.is_dir() {
            return Err(anyhow::anyhow!(
                "Base path is not a directory: {}",
                self.source.base_path.display()
            ));
        }

        let walker = WalkDir::new(&self.source.base_path)
            .follow_links(false)
            .max_depth(100);

        for entry in walker {
            match entry {
                Ok(entry) => {
                    if let Some(file) = self.process_entry(entry).await? {
                        files.push(file);
                    }
                }
                Err(e) => {
                    warn!("Error walking directory: {}", e);
                    continue;
                }
            }
        }

        info!("Completed filesystem scan, found {} files", files.len());
        Ok(files)
    }

    async fn process_entry(&self, entry: walkdir::DirEntry) -> Result<Option<FilesystemFile>> {
        let path = entry.path().to_path_buf();
        let metadata = match entry.metadata() {
            Ok(m) => m,
            Err(e) => {
                warn!("Failed to get metadata for {}: {}", path.display(), e);
                return Ok(None);
            }
        };

        let is_directory = metadata.is_dir();

        // Skip directories for now, we only want files
        if is_directory {
            return Ok(None);
        }

        // Check if file should be included based on filters
        if !self.source.should_include_file(&path) {
            debug!("Skipping file due to filters: {}", path.display());
            return Ok(None);
        }

        // Check file size limit
        if let Some(max_size) = self.source.max_file_size_bytes {
            if metadata.len() > max_size {
                debug!(
                    "Skipping file due to size limit ({} > {}): {}",
                    metadata.len(),
                    max_size,
                    path.display()
                );
                return Ok(None);
            }
        }

        let name = entry.file_name().to_string_lossy().to_string();

        let mime_type = mime_guess::from_path(&path)
            .first_or_octet_stream()
            .to_string();

        let permissions = self.get_file_permissions(&path)?;

        let filesystem_file = FilesystemFile {
            path: path.clone(),
            name,
            size: metadata.len(),
            mime_type,
            created_time: metadata.created().ok(),
            modified_time: metadata.modified().ok(),
            is_directory,
            permissions,
        };

        debug!("Processed file: {}", path.display());
        Ok(Some(filesystem_file))
    }

    fn get_file_permissions(&self, path: &PathBuf) -> Result<FilesystemPermissions> {
        let metadata = fs::metadata(path)
            .with_context(|| format!("Failed to get metadata for {}", path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = metadata.permissions().mode();

            // Check owner permissions (assuming we're the owner for simplicity)
            let readable = (mode & 0o400) != 0;
            let writable = (mode & 0o200) != 0;
            let executable = (mode & 0o100) != 0;

            Ok(FilesystemPermissions {
                readable,
                writable,
                executable,
            })
        }

        #[cfg(windows)]
        {
            let readonly = metadata.permissions().readonly();

            Ok(FilesystemPermissions {
                readable: true, // Assume readable if we can access it
                writable: !readonly,
                executable: false, // Windows doesn't have simple executable bit
            })
        }
    }

    pub async fn read_file_content(&self, file: &FilesystemFile) -> Result<String> {
        if file.is_directory {
            return Ok(String::new());
        }

        // For large files, we might want to limit how much we read
        const MAX_CONTENT_SIZE: u64 = 10 * 1024 * 1024; // 10MB
        if file.size > MAX_CONTENT_SIZE {
            warn!(
                "File too large to read content: {} ({}MB)",
                file.path.display(),
                file.size / 1024 / 1024
            );
            return Ok(String::new());
        }

        // Only try to read text files
        if !self.is_text_file(&file.mime_type) {
            debug!("Skipping binary file: {}", file.path.display());
            return Ok(String::new());
        }

        match tokio::fs::read_to_string(&file.path).await {
            Ok(content) => {
                debug!("Read {} bytes from {}", content.len(), file.path.display());
                Ok(content)
            }
            Err(e) => {
                error!("Failed to read file {}: {}", file.path.display(), e);
                Ok(String::new())
            }
        }
    }

    fn is_text_file(&self, mime_type: &str) -> bool {
        mime_type.starts_with("text/")
            || matches!(
                mime_type,
                "application/json"
                    | "application/xml"
                    | "application/javascript"
                    | "application/x-sh"
                    | "application/x-python"
                    | "application/x-ruby"
            )
    }
}
