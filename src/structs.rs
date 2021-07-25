use colored::*;

use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};

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
pub struct Path(Vec<PathComponent>);

#[derive(Debug, Clone)]
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

impl Path {
    pub fn push_name(&mut self, comp: String) {
        self.0.push(PathComponent::Name(comp));
    }
}

impl From<Vec<String>> for Path {
    fn from(comps: Vec<String>) -> Self {
        Self(comps.into_iter().map(PathComponent::from).collect())
    }
}

impl Display for Path {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let comps: Vec<String> = self.0.iter().map(PathComponent::to_string).collect();
        write!(f, "{}", comps.join("::"))
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

pub fn structs_from_items(items: &[syn::Item], module: Path) -> (Vec<Struct>, Vec<ModuleInfo>) {
    use syn::Item;
    let mut structs = Vec::new();
    let mut infos = Vec::new();
    for item in items {
        match item {
            Item::Struct(item) => structs.push(Struct::from_syn(item, module.clone())),
            Item::Mod(item) => {
                let mut new_module = module.clone();
                new_module.push_name(item.ident.to_string());
                if let Some((_, content)) = &item.content {
                    let mut info =
                        ModuleInfo::new(item.ident.to_string(), Visibility::from_syn(&item.vis));
                    let (mut new_structs, new_infos) =
                        structs_from_items(content, new_module.clone());
                    structs.append(&mut new_structs);
                    info.add_children(new_infos);
                    infos.push(info);
                }
            }
            _ => {}
        }
    }
    (structs, infos)
}

#[derive(Debug)]
pub struct ModuleInfo {
    name: String,
    vis: Visibility,
    children: HashMap<String, ModuleInfo>,
}

impl ModuleInfo {
    pub fn new(name: String, vis: Visibility) -> Self {
        Self {
            name,
            vis,
            children: HashMap::new(),
        }
    }

    pub fn add_child(&mut self, name: String, vis: Visibility) {
        self.children.insert(name.clone(), Self::new(name, vis));
    }

    pub fn add_child_mod(&mut self, info: ModuleInfo) {
        self.children.insert(info.name.clone(), info);
    }

    pub fn add_children(&mut self, children: Vec<ModuleInfo>) {
        for child in children {
            self.add_child_mod(child);
        }
    }
}
