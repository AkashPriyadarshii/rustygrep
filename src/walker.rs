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
            let mut builder = WalkBuilder::new(&paths[0]);
            builder
                .hidden(!hidden)
                .ignore(!no_ignore)
                .git_ignore(!no_ignore)
                .threads(thread_count);
            for p in &paths[1..] {
                builder.add(p);
            }
            let walker = builder.build();

            for entry in walker.flatten() {
                let path = entry.path();

                if entry.file_type().is_some_and(|ft| ft.is_dir()) {
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
        });

        rx.iter().collect()
    }
}

fn is_binary(path: &Path) -> bool {
    use std::io::Read;
    if let Ok(metadata) = std::fs::metadata(path) {
        if metadata.len() > 10_000_000 {
            return true;
        }
    }
    let mut f = match std::fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return true,
    };
    let mut buf = [0u8; 8192];
    let n = f.read(&mut buf).unwrap_or(0);
    buf[..n].contains(&0)
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
