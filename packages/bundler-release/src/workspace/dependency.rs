//! Dependency graph analysis and topological sorting for package publishing.
//!
//! This module builds dependency graphs from workspace information and determines
//! the optimal publishing order to ensure dependencies are available before dependents.

use crate::error::{Result, WorkspaceError};
use crate::workspace::WorkspaceInfo;
use petgraph::Graph;
use petgraph::algo::toposort;
use petgraph::graph::NodeIndex;
use std::collections::HashMap;

/// Dependency graph representing package relationships
#[derive(Debug, Clone)]
pub struct DependencyGraph {
    /// Directed graph where edges point from dependency to dependent
    graph: Graph<String, (), petgraph::Directed>,
    /// Mapping from package names to graph node indices
    node_map: HashMap<String, NodeIndex>,
    /// Reverse mapping from node indices to package names
    index_map: HashMap<NodeIndex, String>,
}

/// Publishing order with packages grouped into tiers
#[derive(Debug, Clone)]
pub struct PublishOrder {
    /// Ordered tiers of packages that can be published in parallel within each tier
    pub tiers: Vec<PublishTier>,
    /// Total number of packages to be published
    pub total_packages: usize,
}

/// A tier of packages that can be published in parallel
#[derive(Debug, Clone)]
pub struct PublishTier {
    /// Package names in this tier
    pub packages: Vec<String>,
    /// Tier number (0-based)
    pub tier_number: usize,
}

impl DependencyGraph {
    /// Build dependency graph from workspace information
    pub fn build(workspace: &WorkspaceInfo) -> Result<Self> {
        let mut graph = Graph::new();
        let mut node_map = HashMap::with_capacity(workspace.packages.len());
        let mut index_map = HashMap::with_capacity(workspace.packages.len());

        // Add all packages as nodes first
        for package_name in workspace.packages.keys() {
            let node_index = graph.add_node(package_name.clone());
            node_map.insert(package_name.clone(), node_index);
            index_map.insert(node_index, package_name.clone());
        }

        // Add dependency edges (from dependency to dependent)
        for (package_name, dependencies) in &workspace.internal_dependencies {
            let dependent_index =
                node_map
                    .get(package_name)
                    .ok_or_else(|| WorkspaceError::PackageNotFound {
                        name: package_name.clone(),
                    })?;

            for dependency_name in dependencies {
                let dependency_index = node_map.get(dependency_name).ok_or_else(|| {
                    WorkspaceError::PackageNotFound {
                        name: dependency_name.clone(),
                    }
                })?;

                // Edge from dependency to dependent (dependency must be published first)
                graph.add_edge(*dependency_index, *dependent_index, ());
            }
        }

        Ok(Self {
            graph,
            node_map,
            index_map,
        })
    }

    /// Generate publishing order using topological sorting
    pub fn publish_order(&self) -> Result<PublishOrder> {
        // Detect circular dependencies first
        self.validate_no_cycles()?;

        // Perform topological sort
        let sorted_indices = toposort(&self.graph, None).map_err(|cycle| {
            let cycle_packages: Vec<String> = self
                .index_map
                .get(&cycle.node_id())
                .map(|name| vec![name.clone()])
                .unwrap_or_else(|| vec!["unknown".to_string()]);

            WorkspaceError::CircularDependency {
                packages: cycle_packages,
            }
        })?;

        // Convert to package names maintaining topological order
        let ordered_packages: Vec<String> = sorted_indices
            .into_iter()
            .filter_map(|idx| self.index_map.get(&idx).cloned())
            .collect();

        // Group into tiers for parallel publishing within each tier
        let tiers = self.group_into_tiers(&ordered_packages);
        let total_packages = ordered_packages.len();

        Ok(PublishOrder {
            tiers,
            total_packages,
        })
    }

    /// Group packages into publishing tiers
    fn group_into_tiers(&self, ordered_packages: &[String]) -> Vec<PublishTier> {
        let mut tiers = Vec::new();
        let mut remaining_packages: std::collections::VecDeque<_> =
            ordered_packages.iter().collect();
        let mut tier_number = 0;

        while !remaining_packages.is_empty() {
            let mut current_tier = Vec::new();
            let mut indices_to_remove = Vec::new();

            // Find packages with no remaining dependencies in the current set
            for (index, package_name) in remaining_packages.iter().enumerate() {
                if self.can_publish_in_current_tier(package_name, &remaining_packages) {
                    current_tier.push((*package_name).clone());
                    indices_to_remove.push(index);
                }
            }

            // Remove processed packages (in reverse order to maintain indices)
            for &index in indices_to_remove.iter().rev() {
                remaining_packages.remove(index);
            }

            // If no packages can be published, there might be a subtle circular dependency
            if current_tier.is_empty() && !remaining_packages.is_empty() {
                // Take the first remaining package to break potential deadlock
                if let Some(package) = remaining_packages.pop_front() {
                    current_tier.push(package.clone());
                }
            }

            if !current_tier.is_empty() {
                // Sort packages within tier by number of dependents (descending)
                // This ensures packages with more dependents are published first,
                // maximizing parallelism for subsequent tiers
                current_tier
                    .sort_by_cached_key(|pkg| std::cmp::Reverse(self.dependents(pkg).len()));

                tiers.push(PublishTier {
                    packages: current_tier,
                    tier_number,
                });
                tier_number += 1;
            }
        }

        tiers
    }

    /// Check if a package can be published in the current tier
    fn can_publish_in_current_tier(
        &self,
        package_name: &str,
        remaining_packages: &std::collections::VecDeque<&String>,
    ) -> bool {
        let package_index = match self.node_map.get(package_name) {
            Some(idx) => *idx,
            None => return false,
        };

        // Check if any dependencies are still in the remaining packages
        let neighbors = self
            .graph
            .neighbors_directed(package_index, petgraph::Direction::Incoming);

        for dependency_index in neighbors {
            if let Some(dependency_name) = self.index_map.get(&dependency_index)
                && remaining_packages
                    .iter()
                    .any(|&name| name == dependency_name)
            {
                return false; // Dependency still needs to be published
            }
        }

        true
    }

    /// Validate that the dependency graph has no circular dependencies
    fn validate_no_cycles(&self) -> Result<()> {
        use petgraph::algo::tarjan_scc;

        // Single-pass SCC computation: O(V + E)
        let sccs = tarjan_scc(&self.graph);

        // Find any SCC with more than 1 node (= cycle exists)
        for scc in sccs {
            if scc.len() > 1 {
                // Extract package names for error reporting
                let cycle_packages: Vec<String> = scc
                    .into_iter()
                    .filter_map(|idx| self.index_map.get(&idx).cloned())
                    .collect();

                return Err(WorkspaceError::CircularDependency {
                    packages: cycle_packages,
                }
                .into());
            }

            // Check for self-loops (single-node cycles)
            if scc.len() == 1 {
                let node = scc[0];
                if self.graph.contains_edge(node, node) {
                    let package_name = self
                        .index_map
                        .get(&node)
                        .cloned()
                        .unwrap_or_else(|| "unknown".to_string());

                    return Err(WorkspaceError::CircularDependency {
                        packages: vec![package_name],
                    }
                    .into());
                }
            }
        }

        Ok(())
    }

    /// Get all packages that depend on the given package
    pub fn dependents(&self, package_name: &str) -> Vec<String> {
        let package_index = match self.node_map.get(package_name) {
            Some(idx) => *idx,
            None => return Vec::new(),
        };

        self.graph
            .neighbors_directed(package_index, petgraph::Direction::Outgoing)
            .filter_map(|idx| self.index_map.get(&idx).cloned())
            .collect()
    }

    /// Get all dependencies of the given package
    pub fn dependencies(&self, package_name: &str) -> Vec<String> {
        let package_index = match self.node_map.get(package_name) {
            Some(idx) => *idx,
            None => return Vec::new(),
        };

        self.graph
            .neighbors_directed(package_index, petgraph::Direction::Incoming)
            .filter_map(|idx| self.index_map.get(&idx).cloned())
            .collect()
    }

    /// Check if package A depends on package B (directly or transitively)
    pub fn depends_on(&self, package_a: &str, package_b: &str) -> bool {
        let start_index = match self.node_map.get(package_a) {
            Some(idx) => *idx,
            None => return false,
        };

        let target_index = match self.node_map.get(package_b) {
            Some(idx) => *idx,
            None => return false,
        };

        // Use petgraph's has_path_connecting for efficient path detection
        petgraph::algo::has_path_connecting(&self.graph, start_index, target_index, None)
    }

    /// Get the dependency depth of a package (longest path from any root)
    pub fn dependency_depth(&self, package_name: &str) -> usize {
        let package_index = match self.node_map.get(package_name) {
            Some(idx) => *idx,
            None => return 0,
        };

        self.calculate_depth_recursive(package_index, &mut std::collections::HashSet::new())
    }

    /// Recursively calculate dependency depth with cycle prevention
    fn calculate_depth_recursive(
        &self,
        node: NodeIndex,
        visited: &mut std::collections::HashSet<NodeIndex>,
    ) -> usize {
        if visited.contains(&node) {
            return 0; // Cycle detected, return 0 to avoid infinite recursion
        }

        visited.insert(node);

        let max_dependency_depth = self
            .graph
            .neighbors_directed(node, petgraph::Direction::Incoming)
            .map(|dep_idx| self.calculate_depth_recursive(dep_idx, visited))
            .max()
            .unwrap_or(0);

        visited.remove(&node);
        max_dependency_depth + 1
    }
}

impl PublishOrder {
    /// Get the tier number for a specific package
    pub fn tier_for_package(&self, package_name: &str) -> Option<usize> {
        self.tiers
            .iter()
            .find(|tier| tier.packages.contains(&package_name.to_string()))
            .map(|tier| tier.tier_number)
    }

    /// Get all packages in a specific tier
    pub fn packages_in_tier(&self, tier_number: usize) -> Vec<String> {
        self.tiers
            .get(tier_number)
            .map(|tier| tier.packages.clone())
            .unwrap_or_default()
    }

    /// Get the total number of tiers
    pub fn tier_count(&self) -> usize {
        self.tiers.len()
    }

    /// Check if a package exists in the publish order
    pub fn contains_package(&self, package_name: &str) -> bool {
        self.tiers
            .iter()
            .any(|tier| tier.packages.contains(&package_name.to_string()))
    }

    /// Iterate over all packages in publishing order
    pub fn ordered_packages(&self) -> impl Iterator<Item = &String> {
        self.tiers.iter().flat_map(|tier| tier.packages.iter())
    }
}

impl PublishTier {
    /// Check if this tier contains the specified package
    pub fn contains(&self, package_name: &str) -> bool {
        self.packages.contains(&package_name.to_string())
    }

    /// Get the number of packages in this tier
    pub fn package_count(&self) -> usize {
        self.packages.len()
    }

    /// Check if this tier can be published in parallel
    pub fn is_parallel_publishable(&self) -> bool {
        self.packages.len() > 1
    }
}
