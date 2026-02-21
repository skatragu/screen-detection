use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

// ============================================================================
// CLI Argument Parsing (clap derive)
// ============================================================================

#[derive(Parser, Debug)]
#[command(
    name = "screen-detection",
    version,
    about = "AI-powered autonomous UI testing tool"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Ollama API endpoint
    #[arg(long, global = true)]
    pub ollama_endpoint: Option<String>,

    /// Ollama model name
    #[arg(long, global = true)]
    pub ollama_model: Option<String>,

    /// Path to config file (default: screen-detection.yaml in current dir)
    #[arg(long, global = true)]
    pub config: Option<String>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Explore a website via BFS crawling
    Explore {
        /// URL to start exploring from
        #[arg(long)]
        url: String,

        /// Maximum pages to visit
        #[arg(long, default_value_t = 10)]
        max_pages: usize,

        /// Maximum BFS depth
        #[arg(long, default_value_t = 3)]
        max_depth: usize,

        /// Explore forms during crawling
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        explore_forms: bool,

        /// Max forms to submit per page
        #[arg(long, default_value_t = 3)]
        max_forms_per_page: usize,

        /// Page analyzer: mock or llm
        #[arg(long, default_value = "mock")]
        analyzer: String,
    },

    /// Run test specs from YAML files
    Run {
        /// Path to test spec YAML file or directory of YAML files
        #[arg(long)]
        spec: String,

        /// Output format: console, html, junit
        #[arg(long, default_value = "console")]
        format: String,

        /// Output file path (default: stdout for console, report.html / report.xml for others)
        #[arg(short, long)]
        output: Option<String>,
    },

    /// Explore a site and generate test spec YAML files
    Generate {
        /// URL to start exploring from
        #[arg(long)]
        url: String,

        /// Output directory for generated YAML specs
        #[arg(short, long, default_value = "tests/generated")]
        output_dir: String,

        /// Maximum pages to visit
        #[arg(long, default_value_t = 10)]
        max_pages: usize,

        /// Maximum BFS depth
        #[arg(long, default_value_t = 3)]
        max_depth: usize,

        /// Explore forms during crawling
        #[arg(long, default_value_t = true, action = clap::ArgAction::Set)]
        explore_forms: bool,

        /// Max forms to submit per page
        #[arg(long, default_value_t = 3)]
        max_forms_per_page: usize,

        /// Page analyzer: mock or llm
        #[arg(long, default_value = "mock")]
        analyzer: String,
    },
}

// ============================================================================
// Config File Model (optional YAML)
// ============================================================================

/// Optional YAML config file: `screen-detection.yaml`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub explore: ExploreConfig,
    #[serde(default)]
    pub run: RunConfig,
    #[serde(default)]
    pub ollama: OllamaConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            explore: ExploreConfig::default(),
            run: RunConfig::default(),
            ollama: OllamaConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExploreConfig {
    #[serde(default = "default_ten")]
    pub max_pages: usize,

    #[serde(default = "default_three")]
    pub max_depth: usize,

    #[serde(default = "default_true")]
    pub explore_forms: bool,

    #[serde(default = "default_three")]
    pub max_forms_per_page: usize,

    #[serde(default = "default_mock")]
    pub analyzer: String,
}

impl Default for ExploreConfig {
    fn default() -> Self {
        Self {
            max_pages: 10,
            max_depth: 3,
            explore_forms: true,
            max_forms_per_page: 3,
            analyzer: "mock".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunConfig {
    #[serde(default = "default_console")]
    pub format: String,

    pub output: Option<String>,
}

impl Default for RunConfig {
    fn default() -> Self {
        Self {
            format: "console".to_string(),
            output: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OllamaConfig {
    pub endpoint: Option<String>,
    pub model: Option<String>,
}

// Serde default helpers
fn default_ten() -> usize { 10 }
fn default_three() -> usize { 3 }
fn default_true() -> bool { true }
fn default_mock() -> String { "mock".to_string() }
fn default_console() -> String { "console".to_string() }

// ============================================================================
// Config File Loading
// ============================================================================

/// Load config from a YAML file. Returns defaults if file is missing or malformed.
pub fn load_config(path: Option<&str>) -> AppConfig {
    let config_path = path.unwrap_or("screen-detection.yaml");
    match std::fs::read_to_string(config_path) {
        Ok(content) => serde_yaml::from_str(&content).unwrap_or_default(),
        Err(_) => AppConfig::default(),
    }
}

// ============================================================================
// Config Builders (merge CLI args with config file)
// ============================================================================

/// Build an ExplorerConfig from resolved CLI/config values.
pub fn build_explorer_config(
    url: &str,
    max_pages: usize,
    max_depth: usize,
    explore_forms: bool,
    max_forms_per_page: usize,
) -> crate::explorer::app_map::ExplorerConfig {
    crate::explorer::app_map::ExplorerConfig {
        start_url: url.to_string(),
        max_pages,
        max_depth,
        same_origin_only: true,
        explore_forms,
        max_forms_per_page,
    }
}
