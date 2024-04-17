use serde::Deserialize;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use toml::de::Error as TomlError;

#[derive(Deserialize)]
pub struct CargoToml {
    pub package: Package,
}

#[derive(Deserialize)]
pub struct Package {
    pub name: String,
    pub repository: Option<String>,
    pub build: Option<String>,
}

pub fn parse_cargo_toml(path: &Path) -> Result<CargoToml, TomlError> {
    let mut cargo_toml = File::open(path.to_path_buf()).unwrap();
    let mut cargo_toml_content = String::new();

    cargo_toml.read_to_string(&mut cargo_toml_content).unwrap();
    toml::from_str(cargo_toml_content.as_str())
}
