use termy_config_core::{AppConfig, ConfigDiagnosticKind, config_path};

pub fn run() {
    let path = match config_path() {
        Some(p) => p,
        None => {
            eprintln!("Could not determine config directory");
            std::process::exit(1);
        }
    };

    println!("Config file: {}", path.display());

    if !path.exists() {
        println!("Status: File does not exist (using defaults)");
        println!("Result: Valid");
        return;
    }

    let contents = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            println!("Status: Failed to read file");
            println!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let ValidationReport { errors, warnings } = validate_contents(&contents);

    // Print results
    if errors.is_empty() && warnings.is_empty() {
        println!("Status: Valid");
    } else {
        if !errors.is_empty() {
            println!();
            println!("Errors:");
            for error in &errors {
                println!("  {}", error);
            }
        }

        if !warnings.is_empty() {
            println!();
            println!("Warnings:");
            for warning in &warnings {
                println!("  {}", warning);
            }
        }

        println!();
        if errors.is_empty() {
            println!("Result: Valid (with warnings)");
        } else {
            println!("Result: Invalid");
            std::process::exit(1);
        }
    }
}

pub struct ValidationReport {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

pub fn validate_contents(contents: &str) -> ValidationReport {
    let report = AppConfig::from_contents_with_report(contents);
    let mut errors = Vec::new();
    let mut warnings = Vec::new();

    for diagnostic in report.diagnostics {
        let message = format!("Line {}: {}", diagnostic.line_number, diagnostic.message);
        match diagnostic.kind {
            ConfigDiagnosticKind::InvalidSyntax | ConfigDiagnosticKind::InvalidValue => {
                errors.push(message);
            }
            ConfigDiagnosticKind::UnknownSection
            | ConfigDiagnosticKind::UnknownRootKey
            | ConfigDiagnosticKind::UnknownColorKey
            | ConfigDiagnosticKind::DuplicateRootKey => {
                warnings.push(message);
            }
        }
    }

    ValidationReport { errors, warnings }
}

#[cfg(test)]
mod tests {
    use super::validate_contents;

    #[test]
    fn mixed_case_root_keys_are_validated_case_insensitively() {
        let report = validate_contents(
            "Theme = termy\n\
             FoNt_SiZe = 13\n\
             CuRsOr_BlInK = true\n",
        );

        assert!(
            report.errors.is_empty(),
            "unexpected errors: {:?}",
            report.errors
        );
        assert!(
            report.warnings.is_empty(),
            "unexpected warnings: {:?}",
            report.warnings
        );
    }

    #[test]
    fn mixed_case_theme_key_parses_like_runtime_parser() {
        let report = validate_contents("THEME = custom-theme\n");

        assert!(report.errors.is_empty());
        assert!(report.warnings.is_empty());
    }

    #[test]
    fn boolean_aliases_and_positive_font_size_follow_parser_rules() {
        let report = validate_contents(
            "cursor_blink = yes\n\
             background_blur = 0\n\
             font_size = 0\n",
        );

        assert_eq!(report.errors.len(), 1);
        assert!(report.errors[0].contains("font_size"));
        assert!(report.warnings.is_empty());
    }
}
