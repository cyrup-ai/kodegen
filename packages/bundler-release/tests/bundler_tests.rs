#[cfg(test)]
mod tests {
    use kodegen_bundler_release::bundler::PackageType;

    #[test]
    fn test_package_type_priority() {
        assert_eq!(PackageType::MacOsBundle.priority(), 0);
        assert_eq!(PackageType::Dmg.priority(), 1);
        assert!(PackageType::MacOsBundle.priority() < PackageType::Dmg.priority());
    }

    #[test]
    fn test_package_type_short_names() {
        assert_eq!(PackageType::Deb.short_name(), "deb");
        assert_eq!(PackageType::MacOsBundle.short_name(), "app");
        assert_eq!(PackageType::WindowsMsi.short_name(), "msi");
    }

    #[test]
    fn test_current_platform_types() {
        let types = PackageType::all_for_current_platform();
        assert!(!types.is_empty());

        #[cfg(target_os = "linux")]
        assert!(types.contains(&PackageType::Deb));

        #[cfg(target_os = "macos")]
        assert!(types.contains(&PackageType::MacOsBundle));

        #[cfg(target_os = "windows")]
        assert!(types.contains(&PackageType::WindowsMsi));
    }
}
