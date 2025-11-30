use regex::Regex;

fn main() {
    let text = " 123";
    let re = Regex::new(r"\s").unwrap();
    
    // Check text before position 1 (the start of "123")
    let prefix = &text[..1];
    println!("Text: {:?}", text);
    println!("Prefix (text[..1]): {:?}", prefix);
    println!("Regex \\s matches prefix: {}", re.is_match(prefix));
    
    if let Some(last_match) = re.find_iter(prefix).last() {
        println!("Last match in prefix: start={}, end={}", last_match.start(), last_match.end());
        println!("Match ends at position 1: {}", last_match.end() == 1);
    } else {
        println!("No match found in prefix");
    }
    
    println!("\nFor negative lookbehind (?<!\\s):");
    println!("Should NOT match because space ends at position 1");
}
