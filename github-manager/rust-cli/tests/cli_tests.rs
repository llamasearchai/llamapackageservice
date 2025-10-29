use clap::Parser;
use llamasearch_cli::cli::{Cli, Commands, ScanArgs, OutputFormat};

#[test]
fn test_cli_parsing() {
    let args = vec!["llamasearch", "scan", "--org", "test", "--format", "json"];
    let cli = Cli::try_parse_from(args).unwrap();
    
    match cli.command {
        Some(Commands::Scan(args)) => {
            assert_eq!(args.org, "test");
            assert!(matches!(args.format, OutputFormat::Json));
        }
        _ => panic!("Expected Scan command"),
    }
}

#[test]
fn test_cli_help() {
    let args = vec!["llamasearch", "--help"];
    let result = Cli::try_parse_from(args);
    assert!(result.is_err()); // Help causes an error exit
}

#[test]
fn test_cli_version() {
    let args = vec!["llamasearch", "--version"];
    let result = Cli::try_parse_from(args);
    assert!(result.is_err()); // Version causes an error exit
}

#[test]
fn test_search_command() {
    let args = vec!["llamasearch", "search", "TODO", "--regex", "-C", "5"];
    let cli = Cli::try_parse_from(args).unwrap();
    
    match cli.command {
        Some(Commands::Search(args)) => {
            assert_eq!(args.query, "TODO");
            assert!(args.regex);
            assert_eq!(args.context, 5);
        }
        _ => panic!("Expected Search command"),
    }
}

#[test]
fn test_clone_command() {
    let args = vec!["llamasearch", "clone", "repo1", "repo2", "--update", "--parallel", "8"];
    let cli = Cli::try_parse_from(args).unwrap();
    
    match cli.command {
        Some(Commands::Clone(args)) => {
            assert_eq!(args.repos, vec!["repo1", "repo2"]);
            assert!(args.update);
            assert_eq!(args.parallel, 8);
        }
        _ => panic!("Expected Clone command"),
    }
}