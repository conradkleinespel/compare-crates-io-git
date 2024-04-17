use flate2::read::GzDecoder;
use serde::Deserialize;
use std::fs::File;
use std::io::{Cursor, Error, Read, Result, Write};
use std::path::{Path, PathBuf};
use tar::Archive;

#[derive(Deserialize)]
pub struct CargoVcsInfoJson {
    pub git: CargoVcsInfoJsonGit,
}

#[derive(Deserialize)]
pub struct CargoVcsInfoJsonGit {
    pub sha1: String,
}

pub fn parse_cargo_vcs_info_json(path: &Path) -> Result<CargoVcsInfoJson> {
    let mut cargo_vcs_info_json = File::open(path.to_path_buf())?;
    let mut cargo_vcs_info_json_content = String::new();

    cargo_vcs_info_json.read_to_string(&mut cargo_vcs_info_json_content)?;
    serde_json::from_str(cargo_vcs_info_json_content.as_str()).map_err(|err| Error::from(err))
}

pub fn get_expected_sha1_from_crate(crates_io_path: &Path) -> Option<String> {
    let cargo_vcs_info_path = crates_io_path.join(".cargo_vcs_info.json");
    if !cargo_vcs_info_path.is_file() {
        return None;
    }

    let config = parse_cargo_vcs_info_json(cargo_vcs_info_path.as_path()).unwrap();

    return Some(config.git.sha1.to_string());
}

pub fn download_crate(name: &str, version: &str) -> Result<PathBuf> {
    let crates_io_path = tempfile::Builder::new()
        .prefix("crates-io-")
        .tempdir()
        .unwrap()
        .into_path();

    let tgz_file_name = "archive.tar.gz";
    println!(
        "Downloading {}/{} to {}/{}",
        name,
        version,
        crates_io_path.as_path().to_str().unwrap(),
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

    File::create_new(crates_io_path.join(tgz_file_name))
        .unwrap()
        .write_all(archive.as_slice())
        .unwrap();

    Archive::new(GzDecoder::new(Cursor::new(archive))).unpack(crates_io_path.as_path())?;

    Ok(crates_io_path.join(format!("{}-{}", name, version)))
}
