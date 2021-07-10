use crate::error::Error;
use log::debug;
use semver::VersionReq;
use std::{
    fs::File,
    io::Read,
    path::{self, PathBuf},
};
use toml::Value;

pub struct Dependency {
    name: String,
    version: DependencyVersion,
}

pub enum DependencyVersion {
    Simple(VersionReq),
    Git(GitDependency),
}

pub struct GitDependency {
    url: String,
    version: GitVersion,
}

pub enum GitVersion {
    Rev(String),
    Tag(String),
    Branch(String),
}

pub fn crate_dependencies<T: AsRef<path::Path>>(crate_path: T) -> Result<Vec<Dependency>, Error> {
    let mut toml_path = PathBuf::from(crate_path.as_ref());
    toml_path.push("Cargo.toml");
    let mut toml_file = File::open(toml_path)?;
    let mut toml_content = String::new();
    toml_file.read_to_string(&mut toml_content)?;

    let spec = toml_content.parse::<Value>().unwrap();
    let deps = &spec["dependencies"];
    debug!("\n{}", deps);
    debug!("Deps is table: {}", deps.is_table());
    let deps = deps.as_table().unwrap();
    for (key, value) in deps {}

    let deps = Vec::new();
    Ok(deps)
}
