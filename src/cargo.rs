use crate::error::Error;
use cargo::{
    core::{Dependency, Manifest, Package, Source, SourceId},
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

pub fn parse_cargo<T: AsRef<path::Path>>(
    crate_root: T,
    config: &Config,
) -> Result<Manifest, Error> {
    let mut toml_path = PathBuf::from(crate_root.as_ref());
    toml_path.push("Cargo.toml");
    let mut toml_file = File::open(toml_path)?;
    let mut toml_content = String::new();
    toml_file.read_to_string(&mut toml_content)?;

    let toml_manifest: TomlManifest = toml::from_str(&toml_content)?;
    let toml_manifest = Rc::new(toml_manifest);
    let cargo_source = SourceId::crates_io(&config)?;
    let (manifest, paths) =
        TomlManifest::to_real_manifest(&toml_manifest, cargo_source, crate_root.as_ref(), &config)?;
    debug!("{}: {:?}", "Paths".red(), paths);
    Ok(manifest)
}

pub fn download_dependency<'a, T>(
    dep: &Dependency,
    mut src: T,
    config: &Config,
) -> Result<Package, Error>
where
    T: Source + 'a,
{
    let opts = src.query_vec(dep)?;
    let latest = opts
        .iter()
        .max_by_key(|x| x.version())
        .ok_or(Error::PackageNotFound(String::from(
            dep.name_in_toml().as_str(),
        )))?;
    let pkg = Box::new(src).download_now(latest.package_id(), config)?;
    Ok(pkg)
}
