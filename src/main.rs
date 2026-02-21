use clap::Parser;
use screen_detection::cli::commands::{cmd_explore, cmd_generate, cmd_run};
use screen_detection::cli::config::{Cli, Commands, load_config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = load_config(cli.config.as_deref());

    // Resolve Ollama settings: CLI > config > env > defaults
    let ollama_endpoint = cli
        .ollama_endpoint
        .as_deref()
        .or(config.ollama.endpoint.as_deref());
    let ollama_model = cli
        .ollama_model
        .as_deref()
        .or(config.ollama.model.as_deref());

    match cli.command {
        Commands::Explore {
            url,
            max_pages,
            max_depth,
            explore_forms,
            max_forms_per_page,
            analyzer,
        } => {
            cmd_explore(
                &url,
                max_pages,
                max_depth,
                explore_forms,
                max_forms_per_page,
                &analyzer,
                cli.verbose,
                ollama_endpoint,
                ollama_model,
            )?;
        }
        Commands::Run {
            spec,
            format,
            output,
        } => {
            let all_passed = cmd_run(&spec, &format, output.as_deref(), cli.verbose)?;
            if !all_passed {
                std::process::exit(1);
            }
        }
        Commands::Generate {
            url,
            output_dir,
            max_pages,
            max_depth,
            explore_forms,
            max_forms_per_page,
            analyzer,
        } => {
            cmd_generate(
                &url,
                &output_dir,
                max_pages,
                max_depth,
                explore_forms,
                max_forms_per_page,
                &analyzer,
                cli.verbose,
                ollama_endpoint,
                ollama_model,
            )?;
        }
    }

    Ok(())
}
