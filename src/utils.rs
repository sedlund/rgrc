//! # utils.rs - Utility functions for rgrc
//!
//! This module contains various utility functions used throughout the rgrc application.

/// Simple command existence check without external dependencies
/// Check whether an executable named `cmd` exists on the user's `PATH`.
///
/// This performs a lightweight search of directories in the `PATH` environment
/// variable and returns `true` if a file with the given name exists in any
/// directory. On Windows, common executable extensions are also considered.
///
/// # Examples
///
/// ```ignore
/// assert!(rgrc::utils::command_exists("ls"));
/// assert!(!rgrc::utils::command_exists("this-command-doesnt-exist-xyz"));
/// ```
pub fn command_exists(cmd: &str) -> bool {
    // Empty command is not valid
    if cmd.is_empty() {
        return false;
    }

    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let full_path = dir.join(cmd);
            if full_path.exists() {
                return true;
            }
            // Also check with common extensions on Windows
            #[cfg(target_os = "windows")]
            {
                for ext in &[".exe", ".cmd", ".bat", ".com"] {
                    let full_path_with_ext = dir.join(format!("{}{}", cmd, ext));
                    if full_path_with_ext.exists() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Print help message to stdout
/// Print a short help/usage message to stdout.
///
/// This is a convenience helper used by the binary; it prints the basic
/// options and examples to the standard output.
pub fn print_help() {
    println!("Rusty Generic Colouriser");
    println!();
    println!("Usage: rgrc [OPTIONS] COMMAND [ARGS...]");
    println!();
    println!("Options:");
    println!("  --color MODE      Override color output (on, off, auto)");
    println!("  --aliases         Output shell aliases for available binaries");
    println!("  --all-aliases     Output all shell aliases");
    println!("  --except CMD,..   Exclude commands from alias generation");
    println!("  --help, -h        Show this help message");
    println!();
    println!("Examples:");
    println!("  rgrc ping -c 4 google.com");
    println!("  rgrc --color=off ls -la");
    println!("  rgrc --aliases");
}

/// Quick check if a command is likely to benefit from colorization (used for Smart strategy)
/// This is a lightweight check that doesn't require loading rules
/// Heuristic: does `command` likely benefit from colorization?
///
/// This lightweight function returns `true` for commands that historically
/// produce output that benefits from color highlighting (e.g., `ls`, `ping`,
/// `df`). It is intentionally conservative and inexpensive to compute.
///
/// # Examples
///
/// ```ignore
/// assert!(rgrc::utils::should_use_colorization_for_command_benefit("ls"));
/// assert!(!rgrc::utils::should_use_colorization_for_command_benefit("echo"));
/// ```
pub fn should_use_colorization_for_command_benefit(command: &str) -> bool {
    // Commands that definitely benefit from colorization (have meaningful output to colorize)
    match command {
        "ant" | "blkid" | "curl" | "cvs" | "df" | "diff" | "dig" | "dnf" | "docker" | "du"
        | "env" | "esperanto" | "fdisk" | "findmnt" | "free" | "gcc" | "getfacl" | "getsebool"
        | "id" | "ifconfig" | "ip" | "iptables" | "irclog" | "iwconfig" | "jobs" | "kubectl"
        | "tail" | "last" | "ldap" | "log" | "lolcat" | "lsattr" | "lsblk" | "lsmod" | "lsof"
        | "lspci" | "lsusb" | "mount" | "mvn" | "netstat" | "nmap" | "ntpdate" | "php" | "ping"
        | "ping2" | "proftpd" | "ps" | "pv" | "semanage" | "sensors" | "showmount" | "sockstat"
        | "sql" | "ss" | "stat" | "sysctl" | "systemctl" | "tcpdump" | "traceroute" | "tune2fs"
        | "ulimit" | "vmstat" | "wdiff" | "whois" | "yaml" | "go" | "iostat" | "ls" => true,
        // For other commands, assume they don't benefit from colorization
        _ => false,
    }
}

/// Curated list of commands known to work well with grc
/// These commands have colorization rules that provide meaningful visual improvements
/// Curated list of commands that ship with colorization rules.
///
/// This array contains the command identifiers corresponding to files in
/// `share/conf.*` and is used by alias generation and the "Always" color
/// strategy to decide which commands are supported.
///
/// # Example
///
/// ```ignore
/// if rgrc::utils::SUPPORTED_COMMANDS.contains(&"ping") {
///     println!("ping is supported for colorization");
/// }
/// ```
pub const SUPPORTED_COMMANDS: &[&str] = &[
    "ant",
    "blkid",
    "common",
    "curl",
    "cvs",
    "df",
    "diff",
    "dig",
    "dnf",
    "docker",
    "du",
    "dummy",
    "env",
    "esperanto",
    "fdisk",
    "findmnt",
    "free",
    "gcc",
    "getfacl",
    "getsebool",
    "id",
    "ifconfig",
    "ip",
    "iptables",
    "irclog",
    "iwconfig",
    "jobs",
    "kubectl",
    "last",
    "ldap",
    "log",
    "lolcat",
    "lsattr",
    "lsblk",
    "lsmod",
    "lsof",
    "lspci",
    "ls",
    "lsusb",
    "mount",
    "mvn",
    "netstat",
    "nmap",
    "ntpdate",
    "php",
    "ping",
    "ping2",
    "proftpd",
    "ps",
    "pv",
    "semanage",
    "sensors",
    "showmount",
    "sockstat",
    "sql",
    "ss",
    "stat",
    "sysctl",
    "systemctl",
    "tail",
    "tcpdump",
    "traceroute",
    "tune2fs",
    "ulimit",
    "uptime",
    "vmstat",
    "wdiff",
    "whois",
    "yaml",
    "docker",
    "go",
    "iostat",
];

/// Check if a command has colorization rules available (used for Always strategy)
/// Return `true` when a command has shipped colorization rules (present in
/// `SUPPORTED_COMMANDS`). This is a simple membership check used by the
/// Always colorization strategy.
///
/// # Examples
///
/// ```ignore
/// assert!(rgrc::utils::should_use_colorization_for_command_supported("ls"));
/// assert!(!rgrc::utils::should_use_colorization_for_command_supported("unknown"));
/// ```
pub fn should_use_colorization_for_command_supported(command: &str) -> bool {
    SUPPORTED_COMMANDS.contains(&command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_exists() {
        // Test existing commands
        assert!(command_exists("echo"), "echo command should exist");
        assert!(command_exists("ls"), "ls command should exist");

        // Test non-existing command
        assert!(
            !command_exists("nonexistent_command_xyz123"),
            "nonexistent command should not exist"
        );

        // Test with absolute path (if it exists)
        assert!(
            command_exists("/bin/echo") || command_exists("/usr/bin/echo"),
            "echo should exist in standard locations"
        );

        // Test empty string
        assert!(
            !command_exists(""),
            "empty string should not be a valid command"
        );

        // Test command with spaces (should not exist)
        assert!(
            !command_exists("command with spaces"),
            "commands with spaces should not exist"
        );
    }

    #[test]
    fn test_should_use_colorization_for_command_benefit() {
        // Test commands that benefit from colorization
        assert!(should_use_colorization_for_command_benefit("ping"));
        assert!(should_use_colorization_for_command_benefit("ls"));
        assert!(should_use_colorization_for_command_benefit("df"));

        // Test commands that don't benefit from colorization
        assert!(!should_use_colorization_for_command_benefit(
            "unknown_command"
        ));
        assert!(!should_use_colorization_for_command_benefit(""));
    }

    #[test]
    fn test_should_use_colorization_for_command_supported() {
        // Test supported commands
        assert!(should_use_colorization_for_command_supported("ping"));
        assert!(should_use_colorization_for_command_supported("ls"));
        assert!(should_use_colorization_for_command_supported("df"));

        // Test unsupported commands
        assert!(!should_use_colorization_for_command_supported(
            "unknown_command"
        ));
        assert!(!should_use_colorization_for_command_supported(""));
    }
}
