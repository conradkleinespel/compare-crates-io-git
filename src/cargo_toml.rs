use serde::Deserialize;
use std::fs::File;
use std::io::{Error, ErrorKind, Read, Result};
use std::path::Path;
use url::Url;

#[derive(Deserialize)]
pub struct CargoToml {
    pub package: Package,
}

#[derive(Deserialize)]
pub struct Package {
    pub name: String,
    pub repository: Option<String>,
    pub repository_subpath: Option<String>,
    pub build: Option<String>,
}

pub fn parse_cargo_toml(path: &Path) -> Result<CargoToml> {
    let mut cargo_toml = File::open(path.to_path_buf())?;
    let mut cargo_toml_content = String::new();

    cargo_toml.read_to_string(&mut cargo_toml_content)?;

    let mut config: CargoToml = toml::from_str(cargo_toml_content.as_str())
        .map_err(|_| Error::new(ErrorKind::InvalidData, "could not deserialize Cargo.toml"))?;

    let (repository, subpath) = match config.package.repository.as_ref() {
        None => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "no repository found in Cargo.toml",
            ))
        }
        Some(repository) => {
            let url = Url::parse(repository).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidData,
                    "could not parse repository URL in Cargo.toml",
                )
            })?;

            (
                get_repository_from_repository_url(&url),
                get_subpath_from_repository_url(&url),
            )
        }
    };

    config.package.repository = Some(repository.clone());
    config.package.repository_subpath = subpath;

    println!(
        "Repository is {}, subpath is {:?}",
        repository, &config.package.repository_subpath
    );

    Ok(config)
}

pub fn get_repository_from_repository_url(url: &Url) -> String {
    if url.as_str().ends_with(".git") {
        return url.as_str().to_string();
    }

    if url.host_str().unwrap() == "github.com" {
        let paths: Vec<String> = url.path().split("/").map(|s| s.to_string()).collect();

        let needs_git_append = paths[2].ends_with(".git");

        return format!(
            "{}://{}/{}/{}{}",
            url.scheme(),
            url.host_str().unwrap(),
            paths[1],
            paths[2],
            if needs_git_append { "" } else { ".git" }
        );
    }

    url.as_str().to_string()
}

pub fn get_subpath_from_repository_url(url: &Url) -> Option<String> {
    if url.as_str().ends_with(".git") {
        return None;
    }

    if url.host_str().unwrap() == "github.com" {
        let paths: Vec<String> = url.path().split("/").map(|s| s.to_string()).collect();

        // Repository URLs such as https://github.com/org/repo/tree/branch-name/some/path/here
        if paths.len() >= 6 {
            return Some(paths[5..].join("/"));
        }
        return None;
    }
    None
}
