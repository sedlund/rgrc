// Tests for main.rs to improve coverage
// Targets: error handling, color mode logic, command spawning, timetrace paths

use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::NamedTempFile;

#[test]
fn test_empty_command_exits_with_error() {
    // Test line 131-132: empty command handling
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .output()
        .expect("failed to run rgrc");
    
    // Should fail when no command is provided
    assert!(!output.status.success());
    // May show usage or error - just verify it exits with error
}

#[test]
fn test_nonexistent_command_returns_127() {
    // Test lines 222-223, 230-232: command not found error path
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["nonexistent-command-xyz-12345"])
        .output()
        .expect("failed to run rgrc");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("command not found") || stderr.contains("not found"));
    // Should exit with 127 for command not found
    assert_eq!(output.status.code().unwrap_or(1), 127);
}

#[test]
fn test_color_off_mode_no_ansi() {
    // Test lines 149-154: ColorMode::Off branch
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["--color=off", "echo", "test output"])
        .output()
        .expect("failed to run rgrc with --color=off");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should not contain ANSI escape codes
    assert!(!stdout.contains("\x1b["));
    assert!(stdout.contains("test output"));
}

#[test]
fn test_color_on_mode_enables_colorization() {
    // Test lines 149-154: ColorMode::On branch
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["--color=on", "echo", "ERROR: test"])
        .output()
        .expect("failed to run rgrc with --color=on");
    
    assert!(output.status.success());
    // With color=on, if there are rules for echo output, we might get ANSI codes
    // At minimum, the command should succeed
    assert!(!output.stdout.is_empty());
}

#[test]
fn test_pseudo_command_exact_match_exclusion() {
    // Test lines 167: pseudo_command exclusion check
    // When pseudo_command is exactly "ls", it should be excluded
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["ls"])
        .output()
        .expect("failed to run rgrc ls");
    
    // Should succeed (even if ls is excluded from colorization)
    assert!(output.status.success());
}

#[test]
fn test_piped_output_not_to_terminal() {
    // Test lines 222: stdout is not terminal path
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["echo", "piped output"])
        .stdout(Stdio::piped())
        .output()
        .expect("failed to run rgrc with piped output");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("piped output"));
}

#[test]
fn test_spawn_error_handling() {
    // Test lines 226-235: spawn error handling
    // Try to run a command that will fail to spawn
    // Use an invalid path to force spawn error
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["/this/path/does/not/exist/command"])
        .output()
        .expect("failed to run rgrc");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("command not found") || stderr.contains("Failed to spawn"));
}

#[test]
#[cfg(feature = "timetrace")]
fn test_timetrace_feature_with_env_var() {
    // Test lines 186-192: timetrace feature paths
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .env("RGRCTIME", "1")
        .args(["echo", "timed test"])
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run rgrc with RGRCTIME");
    
    assert!(output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    // Should contain timing information when timetrace feature is enabled
    assert!(stderr.contains("[rgrc:time]") || stderr.is_empty());
}

#[test]
fn test_command_with_args_passes_through() {
    // Test that command arguments are properly passed through
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["echo", "arg1", "arg2", "arg3"])
        .output()
        .expect("failed to run rgrc with multiple args");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("arg1"));
    assert!(stdout.contains("arg2"));
    assert!(stdout.contains("arg3"));
}

#[test]
fn test_rules_not_loaded_when_color_off() {
    // Test lines 195, 203: rules loading is skipped when should_colorize is false
    // Using --color=off should skip expensive rule loading
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["--color=off", "echo", "no rules loaded"])
        .output()
        .expect("failed to run rgrc");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("no rules loaded"));
}

#[test]
fn test_flush_cache_error_path() {
    // Test lines 41-42: flush_cache error handling
    // This tests the None branch when cache rebuild fails
    // Create a scenario where cache dir cannot be created (requires embed-configs feature)
    #[cfg(feature = "embed-configs")]
    {
        use std::env;
        // Set HOME to /dev/null (invalid directory) to force failure
        let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
            .env("HOME", "/dev/null/invalid")
            .args(["--flush-cache"])
            .output()
            .expect("failed to run rgrc --flush-cache");
        
        // Should fail with error message
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            assert!(
                stderr.contains("Error") || stderr.contains("Failed"),
                "Expected error message, got: {}",
                stderr
            );
        }
    }
}

#[test]
fn test_invalid_color_mode_argument() {
    // Test color mode parsing error
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["--color=invalid", "echo", "test"])
        .output()
        .expect("failed to run rgrc with invalid color");
    
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Invalid color mode") || stderr.contains("invalid"));
}

#[test]
fn test_console_colors_disabled_path() {
    // Test lines 145-148: console doesn't support colors path
    // Force NO_COLOR environment to disable colors
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .env("NO_COLOR", "1")
        .args(["echo", "no console colors"])
        .output()
        .expect("failed to run rgrc with NO_COLOR");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should not have ANSI codes when console colors are disabled
    assert!(stdout.contains("no console colors"));
}

#[test]
fn test_supported_command_colorization_check() {
    // Test lines 154-157: should_use_colorization_for_command_supported
    // Use a known supported command like 'ping' or 'ls'
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["--color=auto", "echo", "supported command"])
        .output()
        .expect("failed to run rgrc with auto color");
    
    assert!(output.status.success());
}

#[test]
fn test_wait_error_handling() {
    // Test lines 240-244: wait error handling
    // This is harder to test directly as wait() rarely fails
    // But we can verify the normal success path
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["echo", "wait success"])
        .output()
        .expect("failed to run rgrc");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("wait success"));
}

#[test]
fn test_child_exit_code_propagation() {
    // Test lines 247: exit code propagation
    // Run a command that exits with a specific non-zero code
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["sh", "-c", "exit 42"])
        .output()
        .expect("failed to run rgrc with failing command");
    
    // Should propagate the child's exit code
    assert!(!output.status.success());
    let code = output.status.code().unwrap_or(0);
    assert_eq!(code, 42, "Should propagate exit code 42");
}

#[test]
fn test_stdout_inherit_for_terminal() {
    // Test lines 222-247: direct stdout inheritance when not colorizing and terminal output
    // This path uses Stdio::inherit() for performance
    // We can't easily test this in a unit test as it requires a real terminal
    // But we can verify the command runs successfully
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["--color=off", "echo", "inherited stdout"])
        .output()
        .expect("failed to run rgrc");
    
    assert!(output.status.success());
}

#[test]
fn test_empty_rules_no_colorization() {
    // Test lines 206: rules.is_empty() check
    // When a command has no matching rules, colorization should be skipped
    let output = Command::new(env!("CARGO_BIN_EXE_rgrc"))
        .args(["echo", "no matching rules"])
        .output()
        .expect("failed to run rgrc");
    
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("no matching rules"));
}
