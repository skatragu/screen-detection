use clap::Parser;
use screen_detection::cli::commands::sanitize_filename;
use screen_detection::cli::config::{
    build_explorer_config, load_config, AppConfig, Cli, Commands,
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
