use crate::error::Error;
use log::debug;
use semver::VersionReq;
use std::{
    fmt::{self, Display, Formatter},
    fs::File,
    io::Read,
    path::{self, PathBuf},
};
use toml::Value;

#[derive(Debug)]
pub struct Dependency {
    name: String,
    version: DependencyVersion,
}

impl Display for Dependency {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "name: {}, version: {}", self.name, self.version)
    }
}

#[derive(Debug)]
pub enum DependencyVersion {
    Simple(VersionReq),
    Git(GitDependency),
}

impl Display for DependencyVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            DependencyVersion::Simple(req) => write!(f, "{}", req),
            DependencyVersion::Git(git) => write!(f, "{}", git),
        }
    }
}

#[derive(Debug)]
pub struct GitDependency {
    url: String,
    version: Option<GitVersion>,
}

impl Display for GitDependency {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "url: {}", self.url)?;
        if let Some(version) = &self.version {
            write!(f, ", {}", version)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum GitVersion {
    Rev(String),
    Tag(String),
    Branch(String),
}

impl Display for GitVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            GitVersion::Rev(rev) => write!(f, "revision: {}", rev),
            GitVersion::Tag(tag) => write!(f, "tag: {}", tag),
            GitVersion::Branch(branch) => write!(f, "branch: {}", branch),
        }
    }
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

    let mut crate_deps = Vec::new();
    for (key, value) in deps {
        let name = key;
        if value.is_str() {
            // Then it is a simple version
            let value = value.as_str().unwrap();
            let version_req = VersionReq::parse(value)?;
            crate_deps.push(Dependency {
                name: String::from(name),
                version: DependencyVersion::Simple(version_req),
            })
        } else if value.is_table() {
            let value = value.as_table().unwrap();
            // TODO: Handle other types of composite specs
            if let Some(url) = value.get("git") {
                let version = if let Some(rev) = value.get("rev") {
                    Some(GitVersion::Rev(String::from(rev.as_str().unwrap())))
                } else if let Some(tag) = value.get("tag") {
                    Some(GitVersion::Tag(String::from(tag.as_str().unwrap())))
                } else if let Some(branch) = value.get("branch") {
                    Some(GitVersion::Tag(String::from(branch.as_str().unwrap())))
                } else {
                    None
                };
                crate_deps.push(Dependency {
                    name: String::from(name),
                    version: DependencyVersion::Git(GitDependency {
                        url: String::from(url.as_str().unwrap()),
                        version,
                    }),
                });
            } else if let Some(version) = value.get("version") {
                let version_req = VersionReq::parse(version.as_str().unwrap())?;
                crate_deps.push(Dependency {
                    name: String::from(name),
                    version: DependencyVersion::Simple(version_req),
                });
            }
        } else {
            unreachable!("expected Cargo.toml to be a valid spec file");
        }
    }
    Ok(crate_deps)
}
