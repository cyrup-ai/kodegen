//! Diagnostic tool to debug publish order issues
//!
//! Run with: cargo run --package `kodegen_bundler_release` --example `debug_publish_order`

use kodegen_bundler_release::workspace::{DependencyGraph, WorkspaceInfo};
use std::sync::Arc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Analyzing workspace dependency structure\n");

    // Analyze workspace
    let workspace = Arc::new(WorkspaceInfo::analyze(".")?);

    println!("📦 Discovered {} packages:", workspace.packages.len());
    for (name, pkg) in &workspace.packages {
        println!("   • {} (v{})", name, pkg.version);
    }

    println!("\n🔗 Internal dependencies map:");
    for (pkg_name, deps) in &workspace.internal_dependencies {
        if deps.is_empty() {
            println!("   {pkg_name} → [] (NO DEPENDENCIES - SHOULD BE TIER 0)");
        } else {
            println!("   {pkg_name} → {deps:?}");
        }
    }

    println!("\n📊 Workspace dependency details:");
    for (name, pkg) in &workspace.packages {
        println!("\n   Package: {name}");
        println!(
            "     workspace_dependencies: {:?}",
            pkg.workspace_dependencies
        );
        println!(
            "     all_dependencies ({} total):",
            pkg.all_dependencies.len()
        );
        for (dep_name, dep_spec) in &pkg.all_dependencies {
            if let Some(path) = &dep_spec.path {
                println!("       {dep_name} (path: {path})");
            }
        }
    }

    // Build dependency graph
    println!("\n🕸️  Building dependency graph...");
    let dep_graph = DependencyGraph::build(&workspace)?;

    // Get publish order
    println!("\n📋 Computing publish order...");
    let publish_order = dep_graph.publish_order()?;

    println!(
        "\n🎯 Publish Order ({} tiers, {} packages):\n",
        publish_order.tier_count(),
        publish_order.total_packages
    );

    for tier in &publish_order.tiers {
        println!(
            "   Tier {} ({} package{}):",
            tier.tier_number,
            tier.packages.len(),
            if tier.packages.len() == 1 { "" } else { "s" }
        );
        for pkg in &tier.packages {
            let deps = dep_graph.dependencies(pkg);
            let dependents = dep_graph.dependents(pkg);
            if deps.is_empty() {
                println!(
                    "      • {} (no dependencies, depended on by: {})",
                    pkg,
                    if dependents.is_empty() {
                        "none".to_string()
                    } else {
                        dependents.join(", ")
                    }
                );
            } else {
                println!(
                    "      • {} (depends on: {}, depended on by: {})",
                    pkg,
                    deps.join(", "),
                    if dependents.is_empty() {
                        "none".to_string()
                    } else {
                        dependents.join(", ")
                    }
                );
            }
        }
        println!();
    }

    // Check for kodegen_tool specifically
    println!("\n🔎 Analysis of 'kodegen_tool':");
    if let Some(pkg) = workspace.packages.get("kodegen_tool") {
        println!("   ✓ Package exists");
        println!("   Version: {}", pkg.version);
        println!(
            "   workspace_dependencies: {:?}",
            pkg.workspace_dependencies
        );

        if let Some(deps) = workspace.internal_dependencies.get("kodegen_tool") {
            println!("   internal_dependencies: {deps:?}");
            if deps.is_empty() {
                println!("   ✓ Has NO internal dependencies - should be in Tier 0!");
            } else {
                println!("   ⚠ Has internal dependencies: {deps:?}");
            }
        } else {
            println!("   ✗ NOT in internal_dependencies map!");
        }

        let graph_deps = dep_graph.dependencies("kodegen_tool");
        println!("   Graph dependencies: {graph_deps:?}");

        let graph_dependents = dep_graph.dependents("kodegen_tool");
        println!("   Graph dependents: {graph_dependents:?}");

        if let Some(tier) = publish_order.tier_for_package("kodegen_tool") {
            println!("   Publish tier: {tier}");
            if tier == 0 {
                println!("   ✓ Correctly placed in Tier 0");
            } else {
                println!("   ✗ BUG: Should be in Tier 0 but is in Tier {tier}!");
            }
        } else {
            println!("   ✗ NOT in publish order!");
        }
    } else {
        println!("   ✗ Package NOT FOUND in workspace!");
    }

    Ok(())
}
