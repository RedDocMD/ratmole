use crate::{
    cargo::{download_dependency, parse_cargo},
    error::Error,
    structs::{structs_from_items, Path, Struct},
};
use cargo::{
    core::{manifest::TargetSourcePath, Package, SourceId},
    sources::SourceConfigMap,
    Config,
};
use log::{debug, warn};
use regex::Regex;
use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};

pub fn structs_in_crate<T: AsRef<std::path::Path>>(lib_path: T) -> Result<Vec<Struct>, Error> {
    let mut src_path = PathBuf::from(lib_path.as_ref());
    src_path.pop();
    let mut structs = Vec::new();
    for entry in fs::read_dir(src_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let name = path
                .file_name()
                .unwrap()
                .to_str()
                .ok_or(Error::Utf8("failed to convert OsStr to str"))?;
            if is_rust_filename(name) {
                let mod_name = &name[..name.len() - 3];
                structs.append(
                    &mut structs_from_file(&path, Path::from(vec![String::from(mod_name)]))?
                        .unwrap_or_default(),
                );
            }
        }
        if path.is_dir() {
            let dir_name = path
                .file_name()
                .unwrap()
                .to_str()
                .ok_or(Error::Utf8("failed to convert OsStr to str"))?;
            for entry in fs::read_dir(&path)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let name = path
                        .file_name()
                        .unwrap()
                        .to_str()
                        .ok_or(Error::Utf8("failed to convert OsStr to str"))?;
                    if name == "mod.rs" {
                        structs.append(
                            &mut structs_from_file(path, Path::from(vec![String::from(dir_name)]))?
                                .unwrap_or_default(),
                        );
                    } else if is_rust_filename(name) {
                        let mod_name = &name[..name.len() - 3];
                        structs.append(
                            &mut structs_from_file(
                                &path,
                                Path::from(vec![String::from(dir_name), String::from(mod_name)]),
                            )?
                            .unwrap_or_default(),
                        );
                    }
                }
            }
        }
    }
    Ok(structs)
}

fn is_rust_filename(name: &str) -> bool {
    lazy_static! {
        static ref RS_REG: Regex = Regex::new(r"^[^.]+\.rs$").unwrap();
    }
    RS_REG.is_match(name)
}

fn structs_from_file<T: AsRef<std::path::Path>>(
    file_path: T,
    module: crate::structs::Path,
) -> Result<Option<Vec<Struct>>, Error> {
    debug!("{}", file_path.as_ref().as_os_str().to_str().unwrap());
    let mut file = File::open(file_path.as_ref())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    match syn::parse_file(&contents) {
        Ok(ast) => Ok(Some(structs_from_items(&ast.items, module))),
        Err(err) => {
            warn!("{}", err);
            Ok(None)
        }
    }
}

pub fn structs_in_crate_and_deps<T: AsRef<std::path::Path>>(
    main_crate_root: T,
) -> Result<Vec<Struct>, Error> {
    let config = Config::default()?;
    let (manifest, manifest_path) = parse_cargo(&main_crate_root, &config)?;
    let main_pkg = Package::new(manifest, &manifest_path);
    let mut structs = Vec::new();
    debug!("Exploring {}", main_pkg.name());
    for targ in main_pkg.targets() {
        if let TargetSourcePath::Path(path) = targ.src_path() {
            structs.append(&mut structs_in_crate(path)?);
        }
    }
    structs.append(&mut structs_in_crate(&main_crate_root)?);
    let pkgs = get_dependencies(&main_crate_root)?;
    for pkg in &pkgs {
        debug!("Exploring {}", pkg.name());
        if let Some(lib) = pkg.library() {
            if let TargetSourcePath::Path(path) = lib.src_path() {
                debug!("Lib root: {}", path.as_os_str().to_str().unwrap());
                structs.append(&mut structs_in_crate(path)?);
            }
        }
    }
    Ok(structs)
}

fn get_dependencies<T: AsRef<std::path::Path>>(main_crate_root: T) -> Result<Vec<Package>, Error> {
    let config = Config::default()?;
    let _lock = config.acquire_package_cache_lock()?;
    let crates_io_id = SourceId::crates_io(&config)?;
    let config_map = SourceConfigMap::new(&config)?;
    let mut crates_io = config_map.load(crates_io_id, &Default::default())?;
    crates_io.update()?;

    let (manifest, _) = parse_cargo(main_crate_root, &config)?;
    let mut pkgs = Vec::new();
    for dep in manifest.dependencies() {
        debug!("Downloading {} ...", dep.name_in_toml());
        if dep.source_id() == crates_io_id {
            pkgs.push(download_dependency(dep, &mut crates_io, &config)?);
        } else {
            let config_map = SourceConfigMap::new(&config)?;
            let mut src = config_map.load(dep.source_id(), &Default::default())?;
            src.update()?;
            pkgs.push(download_dependency(dep, &mut src, &config)?);
        }
        debug!(" ... downloaded {}", dep.name_in_toml());
    }
    Ok(pkgs)
}
