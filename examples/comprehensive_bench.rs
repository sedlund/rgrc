// Comprehensive benchmark comparing all three regex engines

use std::time::Instant;
use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use rgrc::enhanced_regex::EnhancedRegex;

fn benchmark_pattern(name: &str, pattern: &str, test_data: &[&str], iterations: usize) {
    println!("\n=== {} ===", name);
    println!("Pattern: {}", pattern);
    
    // Try Fast regex (if supported)
    if let Ok(fast_re) = Regex::new(pattern) {
        let start = Instant::now();
        let mut matches = 0;
        for _ in 0..iterations {
            for line in test_data {
                if fast_re.is_match(line) {
                    matches += 1;
                }
            }
        }
        let duration = start.elapsed();
        println!("Fast (regex):     {:>10.3}ms ({:>6} matches)", duration.as_secs_f64() * 1000.0, matches);
    }
    
    // Try EnhancedRegex
    if let Ok(enhanced_re) = EnhancedRegex::new(pattern) {
        let start = Instant::now();
        let mut matches = 0;
        for _ in 0..iterations {
            for line in test_data {
                if enhanced_re.is_match(line) {
                    matches += 1;
                }
            }
        }
        let duration = start.elapsed();
        println!("EnhancedRegex:    {:>10.3}ms ({:>6} matches)", duration.as_secs_f64() * 1000.0, matches);
    }
    
    // Fancy regex
    if let Ok(fancy_re) = FancyRegex::new(pattern) {
        let start = Instant::now();
        let mut matches = 0;
        for _ in 0..iterations {
            for line in test_data {
                if fancy_re.is_match(line).unwrap_or(false) {
                    matches += 1;
                }
            }
        }
        let duration = start.elapsed();
        println!("Fancy (fancy):    {:>10.3}ms ({:>6} matches)", duration.as_secs_f64() * 1000.0, matches);
    }
}

fn main() {
    println!("=== Regex Engine Performance Comparison ===");
    
    let iterations = 10000;
    
    // Test 1: Simple pattern (no lookaround)
    let simple_data = vec![
        "total 1234567",
        "drwxr-xr-x  5 user staff  160 Mar 22 22:51 bin",
        "-rw-r--r--  1 user staff 1024 Nov 23 16:13 file.txt",
    ];
    benchmark_pattern(
        "Simple Pattern (no lookaround)",
        r"\d+",
        &simple_data,
        iterations
    );
    
    // Test 2: Pattern with lookahead (from conf.ls)
    let ls_data = vec![
        "-rw-r--r--   1 user staff 344M Mar 22 22:51 MVI_8735.m4v",
        "-rw-r--r--   1 user staff 360050327 Mar 22 22:51 MVI_8735.m4v",
        "-rw-r--r--.  1 user staff 1.0G Nov 23 16:13 testg",
        "-rw-r--r--.  1 user staff 1.0K Nov 23 16:13 testk",
        "-rw-r--r--.  1 user staff 1.0M Nov 23 16:13 testm",
    ];
    benchmark_pattern(
        "Lookahead Pattern (conf.ls file size)",
        r"\s+(\d{7}|\d(?:[,.]?\d+)?[KM])(?=\s[A-Z][a-z]{2}\s)",
        &ls_data,
        iterations
    );
    
    // Test 3: Pattern with lookbehind (from conf.ps)
    let ps_data = vec![
        "user  1234 0.0 0.5 123456 78900 ?? S    10:30AM   0:01.23 /usr/bin/process",
        "root  5678 1.5 1.2 234567 12345 ?? R    11:45AM   1:23.45 kernel_task",
        "admin 9012 0.1 0.3  45678  5678 ?? S     9:15AM   0:00.12 system_process",
    ];
    benchmark_pattern(
        "Lookbehind Pattern (conf.ps options)",
        r"(?<=\s)-[\w\d]+(?=\s|$)",
        &ps_data,
        iterations
    );
    
    // Test 4: End boundary pattern
    let boundary_data = vec![
        "test 123 end",
        "test 456",
        "test 789 more",
        "final 999",
    ];
    benchmark_pattern(
        "Boundary Pattern (lookahead to end)",
        r"\d+(?=\s|$)",
        &boundary_data,
        iterations
    );
    
    println!("\n=== Summary ===");
    println!("- Fast regex: Fastest but no lookaround support");
    println!("- EnhancedRegex: Post-processing validation (slower but covers 80% cases)");
    println!("- fancy-regex: Full backtracking support (slower but handles all patterns)");
}
