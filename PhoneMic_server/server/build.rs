//! Embeds the archived phone pages, `pages/<version>/<file>`, so the relay
//! keeps serving the page of older Plumed versions (see `protocol/src/page.rs`).
//! Generates `ARCHIVED: &[(version, &[(file name, bytes)])]`.

use std::fmt::Write;
use std::path::Path;
use std::{env, fs};

fn main() {
    let pages = Path::new(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("pages");
    println!("cargo:rerun-if-changed={}", pages.display());
    let mut code = String::from("pub static ARCHIVED: &[(u32, &[(&str, &[u8])])] = &[\n");
    let mut dirs: Vec<_> = fs::read_dir(&pages)
        .map(|entries| entries.map(|e| e.unwrap().path()).collect())
        .unwrap_or_default();
    dirs.sort();
    for dir in dirs.iter().filter(|d| d.is_dir()) {
        let name = dir.file_name().unwrap().to_str().unwrap();
        let version: u32 = name
            .parse()
            .unwrap_or_else(|_| panic!("pages/{name}: not a page version number"));
        println!("cargo:rerun-if-changed={}", dir.display());
        let mut files: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        files.sort();
        writeln!(code, "    ({version}, &[").unwrap();
        for file in files {
            let file_name = file.file_name().unwrap().to_str().unwrap();
            writeln!(
                code,
                "        ({file_name:?}, include_bytes!({:?})),",
                file.display().to_string()
            )
            .unwrap();
        }
        writeln!(code, "    ]),").unwrap();
    }
    code.push_str("];\n");
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("archived_pages.rs"),
        code,
    )
    .unwrap();
}
