//! Tests for GitHub user search operation with type-safe parameters.

use kodegen_tools_github::{SearchOrder, UserSearchSort};

#[test]
fn test_user_search_sort_as_str() {
    assert_eq!(UserSearchSort::Followers.as_str(), "followers");
    assert_eq!(UserSearchSort::Repositories.as_str(), "repositories");
    assert_eq!(UserSearchSort::Joined.as_str(), "joined");
}

#[test]
fn test_search_order_as_str() {
    assert_eq!(SearchOrder::Asc.as_str(), "asc");
    assert_eq!(SearchOrder::Desc.as_str(), "desc");
}

#[test]
fn test_enum_copy() {
    // Verify enums are Copy
    let sort = UserSearchSort::Followers;
    let _copy = sort;
    let _original = sort; // Should still be usable

    let order = SearchOrder::Desc;
    let _copy = order;
    let _original = order; // Should still be usable
}

#[test]
fn test_enum_equality() {
    assert_eq!(UserSearchSort::Followers, UserSearchSort::Followers);
    assert_ne!(UserSearchSort::Followers, UserSearchSort::Repositories);

    assert_eq!(SearchOrder::Asc, SearchOrder::Asc);
    assert_ne!(SearchOrder::Asc, SearchOrder::Desc);
}
