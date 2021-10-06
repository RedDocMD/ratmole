use std::fmt::{self, Display, Formatter};

use crate::{printer::TreePrintable, tree::TreeItem};

use super::structs::Path;

pub struct Module {
    path: Path,
    name: String,
    parent: Path,
}

impl Module {
    pub fn new(names: &[String]) -> Self {
        let path = Path::from(names.to_vec());
        let parent = path.parent();
        Self {
            path,
            name: names.last().unwrap().clone(),
            parent,
        }
    }
}

impl Display for Module {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path)
    }
}

impl TreePrintable for Module {
    fn single_write(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.path.components().last().unwrap())
    }

    fn children(&self) -> Vec<&dyn TreePrintable> {
        Vec::new()
    }
}

impl TreeItem for Module {
    fn name(&self) -> &str {
        &self.name
    }

    fn module(&self) -> &Path {
        &self.parent
    }
}
