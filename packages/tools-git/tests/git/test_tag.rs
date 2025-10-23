//! Tests for Git tag operations

use kodegen_tools_git::{create_tag, delete_tag, tag_exists, list_tags, TagOpts, init_repo};
use tempfile::TempDir;

#[tokio::test]
async fn test_create_and_list_tags() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo = init_repo(temp_dir.path())?;
    
    // Create initial commit
    std::fs::write(temp_dir.path().join("test.txt"), "test")?;
    kodegen_tools_git::add(&repo, kodegen_tools_git::AddOpts {
        paths: vec![temp_dir.path().join("test.txt")],
        update: false,
    }).await?;
    
    kodegen_tools_git::commit(&repo, kodegen_tools_git::CommitOpts {
        message: "Initial commit".to_string(),
        ..Default::default()
    }).await?;
    
    // Create tag
    let tag_info = create_tag(&repo, TagOpts {
        name: "v1.0.0".to_string(),
        message: Some("Release v1.0.0".to_string()),
        target: None,
        force: false,
    }).await?;
    
    assert_eq!(tag_info.name, "v1.0.0");
    assert!(tag_info.is_annotated);
    
    // Check tag exists
    assert!(tag_exists(&repo, "v1.0.0").await?);
    
    // List tags
    let tags = list_tags(&repo).await?;
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "v1.0.0");
    
    Ok(())
}

#[tokio::test]
async fn test_delete_tag() -> Result<(), Box<dyn std::error::Error>> {
    let temp_dir = TempDir::new()?;
    let repo = init_repo(temp_dir.path())?;
    
    // Create initial commit
    std::fs::write(temp_dir.path().join("test.txt"), "test")?;
    kodegen_tools_git::add(&repo, kodegen_tools_git::AddOpts {
        paths: vec![temp_dir.path().join("test.txt")],
        update: false,
    }).await?;
    
    kodegen_tools_git::commit(&repo, kodegen_tools_git::CommitOpts {
        message: "Initial commit".to_string(),
        ..Default::default()
    }).await?;
    
    // Create and delete tag
    create_tag(&repo, TagOpts {
        name: "v1.0.0".to_string(),
        message: Some("Release".to_string()),
        target: None,
        force: false,
    }).await?;
    
    assert!(tag_exists(&repo, "v1.0.0").await?);
    
    delete_tag(&repo, "v1.0.0").await?;
    
    assert!(!tag_exists(&repo, "v1.0.0").await?);
    
    Ok(())
}
