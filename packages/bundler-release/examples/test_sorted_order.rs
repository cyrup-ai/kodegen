//! Test that the sorting fix works correctly

use kodegen_bundler_release::workspace::{DependencyGraph, WorkspaceInfo};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = Arc::new(WorkspaceInfo::analyze(".")?);
    let dep_graph = DependencyGraph::build(&workspace)?;
    let publish_order = dep_graph.publish_order()?;

    println!("\n🎯 Tier 0 packages (should be sorted by dependents descending):\n");

    if let Some(tier_0) = publish_order.tiers.first() {
        for (i, pkg) in tier_0.packages.iter().enumerate() {
            let dependents = dep_graph.dependents(pkg);
            println!("   {}. {} ({} dependents)", i + 1, pkg, dependents.len());
        }

        // Verify kodegen_tool is first
        if tier_0.packages.first() == Some(&"kodegen_tool".to_string()) {
            println!("\n✅ CORRECT: kodegen_tool is first in Tier 0");
        } else {
            println!(
                "\n❌ WRONG: kodegen_tool should be first but is at position {}",
                tier_0
                    .packages
                    .iter()
                    .position(|p| p == "kodegen_tool")
                    .unwrap_or(999)
                    + 1
            );
        }
    }

    Ok(())
}
