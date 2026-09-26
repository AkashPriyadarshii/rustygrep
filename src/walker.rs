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
            // max_filesize: skip the 10MB+ probe in is_binary for the
            // common case (walker never yields huge files when set).
            let mut builder = WalkBuilder::new(&paths[0]);
            builder
                .hidden(!hidden)
                .ignore(!no_ignore)
                .git_ignore(!no_ignore)
                .git_global(!no_ignore)
                .git_exclude(!no_ignore)
                .require_git(!no_ignore)
                .max_filesize(Some(10_000_000))
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

                if !Self::keep(path, no_binary, &file_type, &file_type_not) {
                    continue;
                }

                let _ = tx.send(path.to_path_buf());
            }
        });

        rx.iter().collect()
    }

    /// Parallel streaming walk: `f` runs on each file as it is found,
    /// on the walker's own worker threads. Returns files visited.
    /// Both phases truly overlap: search starts on the first yielded
    /// file while the walker keeps discovering the rest.
    pub fn walk_parallel<F>(&self, f: F) -> usize
    where
        F: Fn(&Path) + Sync,
    {
        use ignore::WalkState;
        let thread_count = if self.threads == 0 {
            num_cpus()
        } else {
            self.threads
        };
        let mut builder = WalkBuilder::new(&self.paths[0]);
        builder
            .hidden(!self.hidden)
            .ignore(!self.no_ignore)
            .git_ignore(!self.no_ignore)
            .git_global(!self.no_ignore)
            .git_exclude(!self.no_ignore)
            .require_git(!self.no_ignore)
            .max_filesize(Some(10_000_000))
            .threads(thread_count);
        for p in &self.paths[1..] {
            builder.add(p);
        }
        let count = std::sync::atomic::AtomicUsize::new(0);
        builder.build_parallel().run(|| {
            let f = &f;
            let count = &count;
            let file_type = &self.file_type;
            let file_type_not = &self.file_type_not;
            let no_binary = self.no_binary;
            Box::new(move |entry| {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => return WalkState::Continue,
                };
                if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                    return WalkState::Continue;
                }
                let path = entry.path();
                if !Self::keep(path, no_binary, file_type, file_type_not) {
                    return WalkState::Continue;
                }
                count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                f(path);
                WalkState::Continue
            })
        });
        count.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn keep(
        path: &Path,
        no_binary: bool,
        file_type: &Option<String>,
        file_type_not: &Option<String>,
    ) -> bool {
        // --no-binary is opt-IN (default: search everything,
        // binary filtered later by the UTF-8 check in search).
        if no_binary && is_binary(path) {
            return false;
        }
        if let Some(ref ext) = file_type {
            if !matches_type(path, ext) {
                return false;
            }
        }
        if let Some(ref ext) = file_type_not {
            if matches_type(path, ext) {
                return false;
            }
        }
        true
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
