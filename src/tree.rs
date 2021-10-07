use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
    ops::Deref,
};

use crate::{
    item::structs::{Path, PathComponent},
    printer::TreePrintable,
    use_path::{UsePath, UsePathComponent},
};

use colored::*;

#[derive(Debug)]
pub struct PathNode<'s, T> {
    name: String,
    child_mods: HashMap<String, PathNode<'s, T>>,
    child_items: HashMap<String, &'s T>,
}

impl<T> PathNode<'_, T> {
    fn new(name: String) -> Self {
        Self {
            name,
            child_mods: HashMap::new(),
            child_items: HashMap::new(),
        }
    }

    fn resolve_use_path<'item>(&'item self, use_path: &[UsePathComponent]) -> Vec<&'item T> {
        if use_path.len() > 1 {
            let first = use_path[0].as_name().unwrap();
            let child = match self.child_mods.get(first) {
                Some(child) => child,
                None => return Vec::new(),
            };
            child.resolve_use_path(&use_path[1..])
        } else {
            fn resolve_name<'item, T>(node: &'item PathNode<'_, T>, name: &str) -> Vec<&'item T> {
                if node.child_items.contains_key(name) {
                    vec![node.child_items[name]]
                } else {
                    Vec::new()
                }
            }

            match &use_path[0] {
                UsePathComponent::Name(name) => resolve_name(self, name),
                UsePathComponent::Rename(name, _) => resolve_name(self, name),
                UsePathComponent::Glob => self.child_items.values().copied().collect(),
            }
        }
    }
}

#[derive(Debug)]
pub struct ItemTree<'t, T> {
    root: PathNode<'t, T>,
}

impl<'t, T> ItemTree<'t, T>
where
    T: TreeItem,
{
    pub fn new(items: &'t [T]) -> Self {
        let mut tree = Self {
            root: PathNode::new(String::from("<root>")),
        };
        for t in items {
            tree.add_item(t);
        }
        tree
    }

    fn add_item(&mut self, t: &'t T) {
        let comps: Vec<&str> = t
            .module()
            .components()
            .iter()
            .map(|comp| {
                if let PathComponent::Name(name) = comp {
                    name.deref()
                } else {
                    panic!(
                        "expected {} to have a path consisting of only names",
                        t.name()
                    )
                }
            })
            .collect();
        node_add_item(&mut self.root, &comps, t);
    }

    pub fn resolve_use_path<'item>(
        &'item self,
        use_path: &UsePath,
        start_mod: &Path,
    ) -> Vec<&'item T> {
        let mut node = &self.root;
        for comp in start_mod.components() {
            node = match node.child_mods.get(&comp.to_string()) {
                Some(node) => node,
                None => return Vec::new(),
            };
        }
        node.resolve_use_path(use_path.components()).to_vec()
    }
}

fn node_add_item<'t, 'c, T>(node: &mut PathNode<'t, T>, comps: &'c [&'t str], item: &'t T)
where
    T: TreeItem,
{
    if comps.is_empty() {
        node.child_items.insert(String::from(item.name()), item);
    } else {
        if !node.child_mods.contains_key(comps[0]) {
            let name = String::from(comps[0]);
            node.child_mods.insert(name.clone(), PathNode::new(name));
        }
        let new_node = node.child_mods.get_mut(comps[0]).unwrap();
        node_add_item(new_node, &comps[1..], item);
    }
}

impl<T> TreePrintable for PathNode<'_, T>
where
    T: TreePrintable,
{
    fn single_write(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", "mod".magenta(), self.name)
    }

    fn children(&self) -> Vec<&dyn TreePrintable> {
        let mods = self.child_mods.values().map(|x| x as &dyn TreePrintable);
        let items = self.child_items.values().map(|x| *x as &dyn TreePrintable);
        mods.chain(items).collect()
    }
}

impl<T> Display for ItemTree<'_, T>
where
    T: TreePrintable,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.root.tree_print(f)
    }
}

pub trait TreeItem {
    fn name(&self) -> &str;
    fn module(&self) -> &Path;
}
