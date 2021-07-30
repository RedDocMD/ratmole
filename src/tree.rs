use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    ops::Deref,
};

use crate::{
    printer::TreePrintable,
    structs::{PathComponent, Struct},
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

    fn resolve_use_path(&self, use_path: &[UsePathComponent]) -> Vec<Struct> {
        if use_path.len() > 1 {
            let first = use_path[0].as_name().unwrap();
            let child = &self.child_mods[first];
            child.resolve_use_path(&use_path[1..])
        } else {
            match &use_path[0] {
                UsePathComponent::Name(name) => vec![self.child_structs[name].clone()],
                UsePathComponent::Rename(name, rename) => {
                    vec![self.child_structs[name].renamed(rename)]
                }
                UsePathComponent::Glob => {
                    self.child_structs.values().map(|s| (*s).clone()).collect()
                }
            }
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

    pub fn resolve_use_path(&self, use_path: &UsePath) -> Vec<Struct> {
        let mut structs = self.root.resolve_use_path(use_path.components());
        for s in &mut structs {
            s.set_visibility(use_path.visibility().clone());
        }
        structs
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
