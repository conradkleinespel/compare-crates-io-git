use std::env::args;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use flate2::read::GzDecoder;
use git2::Repository;
use serde::Deserialize;
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

fn main() {
    let argv: Vec<String> = args().collect();

    let crate_name = argv[1].as_str();
    let crate_version = argv[2].as_str();

    let git_tempdir = tempfile::tempdir().unwrap();
    let crates_io_tempdir = tempfile::tempdir().unwrap();
    download_crate(crate_name, crate_version, crates_io_tempdir.path());

    let crates_io_path = crates_io_tempdir.into_path()
        .join(format!("{}-{}", crate_name, crate_version));

    let (repository_url, subpath) = get_repository_and_subpath_from_repository_url(
        crates_io_path.as_path()
    );
    println!("Repository is {}, subpath is '{}'", repository_url, subpath);

    let repository = match Repository::clone(repository_url.as_str(), git_tempdir.path()) {
        Err(_) => {
            println!("Invalid repository");
            return
        }
        Ok(repository) => {repository}
    };

    println!("Cloned repository to {}", repository.path().parent().unwrap().to_str().unwrap());
}

fn get_repository_and_subpath_from_repository_url(crates_io_path: &Path) -> (String, String) {
    let raw_repository_url = get_repository_url_from_cargo_toml(crates_io_path, );
    let url_parsed = Url::parse(raw_repository_url.as_str()).unwrap();

    if url_parsed.host_str().unwrap() == "github.com" {
        let paths: Vec<String> = url_parsed.path().split("/").map(|s| s.to_string()).collect();
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
            }
        )
    }

    (
        format!(
            "{}://{}{}.git",
            url_parsed.scheme(),
            url_parsed.host_str().unwrap(),
            url_parsed.path(),
        ),
        "".to_string()
    )
}

fn get_repository_url_from_cargo_toml(crates_io_path: &Path) -> String {
    let mut cargo_toml = File::open(
        crates_io_path
            .join("Cargo.toml")
            .to_path_buf()
    ).unwrap();
    let mut cargo_toml_content = String::new();

    cargo_toml.read_to_string(&mut cargo_toml_content).unwrap();
    let config: CargoToml = toml::from_str(cargo_toml_content.as_str()).unwrap();

    return config.package.repository.to_string();
}

fn download_crate(name: &str, version: &str, destination: &Path) {
    println!("Downloading {}/{} to {}", name, version, destination.to_str().unwrap());

    let url = format!("https://crates.io/api/v1/crates/{}/{}/download", name, version);
    let archive: Vec<u8> = reqwest::blocking::get(url).unwrap().bytes().unwrap().to_vec();

    Archive::new(GzDecoder::new(Cursor::new(archive))).unpack(destination).unwrap();
}
