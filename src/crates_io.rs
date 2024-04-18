use flate2::read::GzDecoder;
use serde::Deserialize;
use std::error::Error as ErrorTrait;
use std::fs::File;
use std::io::{Cursor, Error, ErrorKind, Read, Result, Write};
use std::path::{Path, PathBuf};
use tar::Archive;

#[derive(Deserialize)]
pub struct CargoVcsInfoJson {
    pub git: CargoVcsInfoJsonGit,
    // TODO: path_in_vcs: Option<String>
}

#[derive(Deserialize)]
pub struct CargoVcsInfoJsonGit {
    pub sha1: String,
}

pub fn parse_cargo_vcs_info_json(path: &Path) -> Result<CargoVcsInfoJson> {
    let mut cargo_vcs_info_json = File::open(path)?;
    let mut cargo_vcs_info_json_content = String::new();

    cargo_vcs_info_json.read_to_string(&mut cargo_vcs_info_json_content)?;
    serde_json::from_str(cargo_vcs_info_json_content.as_str()).map_err(Error::from)
}

pub fn get_expected_sha1_from_crate(crates_io_path: &Path) -> Result<Option<String>> {
    let cargo_vcs_info_path = crates_io_path.join(".cargo_vcs_info.json");
    if !cargo_vcs_info_path.is_file() {
        return Ok(None);
    }

    let config = parse_cargo_vcs_info_json(cargo_vcs_info_path.as_path())?;

    Ok(Some(config.git.sha1.to_string()))
}

pub fn download_crate(name: &str, version: &str) -> Result<PathBuf> {
    let crates_io_path = tempfile::Builder::new()
        .prefix("crates-io-")
        .tempdir()?
        .into_path();

    let tgz_file_name = "archive.tar.gz";
    log::info!(
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
    let archive = reqwest::blocking::get(url)
        .map_err(|err| {
            Error::new(
                ErrorKind::Other,
                format!(
                    "Couldn't call crate download API at crates.io: {}, {}",
                    err,
                    err.source()
                        .map(|e| e.to_string())
                        .unwrap_or("no details".to_string())
                ),
            )
        })?
        .bytes()
        .map_err(|err| {
            Error::new(
                ErrorKind::Other,
                format!(
                    "Couldn't read response body from crates.io: {}, {}",
                    err,
                    err.source()
                        .map(|e| e.to_string())
                        .unwrap_or("no details".to_string())
                ),
            )
        })?
        .to_vec();

    File::create_new(crates_io_path.join(tgz_file_name))?.write_all(archive.as_slice())?;
    Archive::new(GzDecoder::new(Cursor::new(archive)))
        .unpack(crates_io_path.as_path())
        .map_err(|err| {
            Error::new(
                ErrorKind::Other,
                format!(
                    "Couldn't unpack archive downloaded from crates.io: {}, {}",
                    err,
                    err.source()
                        .map(|e| e.to_string())
                        .unwrap_or("no details".to_string())
                ),
            )
        })?;

    Ok(crates_io_path.join(format!("{}-{}", name, version)))
}
