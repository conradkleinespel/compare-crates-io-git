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
    let mut cargo_toml = File::open(path.to_path_buf()).unwrap();
    let mut cargo_toml_content = String::new();

    cargo_toml.read_to_string(&mut cargo_toml_content).unwrap();

    let mut config: CargoToml = toml::from_str(cargo_toml_content.as_str())
        .map_err(|_| Error::new(ErrorKind::InvalidData, "could not deserialize Cargo.toml"))?;

    let (repository, subpath) = match config.package.repository.as_ref() {
        None => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "no repository found in Cargo.toml",
            ))
        }
        Some(repository) => (
            get_repository_from_repository_url(repository),
            get_subpath_from_repository_url(repository.as_str()),
        ),
    };

    config.package.repository = Some(repository.clone());
    config.package.repository_subpath = subpath;

    println!(
        "Repository is {}, subpath is {:?}",
        repository, &config.package.repository_subpath
    );

    Ok(config)
}

pub fn get_repository_from_repository_url(raw_repository_url: &str) -> String {
    let url_parsed = Url::parse(raw_repository_url).unwrap();

    if url_parsed.host_str().unwrap() == "github.com" {
        let paths: Vec<String> = url_parsed
            .path()
            .split("/")
            .map(|s| s.to_string())
            .collect();

        format!(
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
        )
    } else {
        format!(
            "{}://{}{}{}",
            url_parsed.scheme(),
            url_parsed.host_str().unwrap(),
            url_parsed.path(),
            if url_parsed.path().ends_with(".git") {
                ""
            } else {
                ".git"
            }
        )
    }
}

pub fn get_subpath_from_repository_url(raw_repository_url: &str) -> Option<String> {
    let url_parsed = Url::parse(raw_repository_url).unwrap();

    if url_parsed.host_str().unwrap() == "github.com" {
        let paths: Vec<String> = url_parsed
            .path()
            .split("/")
            .map(|s| s.to_string())
            .collect();

        println!("{:?}", paths);

        if paths.len() >= 6 {
            // Repository URLs such as https://github.com/org/repo/tree/branch-name/some/path/here
            Some(paths[5..].join("/"))
        } else {
            None
        }
    } else {
        None
    }
}
