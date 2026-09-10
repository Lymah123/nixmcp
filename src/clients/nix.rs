use std::collections::HashMap;

use serde::Deserialize;
use tokio::process::Command;

use crate::models::{NixosOption, Package};

#[derive(Debug, Clone, Default)]
pub struct NixClient;

impl NixClient {
    pub fn new() -> Self {
        Self
    }

    /// Search the Nix package collection using the local Nix installation.
    pub async fn search_packages(&self, query: &str) -> Result<Vec<Package>, String> {
        let output = Command::new("nix")
            .args(["search", "nixpkgs", query, "--json"])
            .output()
            .await
            .map_err(|error| format!("Failed to execute nix: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            return Err(format!(
                "nix search failed with status {}: {}",
                output.status,
                stderr.trim()
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| format!("Nix returned invalid UTF-8: {error}"))?;

        parse_search_results(&stdout)
    }

    /// Get information about a specific Nix package.
    pub async fn get_package(&self, attribute: &str) -> Result<Package, String> {
        let expression = r#"
let
  nixpkgs = builtins.getFlake "nixpkgs";
  attribute = builtins.getEnv "NIXMCP_ATTRIBUTE";
  parts = nixpkgs.lib.splitString "." attribute;

  package = builtins.foldl'
    (value: part: builtins.getAttr part value)
    nixpkgs.legacyPackages.x86_64-linux
    parts;

in
{
  pname = package.pname;
  version = package.version;
  description = package.meta.description or null;
}
"#;

        let output = Command::new("nix")
            .env("NIXMCP_ATTRIBUTE", attribute)
            .args(["eval", "--json", "--impure", "--expr", expression])
            .output()
            .await
            .map_err(|error| format!("Failed to execute nix: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            return Err(format!(
                "nix package evaluation failed with status {}: {}",
                output.status,
                stderr.trim()
            ));
        }

        #[derive(Debug, Deserialize)]
        struct PackageMetadata {
            pname: String,
            version: String,
            description: Option<String>,
        }

        let metadata: PackageMetadata = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("Failed to parse Nix package output: {error}"))?;

        Ok(Package {
            attribute: attribute.to_string(),
            pname: metadata.pname,
            version: metadata.version,
            description: metadata.description,
        })
    }

    pub async fn search_options(&self, query: &str) -> Result<Vec<NixosOption>, String> {
        let expression = r#"
let
  nixpkgs = builtins.getFlake "nixpkgs";
  query = builtins.getEnv "NIXMCP_QUERY";

  system = nixpkgs.lib.nixosSystem {
    system = "x86_64-linux";
    modules = [];
  };

  options = nixpkgs.lib.optionAttrSetToDocList system.options;

  matches = builtins.filter
    (option:
      nixpkgs.lib.hasInfix query option.name
    )
    options;

in
  nixpkgs.lib.take 20 matches
"#;

        let output = Command::new("nix")
            .env("NIXMCP_QUERY", query)
            .args(["eval", "--json", "--impure", "--expr", expression])
            .output()
            .await
            .map_err(|error| format!("failed to execute nix: {error}"))?;

        if !output.status.success() {
            return Err(String::from_utf8_lossy(&output.stderr).to_string());
        }

        let options: Vec<serde_json::Value> = serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("failed to parse nix output: {error}"))?;

        options
            .into_iter()
            .map(|option| {
                Ok(NixosOption {
                    path: option["name"].as_str().unwrap_or_default().to_string(),
                    option_type: option["type"].as_str().map(String::from),
                    description: option["description"].as_str().map(String::from),
                    default: option.get("default").cloned(),
                })
            })
            .collect()
    }

    pub async fn get_option(&self, path: &str) -> Result<NixosOption, String> {
        let expression = format!(
            r#"
        let
          nixpkgs = builtins.getFlake "nixpkgs";
          system = nixpkgs.lib.nixosSystem {{
            system = "x86_64-linux";
            modules = [];
          }};
          option = system.options.{path};
        in
          {{
            path = "{path}";
            description = if option ? description then option.description else null;
            option_type = if option ? type && option.type ? name then option.type.name else null;
            default = if option ? default then option.default else null;
          }}
        "#
        );

        let output = Command::new("nix")
            .args(["eval", "--json", "--impure", "--expr", &expression])
            .output()
            .await
            .map_err(|error| format!("Failed to execute nix: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            return Err(format!(
                "nix option evaluation failed with status {}: {}",
                output.status,
                stderr.trim()
            ));
        }

        serde_json::from_slice(&output.stdout)
            .map_err(|error| format!("Failed to parse NixOS option output: {error}"))
    }

    /// Inspect the outputs exposed by a Nix flake.
    pub async fn inspect_flake(&self, reference: &str) -> Result<serde_json::Value, String> {
        let output = Command::new("nix")
            .args(["flake", "show", reference, "--json"])
            .output()
            .await
            .map_err(|error| format!("Failed to execute nix: {error}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            return Err(format!(
                "nix flake show failed with status {}: {}",
                output.status,
                stderr.trim()
            ));
        }

        let stdout = String::from_utf8(output.stdout)
            .map_err(|error| format!("Nix returned invalid UTF-8: {error}"))?;

        serde_json::from_str(&stdout)
            .map_err(|error| format!("Failed to parse nix flake output: {error}"))
    }
}

#[derive(Debug, Deserialize)]
struct NixSearchPackage {
    pname: String,
    version: String,
    description: Option<String>,
}

fn parse_search_results(input: &str) -> Result<Vec<Package>, String> {
    let results: HashMap<String, NixSearchPackage> = serde_json::from_str(input)
        .map_err(|error| format!("Failed to parse nix search output: {error}"))?;

    Ok(results
        .into_iter()
        .map(|(attribute, package)| Package {
            attribute,
            pname: package.pname,
            version: package.version,
            description: package.description,
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nix_search_results() {
        let input = r#"
        {
            "legacyPackages.x86_64-linux.ripgrep": {
                "pname": "ripgrep",
                "version": "15.2.0",
                "description": "A fast search tool"
            }
        }
        "#;

        let packages = parse_search_results(input).expect("expected valid search results");

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].attribute, "legacyPackages.x86_64-linux.ripgrep");
        assert_eq!(packages[0].pname, "ripgrep");
        assert_eq!(packages[0].version, "15.2.0");
        assert_eq!(
            packages[0].description.as_deref(),
            Some("A fast search tool")
        );
    }

    #[test]
    fn parses_missing_description() {
        let input = r#"
        {
            "legacyPackages.x86_64-linux.example": {
                "pname": "example",
                "version": "1.0.0"
            }
        }
        "#;

        let packages = parse_search_results(input).expect("expected valid search results");

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].pname, "example");
        assert_eq!(packages[0].description, None);
    }

    #[test]
    fn rejects_invalid_json() {
        let result = parse_search_results("not valid json");

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .contains("Failed to parse nix search output")
        );
    }

    #[tokio::test]
    async fn gets_real_package() {
        let client = NixClient::new();

        let package = client
            .get_package("ripgrep")
            .await
            .expect("nix eval should succeed");

        assert_eq!(package.attribute, "ripgrep");
        assert_eq!(package.pname, "ripgrep");
        assert_eq!(package.version, "15.2.0");
        assert_eq!(
            package.description.as_deref(),
            Some(
                "Utility that combines the usability of The Silver Searcher with the raw speed of grep"
            )
        );
    }

    #[tokio::test]
    async fn gets_real_nested_package() {
        let client = NixClient::new();

        let package = client
            .get_package("python3Packages.requests")
            .await
            .expect("nix eval should succeed");

        assert_eq!(package.attribute, "python3Packages.requests");
        assert_eq!(package.pname, "requests");
    }

    #[tokio::test]
    async fn gets_real_nixos_option() {
        let client = NixClient::new();

        let option = client
            .get_option("services.postgresql.enable")
            .await
            .expect("nix option evaluation should succeed");

        assert_eq!(option.path, "services.postgresql.enable");
        assert_eq!(
            option.description.as_deref(),
            Some("Whether to enable PostgreSQL Server.")
        );
        assert_eq!(option.option_type.as_deref(), Some("bool"));
        assert_eq!(option.default, Some(serde_json::json!(false)));
    }

    #[tokio::test]
    async fn gets_nixos_option_without_default() {
        let client = NixClient::new();

        let option = client
            .get_option("services.postgresql.port")
            .await
            .expect("nix option evaluation should succeed");

        assert_eq!(option.path, "services.postgresql.port");
        assert_eq!(
            option.description.as_deref(),
            Some("Alias of {option}`services.postgresql.settings.port`.")
        );
        assert_eq!(option.option_type.as_deref(), Some("submodule"));
        assert_eq!(option.default, None);
    }

    #[tokio::test]
    async fn rejects_invalid_nixos_option() {
        let client = NixClient::new();

        let result = client
            .get_option("services.this.option.does.not.exist")
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nix option evaluation failed"));
    }

    #[tokio::test]
    async fn gets_nixos_option_from_another_subsystem() {
        let client = NixClient::new();

        let option = client
            .get_option("services.openssh.enable")
            .await
            .expect("nix option evaluation should succeed");

        assert_eq!(option.path, "services.openssh.enable");
        assert_eq!(
            option.description.as_deref(),
            Some(
                "Whether to enable the OpenSSH secure shell daemon, which\nallows secure remote logins.\n"
            )
        );
        assert_eq!(option.option_type.as_deref(), Some("bool"));
        assert_eq!(option.default, Some(serde_json::json!(false)));
    }

    #[tokio::test]
    async fn inspects_real_flake() {
        let client = NixClient::new();

        let flake = client
            .inspect_flake("nixpkgs")
            .await
            .expect("nix flake show should succeed");

        assert!(flake.is_object());
        assert!(!flake.as_object().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_invalid_flake() {
        let client = NixClient::new();

        let result = client
            .inspect_flake("github:does-not-exist/nixmcp-invalid-flake")
            .await;

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("nix flake show failed"));
    }

    #[tokio::test]
    async fn searches_real_nix_installation() {
        let client = NixClient::new();

        let packages = client
            .search_packages("ripgrep")
            .await
            .expect("nix search should succeed");

        assert!(!packages.is_empty());
        assert!(packages.iter().any(|package| package.pname == "ripgrep"));
    }
}
