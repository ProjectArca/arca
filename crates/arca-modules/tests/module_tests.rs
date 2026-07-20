use arca_modules::{ModuleResolver, PackageManifest};
use std::fs;

#[test]
fn test_package_manifest_parsing() {
    let toml_str = r#"
[package]
name = "my-arca-app"
version = "0.1.0"
edition = "2026"

[dependencies]
http = "1.0"
"#;
    let manifest = PackageManifest::parse(toml_str).unwrap();
    assert_eq!(manifest.package.name, "my-arca-app");
    assert_eq!(manifest.package.version, "0.1.0");
    assert_eq!(manifest.package.edition, "2026");
    assert_eq!(manifest.dependencies.get("http"), Some(&"1.0".to_string()));
}

#[test]
fn test_module_resolver_file_loading() {
    let temp_dir = std::env::temp_dir().join("arca_module_test");
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let main_file = temp_dir.join("main.arca");
    let mod_file = temp_dir.join("helper.arca");

    fs::write(
        &main_file,
        r#"
import { Helper } from "./helper"

fn main() {
    let h = Helper {}
}
"#,
    )
    .unwrap();

    fs::write(
        &mod_file,
        r#"
export struct Helper {
    id: i32
}
"#,
    )
    .unwrap();

    let mut resolver = ModuleResolver::new();
    let res = resolver.resolve_package(&main_file);

    assert!(res.is_ok(), "Expected valid module resolution, got diags: {:#?}", resolver.diagnostics);
    assert_eq!(resolver.modules.len(), 2);

    let _ = fs::remove_dir_all(&temp_dir);
}
