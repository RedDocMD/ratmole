use colored::*;

use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

use crate::{printer::TreePrintable, tree::TreeItem};

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

impl TreeItem for Struct {
    fn module(&self) -> &Path {
        &self.module
    }

    fn name(&self) -> &str {
        &self.name
    }
}

impl TreePrintable for Struct {
    fn single_write(&self, f: &mut Formatter<'_>) -> fmt::Result {
        self.fmt(f)
    }

    fn children(&self) -> Vec<&dyn TreePrintable> {
        Vec::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Path(Vec<PathComponent>);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PathComponent {
    Global,       // ::
    SmallSelf,    // self
    BigSelf,      // Self
    Super,        // super
    Crate,        // crate
    Name(String), // everything else
}

impl Display for PathComponent {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            PathComponent::Global => write!(f, ""),
            PathComponent::SmallSelf => write!(f, "self"),
            PathComponent::BigSelf => write!(f, "Self"),
            PathComponent::Super => write!(f, "super"),
            PathComponent::Crate => write!(f, "crate"),
            PathComponent::Name(name) => write!(f, "{}", name),
        }
    }
}

impl From<String> for PathComponent {
    fn from(comp: String) -> Self {
        use PathComponent::*;
        match comp.as_str() {
            "" => Global,
            "self" => SmallSelf,
            "Self" => BigSelf,
            "super" => Super,
            "crate" => Crate,
            _ => Name(comp),
        }
    }
}

impl From<&'static str> for PathComponent {
    fn from(comp: &'static str) -> Self {
        PathComponent::from(String::from(comp))
    }
}

impl Path {
    pub fn new(comps: Vec<PathComponent>) -> Self {
        Self(comps)
    }

    pub fn push_name(&mut self, comp: String) {
        self.0.push(PathComponent::Name(comp));
    }

    pub fn components(&self) -> &[PathComponent] {
        &self.0
    }

    pub(crate) fn pop(&mut self) {
        self.0.pop();
    }

    pub fn parent(&self) -> Path {
        Self(self.0[..self.0.len() - 1].to_vec())
    }

    pub fn first_as_path(&self) -> Path {
        Path(vec![self.components().first().unwrap().clone()])
    }
}

impl From<Vec<String>> for Path {
    fn from(comps: Vec<String>) -> Self {
        Self(comps.into_iter().map(PathComponent::from).collect())
    }
}

impl From<Vec<&'static str>> for Path {
    fn from(comps: Vec<&'static str>) -> Self {
        Self(comps.into_iter().map(PathComponent::from).collect())
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let comps: Vec<String> = self.0.iter().map(PathComponent::to_string).collect();
        write!(f, "{}", comps.join("::"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub(crate) fn renamed(&self, new_name: &str) -> Self {
        Self {
            name: String::from(new_name),
            vis: self.vis.clone(),
            params: self.params.clone(),
            module: self.module.clone(),
        }
    }

    pub(crate) fn set_visibility(&mut self, vis: Visibility) {
        self.vis = vis;
    }
}

impl Visibility {
    pub fn from_syn(item: &syn::Visibility) -> Self {
        match item {
            syn::Visibility::Public(_) => Self::Public,
            syn::Visibility::Crate(_) => Self::Crate,
            syn::Visibility::Restricted(item) => {
                let path: Vec<PathComponent> = item
                    .path
                    .segments
                    .iter()
                    .map(|seg| PathComponent::from(seg.ident.to_string()))
                    .collect();
                Self::Restricted(Path(path))
            }
            syn::Visibility::Inherited => Self::Private,
        }
    }
}

pub fn structs_from_items(items: &[syn::Item], module: &mut Path) -> HashMap<Path, Vec<Struct>> {
    use syn::Item;
    let mut structs: HashMap<Path, Vec<Struct>> = HashMap::new();
    for item in items {
        match item {
            Item::Struct(item) => {
                let s = Struct::from_syn(item, module.clone());
                if let Some(existing_structs) = structs.get_mut(module) {
                    existing_structs.push(s);
                } else {
                    structs.insert(module.clone(), vec![s]);
                }
            }
            Item::Mod(item) => {
                module.push_name(item.ident.to_string());
                if let Some((_, content)) = &item.content {
                    let mut new_structs = structs_from_items(content, module);
                    structs.extend(new_structs);
                }
                module.pop();
            }
            _ => {}
        }
    }
    structs
}
