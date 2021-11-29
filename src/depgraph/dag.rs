use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    io::Write,
};

use crate::{cargo::DependentPackage, error::Result};

#[derive(Debug, Eq)]
pub(super) struct Node<'pkg> {
    pkg: &'pkg DependentPackage,
    dependents: Vec<&'pkg DependentPackage>,
}

impl PartialEq for Node<'_> {
    fn eq(&self, rhs: &Self) -> bool {
        self.pkg == rhs.pkg
    }
}

impl Hash for Node<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.pkg.hash(state);
    }
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

    pub(super) fn has_dependent(&self, node: &'pkg DependentPackage) -> bool {
        self.dependents.contains(&node)
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

    pub fn dump_graphviz<W: Write>(&self, file: &mut W) -> Result<()> {
        let idx_map: HashMap<_, _> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(idx, node)| (node.pkg, idx))
            .collect();

        writeln!(file, "digraph G {{")?;
        for node in &self.nodes {
            let from_idx = idx_map[node.pkg];
            for dep in &node.dependents {
                let to_idx = idx_map[dep];
                writeln!(file, "  {} -> {};", from_idx, to_idx)?;
            }
            writeln!(file, "  {} [label = \"{}\"];", from_idx, node.pkg.name())?;
        }
        writeln!(file, "}}")?;
        Ok(())
    }

    pub fn topological_order(&self) -> Vec<&DependentPackage> {
        let mut visited: HashMap<_, _> = self.nodes.iter().map(|n| (n.pkg, false)).collect();
        let node_map: HashMap<_, _> = self.nodes.iter().map(|n| (n.pkg, n)).collect();
        let mut pkg_order = Vec::new();

        fn toposort_rec<'pkg, 'node>(
            node: &'node Node<'pkg>,
            visited: &mut HashMap<&'node DependentPackage, bool>,
            node_map: &HashMap<&'node DependentPackage, &'node Node<'pkg>>,
            pkg_order: &mut Vec<&'node DependentPackage>,
        ) {
            visited.insert(node.pkg, true);
            for dep in &node.dependents {
                if !visited[dep] {
                    toposort_rec(node_map[dep], visited, node_map, pkg_order);
                }
            }
            pkg_order.push(node.pkg);
        }

        for node in &self.nodes {
            if !visited[&node.pkg] {
                toposort_rec(node, &mut visited, &node_map, &mut pkg_order);
            }
        }
        pkg_order.reverse();
        pkg_order
    }
}
