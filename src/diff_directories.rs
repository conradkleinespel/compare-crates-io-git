use md5::Context;
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;

pub fn diff_directories(crates_io_path: &Path, git_crate_path: &Path) {
    println!(
        "Diffing {} and {}",
        crates_io_path.to_str().unwrap(),
        git_crate_path.to_str().unwrap()
    );

    let crates_io_file_hashes = get_file_hashes(crates_io_path);
    let git_crate_file_hashes = get_file_hashes(git_crate_path);
    for (rel_path, hash) in &crates_io_file_hashes {
        if let Some(other_hash) = git_crate_file_hashes.get(rel_path) {
            if other_hash != hash {
                println!(
                    "Files differ: {} and {}{}",
                    crates_io_path.join(rel_path).to_str().unwrap(),
                    git_crate_path.join(rel_path).to_str().unwrap(),
                    if !crate::is_file_utf8(crates_io_path.join(rel_path).as_path())
                        || !crate::is_file_utf8(git_crate_path.join(rel_path).as_path())
                    {
                        ", non utf-8, possibly binary file"
                    } else {
                        ""
                    }
                );
            }
        }
    }

    for rel_path in crates_io_file_hashes.keys() {
        if !git_crate_file_hashes.contains_key(rel_path) {
            println!(
                "Only in crates.io: {}{}",
                crates_io_path.join(rel_path).to_str().unwrap(),
                if !crate::is_file_utf8(crates_io_path.join(rel_path).as_path()) {
                    ", non utf-8, possibly binary file"
                } else {
                    ""
                }
            );
        }
    }

    for rel_path in git_crate_file_hashes.keys() {
        if !crates_io_file_hashes.contains_key(rel_path) {
            println!(
                "Only in git: {}{}",
                git_crate_path.join(rel_path).to_str().unwrap(),
                if !crate::is_file_utf8(git_crate_path.join(rel_path).as_path()) {
                    ", non utf-8, possibly binary file"
                } else {
                    ""
                }
            );
        }
    }
}

fn compute_file_hash(file_path: &Path) -> Option<Vec<u8>> {
    let mut file = File::open(file_path).ok()?;
    let mut hasher = Context::new();
    let mut buffer = [0; 1024];
    loop {
        let n = file.read(&mut buffer).ok()?;
        if n == 0 {
            break;
        }
        hasher.consume(&buffer[0..n]);
    }
    Some(hasher.compute().0.to_vec())
}

fn get_file_hashes(dir: &Path) -> HashMap<String, Vec<u8>> {
    let mut hash_map = HashMap::new();
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| match e.ok() {
        Some(entry) => {
            if entry.path().starts_with(dir.join(".git")) {
                None
            } else {
                Some(entry)
            }
        }
        None => None,
    }) {
        if !entry.file_type().is_file() {
            continue;
        }
        if let Some(stripped_path) = entry
            .path()
            .strip_prefix(dir)
            .ok()
            .map(|p| p.to_str())
            .flatten()
        {
            if let Some(file_hash) = compute_file_hash(&entry.path()) {
                hash_map.insert(stripped_path.to_owned(), file_hash);
            }
        }
    }
    hash_map
}
