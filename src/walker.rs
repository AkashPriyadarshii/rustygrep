use ignore::WalkBuilder;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;

pub struct FileWalker {
    paths: Vec<PathBuf>,
    hidden: bool,
    no_ignore: bool,
    no_binary: bool,
    file_type: Option<String>,
    file_type_not: Option<String>,
    threads: usize,
}

impl FileWalker {
    pub fn new(
        paths: Vec<PathBuf>,
        hidden: bool,
        no_ignore: bool,
        no_binary: bool,
        file_type: Option<String>,
        file_type_not: Option<String>,
        threads: usize,
    ) -> Self {
        Self {
            paths,
            hidden,
            no_ignore,
            no_binary,
            file_type,
            file_type_not,
            threads,
        }
    }

    pub fn walk(&self) -> Vec<PathBuf> {
        let (tx, rx) = mpsc::channel();
        let thread_count = if self.threads == 0 {
            num_cpus()
        } else {
            self.threads
        };

        let hidden = self.hidden;
        let no_ignore = self.no_ignore;
        let no_binary = self.no_binary;
        let file_type = self.file_type.clone();
        let file_type_not = self.file_type_not.clone();
        let paths = self.paths.clone();

        thread::spawn(move || {
            let walker = WalkBuilder::new(&paths[0])
                .hidden(!hidden)
                .ignore(!no_ignore)
                .git_ignore(!no_ignore)
                .threads(thread_count)
                .build();

            for entry in walker {
                if let Ok(entry) = entry {
                    let path = entry.path();

                    if path.is_dir() {
                        continue;
                    }

                    if no_binary && is_binary(path) {
                        continue;
                    }

                    if let Some(ref ext) = file_type {
                        if !matches_type(path, ext) {
                            continue;
                        }
                    }

                    if let Some(ref ext) = file_type_not {
                        if matches_type(path, ext) {
                            continue;
                        }
                    }

                    let _ = tx.send(path.to_path_buf());
                }
            }
        });

        rx.iter().collect()
    }
}

fn is_binary(path: &Path) -> bool {
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.len() > 10_000_000 {
            return true;
        }
    }

    if let Ok(bytes) = std::fs::read(path) {
        let check_len = bytes.len().min(8192);
        for &byte in &bytes[..check_len] {
            if byte == 0 {
                return true;
            }
        }
    }

    false
}

fn matches_type(path: &Path, file_type: &str) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|ext| ext == file_type)
        .unwrap_or(false)
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}
