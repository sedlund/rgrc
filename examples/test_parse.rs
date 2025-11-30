use rgrc::enhanced_regex::EnhancedRegex;

fn main() {
    let pattern = r"(?<!\s)\d+";
    println!("Original pattern: {}", pattern);
    
    match EnhancedRegex::new(pattern) {
        Ok(re) => {
            println!("Successfully parsed!");
            println!("Testing ' 123':");
            let result = re.is_match(" 123");
            println!("  is_match result: {}", result);
            
            if let Some(m) = re.find_from_pos(" 123", 0) {
                println!("  Found match: {:?} at {}..{}", m.as_str(), m.start(), m.end());
            } else {
                println!("  No match found");
            }
        }
        Err(e) => {
            println!("Failed to parse: {:?}", e);
        }
    }
}
