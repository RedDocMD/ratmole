use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    ops::Deref,
};

use crate::{
    printer::TreePrintable,
    structs::{Path, PathComponent, Struct},
    use_path::{UsePath, UsePathComponent},
};

use colored::*;

#[derive(Debug)]
pub struct PathNode<'s> {
    name: String,
    child_mods: HashMap<String, PathNode<'s>>,
    child_structs: HashMap<String, &'s Struct>,
}

impl PathNode<'_> {
    fn new(name: String) -> Self {
        Self {
            name,
            child_mods: HashMap::new(),
            child_structs: HashMap::new(),
        }
    }

    fn resolve_use_path<'item>(
        &'item self,
        use_path: &[UsePathComponent],
        mod_path: &mut Vec<PathComponent>,
    ) -> Vec<ResolvedUsePath<'item>> {
        if use_path.len() > 1 {
            let first = use_path[0].as_name().unwrap();
            let child = match self.child_mods.get(first) {
                Some(child) => child,
                None => return Vec::new(),
            };
            mod_path.push(PathComponent::Name(first.clone()));
            let things = child.resolve_use_path(&use_path[1..], mod_path);
            mod_path.pop();
            things
        } else {
            fn resolve_name<'item>(
                node: &'item PathNode<'_>,
                name: &String,
                mod_path: &Vec<PathComponent>,
            ) -> Vec<ResolvedUsePath<'item>> {
                if node.child_structs.contains_key(name) {
                    vec![ResolvedUsePath::Struct(&node.child_structs[name])]
                } else if node.child_mods.contains_key(name) {
                    let mut new_path_comps = mod_path.clone();
                    new_path_comps.push(PathComponent::Name(name.clone()));
                    vec![ResolvedUsePath::Module(Path::new(new_path_comps))]
                } else {
                    Vec::new()
                }
            }

            match &use_path[0] {
                UsePathComponent::Name(name) => resolve_name(self, name, mod_path),
                UsePathComponent::Rename(name, _) => resolve_name(self, name, mod_path),
                UsePathComponent::Glob => self
                    .child_structs
                    .values()
                    .copied()
                    .map(|item| ResolvedUsePath::Struct(item))
                    .chain(self.child_mods.keys().map(|name| {
                        let mut new_path_comps = mod_path.clone();
                        new_path_comps.push(PathComponent::Name(name.clone()));
                        ResolvedUsePath::Module(Path::new(new_path_comps))
                    }))
                    .collect(),
            }
        }
    }
}

pub enum ResolvedUsePath<'item> {
    Struct(&'item Struct),
    Module(Path),
}

impl Display for ResolvedUsePath<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ResolvedUsePath::Struct(s) => write!(f, "{}", s),
            ResolvedUsePath::Module(p) => write!(f, "{}", p),
        }
    }
}

#[derive(Debug)]
pub struct StructTree<'s> {
    structs: &'s [Struct],
    root: PathNode<'s>,
}

impl<'s> StructTree<'s> {
    pub fn new(structs: &'s [Struct]) -> Self {
        let mut tree = Self {
            structs,
            root: PathNode::new(String::from("<root>")),
        };
        for st in structs {
            tree.add_struct(st);
        }
        tree
    }

    fn add_struct(&mut self, st: &'s Struct) {
        let comps: Vec<&str> = st
            .module()
            .components()
            .iter()
            .map(|comp| {
                if let PathComponent::Name(name) = comp {
                    name.deref()
                } else {
                    panic!("expected {} to have a path consisting of only names", st)
                }
            })
            .collect();
        node_add_struct(&mut self.root, &comps, st);
    }

    pub fn resolve_use_path<'item>(
        &'item self,
        use_path: &UsePath,
        start_mod: &Path,
    ) -> Vec<ResolvedUsePath<'item>> {
        let mut node = &self.root;
        for comp in start_mod.components() {
            node = match node.child_mods.get(&comp.to_string()) {
                Some(node) => node,
                None => return Vec::new(),
            };
        }
        node.resolve_use_path(use_path.components(), &mut start_mod.components().to_vec())
    }
}

fn node_add_struct<'s, 'c>(node: &mut PathNode<'s>, comps: &'c [&'s str], st: &'s Struct) {
    if comps.is_empty() {
        node.child_structs.insert(String::from(st.name()), st);
    } else {
        if !node.child_mods.contains_key(comps[0]) {
            let name = String::from(comps[0]);
            node.child_mods.insert(name.clone(), PathNode::new(name));
        }
        let new_node = node.child_mods.get_mut(comps[0]).unwrap();
        node_add_struct(new_node, &comps[1..], st);
    }
}

impl TreePrintable for PathNode<'_> {
    fn single_write(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", "mod".magenta(), self.name)
    }

    fn children(&self) -> Vec<&dyn TreePrintable> {
        let mods = self.child_mods.values().map(|x| x as &dyn TreePrintable);
        let structs = self.child_structs.values().map(|x| x as &dyn TreePrintable);
        mods.chain(structs).collect()
    }
}

impl Display for StructTree<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.root.tree_print(f)
    }
}
