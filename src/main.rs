mod cargo_toml;
mod crates_io;
mod diff_directories;
mod git;

use cargo_toml::*;
use crates_io::*;
use git::*;
use git2::Oid;
use std::env::args;
use std::error::Error;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::path::Path;

fn main() {
    let argv: Vec<String> = args().collect();

    let crate_name = argv[1].as_str();
    let crate_version = argv[2].as_str();

    let crates_io_path = match download_crate(crate_name, crate_version) {
        Err(err) => {
            println!(
                "Couldn't download crate from crates.io: {}, {}",
                err,
                err.source()
                    .map(|e| e.to_string())
                    .unwrap_or("no details".to_string())
            );
            return;
        }
        Ok(path) => path,
    };

    let cargo_toml = parse_cargo_toml(crates_io_path.join("Cargo.toml").as_path()).unwrap();

    let cargo_toml_repository = match cargo_toml.package.repository {
        Some(r) => r,
        None => {
            println!("No repository URL configured on crate");
            return;
        }
    };

    let (repository_url, mut subpath) =
        match get_repository_and_subpath_from_repository_url(cargo_toml_repository.as_str()) {
            (Some(repo), path) => (repo, path),
            _ => {
                println!("No repository found");
                return;
            }
        };
    println!("Repository is {}, subpath is {:?}", repository_url, subpath);

    let repository = match clone_git_repository(repository_url.as_str()) {
        Err(err) => {
            println!("Git clone failed {}", err);
            return;
        }
        Ok(repository) => repository,
    };
    let head = repository.head().unwrap();
    println!("Default branch is {}", head.shorthand().unwrap());

    match get_expected_sha1_from_crate(crates_io_path.as_path()) {
        Some(sha1) => {
            println!("Sha1 announced in crates.io is {}", sha1);
            if is_commit_head_or_head_ancestor(&repository, &head, sha1.as_str()) {
                println!("Commit is in history, checking it out");
                let commit = repository
                    .find_commit(Oid::from_str(sha1.as_str()).unwrap())
                    .unwrap();
                repository.checkout_tree(commit.as_object(), None).unwrap();
            } else {
                println!("Commit not in default branch history")
            }
        }
        None => {
            println!("No sha1 announced in crates.io, crate packaged with --allow-dirty");
            println!("Trying to find matching version tag on git repository");

            match repository
                .tag_names(Some(crate_version))
                .or_else(|_| repository.tag_names(Some(format!("v{}", crate_version).as_str())))
            {
                Ok(tags) => match tags.get(0) {
                    Some(tag) => {
                        let tag_ref_name = format!("refs/tags/{}", tag);
                        let tag_oid = repository.refname_to_id(tag_ref_name.as_str()).unwrap();
                        let commit = repository.find_commit(tag_oid).unwrap();
                        println!(
                            "Found matching version tag {} pointing to commit {}: {}",
                            tag,
                            commit.id(),
                            commit.summary().unwrap_or("no commit message")
                        );
                        repository.checkout_tree(commit.as_object(), None).unwrap();
                    }
                    None => {
                        println!("No matching version tag found");
                    }
                },
                Err(_) => {
                    println!("No matching version tag found");
                }
            }
        }
    }

    // If subpath isn't the path to a crate, look for a crate with the same name anywhere in the git repository
    let git_crate_path = match subpath.as_ref() {
        Some(s) => repository.path().parent().unwrap().join(s),
        None => repository.path().parent().unwrap().to_path_buf(),
    };
    if !is_path_crate_with_name(git_crate_path.as_path(), crate_name) {
        println!(
            "No crate found at {}, looking for crate",
            git_crate_path.as_path().to_str().unwrap()
        );
        subpath = find_subpath_for_crate_with_name(repository.path().parent().unwrap(), crate_name)
    }

    let git_crate_path = match subpath.as_ref() {
        Some(s) => repository.path().parent().unwrap().join(s),
        None => repository.path().parent().unwrap().to_path_buf(),
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

    diff_directories::diff_directories(crates_io_path.as_path(), git_crate_path.as_path());
}

fn is_file_utf8(filename: &Path) -> bool {
    let file = match File::open(filename) {
        Err(_) => return false,
        Ok(f) => f,
    };
    let reader = BufReader::new(file);

    for line in reader.lines() {
        let line = match line {
            Err(_) => return false,
            Ok(l) => l,
        };
        if std::str::from_utf8(line.as_bytes()).is_err() {
            return false;
        }
    }
    true
}
