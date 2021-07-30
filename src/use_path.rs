use std::fmt::{self, Display, Formatter};

use crate::structs::Visibility;

#[derive(Clone)]
pub enum UsePathComponent {
    Name(String),
    Rename(String, String),
    Glob,
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
        }
    }
}

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
}

impl Display for UsePath {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let comps: Vec<String> = self.path.iter().map(UsePathComponent::to_string).collect();
        write!(f, "{}", comps.join("::"))
    }
}

fn use_paths_from_use_tree(tree: &syn::UseTree, vis: &Visibility) -> Vec<UsePath> {
    match tree {
        syn::UseTree::Path(path) => {
            let first = UsePathComponent::Name(path.ident.to_string());
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
            vec![UsePathComponent::Name(name.ident.to_string())],
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

pub fn use_paths_from_items(items: &[syn::Item]) -> Vec<UsePath> {
    items
        .iter()
        .filter_map(|item| {
            if let syn::Item::Use(item) = item {
                Some(use_paths_from_use_tree(
                    &item.tree,
                    &Visibility::from_syn(&item.vis),
                ))
            } else {
                None
            }
        })
        .flatten()
        .collect()
}
