use crate::{
    cargo::{download_dependency, parse_cargo},
    error::{Error, Result},
    structs::{structs_from_items, Path, Struct},
};
use cargo::{
    core::{manifest::TargetSourcePath, Package, SourceId, Target, TargetKind},
    sources::SourceConfigMap,
    Config,
};
use log::{debug, warn};
use rayon::prelude::*;
use std::{fs::File, io::Read, path::PathBuf};
use syn::{parenthesized, parse::Parse, token, Item, LitStr, Token};

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

struct SimplePackage {
    targets: Vec<SimpleTarget>,
    name: String,
}

impl SimplePackage {
    fn from_cargo(pkg: Package) -> Self {
        let targets: Vec<SimpleTarget> = pkg
            .targets()
            .into_iter()
            .map(SimpleTarget::from_cargo)
            .collect();
        Self {
            targets,
            name: String::from(pkg.name().as_str()),
        }
    }

    fn targets(&self) -> &[SimpleTarget] {
        self.targets.as_slice()
    }

    fn library(&self) -> Option<&SimpleTarget> {
        self.targets.iter().find(|targ| {
            if let TargetKind::Lib(_) = targ.kind {
                true
            } else {
                false
            }
        })
    }

    fn name(&self) -> &String {
        &self.name
    }
}

struct SimpleTarget {
    crate_name: String,
    kind: TargetKind,
    src_path: TargetSourcePath,
}

impl SimpleTarget {
    fn from_cargo(targ: &Target) -> Self {
        Self {
            crate_name: targ.crate_name(),
            kind: targ.kind().clone(),
            src_path: targ.src_path().clone(),
        }
    }

    fn crate_name(&self) -> &String {
        &self.crate_name
    }

    fn src_path(&self) -> &TargetSourcePath {
        &self.src_path
    }
}

pub fn structs_in_crate_and_deps<T: AsRef<std::path::Path>>(
    main_crate_root: T,
) -> Result<Vec<Struct>> {
    let config = Config::default()?;
    let (manifest, manifest_path) = parse_cargo(&main_crate_root, &config)?;
    let main_pkg = SimplePackage::from_cargo(Package::new(manifest, &manifest_path));

    let mut structs = Vec::new();
    debug!("Exploring {}", main_pkg.name());
    structs.append(&mut structs_in_main_crate(&main_pkg)?);

    let pkgs = get_dependencies(&main_crate_root)?;
    let pkgs: Vec<SimplePackage> = pkgs.into_iter().map(SimplePackage::from_cargo).collect();
    for pkg in &pkgs {
        debug!("Exploring {}", pkg.name());
        structs.append(&mut structs_in_dependency(pkg)?);
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

fn structs_in_main_crate(pkg: &SimplePackage) -> Result<Vec<Struct>> {
    let mut structs = Vec::new();
    for target in pkg.targets() {
        structs.append(&mut structs_in_target(target)?);
    }
    Ok(structs)
}

fn structs_in_dependency(pkg: &SimplePackage) -> Result<Vec<Struct>> {
    match pkg.library() {
        Some(lib) => structs_in_target(lib),
        None => Ok(Vec::new()),
    }
}

fn structs_in_target(targ: &SimpleTarget) -> Result<Vec<Struct>> {
    let src_path = match targ.src_path() {
        TargetSourcePath::Path(path) => path,
        TargetSourcePath::Metabuild => return Ok(vec![]),
    };
    let crate_name = targ.crate_name();
    let mut structs = Vec::new();
    structs.append(
        &mut structs_from_file(&src_path, Path::from(vec![crate_name.clone()]))?.unwrap_or_else(
            || {
                warn!("failed to parse {}", src_path.as_os_str().to_str().unwrap());
                vec![]
            },
        ),
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

            let mut new_path = module.path.clone();
            new_path.pop();
            new_path.push(path);

            let cat = if new_path.file_name().unwrap() == "mod.rs" {
                ModuleCategory::Mod
            } else {
                ModuleCategory::Direct
            };

            sub_mods.push(Module {
                path: new_path,
                rust_path: new_mod_path,
                name: &ast_mod.name,
                cat,
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
            &mut structs_from_file(&sub_mod.path, sub_mod.rust_path.clone())?.unwrap_or_else(
                || {
                    warn!(
                        "failed to parse {}",
                        sub_mod.path.as_os_str().to_str().unwrap()
                    );
                    vec![]
                },
            ),
        );
        structs.append(&mut structs_from_submodules(sub_mod)?);
    }
    Ok(structs)
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
        mod_path.push("mod.rs");
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
    _eq: Token![=],
    path: LitStr,
}

impl Parse for PathAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        Ok(PathAttr {
            _eq: input.parse()?,
            path: input.parse()?,
        })
    }
}

struct CfgAttrWithPath {
    _paren: token::Paren,
    _cond: syn::Ident,
    _comma: Token![,],
    _path_word: syn::Ident,
    _eq: Token![=],
    path: LitStr,
}

impl Parse for CfgAttrWithPath {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let content;
        Ok(CfgAttrWithPath {
            _paren: parenthesized!(content in input),
            _cond: content.parse()?,
            _comma: content.parse()?,
            _path_word: content.parse()?,
            _eq: content.parse()?,
            path: content.parse()?,
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
                        let mut mod_path = None;
                        for attr in &module.attrs {
                            let seg = &attr.path.segments;
                            if seg.iter().count() == 1 {
                                let path = &seg.iter().next().unwrap().ident;
                                if path == "path" {
                                    let path_attr: PathAttr = syn::parse2(attr.tokens.clone())?;
                                    mod_path = Some(PathBuf::from(path_attr.path.value()));
                                    break;
                                } else if path == "cfg_attr" {
                                    let cfg_attr: std::result::Result<CfgAttrWithPath, syn::Error> =
                                        syn::parse2(attr.tokens.clone());
                                    if let Ok(cfg_attr) = cfg_attr {
                                        mod_path = Some(PathBuf::from(cfg_attr.path.value()));
                                        break;
                                    }
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
