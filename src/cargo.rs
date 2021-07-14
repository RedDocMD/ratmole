use crate::error::Error;
use cargo::{
    core::{Manifest, SourceId},
    util::toml::TomlManifest,
    Config,
};
use colored::*;
use log::debug;
use std::{
    fs::File,
    io::Read,
    path::{self, PathBuf},
    rc::Rc,
};

pub fn parse_cargo<T: AsRef<path::Path>>(crate_root: T) -> Result<Manifest, Error> {
    let mut toml_path = PathBuf::from(crate_root.as_ref());
    toml_path.push("Cargo.toml");
    let mut toml_file = File::open(toml_path)?;
    let mut toml_content = String::new();
    toml_file.read_to_string(&mut toml_content)?;

    let toml_manifest: TomlManifest = toml::from_str(&toml_content)?;
    let toml_manifest = Rc::new(toml_manifest);
    let config = Config::default()?;
    let cargo_source = SourceId::crates_io(&config)?;
    let (manifest, paths) =
        TomlManifest::to_real_manifest(&toml_manifest, cargo_source, crate_root.as_ref(), &config)?;
    debug!("{}: {:?}", "Paths".red(), paths);
    Ok(manifest)
}
