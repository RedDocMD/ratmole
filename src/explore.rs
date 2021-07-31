use crate::{
    cargo::{download_dependency, parse_cargo},
    error::{Error, Result},
    structs::{structs_from_items, ModuleInfo, Path, Struct, Visibility},
    use_path::{use_paths_from_items, UsePath},
};
use cargo::{
    core::{
        dependency::DepKind, manifest::TargetSourcePath, Package, Source, SourceId, Target,
        TargetKind,
    },
    sources::{GitSource, PathSource, SourceConfigMap},
    Config,
};
use colored::*;
use log::{debug, warn};
use rayon::prelude::*;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    fs::File,
    io::Read,
    path::PathBuf,
};
use syn::{parenthesized, parse::Parse, token, Item, LitStr, Token};

fn things_from_file<T, F, R>(
    file_path: T,
    mut module: crate::structs::Path,
    f: F,
) -> Result<Option<R>>
where
    T: AsRef<std::path::Path>,
    F: Fn(&[syn::Item], &mut Path) -> R,
{
    debug!("{}", file_path.as_ref().as_os_str().to_str().unwrap());
    let mut file = File::open(file_path.as_ref())?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    match syn::parse_file(&contents) {
        Ok(ast) => Ok(Some(f(&ast.items, &mut module))),
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
        let targets: Vec<SimpleTarget> =
            pkg.targets().iter().map(SimpleTarget::from_cargo).collect();
        Self {
            targets,
            name: String::from(pkg.name().as_str()),
        }
    }

    fn targets(&self) -> &[SimpleTarget] {
        self.targets.as_slice()
    }

    fn library(&self) -> Option<&SimpleTarget> {
        self.targets
            .iter()
            .find(|targ| matches!(targ.kind, TargetKind::Lib(_)))
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

pub struct MainCrateInfo {
    structs: Vec<Struct>,
    main_mod_info: HashMap<SimpleTargetKind, ModuleInfo>,
    dep_mod_info: HashMap<String, ModuleInfo>,
}

impl MainCrateInfo {
    pub fn structs(&self) -> &[Struct] {
        &self.structs
    }
}

impl Display for MainCrateInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        writeln!(f, "{}", "STRUCTS".bright_red())?;
        for s in &self.structs {
            writeln!(f, "{}", s)?;
        }
        writeln!(f, "\n{}", "MAIN CRATE".bright_red())?;
        for (targ, info) in &self.main_mod_info {
            writeln!(f, "{}", targ)?;
            writeln!(f, "{}", info)?;
        }
        writeln!(f)?;
        for (name, info) in &self.dep_mod_info {
            writeln!(f, "{}", name.bright_red())?;
            writeln!(f, "{}", info)?;
        }
        Ok(())
    }
}

#[derive(PartialEq, Eq, Hash)]
pub enum SimpleTargetKind {
    Binary,
    Library,
    ExampleLib,
    ExampleBin,
    Benchmark,
    Test,
    Custom,
}

impl From<TargetKind> for SimpleTargetKind {
    fn from(kind: TargetKind) -> SimpleTargetKind {
        match kind {
            TargetKind::Lib(_) => SimpleTargetKind::Library,
            TargetKind::Bin => SimpleTargetKind::Binary,
            TargetKind::Test => SimpleTargetKind::Test,
            TargetKind::Bench => SimpleTargetKind::Benchmark,
            TargetKind::ExampleLib(_) => SimpleTargetKind::ExampleLib,
            TargetKind::ExampleBin => SimpleTargetKind::ExampleBin,
            TargetKind::CustomBuild => SimpleTargetKind::Custom,
        }
    }
}

impl Display for SimpleTargetKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SimpleTargetKind::Binary => write!(f, "Binary"),
            SimpleTargetKind::Library => write!(f, "Library"),
            SimpleTargetKind::ExampleLib => write!(f, "ExampleLib"),
            SimpleTargetKind::ExampleBin => write!(f, "ExampleBin"),
            SimpleTargetKind::Benchmark => write!(f, "Benchmark"),
            SimpleTargetKind::Test => write!(f, "Test"),
            SimpleTargetKind::Custom => write!(f, "Custom"),
        }
    }
}

pub fn crate_info<T: AsRef<std::path::Path>>(main_crate_root: T) -> Result<MainCrateInfo> {
    let config = Config::default()?;
    let (manifest, manifest_path) = parse_cargo(&main_crate_root, &config)?;
    let main_cargo_pkg = Package::new(manifest, &manifest_path);
    let pkgs = download_dependencies(&main_cargo_pkg, &config)?;

    let main_pkg = SimplePackage::from_cargo(main_cargo_pkg);
    let mut structs = Vec::new();
    let mut dep_mod_info = HashMap::new();

    debug!("Exploring {}", main_pkg.name());
    let (mut main_structs, main_mod_info) = structs_in_main_crate(&main_pkg)?;
    structs.append(&mut main_structs);

    let pkgs: Vec<SimplePackage> = pkgs.into_iter().map(SimplePackage::from_cargo).collect();
    let mut things = Vec::new();
    pkgs.par_iter()
        .map(|pkg| {
            debug!("Exploring {}", pkg.name());
            structs_in_dependency(pkg).unwrap()
        })
        .collect_into_vec(&mut things);
    let (dep_structs, dep_infos): (Vec<_>, Vec<_>) = things.into_iter().unzip();
    structs.append(&mut dep_structs.into_iter().flatten().collect());
    dep_mod_info.extend(
        dep_infos
            .into_iter()
            .map(|info| (String::from(info.name()), info)),
    );

    for pkg in &pkgs {
        use_paths_in_dependency(pkg)?;
    }

    Ok(MainCrateInfo {
        structs,
        main_mod_info,
        dep_mod_info,
    })
}

fn download_dependencies(pkg: &Package, config: &Config) -> Result<Vec<Package>> {
    let _lock = config.acquire_package_cache_lock()?;
    let crates_io_id = SourceId::crates_io(&config)?;
    let config_map = SourceConfigMap::new(&config)?;
    let mut crates_io = config_map.load(crates_io_id, &Default::default())?;
    crates_io.update()?;

    let mut dep_pkgs = Vec::new();
    for dep in pkg.dependencies() {
        if dep.kind() != DepKind::Normal {
            continue;
        }
        debug!("Downloading {} ...", dep.name_in_toml());
        let dep_src_id = dep.source_id();
        if dep_src_id == crates_io_id {
            debug!("from crates");
            dep_pkgs.push(download_dependency(dep, &mut crates_io, &config)?);
        } else if dep_src_id.is_path() {
            debug!("from path");
            let path = dep_src_id
                .url()
                .to_file_path()
                .expect(&format!("path of {} must be valid", dep.name_in_toml()));
            let mut src = PathSource::new(&path, dep_src_id, &config);
            src.update()?;
            dep_pkgs.push(download_dependency(dep, &mut src, &config)?);
        } else if dep_src_id.is_git() {
            debug!("from git");
            dep_pkgs.push(download_dependency(
                dep,
                GitSource::new(dep_src_id, &config)?,
                &config,
            )?);
        } else {
            debug!("from elsewhere");
            let config_map = SourceConfigMap::new(&config)?;
            let mut src = config_map.load(dep_src_id, &Default::default())?;
            src.update()?;
            dep_pkgs.push(download_dependency(dep, &mut src, &config)?);
        }
        debug!(" ... downloaded {}", dep.name_in_toml());
    }
    Ok(dep_pkgs)
}

fn structs_in_main_crate(
    pkg: &SimplePackage,
) -> Result<(Vec<Struct>, HashMap<SimpleTargetKind, ModuleInfo>)> {
    let mut structs = Vec::new();
    let mut infos = HashMap::new();
    for target in pkg.targets() {
        let (mut new_structs, info) = structs_in_target(target)?;
        structs.append(&mut new_structs);
        infos.insert(SimpleTargetKind::from(target.kind.clone()), info);
    }
    Ok((structs, infos))
}

fn structs_in_dependency(pkg: &SimplePackage) -> Result<(Vec<Struct>, ModuleInfo)> {
    match pkg.library() {
        Some(lib) => Ok(structs_in_target(lib)?),
        None => Ok((
            Vec::new(),
            ModuleInfo::new(pkg.name.clone(), Visibility::Public),
        )),
    }
}

fn use_paths_in_dependency(pkg: &SimplePackage) -> Result<HashMap<Path, Vec<UsePath>>> {
    match pkg.library() {
        Some(lib) => Ok(use_paths_in_target(lib)?),
        None => Ok(HashMap::new()),
    }
}

fn structs_in_target(targ: &SimpleTarget) -> Result<(Vec<Struct>, ModuleInfo)> {
    let crate_name = targ.crate_name();
    let src_path = match targ.src_path() {
        TargetSourcePath::Path(path) => path,
        TargetSourcePath::Metabuild => {
            return Ok((
                vec![],
                ModuleInfo::new(crate_name.clone(), Visibility::Public),
            ))
        }
    };

    let mut structs = Vec::new();
    let mut info = ModuleInfo::new(crate_name.clone(), Visibility::Public);

    let (mut new_structs, child_infos) = things_from_file(
        &src_path,
        Path::from(vec![crate_name.clone()]),
        structs_from_items,
    )?
    .unwrap_or_else(|| {
        warn!("failed to parse {}", src_path.display());
        (vec![], vec![])
    });
    structs.append(&mut new_structs);
    info.add_children(child_infos);

    let (mut new_structs, child_infos) = structs_from_submodules(&Module {
        cat: ModuleCategory::Root,
        name: &crate_name,
        rust_path: Path::from(vec![crate_name.clone()]),
        path: src_path.clone(),
        vis: Visibility::Public,
    })?;
    structs.append(&mut new_structs);
    info.add_children(child_infos);

    Ok((structs, info))
}

fn use_paths_in_target(targ: &SimpleTarget) -> Result<HashMap<Path, Vec<UsePath>>> {
    let crate_name = targ.crate_name();
    let src_path = match targ.src_path() {
        TargetSourcePath::Path(path) => path,
        TargetSourcePath::Metabuild => return Ok(HashMap::new()),
    };
    let mut use_paths = things_from_file(
        &src_path,
        Path::from(vec![crate_name.clone()]),
        use_paths_from_items,
    )?
    .unwrap_or_else(|| {
        warn!("failed to parse {}", src_path.display());
        HashMap::new()
    });
    for (k, v) in &use_paths {
        println!("{}: ", k.to_string().red());
        for p in v {
            println!("    {}", p);
        }
    }
    Ok(use_paths)
}

fn structs_from_submodules(module: &Module<'_>) -> Result<(Vec<Struct>, Vec<ModuleInfo>)> {
    let empty_mods = match empty_modules_from_file(&module.path)? {
        Some(mods) => mods,
        None => return Ok((vec![], vec![])),
    };

    let sub_mods = module.direct_submodules(&empty_mods)?;

    let mut things = Vec::new();
    sub_mods
        .par_iter()
        .map(|sub_mod| {
            let mut sub_mod_info = ModuleInfo::new(String::from(sub_mod.name), sub_mod.vis.clone());
            let (mut self_structs, child_infos) =
                things_from_file(&sub_mod.path, sub_mod.rust_path.clone(), structs_from_items)
                    .unwrap()
                    .unwrap_or_else(|| {
                        warn!(
                            "failed to parse {}",
                            sub_mod.path.as_os_str().to_str().unwrap()
                        );
                        (vec![], vec![])
                    });
            sub_mod_info.add_children(child_infos);
            let (mut sub_structs, child_infos) = structs_from_submodules(sub_mod).unwrap();
            sub_mod_info.add_children(child_infos);

            self_structs.append(&mut sub_structs);
            (self_structs, sub_mod_info)
        })
        .collect_into_vec(&mut things);

    let (vec_structs, infos): (Vec<_>, Vec<_>) = things.into_iter().unzip();
    Ok((vec_structs.into_iter().flatten().collect(), infos))
}

#[derive(Debug)]
struct Module<'par> {
    path: PathBuf,
    rust_path: Path,
    name: &'par str,
    cat: ModuleCategory,
    vis: Visibility,
}

impl Display for Module<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {} of type {}",
            self.rust_path,
            self.path.display(),
            self.cat
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ModuleCategory {
    Root,   // lib.rs
    Direct, // foo.rs
    Mod,    // foo/mod.rs
}

impl Display for ModuleCategory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        use ModuleCategory::*;
        match self {
            Root => write!(f, "Root"),
            Direct => write!(f, "Direct"),
            Mod => write!(f, "Mod"),
        }
    }
}

impl Module<'_> {
    fn submodule<'name>(&self, name: &'name str, vis: Visibility) -> Option<Module<'name>> {
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
                vis,
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
                vis,
            });
        }

        None
    }

    fn direct_submodules<'m>(&self, empty_mods: &'m [ASTModule]) -> Result<Vec<Module<'m>>> {
        let mut sub_mods = Vec::new();
        for ast_mod in empty_mods {
            if let Some(path) = &ast_mod.path {
                let mut new_mod_path = self.rust_path.clone();
                new_mod_path.push_name(ast_mod.name.clone());

                let mut new_path = self.path.clone();
                new_path.pop();
                new_path.push(path);

                let file_name = new_path.file_name().unwrap();
                let cat = if file_name == "mod.rs" {
                    ModuleCategory::Mod
                } else if file_name == "lib.rs" {
                    ModuleCategory::Root
                } else {
                    ModuleCategory::Direct
                };

                sub_mods.push(Module {
                    path: new_path,
                    rust_path: new_mod_path,
                    name: &ast_mod.name,
                    cat,
                    vis: ast_mod.vis.clone(),
                });
            } else {
                sub_mods.push(
                    self.submodule(&ast_mod.name, ast_mod.vis.clone())
                        .ok_or_else(|| {
                            Error::InvalidCrate(format!(
                                "Failed to find sub-self {} for module {}",
                                ast_mod.name, self
                            ))
                        })?,
                );
            }
        }
        Ok(sub_mods)
    }
}

struct ASTModule {
    name: String,
    path: Option<PathBuf>,
    vis: Visibility,
}

struct PathAttr {
    _eq: Token![=],
    path: LitStr,
}

impl Parse for PathAttr {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let eq = input.parse()?;
        let path = input.parse()?;
        Ok(PathAttr { _eq: eq, path })
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
        let paren = parenthesized!(content in input);
        let cond = content.parse()?;
        let comma = content.parse()?;
        let path_word = content.parse()?;
        let eq = content.parse()?;
        let path = content.parse()?;
        Ok(CfgAttrWithPath {
            _paren: paren,
            _cond: cond,
            _comma: comma,
            _path_word: path_word,
            _eq: eq,
            path,
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
                            vis: Visibility::from_syn(&module.vis),
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
