use arca_modules::PackageManifest;
use arca_pkg::{Lockfile, PackageManager};
use std::fs;

#[test]
fn test_lockfile_generation_and_parsing() {
    let mut lock = Lockfile::new();
    lock.packages.insert(
        "http".to_string(),
        arca_pkg::LockPackage {
            name: "http".to_string(),
            version: "1.0.0".to_string(),
            checksum: "sha256:abc12345".to_string(),
        },
    );

    let temp_dir = std::env::temp_dir().join("arca_lockfile_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    lock.save_to_dir(&temp_dir).unwrap();

    let loaded = Lockfile::load_from_dir(&temp_dir).unwrap();
    assert_eq!(loaded.packages.len(), 1);
    assert_eq!(loaded.packages.get("http").unwrap().version, "1.0.0");

    let _ = fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_package_manager_add_and_remove() {
    let temp_dir = std::env::temp_dir().join("arca_pkg_mgr_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let default_manifest = PackageManifest::generate_default("test-pkg");
    fs::write(temp_dir.join("Arca.toml"), default_manifest).unwrap();
    fs::create_dir_all(temp_dir.join("src")).unwrap();
    fs::write(temp_dir.join("src").join("main.arca"), "fn main() {}").unwrap();

    // Add dependency
    PackageManager::add_dependency(&temp_dir, "json", Some("2.0.0")).unwrap();
    let manifest_after_add = PackageManifest::load_from_dir(&temp_dir).unwrap();
    assert_eq!(manifest_after_add.dependencies.get("json"), Some(&"2.0.0".to_string()));

    // Remove dependency
    PackageManager::remove_dependency(&temp_dir, "json").unwrap();
    let manifest_after_remove = PackageManifest::load_from_dir(&temp_dir).unwrap();
    assert!(manifest_after_remove.dependencies.get("json").is_none());

    // Test publish validation
    let pub_res = PackageManager::publish_package(&temp_dir);
    assert!(pub_res.is_ok());

    let _ = fs::remove_dir_all(&temp_dir);
}
