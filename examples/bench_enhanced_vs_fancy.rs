// Performance benchmark: EnhancedRegex vs fancy-regex
// Tests patterns with lookahead/lookbehind

use std::time::Instant;
use fancy_regex::Regex as FancyRegex;
use rgrc::enhanced_regex::EnhancedRegex;

fn main() {
    // Test data: lines similar to ls -l output
    let test_lines = vec![
        "-rw-r--r--   1 user staff 344M Mar 22 22:51 MVI_8735.m4v",
        "-rw-r--r--   1 user staff 360050327 Mar 22 22:51 MVI_8735.m4v",
        "-rw-r--r--.  1 user staff 1.0G Nov 23 16:13 testg",
        "-rw-r--r--.  1 user staff 1.0K Nov 23 16:13 testk",
        "-rw-r--r--.  1 user staff 1.0M Nov 23 16:13 testm",
    ];
    
    // Pattern with lookahead (from conf.ls)
    let pattern = r"\s+(\d{7}|\d(?:[,.]?\d+)?[KM])(?=\s[A-Z][a-z]{2}\s)";
    
    println!("=== Performance Benchmark ===\n");
    println!("Pattern: {}", pattern);
    println!("Test lines: {}", test_lines.len());
    println!();
    
    // Benchmark EnhancedRegex
    let enhanced_re = EnhancedRegex::new(pattern).expect("Failed to compile EnhancedRegex");
    let iterations = 10000;
    
    let start = Instant::now();
    let mut enhanced_matches = 0;
    for _ in 0..iterations {
        for line in &test_lines {
            if enhanced_re.is_match(line) {
                enhanced_matches += 1;
            }
        }
    }
    let enhanced_duration = start.elapsed();
    
    println!("EnhancedRegex:");
    println!("  Total matches: {}", enhanced_matches);
    println!("  Time: {:?}", enhanced_duration);
    println!("  Per iteration: {:?}", enhanced_duration / iterations);
    println!();
    
    // Benchmark fancy-regex
    let fancy_re = FancyRegex::new(pattern).expect("Failed to compile fancy-regex");
    
    let start = Instant::now();
    let mut fancy_matches = 0;
    for _ in 0..iterations {
        for line in &test_lines {
            if fancy_re.is_match(line).unwrap() {
                fancy_matches += 1;
            }
        }
    }
    let fancy_duration = start.elapsed();
    
    println!("fancy-regex:");
    println!("  Total matches: {}", fancy_matches);
    println!("  Time: {:?}", fancy_duration);
    println!("  Per iteration: {:?}", fancy_duration / iterations);
    println!();
    
    // Comparison
    let speedup = fancy_duration.as_secs_f64() / enhanced_duration.as_secs_f64();
    println!("=== Comparison ===");
    println!("EnhancedRegex vs fancy-regex: {:.2}x", speedup);
    
    if speedup > 1.0 {
        println!("✓ EnhancedRegex is {:.1}% faster", (speedup - 1.0) * 100.0);
    } else {
        println!("⚠ EnhancedRegex is {:.1}% slower", (1.0 - speedup) * 100.0);
    }
}
