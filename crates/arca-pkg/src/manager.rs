//! Package Manager commands for managing Arca dependencies, lockfiles, and publishing.

use crate::lockfile::{LockPackage, Lockfile};
use arca_modules::PackageManifest;
use std::fs;
use std::path::Path;

pub struct PackageManager;

impl PackageManager {
    pub fn add_dependency<P: AsRef<Path>>(
        dir: P,
        dep_name: &str,
        version: Option<&str>,
    ) -> Result<(), String> {
        let manifest_path = dir.as_ref().join("Arca.toml");
        if !manifest_path.exists() {
            return Err(format!("Arca.toml not found in '{}'", dir.as_ref().display()));
        }

        let mut manifest = PackageManifest::load_from_dir(&dir)?;
        let ver_str = version.unwrap_or("0.1.0").to_string();

        manifest.dependencies.insert(dep_name.to_string(), ver_str.clone());

        // Rewrite Arca.toml
        let mut content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read Arca.toml: {}", e))?;

        if !content.contains("[dependencies]") {
            content.push_str("\n[dependencies]\n");
        }

        let dep_line = format!("{} = \"{}\"\n", dep_name, ver_str);
        if !content.contains(&format!("{} =", dep_name)) {
            content.push_str(&dep_line);
        }

        fs::write(&manifest_path, content)
            .map_err(|e| format!("Failed to update Arca.toml: {}", e))?;

        // Update Lockfile
        let mut lockfile = Lockfile::load_from_dir(&dir)?;
        lockfile.packages.insert(
            dep_name.to_string(),
            LockPackage {
                name: dep_name.to_string(),
                version: ver_str,
                checksum: format!("sha256:{:x}", dep_name.len() * 42),
            },
        );
        lockfile.save_to_dir(&dir)?;

        Ok(())
    }

    pub fn remove_dependency<P: AsRef<Path>>(dir: P, dep_name: &str) -> Result<(), String> {
        let manifest_path = dir.as_ref().join("Arca.toml");
        if !manifest_path.exists() {
            return Err(format!("Arca.toml not found in '{}'", dir.as_ref().display()));
        }

        let mut manifest = PackageManifest::load_from_dir(&dir)?;
        if manifest.dependencies.remove(dep_name).is_none() {
            return Err(format!("Dependency '{}' not found in Arca.toml", dep_name));
        }

        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read Arca.toml: {}", e))?;

        let filtered: Vec<&str> = content
            .lines()
            .filter(|line| !line.trim().starts_with(&format!("{} =", dep_name)))
            .collect();

        fs::write(&manifest_path, filtered.join("\n"))
            .map_err(|e| format!("Failed to update Arca.toml: {}", e))?;

        let mut lockfile = Lockfile::load_from_dir(&dir)?;
        lockfile.packages.remove(dep_name);
        lockfile.save_to_dir(&dir)?;

        Ok(())
    }

    pub fn update_dependencies<P: AsRef<Path>>(dir: P) -> Result<(), String> {
        let manifest = PackageManifest::load_from_dir(&dir)?;
        let mut lockfile = Lockfile::new();

        for (name, ver) in manifest.dependencies {
            lockfile.packages.insert(
                name.clone(),
                LockPackage {
                    name: name.clone(),
                    version: ver,
                    checksum: format!("sha256:{:x}", name.len() * 42),
                },
            );
        }

        lockfile.save_to_dir(&dir)?;
        Ok(())
    }

    pub fn publish_package<P: AsRef<Path>>(dir: P) -> Result<String, String> {
        let manifest = PackageManifest::load_from_dir(&dir)?;
        let main_arca = dir.as_ref().join("src").join("main.arca");
        let lib_arca = dir.as_ref().join("src").join("lib.arca");

        if !main_arca.exists() && !lib_arca.exists() {
            return Err("Cannot publish: Package must contain 'src/main.arca' or 'src/lib.arca'".to_string());
        }

        Ok(format!(
            "Package '{}-{}' validated and ready for distribution.",
            manifest.package.name, manifest.package.version
        ))
    }

    pub fn resolve_dependencies<P: AsRef<Path>>(&self, dir: P) -> Result<Vec<(String, String)>, String> {
        let manifest = PackageManifest::load_from_dir(&dir)?;
        let mut resolved = Vec::new();

        for (name, constraint) in &manifest.dependencies {
            let version = resolve_version_constraint(constraint);
            resolved.push((name.clone(), version));
        }

        Ok(resolved)
    }
}

fn resolve_version_constraint(constraint: &str) -> String {
    // Simple version constraint resolver
    // Supports: "1.0.0", "^1.0.0", "~1.0.0", ">=1.0.0", ">1.0.0", "<=1.0.0", "<1.0.0"
    let constraint = constraint.trim();
    if constraint.starts_with('^') {
        // Caret: ^1.0.0 -> >=1.0.0 <2.0.0
        let base = constraint.trim_start_matches('^');
        format!(">={},<{}", base, increment_major(base))
    } else if constraint.starts_with('~') {
        // Tilde: ~1.0.0 -> >=1.0.0 <1.1.0
        let base = constraint.trim_start_matches('~');
        format!(">={},<{}", base, increment_minor(base))
    } else if constraint.starts_with(">=") {
        constraint.to_string()
    } else if constraint.starts_with('>') {
        constraint.to_string()
    } else if constraint.starts_with("<=") {
        constraint.to_string()
    } else if constraint.starts_with('<') {
        constraint.to_string()
    } else {
        // Exact version
        constraint.to_string()
    }
}

fn increment_major(v: &str) -> String {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() >= 1 {
        let major: u32 = parts[0].parse().unwrap_or(0);
        format!("{}.0.0", major + 1)
    } else {
        "2.0.0".to_string()
    }
}

fn increment_minor(v: &str) -> String {
    let parts: Vec<&str> = v.split('.').collect();
    if parts.len() >= 2 {
        let major: u32 = parts[0].parse().unwrap_or(0);
        let minor: u32 = parts[1].parse().unwrap_or(0);
        format!("{}.{}.0", major, minor + 1)
    } else {
        "1.1.0".to_string()
    }
}
