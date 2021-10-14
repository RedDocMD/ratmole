use lazy_static::lazy_static;
use regex::Regex;
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use crate::item::structs::{Path, Visibility};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UsePathComponent {
    Name(String),
    Rename(String, String),
    Glob,
    Empty,
}

impl UsePathComponent {
    pub fn as_name(&self) -> Option<&String> {
        if let UsePathComponent::Name(name) = self {
            Some(name)
        } else {
            None
        }
    }
}

impl Display for UsePathComponent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            UsePathComponent::Name(name) => write!(f, "{}", name),
            UsePathComponent::Rename(name, rename) => write!(f, "{} as {}", name, rename),
            UsePathComponent::Glob => write!(f, "*"),
            UsePathComponent::Empty => write!(f, ""),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UsePath {
    path: Vec<UsePathComponent>,
    vis: Visibility,
}

impl UsePath {
    fn new(path: Vec<UsePathComponent>, vis: Visibility) -> Self {
        Self { path, vis }
    }

    pub fn components(&self) -> &[UsePathComponent] {
        &self.path
    }

    pub fn visibility(&self) -> &Visibility {
        &self.vis
    }

    // Given a use_path self belonging to module,
    // this method scans self for special path components
    // crate, self and super. (These are special because they
    // module in a non-sequential fashion).
    // This method removes the components from use_path and
    // returns a new Path from which resoulution must start.
    pub fn delocalize(&mut self, module: &Path) -> Path {
        let mut new_path = Vec::new();
        let mut new_mod = module.components().to_vec();
        for comp in &self.path[0..self.path.len() - 1] {
            if let UsePathComponent::Name(name) = comp {
                if name == "super" {
                    new_mod.pop();
                } else if name == "crate" {
                    new_mod.clear();
                    new_mod.push(module.components()[0].clone());
                } else if name == "self" {
                    // Do nothing
                } else {
                    new_path.push(UsePathComponent::Name(name.clone()));
                }
            } else {
                panic!("Invalid use path {}", self);
            }
        }
        new_path.push(self.path.pop().unwrap());
        self.path = new_path;
        Path::new(new_mod)
    }

    pub fn begins_with(&self, s: &str) -> bool {
        if let Some(first) = self.components().first() {
            match first {
                UsePathComponent::Name(name) => s == name,
                _ => false,
            }
        } else {
            false
        }
    }

    pub fn replace_first(&mut self, new_first: &str) {
        if let Some(UsePathComponent::Name(first)) = self.path.first_mut() {
            first.clear();
            first.push_str(new_first);
        }
    }

    pub fn begins_with_empty(&self) -> bool {
        if let Some(first) = self.components().first() {
            first == &UsePathComponent::Empty
        } else {
            false
        }
    }

    pub fn remove_first(&mut self) {
        self.path.remove(0);
    }
}

impl Display for UsePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let comps: Vec<String> = self.path.iter().map(UsePathComponent::to_string).collect();
        write!(f, "{}{}", self.vis, comps.join("::"))
    }
}

impl From<Vec<&str>> for UsePath {
    fn from(comps: Vec<&str>) -> Self {
        lazy_static! {
            static ref RENAME_REG: Regex = Regex::new(r"([\w\d_]+) as ([\w\d_]+)").unwrap();
        }
        let comps = comps
            .into_iter()
            .map(|item| {
                if item == "*" {
                    UsePathComponent::Glob
                } else if item == "" {
                    UsePathComponent::Empty
                } else if let Some(captures) = RENAME_REG.captures(item) {
                    let from = String::from(&captures[1]);
                    let to = String::from(&captures[2]);
                    UsePathComponent::Rename(from, to)
                } else {
                    UsePathComponent::Name(String::from(item))
                }
            })
            .collect();
        Self {
            path: comps,
            vis: Visibility::Public,
        }
    }
}

fn use_paths_from_use_tree(tree: &syn::UseTree, vis: &Visibility) -> Vec<UsePath> {
    fn name_to_component(s: String) -> UsePathComponent {
        if s == "" {
            UsePathComponent::Empty
        } else {
            UsePathComponent::Name(s)
        }
    }

    match tree {
        syn::UseTree::Path(path) => {
            let first = name_to_component(path.ident.to_string());
            use_paths_from_use_tree(path.tree.as_ref(), vis)
                .into_iter()
                .map(|mut path| {
                    let mut comps = vec![first.clone()];
                    comps.append(&mut path.path);
                    UsePath::new(comps, vis.clone())
                })
                .collect()
        }
        syn::UseTree::Name(name) => vec![UsePath::new(
            vec![name_to_component(name.ident.to_string())],
            vis.clone(),
        )],
        syn::UseTree::Rename(rename) => vec![UsePath::new(
            vec![UsePathComponent::Rename(
                rename.ident.to_string(),
                rename.rename.to_string(),
            )],
            vis.clone(),
        )],
        syn::UseTree::Glob(_) => vec![UsePath::new(vec![UsePathComponent::Glob], vis.clone())],
        syn::UseTree::Group(group) => group
            .items
            .iter()
            .map(|tree| use_paths_from_use_tree(tree, vis))
            .flatten()
            .collect(),
    }
}

pub fn use_paths_from_items(items: &[syn::Item], module: &mut Path) -> HashMap<Path, Vec<UsePath>> {
    let mut paths_map: HashMap<Path, Vec<UsePath>> = HashMap::new();
    for item in items {
        match item {
            syn::Item::Use(item) => {
                let mut new_paths =
                    use_paths_from_use_tree(&item.tree, &Visibility::from_syn(&item.vis));
                if let Some(existing_paths) = paths_map.get_mut(module) {
                    existing_paths.append(&mut new_paths);
                } else {
                    paths_map.insert(module.clone(), new_paths);
                }
            }
            syn::Item::Mod(item) => {
                if let Some((_, items)) = item.content.as_ref() {
                    module.push_name(item.ident.to_string());
                    let new_paths = use_paths_from_items(items, module);
                    for (k, v) in new_paths {
                        paths_map.insert(k, v);
                    }
                    module.pop();
                }
            }
            _ => {}
        }
    }
    paths_map
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::item::structs::*;

    #[test]
    fn test_delocalize() {
        let module = Path::from(vec!["rand", "foo", "bar", "baz"]);

        let mut path = UsePath::from(vec!["self", "super", "Help"]);
        let new_mod = Path::from(vec!["rand", "foo", "bar"]);
        let new_path = UsePath::from(vec!["Help"]);
        assert_eq!(path.delocalize(&module), new_mod);
        assert_eq!(path, new_path);

        let mut path = UsePath::from(vec!["super", "super", "Help"]);
        let new_mod = Path::from(vec!["rand", "foo"]);
        let new_path = UsePath::from(vec!["Help"]);
        assert_eq!(path.delocalize(&module), new_mod);
        assert_eq!(path, new_path);

        let mut path = UsePath::from(vec!["super", "cat", "Help"]);
        let new_mod = Path::from(vec!["rand", "foo", "bar"]);
        let new_path = UsePath::from(vec!["cat", "Help"]);
        assert_eq!(path.delocalize(&module), new_mod);
        assert_eq!(path, new_path);

        let mut path = UsePath::from(vec!["crate", "cat", "Help"]);
        let new_mod = Path::from(vec!["rand"]);
        let new_path = UsePath::from(vec!["cat", "Help"]);
        assert_eq!(path.delocalize(&module), new_mod);
        assert_eq!(path, new_path);
    }
}
