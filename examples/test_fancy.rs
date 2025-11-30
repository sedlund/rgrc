use fancy_regex::Regex;

fn main() {
    let pattern = r"(?<!\s)\d+";
    let re = Regex::new(pattern).unwrap();
    
    println!("Pattern: {}", pattern);
    println!("\nTest: '123'");
    println!("  is_match: {}", re.is_match("123").unwrap());
    
    println!("\nTest: 'a123'");
    println!("  is_match: {}", re.is_match("a123").unwrap());
    
    println!("\nTest: ' 123'");
    println!("  is_match: {}", re.is_match(" 123").unwrap());
    if let Some(m) = re.find(" 123").unwrap() {
        println!("  Found: {:?} at {}..{}", m.as_str(), m.start(), m.end());
    }
}
