use std::{collections::HashMap, ops::Deref};

use crate::structs::{PathComponent, Struct};

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
            root: PathNode::new(String::from("")),
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
