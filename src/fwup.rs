use crate::log::*;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Options for creating a firmware update package with fwup
#[derive(Debug, Clone)]
pub struct FwupOptions {
    /// Path to the fwup configuration file
    pub config_file: PathBuf,
    /// Path for the output firmware package
    pub output_file: PathBuf,
    /// Working directory for the fwup command
    pub working_dir: Option<PathBuf>,
    /// Enable verbose output
    pub verbose: bool,
}

impl FwupOptions {
    /// Create new FwupOptions with required parameters
    pub fn new<P1, P2>(config_file: P1, output_file: P2) -> Self
    where
        P1: Into<PathBuf>,
        P2: Into<PathBuf>,
    {
        Self {
            config_file: config_file.into(),
            output_file: output_file.into(),
            working_dir: None,
            verbose: false,
        }
    }

    /// Set the working directory for the fwup command
    pub fn with_working_dir<P: Into<PathBuf>>(mut self, dir: P) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Enable verbose output
    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }
}

pub fn create_firmware_package(options: &FwupOptions) -> Result<(), String> {
    // Validate inputs
    if !options.config_file.exists() {
        return Err(format!(
            "Configuration file '{}' not found.",
            options.config_file.display()
        ));
    }

    // Create output directory if it doesn't exist
    if let Some(parent) = options.output_file.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return Err(format!(
                "Failed to create output directory '{}': {}",
                parent.display(),
                e
            ));
        }
    }

    // Build the fwup command
    let mut cmd = Command::new("fwup");
    cmd.arg("-c")
        .arg("-f")
        .arg(&options.config_file)
        .arg("-o")
        .arg(&options.output_file);

    // Set working directory if specified
    if let Some(ref working_dir) = options.working_dir {
        cmd.current_dir(working_dir);
    }

    if options.verbose {
        let working_dir_str = options
            .working_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| ".".to_string());

        log_debug(&format!(
            "Executing fwup in '{}': fwup -c -f {} -o {}",
            working_dir_str,
            options.config_file.display(),
            options.output_file.display()
        ));
    }

    // Execute the command
    let status = cmd.status();

    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                log_success(&format!(
                    "Created firmware package '{}' using configuration '{}'.",
                    options.output_file.display(),
                    options.config_file.display()
                ));
                Ok(())
            } else {
                Err(format!(
                    "fwup command failed with exit code: {}",
                    exit_status.code().unwrap_or(-1)
                ))
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Err(
                    "fwup command not found. Please install fwup to build firmware packages."
                        .to_string(),
                )
            } else {
                Err(format!("Failed to execute fwup command: {e}"))
            }
        }
    }
}

pub fn create_firmware_package_simple<P1, P2>(
    config_file: P1,
    output_file: P2,
) -> Result<(), String>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
{
    let options = FwupOptions::new(config_file.as_ref(), output_file.as_ref());
    create_firmware_package(&options)
}

pub fn create_firmware_package_in_dir<P1, P2, P3>(
    config_file: P1,
    output_file: P2,
    working_dir: P3,
) -> Result<(), String>
where
    P1: AsRef<Path>,
    P2: AsRef<Path>,
    P3: AsRef<Path>,
{
    let options = FwupOptions::new(config_file.as_ref(), output_file.as_ref())
        .with_working_dir(working_dir.as_ref());
    create_firmware_package(&options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_fwup_options_creation() {
        let options = FwupOptions::new("config.conf", "output.fw");
        assert_eq!(options.config_file, PathBuf::from("config.conf"));
        assert_eq!(options.output_file, PathBuf::from("output.fw"));
        assert_eq!(options.working_dir, None);
        assert!(!options.verbose);
    }

    #[test]
    fn test_fwup_options_with_working_dir() {
        let options = FwupOptions::new("config.conf", "output.fw").with_working_dir("/tmp");
        assert_eq!(options.working_dir, Some(PathBuf::from("/tmp")));
    }

    #[test]
    fn test_fwup_options_with_verbose() {
        let options = FwupOptions::new("config.conf", "output.fw").with_verbose(true);
        assert!(options.verbose);
    }

    #[test]
    fn test_missing_config_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("nonexistent.conf");
        let output_file = temp_dir.path().join("output.fw");

        let options = FwupOptions::new(&config_file, &output_file);
        let result = create_firmware_package(&options);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_creates_output_directory() {
        let temp_dir = TempDir::new().unwrap();
        let config_file = temp_dir.path().join("test.conf");
        let output_dir = temp_dir.path().join("nested").join("output");
        let output_file = output_dir.join("output.fw");

        // Create a dummy config file
        fs::write(&config_file, "# test config").unwrap();

        let options = FwupOptions::new(&config_file, &output_file);

        // This will fail because fwup isn't installed in test environment,
        // but it should create the output directory first
        let _ = create_firmware_package(&options);

        assert!(output_dir.exists());
    }
}
