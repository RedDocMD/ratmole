use colored::*;
use std::fmt::{self, Display, Formatter};

#[derive(Debug, Clone)]
pub struct Struct {
    name: String,
    vis: Visibility,
    params: Vec<String>,
    module: Path,
}

impl Display for Struct {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{} {}::{}",
            self.vis.to_string().magenta(),
            "struct".green(),
            self.module,
            self.name.yellow(),
        )?;
        if !self.params.is_empty() {
            write!(f, "<{}>", self.params.join(","))?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct Path(Vec<String>);

impl Path {
    fn push(&mut self, comp: String) {
        self.0.push(comp);
    }
}

impl From<Vec<String>> for Path {
    fn from(comps: Vec<String>) -> Self {
        Self(comps)
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.join("::"))
    }
}

#[derive(Debug, Clone)]
pub enum Visibility {
    Public,
    Crate,
    Restricted(Path),
    Private,
}

impl Display for Visibility {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Visibility::Public => write!(f, "pub "),
            Visibility::Crate => write!(f, "pub(crate) "),
            Visibility::Restricted(path) => write!(f, "pub(in {}) ", path),
            Visibility::Private => Ok(()),
        }
    }
}

impl Struct {
    fn from_syn(item: &syn::ItemStruct, module: Path) -> Self {
        let name = item.ident.to_string();
        let vis = Visibility::from_syn(&item.vis);
        let params: Vec<String> = item
            .generics
            .type_params()
            .map(|param| param.ident.to_string())
            .collect();
        Self {
            name,
            vis,
            params,
            module,
        }
    }
}

impl Visibility {
    fn from_syn(item: &syn::Visibility) -> Self {
        match item {
            syn::Visibility::Public(_) => Self::Public,
            syn::Visibility::Crate(_) => Self::Crate,
            syn::Visibility::Restricted(item) => {
                let path: Vec<String> = item
                    .path
                    .segments
                    .iter()
                    .map(|seg| seg.ident.to_string())
                    .collect();
                Self::Restricted(Path(path))
            }
            syn::Visibility::Inherited => Self::Private,
        }
    }
}

pub fn structs_from_items(items: &[syn::Item], module: Path) -> Vec<Struct> {
    use syn::Item;
    let mut structs = Vec::new();
    for item in items {
        match item {
            Item::Struct(item) => structs.push(Struct::from_syn(item, module.clone())),
            Item::Mod(item) => {
                let mut new_module = module.clone();
                new_module.push(item.ident.to_string());
                if let Some((_, content)) = &item.content {
                    structs.append(&mut structs_from_items(content, new_module.clone()));
                }
            }
            _ => {}
        }
    }
    structs
}
