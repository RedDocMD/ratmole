use crate::cargo::DependentPackage;

#[derive(Debug)]
pub(super) struct Node<'pkg> {
    pkg: &'pkg DependentPackage,
    dependents: Vec<&'pkg DependentPackage>,
}

impl<'pkg> Node<'pkg> {
    pub(super) fn free_node(p: &'pkg DependentPackage) -> Self {
        Self {
            pkg: p,
            dependents: Vec::new(),
        }
    }

    pub(super) fn pkg(&self) -> &DependentPackage {
        self.pkg
    }

    pub(super) fn add_dependent(&mut self, node: &'pkg DependentPackage) {
        self.dependents.push(node);
    }
}

#[derive(Debug)]
pub struct Dag<'pkg> {
    nodes: Vec<Node<'pkg>>,
}

impl<'pkg> Dag<'pkg> {
    pub(super) fn new(nodes: Vec<Node<'pkg>>) -> Self {
        Self { nodes }
    }
}
