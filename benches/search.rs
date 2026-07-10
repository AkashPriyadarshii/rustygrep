use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::fs;
use tempfile::TempDir;

fn setup_test_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let dir_path = dir.path();

    let files = vec![
        ("src/main.rs", generate_rust_file(500)),
        ("src/lib.rs", generate_rust_file(300)),
        ("src/cli.rs", generate_rust_file(200)),
        ("src/search.rs", generate_rust_file(400)),
        ("src/output.rs", generate_rust_file(150)),
        ("tests/integration.rs", generate_rust_file(100)),
        ("examples/basic.rs", generate_rust_file(50)),
        ("benches/search.rs", generate_rust_file(75)),
    ];

    for (name, content) in files {
        let path = dir_path.join(name);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, content).unwrap();
    }

    dir
}

fn generate_rust_file(lines: usize) -> String {
    let mut content = String::new();
    content.push_str("use std::collections::HashMap;\n\n");

    for i in 0..lines {
        content.push_str(&format!(
            "pub fn function_{}(input: &str) -> bool {{\n\
             \tlet map: HashMap<String, Vec<i32>> = HashMap::new();\n\
             \tinput.contains(\"pattern_{}\")\n\
             }}\n\n",
            i, i
        ));
    }

    content
}

fn bench_search(c: &mut Criterion) {
    let dir = setup_test_repo();
    let dir_path = dir.path();

    c.bench_function("rustygrep_search", |b| {
        b.iter(|| {
            let output = std::process::Command::new(env!("CARGO_BIN_EXE_rustygrep"))
                .arg("pattern_42")
                .arg(dir_path)
                .output()
                .unwrap();
            black_box(output);
        })
    });

    c.bench_function("rustygrep_search_llm", |b| {
        b.iter(|| {
            let output = std::process::Command::new(env!("CARGO_BIN_EXE_rustygrep"))
                .arg("--llm")
                .arg("pattern_42")
                .arg(dir_path)
                .output()
                .unwrap();
            black_box(output);
        })
    });
}

criterion_group!(benches, bench_search);
criterion_main!(benches);
