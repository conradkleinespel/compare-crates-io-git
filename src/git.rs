use crate::cargo_toml::parse_cargo_toml;
use git2::{Error, Oid, Reference, Repository};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn clone_git_repository(repository_url: &str) -> Result<Repository, Error> {
    let git_path = tempfile::Builder::new()
        .prefix("git-clone-")
        .tempdir()
        .unwrap()
        .into_path();

    let repository = match Repository::clone(repository_url, git_path.as_path()) {
        Err(err) => {
            println!("Invalid repository");
            return Err(err);
        }
        Ok(repository) => repository,
    };
    println!(
        "Cloned repository to {}",
        repository.path().parent().unwrap().to_str().unwrap()
    );

    Ok(repository)
}

pub fn is_commit_head_or_head_ancestor(
    repository: &Repository,
    head: &Reference,
    commit_sha1: &str,
) -> bool {
    let sha1_oid = Oid::from_str(commit_sha1).unwrap();
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

pub fn find_subpath_for_crate_with_name(git_path: &Path, crate_name: &str) -> Option<PathBuf> {
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
                return Some(cargo_toml_dir);
            }
        }
    }

    None
}

pub fn is_path_crate_with_name(path: &Path, crate_name: &str) -> bool {
    let cargo_toml = path.join("Cargo.toml");

    if !cargo_toml.is_file() {
        return false;
    }

    match parse_cargo_toml(cargo_toml.as_path()) {
        Err(_) => false,
        Ok(config) => config.package.name == crate_name,
    }
}
