use std::collections::HashMap;

use clap::Parser;
use screen_detection::cli::commands::sanitize_filename;
use screen_detection::cli::config::{
    build_explorer_config, load_config, AppConfig, AuthConfig, Cli, Commands,
    ExclusionConfig, ValueConfig,
};

// ============================================================================
// CLI Argument Parsing Tests
// ============================================================================

#[test]
fn cli_parse_explore_minimal() {
    let cli = Cli::parse_from(["screen-detection", "explore", "--url", "https://example.com"]);
    match cli.command {
        Commands::Explore {
            url,
            max_pages,
            max_depth,
            analyzer,
            explore_forms,
            max_forms_per_page,
        } => {
            assert_eq!(url, "https://example.com");
            assert_eq!(max_pages, 10);
            assert_eq!(max_depth, 3);
            assert_eq!(analyzer, "mock");
            assert!(explore_forms);
            assert_eq!(max_forms_per_page, 3);
        }
        _ => panic!("Expected Explore command"),
    }
}

#[test]
fn cli_parse_explore_all_args() {
    let cli = Cli::parse_from([
        "screen-detection",
        "explore",
        "--url",
        "https://test.com",
        "--max-pages",
        "5",
        "--max-depth",
        "2",
        "--explore-forms",
        "false",
        "--max-forms-per-page",
        "1",
        "--analyzer",
        "llm",
    ]);
    match cli.command {
        Commands::Explore {
            url,
            max_pages,
            max_depth,
            analyzer,
            explore_forms,
            max_forms_per_page,
        } => {
            assert_eq!(url, "https://test.com");
            assert_eq!(max_pages, 5);
            assert_eq!(max_depth, 2);
            assert_eq!(analyzer, "llm");
            assert!(!explore_forms);
            assert_eq!(max_forms_per_page, 1);
        }
        _ => panic!("Expected Explore command"),
    }
}

#[test]
fn cli_parse_run_minimal() {
    let cli = Cli::parse_from(["screen-detection", "run", "--spec", "test.yaml"]);
    match cli.command {
        Commands::Run {
            spec,
            format,
            output,
        } => {
            assert_eq!(spec, "test.yaml");
            assert_eq!(format, "console");
            assert!(output.is_none());
        }
        _ => panic!("Expected Run command"),
    }
}

#[test]
fn cli_parse_run_with_format() {
    let cli = Cli::parse_from([
        "screen-detection",
        "run",
        "--spec",
        "test.yaml",
        "--format",
        "html",
        "-o",
        "report.html",
    ]);
    match cli.command {
        Commands::Run {
            spec,
            format,
            output,
        } => {
            assert_eq!(spec, "test.yaml");
            assert_eq!(format, "html");
            assert_eq!(output, Some("report.html".to_string()));
        }
        _ => panic!("Expected Run command"),
    }
}

#[test]
fn cli_parse_generate_minimal() {
    let cli = Cli::parse_from(["screen-detection", "generate", "--url", "https://example.com"]);
    match cli.command {
        Commands::Generate {
            url,
            output_dir,
            max_pages,
            max_depth,
            ..
        } => {
            assert_eq!(url, "https://example.com");
            assert_eq!(output_dir, "tests/generated");
            assert_eq!(max_pages, 10);
            assert_eq!(max_depth, 3);
        }
        _ => panic!("Expected Generate command"),
    }
}

#[test]
fn cli_parse_generate_all_args() {
    let cli = Cli::parse_from([
        "screen-detection",
        "generate",
        "--url",
        "https://test.com",
        "-o",
        "./my-specs",
        "--max-pages",
        "20",
        "--max-depth",
        "5",
        "--analyzer",
        "llm",
    ]);
    match cli.command {
        Commands::Generate {
            url,
            output_dir,
            max_pages,
            max_depth,
            analyzer,
            ..
        } => {
            assert_eq!(url, "https://test.com");
            assert_eq!(output_dir, "./my-specs");
            assert_eq!(max_pages, 20);
            assert_eq!(max_depth, 5);
            assert_eq!(analyzer, "llm");
        }
        _ => panic!("Expected Generate command"),
    }
}

#[test]
fn cli_parse_global_verbose() {
    let cli = Cli::parse_from(["screen-detection", "-v", "run", "--spec", "t.yaml"]);
    assert_eq!(cli.verbose, 1);

    let cli2 = Cli::parse_from(["screen-detection", "-vvv", "run", "--spec", "t.yaml"]);
    assert_eq!(cli2.verbose, 3);
}

#[test]
fn cli_parse_global_ollama() {
    let cli = Cli::parse_from([
        "screen-detection",
        "--ollama-endpoint",
        "http://custom:11434/api/generate",
        "--ollama-model",
        "llama3",
        "explore",
        "--url",
        "https://example.com",
    ]);
    assert_eq!(
        cli.ollama_endpoint,
        Some("http://custom:11434/api/generate".to_string())
    );
    assert_eq!(cli.ollama_model, Some("llama3".to_string()));
}

// ============================================================================
// Config File Tests
// ============================================================================

#[test]
fn config_load_missing_file() {
    let config = load_config(Some("nonexistent_file_that_does_not_exist.yaml"));
    // Should return defaults without error
    assert_eq!(config.explore.max_pages, 10);
    assert_eq!(config.explore.max_depth, 3);
    assert_eq!(config.run.format, "console");
}

#[test]
fn config_default_values() {
    let config = AppConfig::default();
    assert_eq!(config.explore.max_pages, 10);
    assert_eq!(config.explore.max_depth, 3);
    assert!(config.explore.explore_forms);
    assert_eq!(config.explore.max_forms_per_page, 3);
    assert_eq!(config.explore.analyzer, "mock");
    assert_eq!(config.run.format, "console");
    assert!(config.run.output.is_none());
    assert!(config.ollama.endpoint.is_none());
    assert!(config.ollama.model.is_none());
}

#[test]
fn config_yaml_roundtrip() {
    let config = AppConfig::default();
    let yaml = serde_yaml::to_string(&config).unwrap();
    let parsed: AppConfig = serde_yaml::from_str(&yaml).unwrap();
    assert_eq!(parsed.explore.max_pages, config.explore.max_pages);
    assert_eq!(parsed.explore.max_depth, config.explore.max_depth);
    assert_eq!(parsed.run.format, config.run.format);
}

#[test]
fn config_partial_yaml() {
    let yaml = r#"
explore:
  max_pages: 20
ollama:
  model: "llama3"
"#;
    let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.explore.max_pages, 20);
    // Other explore fields get defaults
    assert_eq!(config.explore.max_depth, 3);
    assert!(config.explore.explore_forms);
    assert_eq!(config.explore.analyzer, "mock");
    // Run gets full defaults
    assert_eq!(config.run.format, "console");
    // Ollama partially filled
    assert_eq!(config.ollama.model, Some("llama3".to_string()));
    assert!(config.ollama.endpoint.is_none());
}

// ============================================================================
// Builder / Helper Tests
// ============================================================================

#[test]
fn build_explorer_config_wiring() {
    let config = build_explorer_config("https://example.com", 5, 2, false, 1);
    assert_eq!(config.start_url, "https://example.com");
    assert_eq!(config.max_pages, 5);
    assert_eq!(config.max_depth, 2);
    assert!(!config.explore_forms);
    assert_eq!(config.max_forms_per_page, 1);
    assert!(config.same_origin_only);
}

#[test]
fn sanitize_filename_special_chars() {
    assert_eq!(sanitize_filename("Smoke: Login Page"), "smoke__login_page");
    assert_eq!(sanitize_filename("Form test (email)"), "form_test__email_");
    assert_eq!(sanitize_filename("simple-name"), "simple-name");
    assert_eq!(sanitize_filename("under_score"), "under_score");
}

#[test]
fn load_specs_single_file() {
    use screen_detection::cli::commands::load_specs;
    use std::io::Write;

    // Create a temp YAML spec file
    let dir = std::env::temp_dir().join("screen_detection_cli_test");
    std::fs::create_dir_all(&dir).unwrap();
    let spec_path = dir.join("test_spec.yaml");

    let yaml = r##"
name: "Test Smoke"
start_url: "https://example.com"
steps:
  - action: wait
    duration_ms: 1000
  - action: assert
    assertions:
      - type: title_contains
        expected: "Example"
"##;

    let mut f = std::fs::File::create(&spec_path).unwrap();
    f.write_all(yaml.as_bytes()).unwrap();

    let specs = load_specs(spec_path.to_str().unwrap()).unwrap();
    assert_eq!(specs.len(), 1);
    assert_eq!(specs[0].name, "Test Smoke");
    assert_eq!(specs[0].start_url, "https://example.com");
    assert_eq!(specs[0].steps.len(), 2);

    // Cleanup
    std::fs::remove_file(&spec_path).ok();
    std::fs::remove_dir(&dir).ok();
}

// ============================================================================
// AuthConfig tests
// ============================================================================

#[test]
fn auth_config_defaults() {
    let auth = AuthConfig::default();
    assert!(auth.credentials.is_empty());
    assert!(auth.login_url.is_none());
    assert!(auth.submit_label.is_none());
    assert!(auth.success_url_contains.is_none());
    assert!(auth.success_text.is_none());
}

#[test]
fn auth_config_has_credentials() {
    let mut auth = AuthConfig::default();
    assert!(!auth.has_credentials());
    auth.credentials.insert("Email".into(), "admin@test.com".into());
    assert!(auth.has_credentials());
}

#[test]
fn auth_config_yaml_roundtrip() {
    let yaml = r#"
login_url: "https://myapp.com/login"
credentials:
  Email: "admin@staging.com"
  Password: "StagingPass2024!"
submit_label: "Sign In"
success_url_contains: "/dashboard"
success_text: "Welcome"
"#;
    let auth: AuthConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(auth.login_url, Some("https://myapp.com/login".into()));
    assert_eq!(auth.credentials.get("Email"), Some(&"admin@staging.com".to_string()));
    assert_eq!(auth.credentials.get("Password"), Some(&"StagingPass2024!".to_string()));
    assert_eq!(auth.submit_label, Some("Sign In".into()));
    assert_eq!(auth.success_url_contains, Some("/dashboard".into()));
    assert_eq!(auth.success_text, Some("Welcome".into()));

    // Roundtrip
    let serialized = serde_yaml::to_string(&auth).unwrap();
    let parsed: AuthConfig = serde_yaml::from_str(&serialized).unwrap();
    assert_eq!(parsed.login_url, auth.login_url);
    assert_eq!(parsed.credentials, auth.credentials);
}

#[test]
fn auth_config_partial() {
    // Only credentials set — rest default
    let yaml = r#"
credentials:
  Password: "pass123"
"#;
    let auth: AuthConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(auth.has_credentials());
    assert!(auth.login_url.is_none());
    assert!(auth.submit_label.is_none());
}

// ============================================================================
// ValueConfig tests
// ============================================================================

#[test]
fn value_config_resolve_field() {
    let mut config = ValueConfig::default();
    config.fields.insert("Email".into(), "tester@example.com".into());

    let result = config.resolve("Email", None);
    assert_eq!(result, Some(&"tester@example.com".to_string()));
}

#[test]
fn value_config_resolve_category_override() {
    let mut config = ValueConfig::default();
    config.fields.insert("Email".into(), "global@example.com".into());
    let mut cat_map = HashMap::new();
    cat_map.insert("Email".into(), "checkout@example.com".into());
    config.categories.insert("Checkout".into(), cat_map);

    // Category override wins
    assert_eq!(
        config.resolve("Email", Some("Checkout")),
        Some(&"checkout@example.com".to_string())
    );
    // Non-matching category falls back to global
    assert_eq!(
        config.resolve("Email", Some("Login")),
        Some(&"global@example.com".to_string())
    );
}

#[test]
fn value_config_resolve_case_insensitive() {
    let mut config = ValueConfig::default();
    config.fields.insert("email".into(), "ci@example.com".into());

    // Label case doesn't matter
    assert_eq!(config.resolve("Email", None), Some(&"ci@example.com".to_string()));
    assert_eq!(config.resolve("EMAIL", None), Some(&"ci@example.com".to_string()));
    assert_eq!(config.resolve("email", None), Some(&"ci@example.com".to_string()));
}

#[test]
fn value_config_resolve_no_match() {
    let config = ValueConfig::default();
    assert_eq!(config.resolve("Phone", None), None);
    assert_eq!(config.resolve("Phone", Some("Checkout")), None);
}

#[test]
fn value_config_yaml_roundtrip() {
    let yaml = r#"
fields:
  Email: "tester@mycompany.com"
  Phone: "555-0199"
categories:
  Checkout:
    Card Number: "4000056655665556"
    CVV: "314"
"#;
    let config: ValueConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.fields.get("Email"), Some(&"tester@mycompany.com".to_string()));
    assert_eq!(
        config.categories.get("Checkout").and_then(|m| m.get("CVV")),
        Some(&"314".to_string())
    );

    // Roundtrip
    let serialized = serde_yaml::to_string(&config).unwrap();
    let parsed: ValueConfig = serde_yaml::from_str(&serialized).unwrap();
    assert_eq!(parsed.fields, config.fields);
    assert_eq!(parsed.categories, config.categories);
}

// ============================================================================
// ExclusionConfig tests
// ============================================================================

#[test]
fn exclusion_config_should_skip() {
    let config = ExclusionConfig {
        skip_urls: vec!["/logout".into(), "/admin".into()],
        include_urls: vec![],
    };

    assert!(config.should_skip("https://myapp.com/logout"));
    assert!(config.should_skip("https://myapp.com/admin/users"));
    assert!(!config.should_skip("https://myapp.com/dashboard"));
}

#[test]
fn exclusion_config_empty_skips_nothing() {
    let config = ExclusionConfig::default();
    assert!(!config.should_skip("https://myapp.com/anything"));
}

#[test]
fn exclusion_config_yaml_roundtrip() {
    let yaml = r#"
skip_urls:
  - "/logout"
  - "/admin"
include_urls:
  - "https://myapp.com/settings"
"#;
    let config: ExclusionConfig = serde_yaml::from_str(yaml).unwrap();
    assert_eq!(config.skip_urls.len(), 2);
    assert!(config.skip_urls.contains(&"/logout".to_string()));
    assert_eq!(config.include_urls.len(), 1);
    assert_eq!(config.include_urls[0], "https://myapp.com/settings");

    // Roundtrip
    let serialized = serde_yaml::to_string(&config).unwrap();
    let parsed: ExclusionConfig = serde_yaml::from_str(&serialized).unwrap();
    assert_eq!(parsed.skip_urls, config.skip_urls);
    assert_eq!(parsed.include_urls, config.include_urls);
}

// ============================================================================
// AppConfig full-section and backward-compat tests
// ============================================================================

#[test]
fn app_config_with_all_sections() {
    let yaml = r#"
auth:
  login_url: "https://myapp.com/login"
  credentials:
    Email: "admin@staging.com"
    Password: "StagingPass2024!"
  success_url_contains: "/dashboard"

values:
  fields:
    Email: "tester@mycompany.com"
    Phone: "555-0199"
  categories:
    Checkout:
      Card Number: "4000056655665556"

exclusions:
  skip_urls:
    - "/logout"
    - "/admin"
  include_urls:
    - "https://myapp.com/settings"

explore:
  max_pages: 20
  max_depth: 4
"#;
    let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(config.auth.has_credentials());
    assert_eq!(config.auth.login_url, Some("https://myapp.com/login".into()));
    assert_eq!(config.values.fields.get("Phone"), Some(&"555-0199".to_string()));
    assert!(config.exclusions.should_skip("https://myapp.com/logout"));
    assert!(!config.exclusions.should_skip("https://myapp.com/dashboard"));
    assert_eq!(config.explore.max_pages, 20);
}

#[test]
fn app_config_backward_compat() {
    // Old YAML without auth/values/exclusions sections should still parse
    let yaml = r#"
explore:
  max_pages: 5
  max_depth: 2
"#;
    let config: AppConfig = serde_yaml::from_str(yaml).unwrap();
    assert!(!config.auth.has_credentials());
    assert!(config.values.fields.is_empty());
    assert!(config.exclusions.skip_urls.is_empty());
    assert_eq!(config.explore.max_pages, 5);
}

// ============================================================================
// Phase 14 Step 6: Value Overrides + Exclusion Filtering wiring
// ============================================================================

#[test]
fn app_config_exclusions_are_wired() {
    // AppConfig.exclusions is properly populated from YAML and available to commands
    let yaml = r#"
exclusions:
  skip_urls:
    - "/logout"
    - "/admin"
  include_urls:
    - "https://myapp.com/settings"
"#;
    let config: AppConfig = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(config.exclusions.skip_urls.len(), 2);
    assert_eq!(config.exclusions.include_urls.len(), 1);
    assert!(config.exclusions.should_skip("https://myapp.com/logout"));
    assert!(!config.exclusions.should_skip("https://myapp.com/dashboard"));

    // include_urls is accessible
    assert_eq!(
        config.exclusions.include_urls[0],
        "https://myapp.com/settings"
    );
}

#[test]
fn app_config_value_overrides_are_wired() {
    // AppConfig.values is properly populated from YAML and available to commands
    let yaml = r#"
values:
  fields:
    Email: "custom@test.com"
    Phone: "555-9999"
  categories:
    Login:
      Email: "login@test.com"
"#;
    let config: AppConfig = serde_yaml::from_str(yaml).unwrap();

    assert_eq!(config.values.fields.len(), 2);
    assert_eq!(
        config.values.fields.get("Email"),
        Some(&"custom@test.com".to_string())
    );
    // Category-scoped override
    assert_eq!(
        config.values.resolve("Email", Some("Login")),
        Some(&"login@test.com".to_string())
    );
    // Global override
    assert_eq!(
        config.values.resolve("Phone", None),
        Some(&"555-9999".to_string())
    );
    // No match returns None
    assert_eq!(config.values.resolve("Username", None), None);
}

#[test]
fn value_config_domains_resolve() {
    let mut config = ValueConfig::default();
    let mut domain_map = std::collections::HashMap::new();
    domain_map.insert("MSISDN".to_string(), "447700123456".to_string());
    config.domains.insert("telecom".to_string(), domain_map);

    // Domain substring match: "telecom SIM provisioning" contains "telecom"
    assert_eq!(
        config.resolve("MSISDN", Some("telecom SIM provisioning")),
        Some(&"447700123456".to_string())
    );

    // No domain match
    assert_eq!(config.resolve("MSISDN", Some("medical intake")), None);

    // No domain provided falls through to global fields
    assert_eq!(config.resolve("MSISDN", None), None);
}

#[test]
fn value_config_categories_compat_still_works() {
    // Old YAML with "categories:" key still resolves correctly (backward compat)
    let yaml = r#"
categories:
  Checkout:
    CVV: "999"
    Card Number: "4111111111111111"
fields:
  Email: "test@example.com"
"#;
    let config: ValueConfig = serde_yaml::from_str(yaml).unwrap();
    // categories compat: exact match (old behavior)
    assert_eq!(config.resolve("CVV", Some("Checkout")), Some(&"999".to_string()));
    // global field
    assert_eq!(config.resolve("Email", None), Some(&"test@example.com".to_string()));
}
