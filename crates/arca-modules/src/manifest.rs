//! Arca.toml package manifest definition and lightweight zero-dependency parser.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub name: String,
    pub version: String,
    pub edition: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LanguageConfig {
    pub version: String,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PackageManifest {
    pub package: PackageMetadata,
    pub language: LanguageConfig,
    pub dependencies: HashMap<String, String>,
}

impl PackageManifest {
    pub fn parse<S: AsRef<str>>(content: S) -> Result<Self, String> {
        let mut name = String::new();
        let mut version = "0.1.0".to_string();
        let mut edition = "2026".to_string();
        let mut dependencies = HashMap::new();

        let mut lang_version = "1.0".to_string();
        let capabilities = vec!["ffi".to_string(), "comptime".to_string(), "actors".to_string()];

        let mut current_section = "";

        for line in content.as_ref().lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }

            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = &trimmed[1..trimmed.len() - 1];
                continue;
            }

            if let Some((key, val)) = trimmed.split_once('=') {
                let k = key.trim();
                let v = val.trim().trim_matches('"').trim_matches('\'');

                match current_section {
                    "package" => match k {
                        "name" => name = v.to_string(),
                        "version" => version = v.to_string(),
                        "edition" => edition = v.to_string(),
                        _ => {}
                    },
                    "language" => match k {
                        "version" => lang_version = v.to_string(),
                        _ => {}
                    },
                    "dependencies" => {
                        dependencies.insert(k.to_string(), v.to_string());
                    }
                    _ => {}
                }
            }
        }

        if name.is_empty() {
            return Err("Missing required field 'name' in [package] section of Arca.toml".to_string());
        }

        Ok(Self {
            package: PackageMetadata {
                name,
                version,
                edition,
            },
            language: LanguageConfig {
                version: lang_version,
                capabilities,
            },
            dependencies,
        })
    }

    pub fn load_from_dir<P: AsRef<Path>>(dir: P) -> Result<Self, String> {
        let manifest_path = dir.as_ref().join("Arca.toml");
        if !manifest_path.exists() {
            return Err(format!(
                "Manifest file Arca.toml not found in directory '{}'",
                dir.as_ref().display()
            ));
        }

        let content = fs::read_to_string(&manifest_path)
            .map_err(|e| format!("Failed to read 'Arca.toml': {}", e))?;
        Self::parse(content)
    }

    pub fn generate_default(name: &str) -> String {
        format!(
            r#"[package]
name = "{}"
version = "0.1.0"
edition = "2026"

[language]
version = "1.0"
capabilities = ["ffi", "comptime", "actors", "simd"]

[dependencies]
"#,
            name
        )
    }
}
