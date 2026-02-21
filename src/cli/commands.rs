use crate::agent::ai_model::OllamaBackend;
use crate::agent::page_analyzer::{LlmPageAnalyzer, MockPageAnalyzer, PageAnalyzer};
use crate::browser::session::BrowserSession;
use crate::cli::config::build_explorer_config;
use crate::explorer::explorer::explore_live;
use crate::explorer::flow_detector::detect_flows;
use crate::explorer::test_generator::generate_test_plan;
use crate::report::console::format_console_report;
use crate::report::html::generate_html_report;
use crate::report::junit::generate_junit_xml;
use crate::report::report_model::TestSuiteReport;
use crate::spec::runner::TestRunner;
use crate::spec::spec_model::TestSpec;

// ============================================================================
// explore subcommand
// ============================================================================

pub fn cmd_explore(
    url: &str,
    max_pages: usize,
    max_depth: usize,
    explore_forms: bool,
    max_forms_per_page: usize,
    analyzer_name: &str,
    verbose: u8,
    ollama_endpoint: Option<&str>,
    ollama_model: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = build_explorer_config(url, max_pages, max_depth, explore_forms, max_forms_per_page);
    let analyzer = build_analyzer(analyzer_name, ollama_endpoint, ollama_model)?;
    let mut session = BrowserSession::launch()?;

    if verbose > 0 {
        eprintln!(
            "Exploring {} (max_pages={}, max_depth={})...",
            url, max_pages, max_depth
        );
    }

    let app_map = explore_live(&config, &mut session, analyzer.as_ref())?;
    session.quit()?;

    // Print summary
    println!(
        "Explored {} pages, {} transitions",
        app_map.page_count(),
        app_map.transitions.len()
    );

    let flows = detect_flows(&app_map);
    if !flows.is_empty() {
        println!("Detected {} flows:", flows.len());
        for flow in &flows {
            println!("  - {}", flow.name);
        }
    }

    for (_url, node) in &app_map.pages {
        println!(
            "  [{}] {:?} — {} ({} forms)",
            node.depth,
            node.page_model.category,
            node.title,
            node.page_model.forms.len()
        );
    }

    Ok(())
}

// ============================================================================
// run subcommand
// ============================================================================

/// Run test specs and return whether all passed.
pub fn cmd_run(
    spec_path: &str,
    format: &str,
    output: Option<&str>,
    verbose: u8,
) -> Result<bool, Box<dyn std::error::Error>> {
    let specs = load_specs(spec_path)?;

    if specs.is_empty() {
        eprintln!("No test specs found at: {}", spec_path);
        return Ok(true);
    }

    if verbose > 0 {
        eprintln!("Running {} test specs...", specs.len());
    }

    let mut session = BrowserSession::launch()?;
    let start = std::time::Instant::now();

    let mut results = Vec::new();
    for spec in &specs {
        if verbose > 0 {
            eprintln!("  Running: {}", spec.name);
        }
        let result = TestRunner::run(spec, &mut session);
        results.push(result);
    }

    let duration = start.elapsed().as_millis();
    session.quit()?;

    let report = TestSuiteReport::from_results("CLI Run", results).with_duration(duration);
    let all_passed = report.all_passed();

    // Format report
    let output_content = match format {
        "html" => generate_html_report(&report),
        "junit" => generate_junit_xml(&report),
        _ => format_console_report(&report),
    };

    // Write or print
    match output {
        Some(path) => std::fs::write(path, &output_content)?,
        None => print!("{}", output_content),
    }

    Ok(all_passed)
}

/// Load test specs from a single YAML file or a directory of YAML files.
pub fn load_specs(path: &str) -> Result<Vec<TestSpec>, Box<dyn std::error::Error>> {
    let metadata = std::fs::metadata(path)?;
    if metadata.is_dir() {
        let mut specs = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            let p = entry.path();
            if p.extension().map_or(false, |e| e == "yaml" || e == "yml") {
                let content = std::fs::read_to_string(&p)?;
                let spec: TestSpec = serde_yaml::from_str(&content)?;
                specs.push(spec);
            }
        }
        // Sort by name for deterministic order
        specs.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(specs)
    } else {
        let content = std::fs::read_to_string(path)?;
        let spec: TestSpec = serde_yaml::from_str(&content)?;
        Ok(vec![spec])
    }
}

// ============================================================================
// generate subcommand
// ============================================================================

pub fn cmd_generate(
    url: &str,
    output_dir: &str,
    max_pages: usize,
    max_depth: usize,
    explore_forms: bool,
    max_forms_per_page: usize,
    analyzer_name: &str,
    verbose: u8,
    ollama_endpoint: Option<&str>,
    ollama_model: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = build_explorer_config(url, max_pages, max_depth, explore_forms, max_forms_per_page);
    let analyzer = build_analyzer(analyzer_name, ollama_endpoint, ollama_model)?;
    let mut session = BrowserSession::launch()?;

    if verbose > 0 {
        eprintln!("Exploring {} for test generation...", url);
    }

    let app_map = explore_live(&config, &mut session, analyzer.as_ref())?;
    session.quit()?;

    let specs = generate_test_plan(&app_map);

    // Create output directory
    std::fs::create_dir_all(output_dir)?;

    // Write each spec as YAML
    for (i, spec) in specs.iter().enumerate() {
        let filename = format!("{:03}_{}.yaml", i + 1, sanitize_filename(&spec.name));
        let path = std::path::Path::new(output_dir).join(&filename);
        let yaml = serde_yaml::to_string(spec)?;
        std::fs::write(&path, &yaml)?;
        if verbose > 0 {
            eprintln!("  Wrote: {}", path.display());
        }
    }

    println!("Generated {} test specs in {}/", specs.len(), output_dir);
    Ok(())
}

// ============================================================================
// Helpers
// ============================================================================

/// Build the appropriate PageAnalyzer based on name.
fn build_analyzer(
    name: &str,
    ollama_endpoint: Option<&str>,
    ollama_model: Option<&str>,
) -> Result<Box<dyn PageAnalyzer>, Box<dyn std::error::Error>> {
    match name {
        "llm" => {
            let endpoint = ollama_endpoint.unwrap_or("http://localhost:11434/api/generate");
            let model = ollama_model.unwrap_or("qwen2.5:1.5b");
            let backend = OllamaBackend::new(endpoint, model);
            Ok(Box::new(LlmPageAnalyzer::new(Box::new(backend))))
        }
        _ => Ok(Box::new(MockPageAnalyzer)),
    }
}

/// Sanitize a test name into a safe filename.
pub fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
}
