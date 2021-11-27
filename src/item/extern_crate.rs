use std::fmt::{self, Display, Formatter};

use crate::from_items;

use super::structs::{Path, Visibility};

pub struct ExternCrate {
    name: String,
    rename: Option<String>,
    vis: Visibility,
}

impl Display for ExternCrate {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} extern crate {}", self.vis, self.name)?;
        if let Some(rename) = &self.rename {
            write!(f, " as {}", rename)?;
        }
        Ok(())
    }
}

impl ExternCrate {
    fn from_syn(item: &syn::ItemExternCrate, _module: Path) -> Self {
        Self {
            name: item.ident.to_string(),
            vis: Visibility::from_syn(&item.vis),
            rename: item.rename.as_ref().map(|(_, name)| name.to_string()),
        }
    }

    pub fn rename(&self) -> &Option<String> {
        &self.rename
    }

    pub fn name(&self) -> &str {
        &self.name
    }
}

from_items!(extern_crates_from_items, ExternCrate, ExternCrate);
