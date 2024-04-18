mod cargo_toml;
mod crates_io;
mod diff_directories;
mod git;

use cargo_toml::*;
use crates_io::*;
use git::*;
use git2::{Commit, Oid, Reference, Repository};
use log::LevelFilter;
use std::env::args;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

fn main() {
    env_logger::builder()
        .filter_level(LevelFilter::Info)
        .format_target(false)
        .init();

    let argv: Vec<String> = args().collect();

    let crate_name = argv[1].as_str();
    let crate_version = argv[2].as_str();

    let crates_io_path = match download_crate(crate_name, crate_version) {
        Err(err) => {
            log::error!("{}", err.to_string());
            return;
        }
        Ok(path) => path,
    };

    let cargo_toml = parse_cargo_toml(crates_io_path.join("Cargo.toml").as_path()).unwrap();

    let repository = match cargo_toml.package.repository {
        Some(r) => r,
        None => {
            log::error!("No repository URL configured on crate");
            return;
        }
    };

    let repository = match clone_git_repository(repository.as_str()) {
        Err(err) => {
            log::error!("Git clone failed {}", err);
            return;
        }
        Ok(repository) => repository,
    };
    let repository_root = repository.path().parent().unwrap().to_path_buf();
    let head = repository.head().unwrap();
    log::info!("Default branch is {}", head.shorthand().unwrap());

    if let Err(err) = get_expected_sha1_from_crate(crates_io_path.as_path())
        .and_then(|sha1| get_commit_to_checkout(sha1, crate_version, &repository, &head))
        .and_then(|commit| {
            repository
                .checkout_tree(commit.as_object(), None)
                .map_err(|_| io::Error::new(ErrorKind::Other, "Couldn't checkout tree to commit"))
        })
    {
        log::warn!("Could not checkout any version specific commit, staying on latest commit");
        log::warn!("Error was: {:?}", err);
    }

    // If subpath isn't the path to a crate, look for a crate with the same name anywhere in the git repository
    let mut git_crate_path = match cargo_toml.package.repository_subpath {
        Some(s) => repository_root.join(s),
        None => repository_root.to_path_buf(),
    };

    let git_crate_path_contains_crate =
        match parse_cargo_toml(git_crate_path.join("Cargo.toml").as_path()) {
            Ok(config) => config.package.name == crate_name,
            Err(_) => false,
        };
    if !git_crate_path_contains_crate {
        log::warn!(
            "No crate found at {}, looking for crate",
            git_crate_path.as_path().to_str().unwrap()
        );
        git_crate_path = match find_crate(repository_root.as_path(), crate_name) {
            None => {
                log::error!("No crate found at all, aborting");
                return;
            }
            Some(crate_path) => crate_path,
        };
    }

    if crates_io_path.join("build.rs").is_file() {
        log::info!("Has build.rs script");
    }

    if cargo_toml.package.build.is_some() {
        log::info!(
            "Has build script configuration, build = {}",
            cargo_toml.package.build.unwrap_or("".to_string())
        );
    }

    log::info!(
        "Found crate in {}",
        git_crate_path.as_path().to_str().unwrap()
    );

    diff_directories::diff_directories(crates_io_path.as_path(), git_crate_path.as_path());
}

fn get_commit_to_checkout<'repo>(
    sha1: Option<String>,
    crate_version: &str,
    repository: &'repo Repository,
    head: &Reference,
) -> Result<Commit<'repo>, io::Error> {
    match sha1 {
        Some(sha1) => {
            log::info!("Sha1 announced in crates.io is {}", sha1);
            if !is_commit_head_or_head_ancestor(&repository, &head, sha1.as_str()) {
                return Err(io::Error::new(
                    ErrorKind::NotFound,
                    "Commit not in default branch history",
                ));
            }

            repository
                .find_commit(Oid::from_str(sha1.as_str()).unwrap())
                .or_else(|_| {
                    find_commit_from_git_tag(crate_version, repository).map_err(|_| {
                        io::Error::new(
                            ErrorKind::Other,
                            "Couldn't find commit from sha1 or from git tag",
                        )
                    })
                })
        }
        None => find_commit_from_git_tag(crate_version, repository)
            .map_err(|_| io::Error::new(ErrorKind::Other, "Couldn't find commit from git tag")),
    }
}

pub fn find_crate(git_path: &Path, crate_name: &str) -> Option<PathBuf> {
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
            if let Ok(config) = parse_cargo_toml(entry.path()) {
                if config.package.name == crate_name {
                    return Some(entry.path().parent().unwrap().to_path_buf());
                }
            }
        }
    }

    None
}
