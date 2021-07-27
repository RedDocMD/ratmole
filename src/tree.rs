use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    ops::Deref,
};

use crate::{
    printer::TreePrintable,
    structs::{PathComponent, Struct},
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
