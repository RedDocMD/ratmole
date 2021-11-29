pub mod dag;

use std::{
    cell::RefCell,
    cmp::Ordering,
    collections::{HashMap, HashSet},
    fmt::{self, Display, Formatter},
    iter::FromIterator,
    path::Path as StdPath,
};

use crate::{
    cargo::{parse_cargo, DependentPackage},
    depgraph::dag::{Dag, Node},
    error::Result,
    printer::TreePrintable,
};
use cargo::{core::Package, Config};

#[derive(Eq, Clone)]
struct Crate {
    pkg: DependentPackage,
    dependencies: Vec<Crate>,
}

impl Crate {
    // Crate without any dependencies added
    fn bare_crate(pkg: DependentPackage) -> Self {
        Self {
            pkg,
            dependencies: Vec::new(),
        }
    }

    // Assumption: dep isn't already a dependency
    fn add_dependency(&mut self, dep: Crate) {
        self.dependencies.push(dep);
    }

    // Current package and all unique dependencies
    fn sub_crates(&self) -> HashSet<&DependentPackage> {
        let mut pkgs = HashSet::from_iter([&self.pkg]);
        for dep in &self.dependencies {
            pkgs.extend(dep.sub_crates());
        }
        pkgs
    }
}

impl PartialEq for Crate {
    fn eq(&self, ot: &Self) -> bool {
        self.pkg == ot.pkg
    }
}

impl Ord for Crate {
    fn cmp(&self, ot: &Self) -> Ordering {
        self.pkg.cmp(&ot.pkg)
    }
}

impl PartialOrd for Crate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Crate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} v{}", self.pkg.name(), self.pkg.version())
    }
}

impl TreePrintable for Crate {
    fn single_write(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt(f)
    }

    fn children(&self) -> Vec<&dyn TreePrintable> {
        self.dependencies
            .iter()
            .map(|dep| dep as &dyn TreePrintable)
            .collect()
    }
}

pub struct DepGraph {
    root: Crate,
}

impl Display for DepGraph {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.root.tree_print(f)
    }
}

impl DepGraph {
    pub fn new<T: AsRef<StdPath>>(crate_root: T) -> Result<Self> {
        let config = Config::default()?;
        let (manifest, manifest_path) = parse_cargo(&crate_root, &config)?;
        let root_pkg = DependentPackage::default_from_cargo(Package::new(manifest, &manifest_path));
        let crates = RefCell::new(HashMap::new());
        Ok(DepGraph {
            root: rec_graph_create(&root_pkg, &config, &crates, 0)?,
        })
    }

    pub fn crates(&self) -> HashSet<&DependentPackage> {
        self.root.sub_crates()
    }

    pub fn dag(&self) -> Dag<'_> {
        let mut nodes: Vec<_> = self
            .crates()
            .into_iter()
            .map(|pkg| Node::free_node(pkg))
            .collect();

        let mut stack = vec![&self.root];
        while !stack.is_empty() {
            let crt = stack.pop().unwrap();
            for dep in &crt.dependencies {
                let dep_node = nodes.iter_mut().find(|n| n.pkg() == &dep.pkg).unwrap();
                dep_node.add_dependent(&crt.pkg);
                stack.push(dep);
            }
        }

        Dag::new(nodes)
    }
}

fn rec_graph_create(
    pkg: &DependentPackage,
    config: &Config,
    crates: &RefCell<HashMap<String, Crate>>,
    depth: i32,
) -> Result<Crate> {
    let mut bare_crate = Crate::bare_crate(pkg.clone());
    let dep_pkgs = pkg.download_dependencies(config, true)?;
    for dep_pkg in &dep_pkgs {
        let dep_key = dep_pkg.to_string();
        let mut dep_crate = None;
        if let Some(existing_dep_crate) = crates.borrow_mut().get(&dep_key) {
            dep_crate = Some(existing_dep_crate.clone());
        }
        if dep_crate.is_none() {
            let new_dep_crate = rec_graph_create(dep_pkg, config, crates, depth + 1)?;
            crates.borrow_mut().insert(dep_key, new_dep_crate.clone());
            dep_crate = Some(new_dep_crate);
        }
        bare_crate.add_dependency(dep_crate.unwrap());
    }
    bare_crate.dependencies.sort();
    Ok(bare_crate)
}
