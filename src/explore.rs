use crate::{
    cargo::{download_dependency, parse_cargo},
    error::{Error, Result},
    structs::{structs_from_items, Path, Struct},
};
use cargo::{
    core::{manifest::TargetSourcePath, Package, SourceId, Target},
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
use syn::{parse::Parse, Item, LitStr, Token};

pub fn structs_in_crate<T: AsRef<std::path::Path>>(lib_path: T) -> Result<Vec<Struct>> {
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
) -> Result<Option<Vec<Struct>>> {
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
) -> Result<Vec<Struct>> {
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

fn get_dependencies<T: AsRef<std::path::Path>>(main_crate_root: T) -> Result<Vec<Package>> {
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

pub fn structs_in_main_crate<T: AsRef<std::path::Path>>(crate_path: T) -> Result<Vec<Struct>> {
    let config = Config::default()?;
    let (manifest, manifest_path) = parse_cargo(&crate_path, &config)?;
    let pkg = Package::new(manifest, &manifest_path);
    let mut structs = Vec::new();
    for target in pkg.targets() {
        structs.append(&mut structs_in_target(target)?);
    }
    Ok(structs)
}

fn structs_in_target(targ: &Target) -> Result<Vec<Struct>> {
    let src_path = match targ.src_path() {
        TargetSourcePath::Path(path) => path,
        TargetSourcePath::Metabuild => return Ok(vec![]),
    };
    let crate_name = targ.crate_name();
    let mut structs = Vec::new();
    structs.append(
        &mut structs_from_file(&src_path, Path::from(vec![crate_name.clone()]))?
            .unwrap_or_default(),
    );
    structs.append(&mut structs_from_submodules(&Module {
        cat: ModuleCategory::Root,
        name: &crate_name,
        rust_path: Path::from(vec![crate_name.clone()]),
        path: src_path.clone(),
    })?);
    Ok(structs)
}

fn structs_from_submodules(module: &Module<'_>) -> Result<Vec<Struct>> {
    let empty_mods = match empty_modules_from_file(&module.path)? {
        Some(mods) => mods,
        None => return Ok(vec![]),
    };
    let mut sub_mods = Vec::new();
    for ast_mod in &empty_mods {
        if let Some(path) = &ast_mod.path {
            let mut new_mod_path = module.rust_path.clone();
            new_mod_path.push_name(ast_mod.name.clone());
            sub_mods.push(Module {
                path: path.clone(),
                rust_path: new_mod_path,
                name: &ast_mod.name,
                cat: ModuleCategory::Direct,
            });
        } else {
            sub_mods.push(module.submodule(&ast_mod.name).ok_or_else(|| {
                Error::InvalidCrate(format!(
                    "Failed to find sub-module {} for module {:?}",
                    ast_mod.name, module
                ))
            })?);
        }
    }
    let mut structs = Vec::new();
    for sub_mod in &sub_mods {
        structs.append(
            &mut structs_from_file(&sub_mod.path, sub_mod.rust_path.clone())?.unwrap_or_default(),
        );
        structs.append(&mut structs_from_submodules(sub_mod)?);
    }
    Ok(vec![])
}

#[derive(Debug)]
struct Module<'par> {
    path: PathBuf,
    rust_path: Path,
    name: &'par str,
    cat: ModuleCategory,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModuleCategory {
    Root,   // lib.rs
    Direct, // foo.rs
    Mod,    // foo/mod.rs
}

impl Module<'_> {
    fn submodule<'name>(&self, name: &'name str) -> Option<Module<'name>> {
        let mut rust_path = self.rust_path.clone();
        rust_path.push_name(String::from(name));
        // Check the foo.rs form
        let mut mod_path = self.path.clone();
        mod_path.pop();
        if self.cat == ModuleCategory::Direct {
            mod_path.push(self.name);
        }
        mod_path.push(format!("{}.rs", name));
        if mod_path.exists() && mod_path.is_file() {
            return Some(Module {
                path: mod_path,
                name,
                rust_path,
                cat: ModuleCategory::Direct,
            });
        }

        // Check foo/mod.rs form
        let mut mod_path = self.path.clone();
        mod_path.pop();
        if self.cat == ModuleCategory::Direct {
            mod_path.push(self.name);
        }
        mod_path.push(name);
        mod_path.push(format!("{}.rs", name));
        if mod_path.exists() && mod_path.is_file() {
            return Some(Module {
                path: mod_path,
                name,
                rust_path,
                cat: ModuleCategory::Mod,
            });
        }

        None
    }
}

struct ASTModule {
    name: String,
    path: Option<PathBuf>,
}

struct PathAttr {
    eq: Token![=],
    path: LitStr,
}

impl Parse for PathAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(PathAttr {
            eq: input.parse()?,
            path: input.parse()?,
        })
    }
}

fn empty_modules_from_file<T: AsRef<std::path::Path>>(path: T) -> Result<Option<Vec<ASTModule>>> {
    let mut file = File::open(path.as_ref())?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    match syn::parse_file(&content) {
        Ok(ast) => {
            let mut emp_mods = Vec::new();
            for item in &ast.items {
                if let Item::Mod(module) = item {
                    if module.content.is_none() {
                        let name = module.ident.to_string();
                        let attr_name = "path";
                        let mut mod_path = None;
                        for attr in &module.attrs {
                            let seg = &attr.path.segments;
                            if seg.iter().count() == 1 {
                                let path = &seg.iter().next().unwrap().ident;
                                if path == attr_name {
                                    let path_attr: PathAttr = syn::parse2(attr.tokens.clone())?;
                                    mod_path = Some(PathBuf::from(path_attr.path.value()));
                                    break;
                                }
                            }
                        }
                        emp_mods.push(ASTModule {
                            name,
                            path: mod_path,
                        });
                    }
                }
            }
            Ok(Some(emp_mods))
        }
        Err(err) => {
            warn!("{}", err);
            Ok(None)
        }
    }
}
