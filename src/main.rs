use flate2::read::GzDecoder;
use git2::{Oid, Reference, Repository, Sort};
use md5::Context;
use serde::Deserialize;
use std::collections::HashMap;
use std::env::args;
use std::error::Error;
use std::fs::File;
use std::io::{Cursor, Read, Write};
use std::path::Path;
use tar::Archive;
use toml::de::Error as TomlError;
use url::Url;
use walkdir::WalkDir;

#[derive(Deserialize)]
struct CargoToml {
    package: Package,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    repository: String,
    build: Option<String>,
}

#[derive(Deserialize)]
struct CargoVcsInfoJson {
    git: CargoVcsInfoJsonGit,
}

#[derive(Deserialize)]
struct CargoVcsInfoJsonGit {
    sha1: String,
}

fn main() {
    let argv: Vec<String> = args().collect();

    let crate_name = argv[1].as_str();
    let crate_version = argv[2].as_str();

    let git_path = tempfile::Builder::new()
        .prefix("git-clone-")
        .tempdir()
        .unwrap()
        .into_path();
    let crates_io_path = tempfile::Builder::new()
        .prefix("crates-io-")
        .tempdir()
        .unwrap()
        .into_path();
    if let Err(err) = download_crate(crate_name, crate_version, crates_io_path.as_path()) {
        println!(
            "Couldn't download crate from crates.io: {}, {}",
            err,
            err.source()
                .map(|e| e.to_string())
                .unwrap_or("no details".to_string())
        );
        return;
    }

    let crates_io_path = crates_io_path.join(format!("{}-{}", crate_name, crate_version));

    let cargo_toml = parse_cargo_toml(crates_io_path.join("Cargo.toml").as_path()).unwrap();

    let (repository_url, mut subpath) = match get_repository_and_subpath_from_repository_url(
        cargo_toml.package.repository.as_str(),
    ) {
        (Some(repo), path) => (repo, path),
        _ => {
            println!("No repository found");
            return;
        }
    };
    println!("Repository is {}, subpath is {:?}", repository_url, subpath);

    let repository = match Repository::clone(repository_url.as_str(), git_path.as_path()) {
        Err(_) => {
            println!("Invalid repository");
            return;
        }
        Ok(repository) => repository,
    };
    println!(
        "Cloned repository to {}",
        repository.path().parent().unwrap().to_str().unwrap()
    );
    let head = repository.head().unwrap();
    println!("Default branch is {}", head.shorthand().unwrap());

    match get_expected_sha1_from_crate(crates_io_path.as_path()) {
        Some(sha1) => {
            println!("Sha1 announced in crates.io is {}", sha1);
            let commit_in_revwalk = sha1_in_commit_history_on_default_branch_with_revwalk(
                &repository,
                &head,
                sha1.as_str(),
            );
            let commit_in_descendants = sha1_in_commit_history_on_default_branch_with_descendants(
                &repository,
                &head,
                sha1.as_str(),
            );
            if commit_in_revwalk && commit_in_descendants {
                println!("Commit is in history, checking it out");
                let commit = repository
                    .find_commit(Oid::from_str(sha1.as_str()).unwrap())
                    .unwrap();
                repository.checkout_tree(commit.as_object(), None).unwrap();
            }
            if !commit_in_revwalk {
                println!("Commit not in default branch history (using revwalk)")
            }
            if !commit_in_descendants {
                println!("Commit not in default branch history (using descendants)")
            }
        }
        None => {
            println!("No sha1 announced in crates.io, crate packaged with --allow-dirty");
        }
    }

    // If subpath isn't the path to a crate, look for a crate with the same name anywhere in the git repository
    let git_crate_path = match subpath.as_ref() {
        Some(s) => git_path.as_path().to_path_buf().join(s),
        None => git_path.as_path().to_path_buf(),
    };
    if !is_path_crate_with_name(git_crate_path.as_path(), crate_name) {
        println!(
            "No crate found at {}, looking for crate",
            git_crate_path.as_path().to_str().unwrap()
        );
        subpath = find_subpath_for_crate_with_name(git_path.as_path(), crate_name)
    }

    let git_crate_path = match subpath.as_ref() {
        Some(s) => git_path.as_path().to_path_buf().join(s),
        None => git_path.as_path().to_path_buf(),
    };

    if crates_io_path.join("build.rs").is_file() {
        println!("Has build.rs script");
    }
    if cargo_toml.package.build.is_some() {
        println!(
            "Has build script configuration, build = {}",
            cargo_toml.package.build.unwrap_or("".to_string())
        );
    }

    println!(
        "Found crate in {}",
        git_crate_path.as_path().to_str().unwrap()
    );

    diff_directories(crates_io_path.as_path(), git_crate_path.as_path());
}

fn find_subpath_for_crate_with_name(git_path: &Path, crate_name: &str) -> Option<String> {
    for entry in WalkDir::new(git_path)
        .into_iter()
        .filter_map(|e| match e.ok() {
            Some(entry) => {
                if entry.path().starts_with(git_path.join(".git")) {
                    None
                } else {
                    Some(entry)
                }
            }
            None => None,
        })
    {
        if entry.file_name().to_str().unwrap() == "Cargo.toml" {
            let cargo_toml_dir = entry.path().parent().unwrap().to_path_buf();
            if is_path_crate_with_name(cargo_toml_dir.as_path(), crate_name) {
                return Some(cargo_toml_dir.as_path().to_str().unwrap().to_string());
            }
        }
    }

    None
}

fn is_path_crate_with_name(path: &Path, crate_name: &str) -> bool {
    let cargo_toml = path.join("Cargo.toml");

    if !cargo_toml.is_file() {
        return false;
    }

    match parse_cargo_toml(cargo_toml.as_path()) {
        Err(_) => false,
        Ok(config) => config.package.name == crate_name,
    }
}

fn sha1_in_commit_history_on_default_branch_with_revwalk(
    repository: &Repository,
    head: &Reference,
    sha1: &str,
) -> bool {
    let mut revwalk = repository.revwalk().unwrap();
    revwalk.push(head.target().unwrap()).unwrap();
    revwalk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME).unwrap();
    loop {
        let current = match revwalk.next() {
            None => {
                break;
            }
            Some(current) => current,
        };

        let current = match current {
            Err(err) => {
                println!("Revwalk error {:?}", err);
                break;
            }
            Ok(current) => current,
        };

        if current.to_string() == sha1 {
            find_and_print_commit(repository, current);
            return true;
        }
    }

    return false;
}

fn sha1_in_commit_history_on_default_branch_with_descendants(
    repository: &Repository,
    head: &Reference,
    sha1: &str,
) -> bool {
    let sha1_oid = Oid::from_str(sha1).unwrap();
    let head_oid = head.target().unwrap();
    if head_oid == sha1_oid {
        find_and_print_commit(repository, sha1_oid);
        return true;
    }

    match repository.graph_descendant_of(head_oid, sha1_oid) {
        Ok(is_descendant_of_head) => {
            if is_descendant_of_head {
                find_and_print_commit(repository, sha1_oid);
                true
            } else {
                false
            }
        }
        _ => false,
    }
}

fn find_and_print_commit(repository: &Repository, sha1: Oid) {
    let commit = repository.find_commit(sha1).unwrap();
    let commit_datetime = chrono::DateTime::from_timestamp(commit.time().seconds(), 0).unwrap();

    println!(
        "Sha1 commit was {} ({}): {}",
        commit.id(),
        commit_datetime.format("%d/%m/%Y %H:%M"),
        commit.message().unwrap().trim()
    );
}

fn get_expected_sha1_from_crate(crates_io_path: &Path) -> Option<String> {
    let cargo_vcs_info_path = crates_io_path.join(".cargo_vcs_info.json");
    if !cargo_vcs_info_path.is_file() {
        return None;
    }

    let mut vcs_info_json = File::open(cargo_vcs_info_path.to_path_buf()).unwrap();
    let mut vcs_info_json_content = String::new();

    vcs_info_json
        .read_to_string(&mut vcs_info_json_content)
        .unwrap();
    let config: CargoVcsInfoJson = serde_json::from_str(vcs_info_json_content.as_str()).unwrap();

    return Some(config.git.sha1.to_string());
}

fn get_repository_and_subpath_from_repository_url(
    raw_repository_url: &str,
) -> (Option<String>, Option<String>) {
    let url_parsed = Url::parse(raw_repository_url).unwrap();

    if url_parsed.host_str().unwrap() == "github.com" {
        let paths: Vec<String> = url_parsed
            .path()
            .split("/")
            .map(|s| s.to_string())
            .collect();
        return (
            Some(format!(
                "{}://{}/{}/{}{}",
                url_parsed.scheme(),
                url_parsed.host_str().unwrap(),
                paths[1],
                paths[2],
                if paths[2].ends_with(".git") {
                    ""
                } else {
                    ".git"
                }
            )),
            if paths.len() >= 6 {
                // Repository URLs such as https://github.com/org/repo/tree/branch-name/some/path/here
                Some(paths[5..].join("/"))
            } else {
                None
            },
        );
    }

    (
        Some(format!(
            "{}://{}{}{}",
            url_parsed.scheme(),
            url_parsed.host_str().unwrap(),
            url_parsed.path(),
            if url_parsed.path().ends_with(".git") {
                ""
            } else {
                ".git"
            }
        )),
        None,
    )
}

fn parse_cargo_toml(path: &Path) -> Result<CargoToml, TomlError> {
    let mut cargo_toml = File::open(path.to_path_buf()).unwrap();
    let mut cargo_toml_content = String::new();

    cargo_toml.read_to_string(&mut cargo_toml_content).unwrap();
    toml::from_str(cargo_toml_content.as_str())
}

fn download_crate(name: &str, version: &str, destination: &Path) -> std::io::Result<()> {
    let tgz_file_name = "archive.tar.gz";
    println!(
        "Downloading {}/{} to {}/{}",
        name,
        version,
        destination.to_str().unwrap(),
        tgz_file_name,
    );

    let url = format!(
        "https://crates.io/api/v1/crates/{}/{}/download",
        name, version
    );
    let archive: Vec<u8> = reqwest::blocking::get(url)
        .unwrap()
        .bytes()
        .unwrap()
        .to_vec();

    File::create_new(destination.join(tgz_file_name))
        .unwrap()
        .write_all(archive.as_slice())
        .unwrap();

    Archive::new(GzDecoder::new(Cursor::new(archive))).unpack(destination)
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

fn diff_directories(crates_io_path: &Path, git_crate_path: &Path) {
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
                    "Files differ: {} and {}",
                    crates_io_path.join(rel_path).to_str().unwrap(),
                    git_crate_path.join(rel_path).to_str().unwrap(),
                );
            }
        }
    }

    for rel_path in crates_io_file_hashes.keys() {
        if !git_crate_file_hashes.contains_key(rel_path) {
            println!(
                "Only in crates.io: {}",
                crates_io_path.join(rel_path).to_str().unwrap(),
            );
        }
    }

    for rel_path in git_crate_file_hashes.keys() {
        if !crates_io_file_hashes.contains_key(rel_path) {
            println!(
                "Only in git: {}",
                git_crate_path.join(rel_path).to_str().unwrap(),
            );
        }
    }
}
