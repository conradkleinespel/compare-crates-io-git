use flate2::read::GzDecoder;
use git2::{Oid, Reference, Repository, Sort};
use serde::Deserialize;
use std::env::args;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use tar::Archive;
use url::Url;

#[derive(Deserialize)]
struct CargoToml {
    package: Package,
}

#[derive(Deserialize)]
struct Package {
    repository: String,
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

    let git_tempdir = tempfile::tempdir().unwrap();
    let crates_io_tempdir = tempfile::tempdir().unwrap();
    download_crate(crate_name, crate_version, crates_io_tempdir.path());

    let crates_io_path = crates_io_tempdir
        .into_path()
        .join(format!("{}-{}", crate_name, crate_version));

    let (repository_url, subpath) =
        get_repository_and_subpath_from_repository_url(crates_io_path.as_path());
    println!("Repository is {}, subpath is '{}'", repository_url, subpath);

    let repository = match Repository::clone(repository_url.as_str(), git_tempdir.path()) {
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
            if !sha1_in_commit_history_on_default_branch_with_revwalk(&repository, &head, sha1.as_str()) {
                println!("Commit not in default branch history (using revwalk)")
            }
            if !sha1_in_commit_history_on_default_branch_with_descendants(&repository, &head, sha1.as_str()) {
                println!("Commit not in default branch history (using descendants)")
            }
        }
        None => {
            println!("No sha1 announced in crates.io, crate packaged with --allow-dirty");
        }
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

    if repository.graph_descendant_of(head_oid, sha1_oid).unwrap() {
        find_and_print_commit(repository, sha1_oid);
        return true;
    }
    return false;
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

fn get_repository_and_subpath_from_repository_url(crates_io_path: &Path) -> (String, String) {
    let raw_repository_url = get_repository_url_from_cargo_toml(crates_io_path);
    let url_parsed = Url::parse(raw_repository_url.as_str()).unwrap();

    if url_parsed.host_str().unwrap() == "github.com" {
        let paths: Vec<String> = url_parsed
            .path()
            .split("/")
            .map(|s| s.to_string())
            .collect();
        return (
            format!(
                "{}://{}/{}/{}.git",
                url_parsed.scheme(),
                url_parsed.host_str().unwrap(),
                paths[1],
                paths[2],
            ),
            if paths.len() >= 6 {
                // Repository URLs such as https://github.com/org/repo/tree/branch-name/some/path/here
                paths[5..].join("/")
            } else {
                "".to_string()
            },
        );
    }

    (
        format!(
            "{}://{}{}.git",
            url_parsed.scheme(),
            url_parsed.host_str().unwrap(),
            url_parsed.path(),
        ),
        "".to_string(),
    )
}

fn get_repository_url_from_cargo_toml(crates_io_path: &Path) -> String {
    let mut cargo_toml = File::open(crates_io_path.join("Cargo.toml").to_path_buf()).unwrap();
    let mut cargo_toml_content = String::new();

    cargo_toml.read_to_string(&mut cargo_toml_content).unwrap();
    let config: CargoToml = toml::from_str(cargo_toml_content.as_str()).unwrap();

    return config.package.repository.to_string();
}

fn download_crate(name: &str, version: &str, destination: &Path) {
    println!(
        "Downloading {}/{} to {}",
        name,
        version,
        destination.to_str().unwrap()
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

    Archive::new(GzDecoder::new(Cursor::new(archive)))
        .unpack(destination)
        .unwrap();
}
