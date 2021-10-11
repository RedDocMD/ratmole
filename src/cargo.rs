use crate::error::{Error, Result};
use cargo::{
    core::{
        dependency::DepKind, Dependency, FeatureMap, FeatureValue, Manifest, Package, Source,
        SourceId,
    },
    sources::{GitSource, PathSource, SourceConfigMap},
    util::{interning::InternedString, toml::TomlManifest},
    Config,
};
use colored::*;
use log::debug;
use semver::Version;
use std::{
    array::IntoIter as ArrayIntoIter,
    collections::HashSet,
    fmt::{self, Display, Formatter},
    fs::File,
    io::Read,
    iter::FromIterator,
    path::{self, PathBuf},
    rc::Rc,
    result::Result as StdResult,
};

pub fn parse_cargo<T: AsRef<path::Path>>(
    crate_root: T,
    config: &Config,
) -> StdResult<(Manifest, PathBuf), Error> {
    let mut toml_path = PathBuf::from(crate_root.as_ref());
    toml_path.push("Cargo.toml");
    let mut toml_file = File::open(&toml_path)?;
    let mut toml_content = String::new();
    toml_file.read_to_string(&mut toml_content)?;

    let toml_manifest: TomlManifest = toml::from_str(&toml_content)?;
    let toml_manifest = Rc::new(toml_manifest);
    let source_id = SourceId::for_path(crate_root.as_ref())?;
    let (manifest, paths) =
        TomlManifest::to_real_manifest(&toml_manifest, source_id, crate_root.as_ref(), config)?;
    debug!("{}: {:?}", "Paths".red(), paths);
    Ok((manifest, toml_path))
}

fn download_dependency_from_src<'a, T>(
    dep: &Dependency,
    mut src: T,
    config: &Config,
) -> StdResult<Package, Error>
where
    T: Source + 'a,
{
    let opts = src.query_vec(dep)?;
    let latest = opts
        .iter()
        .max_by_key(|x| x.version())
        .ok_or_else(|| Error::PackageNotFound(String::from(dep.name_in_toml().as_str())))?;
    let pkg = Box::new(src).download_now(latest.package_id(), config)?;
    Ok(pkg)
}

pub fn download_package_deps(pkg: &Package, config: &Config) -> Result<Vec<Package>> {
    download_dependencies(pkg.dependencies(), config)
}

pub struct DependentPackage {
    package: Package,
    enabled_features: HashSet<FeatureValue>,
}

impl DependentPackage {
    fn from_cargo(pkg: Package, pkg_parent: &Self, pkg_dep: &Dependency) -> Self {
        let name = pkg_dep.name_in_toml();
        let feature_map = pkg.summary().features();
        let features_from_dep: HashSet<_> = pkg_dep
            .features()
            .iter()
            .map(|feat_name| {
                let feature = FeatureValue::Feature(feat_name.clone());
                transitive_features(&feature, &feature_map)
            })
            .flatten()
            .collect();
        let features_from_parent: HashSet<_> = pkg_parent
            .enabled_features
            .iter()
            .filter_map(|feat| match feat {
                FeatureValue::DepFeature {
                    dep_name,
                    dep_feature,
                    weak: _,
                } => {
                    if dep_name == &name {
                        let feature = FeatureValue::Feature(dep_feature.clone());
                        Some(transitive_features(&feature, &feature_map))
                    } else {
                        None
                    }
                }
                _ => None,
            })
            .flatten()
            .collect();

        let mut enabled_features = features_from_dep;
        enabled_features.extend(features_from_parent.into_iter());
        if pkg_dep.uses_default_features() {
            let default_features = default_features(&pkg);
            enabled_features.extend(default_features.into_iter());
        }

        Self {
            package: pkg,
            enabled_features,
        }
    }

    pub fn default_from_cargo(pkg: Package) -> Self {
        let enabled_features = default_features(&pkg);
        Self {
            package: pkg,
            enabled_features,
        }
    }

    fn dependencies(&self) -> Vec<&Dependency> {
        self.package
            .dependencies()
            .iter()
            .filter(|dep| {
                if dep.kind() == DepKind::Normal {
                    if !dep.is_optional() {
                        true
                    } else {
                        let name = dep.name_in_toml();
                        self.enabled_features.iter().any(|feat| match feat {
                            FeatureValue::Feature(_) => false,
                            FeatureValue::Dep { dep_name } => dep_name == &name,
                            FeatureValue::DepFeature { dep_name, weak, .. } => {
                                dep_name == &name && !weak
                            }
                        })
                    }
                } else {
                    false
                }
            })
            .collect()
    }

    pub fn download_dependencies(
        &self,
        config: &Config,
        update_crates_io: bool,
    ) -> Result<Vec<Self>> {
        let _lock = config.acquire_package_cache_lock()?;
        let crates_io_id = SourceId::crates_io(config)?;
        let config_map = SourceConfigMap::new(config)?;
        let mut crates_io = config_map.load(crates_io_id, &Default::default())?;
        if update_crates_io {
            crates_io.update()?;
        }

        let mut dep_pkgs = Vec::new();
        for dep in self.dependencies() {
            let pkg = download_dependency(dep, &config, &crates_io_id, crates_io.as_mut())?;
            let dep_pkg = Self::from_cargo(pkg, self, dep);
            dep_pkgs.push(dep_pkg);
        }
        Ok(dep_pkgs)
    }

    pub fn name(&self) -> InternedString {
        self.package.name()
    }

    pub fn version(&self) -> &Version {
        self.package.version()
    }
}

impl Display for DependentPackage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} v{}", self.name(), self.version())
    }
}

fn default_features(package: &Package) -> HashSet<FeatureValue> {
    let feature_map = package.summary().features();
    let default_feature_string: InternedString = InternedString::new("default");
    let default_feature: FeatureValue = FeatureValue::Feature(default_feature_string.clone());
    if feature_map.contains_key(&default_feature_string) {
        transitive_features(&default_feature, feature_map)
    } else {
        HashSet::new()
    }
}

fn transitive_features(feature: &FeatureValue, feature_map: &FeatureMap) -> HashSet<FeatureValue> {
    let mut features = HashSet::from_iter(ArrayIntoIter::new([feature.clone()]));
    if let FeatureValue::Feature(feat_str) = feature {
        if let Some(sub_features) = feature_map.get(feat_str) {
            features.extend(
                sub_features
                    .iter()
                    .map(|feat| transitive_features(feat, feature_map))
                    .flatten(),
            )
        }
    }
    features
}

pub fn download_dependencies(dependencies: &[Dependency], config: &Config) -> Result<Vec<Package>> {
    let _lock = config.acquire_package_cache_lock()?;
    let crates_io_id = SourceId::crates_io(config)?;
    let config_map = SourceConfigMap::new(config)?;
    let mut crates_io = config_map.load(crates_io_id, &Default::default())?;
    crates_io.update()?;

    let mut dep_pkgs = Vec::new();
    for dep in dependencies {
        if dep.kind() != DepKind::Normal {
            continue;
        }
        dep_pkgs.push(download_dependency(
            dep,
            config,
            &crates_io_id,
            crates_io.as_mut(),
        )?);
        debug!(" ... downloaded {}", dep.name_in_toml());
    }
    Ok(dep_pkgs)
}

fn download_dependency(
    dep: &Dependency,
    config: &Config,
    crates_io_id: &SourceId,
    crates_io: &mut dyn Source,
) -> Result<Package> {
    debug!("Downloading {} ...", dep.name_in_toml());
    let dep_src_id = dep.source_id();
    if &dep_src_id == crates_io_id {
        debug!("from crates");
        download_dependency_from_src(dep, crates_io, config)
    } else if dep_src_id.is_path() {
        debug!("from path");
        let path = dep_src_id
            .url()
            .to_file_path()
            .unwrap_or_else(|_| panic!("path of {} must be valid", dep.name_in_toml()));
        let mut src = PathSource::new(&path, dep_src_id, config);
        src.update()?;
        download_dependency_from_src(dep, &mut src, config)
    } else if dep_src_id.is_git() {
        debug!("from git");
        download_dependency_from_src(dep, GitSource::new(dep_src_id, config)?, config)
    } else {
        debug!("from elsewhere");
        let config_map = SourceConfigMap::new(config)?;
        let mut src = config_map.load(dep_src_id, &Default::default())?;
        src.update()?;
        download_dependency_from_src(dep, &mut src, config)
    }
}
