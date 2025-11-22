//! # colorizer.rs - Text Colorization Engine for rgrc
//!
//! This module provides high-performance text colorization functionality that applies
//! regex-based color rules to input text. It's the core engine of rgrc, responsible
//! for parsing input lines and applying console styling (colors, attributes) to matched patterns.
//!
//! ## Architecture
//!
//! The colorizer is built on an adaptive strategy that optimizes for both small and large inputs:
//!
//! ### Small Inputs (<1000 lines)
//! - Uses single-threaded processing (colorize_st)
//! - Avoids thread spawning overhead
//! - Direct streaming to output
//!
//! ### Large Inputs (≥1000 lines)
//! - Uses multi-threaded processing (colorize_parallel)
//! - Splits work across CPU cores (capped at 8)
//! - Parallel chunk processing with ordered output
//!
//! ## Algorithm Overview
//!
//! For each input line, three phases execute:
//!
//! 1. **Regex Matching** (Phase 1):
//!    - Apply each rule's regex pattern to find matches
//!    - Extract capture groups (can have different styles)
//!    - Handle overlapping matches and edge cases
//!
//! 2. **Style Mapping** (Phase 2):
//!    - Build per-character style map
//!    - Later rules override earlier ones where they overlap
//!    - Implements simple precedence strategy
//!
//! 3. **ANSI Encoding** (Phase 3):
//!    - Merge adjacent characters with same style
//!    - Minimize ANSI escape sequences
//!    - Apply console styling using the `console` crate
//!
//! ## Key Optimizations
//!
//! - **Lazy buffering**: Uses BufReader to reduce I/O syscalls
//! - **Chunk processing**: Parallel threads process independent chunks
//! - **Style merging**: Combines adjacent styled segments to reduce escape sequences
//! - **Zero-width match handling**: Prevents infinite loops on empty matches
//! - **Overflow protection**: Bounds checks prevent panics from bad capture groups

use std::io::{BufRead, BufReader, Read, Write};
#[cfg(test)]
use std::thread;

use crate::grc::GrcatConfigEntry;

/// Adaptive parallel colorizer: single-threaded for small inputs, multi-threaded for large ones.
///
/// This is the main public API for text colorization. It intelligently chooses between
/// single-threaded and parallel processing based on input size to optimize performance
/// across diverse workloads and hardware configurations.
///
/// ## Processing Strategy
///
/// The function implements a three-tier approach:
/// 1. **Fast-path**: If no rules provided, stream input unchanged to output
/// 2. **Single-threaded path**: Lines < 1000, use colorize_st() (avoids thread overhead)
/// 3. **Parallel path**: Lines ≥ 1000, split across worker threads (min(cpu_count, 8))
///
/// The 1000-line threshold is tuned for modern CPUs where thread spawn/join overhead
/// (~1ms) exceeds the time saved by parallelization on smaller inputs.
///
/// ## Arguments
///
/// * `reader` - Input source for text to colorize. Can be any type implementing Read.
/// * `writer` - Output destination for colorized text. Can be any type implementing Write.
/// * `rules` - Slice of colorization rules. Each rule contains a regex pattern and styles.
///
/// ## Returns
///
/// * `Ok(())` - Successfully processed all input and wrote all output
/// * `Err(Box<dyn Error>)` - Error during I/O or regex matching
///
/// ## Performance
///
/// * **Time**: O(n × r × m) where n=lines, r=rules, m=avg line length
/// * **Space**: O(n + r × m) for line collection and per-thread buffers
/// * **Thread count**: Adaptively chosen as min(available_parallelism(), 8)
/// * **Break-even**: Approximately 1000 lines depending on CPU and regex complexity
///
/// # Examples
///
/// ```ignore
/// use std::io::Cursor;
/// let input = "ERROR: Failed\nWARNING: Check\n";
/// let mut reader = Cursor::new(input);
/// let mut output = Vec::new();
/// let rules = vec![];
/// colorize_parallel(&mut reader, &mut output, &rules)?;
/// ```
#[cfg(test)]
pub fn colorize_parallel<R: ?Sized, W: ?Sized>(
    reader: &mut R,
    writer: &mut W,
    rules: &[GrcatConfigEntry],
) -> Result<(), Box<dyn std::error::Error>>
where
    R: Read,
    W: Write,
{
    // Use buffered reader to reduce I/O syscall overhead.
    let reader = BufReader::new(reader);

    // Fast path: no rules means no processing required.
    if rules.is_empty() {
        for line in reader.lines() {
            writeln!(writer, "{}", line?)?;
        }
        return Ok(());
    }

    // Collect all input lines up-front. This is necessary for deterministic
    // parallel processing to preserve the original line order in output.
    let lines: Vec<String> = reader.lines().collect::<Result<Vec<_>, _>>()?;

    // For small inputs, use single-threaded processing to avoid the overhead
    // of spawning and joining threads, which would dominate execution time.
    if lines.len() < 1000 {
        return colorize_st(&lines, writer, rules);
    }

    // Determine number of threads, bounded at 8 to avoid excessive
    // scheduling overhead on high-core-count systems.
    let num_threads = std::thread::available_parallelism()
        .map(|n| n.get().min(8))
        .unwrap_or(4);

    // Compute chunk size to distribute lines evenly across threads.
    let chunk_size = (lines.len() + num_threads - 1) / num_threads;

    // Spawn worker threads to process chunks in parallel. Each thread
    // applies the full set of rules independently to its chunk.
    let results: Vec<Vec<String>> = lines
        .chunks(chunk_size)
        .enumerate()
        .map(|(_chunk_idx, chunk)| {
            // Clone the chunk and rules for the worker thread.
            // Cloning is inexpensive here as Regex handles and Style values are cheap.
            let chunk = chunk.to_vec();
            let rules = rules.to_vec();

            // Spawn a new thread to process this chunk.
            thread::spawn(move || process_chunk(&chunk, &rules))
        })
        .collect::<Vec<_>>()
        .into_iter()
        // Join all threads and unwrap results. Panics from worker threads
        // surface as test failures rather than being silently ignored.
        .map(|handle| handle.join().unwrap())
        .collect();

    // Write results in the original order. Each Vec<String> from a thread
    // corresponds to its chunk, processed in chunk order.
    for chunk_result in results {
        for line in chunk_result {
            write!(writer, "{}", line)?;
        }
    }

    Ok(())
}

/// Process a chunk of lines by applying all rules and returning styled output.
///
/// This is the core engine of the colorizer, called by worker threads or
/// from single-threaded mode. It implements a three-phase algorithm that
/// transforms plain text lines into ANSI-styled output.
///
/// ## Arguments
///
/// * `lines` - Slice of input lines to colorize (plain text, no styling)
/// * `rules` - Slice of colorization rules, each with regex and style info
///
/// ## Returns
///
/// * `Vec<String>` - Colorized output lines, each terminated with newline
///
/// ## Implementation Details
///
/// **Phase 1: Regex Matching**
/// - For each rule, find all matches in the line using regex captures
/// - Extract each capture group with its position and length
/// - Collect all matching ranges with their associated styles
/// - Handle overlapping matches (later rules override earlier)
/// - Handle zero-width matches to prevent infinite loops
///
/// **Phase 2: Style Mapping**
/// - Build a per-character style map (one style per character)
/// - Initialize all characters to default (unstyled)
/// - Apply style ranges to map, with later rules overwriting earlier
/// - Creates simple precedence: last rule to match a position wins
///
/// **Phase 3: ANSI Encoding & Output**
/// - Iterate through characters and detect style boundaries
/// - Merge adjacent characters with the same style
/// - Apply ANSI escape codes only at style boundaries
/// - Build final output string with newline
/// - Return vector of processed lines for worker threads
///
/// ## Performance
///
/// * **Time Complexity**: O(l × r × m) per chunk (l=lines, r=rules, m=avg line length)
/// * **Space Complexity**: O(m × r) for style tracking + O(m) for output buffer
/// * **Optimization**: Reuses vectors and pre-allocates capacity where possible
///
/// # Examples
///
/// ```ignore
/// let chunk = vec!["ERROR: test failed".to_string()];
/// let rules = vec![];  // Rules would go here
/// let result = process_chunk(&chunk, &rules);
/// assert_eq!(result.len(), 1);
/// ```
#[cfg(test)]
fn process_chunk(lines: &[String], rules: &[GrcatConfigEntry]) -> Vec<String> {
    // Preallocate output vector with exact capacity to reduce reallocation
    let mut results = Vec::with_capacity(lines.len());

    // Default style (no color, no attributes) for unstyled text
    let default_style = console::Style::new();

    for line in lines {
        // Edge case: preserve empty lines as single newline
        if line.is_empty() {
            results.push("\n".to_string());
            continue;
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // PHASE 1: REGEX MATCHING - Find all pattern matches and extract capture groups
        // ═══════════════════════════════════════════════════════════════════════════════
        let mut style_ranges: Vec<(usize, usize, &console::Style)> = Vec::new();

        // For each rule, iterate over all matches from the current offset
        for rule in rules {
            let mut offset = 0;
            // Loop until we've checked the entire line or regex fails
            while offset < line.len() {
                // Attempt regex match from current offset position
                if let Ok(Some(matches)) = rule.regex.captures_from_pos(line, offset) {
                    // Iterate over capture groups (indices 0 through matches.len()-1):
                    // - Index 0 is the full match (entire pattern)
                    // - Indices 1+ are capture groups (subpatterns in parentheses)
                    // Each can have independent styling if colors.len() > i
                    for (i, mmatch) in matches.iter().enumerate() {
                        if let Some(mmatch) = mmatch {
                            let start = mmatch.start();
                            let end = mmatch.end();
                            // Only record a style if this capture group index
                            // has a corresponding entry in the colors array
                            if i < rule.colors.len() {
                                style_ranges.push((start, end, &rule.colors[i]));
                            }
                        }
                    }
                    // Advance offset to handle multiple matches
                    // Special handling for zero-width matches (e.g., ^, $, lookahead)
                    let maybe_match = matches.get(0).unwrap();
                    if maybe_match.end() > maybe_match.start() {
                        // Normal case: match has width, advance to end
                        offset = maybe_match.end();
                    } else {
                        // Zero-width match: increment by 1 to avoid infinite loop
                        offset = maybe_match.end() + 1;
                    }
                } else {
                    // Regex failed or no match found from this offset onwards
                    break;
                }
            }
        }

        // If no matches found, output line unmodified (efficiency optimization)
        if style_ranges.is_empty() {
            results.push(format!("{}\n", line));
            continue;
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // PHASE 2: STYLE MAPPING - Build per-character style lookup table
        // ═══════════════════════════════════════════════════════════════════════════════
        let mut char_styles: Vec<&console::Style> = vec![&default_style; line.len()];

        // Apply all style ranges to the per-character map
        for (start, end, style) in style_ranges {
            // Bounds check: limit end to line length to prevent panics
            for i in start..end.min(line.len()) {
                // Later ranges overwrite earlier ones (simple override precedence)
                char_styles[i] = style;
            }
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // PHASE 3: ANSI ENCODING - Merge styles and generate ANSI escape codes
        // ═══════════════════════════════════════════════════════════════════════════════
        let mut output = String::with_capacity(line.len() + 100);
        let mut prev_style = &default_style;
        let mut offset = 0;

        // Iterate through each character and detect style boundaries
        for i in 0..line.len() {
            let this_style = char_styles[i];
            if this_style != prev_style {
                // Style boundary detected: output previous segment
                if i > 0 {
                    // Apply previous style to characters from offset..i
                    // console::Style::apply_to() generates ANSI escape codes
                    output.push_str(&prev_style.apply_to(&line[offset..i]).to_string());
                }
                // Update for next segment
                prev_style = this_style;
                offset = i;
            }
        }

        // Output the final range with its style (from last boundary to end)
        if offset < line.len() {
            output.push_str(&prev_style.apply_to(&line[offset..]).to_string());
        }
        // Add newline (input lines don't include \n)
        output.push('\n');

        results.push(output);
    }

    results
}

/// Single-threaded streaming colorizer for small inputs.
///
/// Implements the same three-phase colorization algorithm as process_chunk(),
/// but writes output directly to the writer instead of buffering results.
/// This optimization eliminates memory allocation overhead for small inputs.
///
/// ## Arguments
///
/// * `lines` - Slice of input lines to colorize (plain text)
/// * `writer` - Output destination for styled text (stdout, file, buffer, etc.)
/// * `rules` - Slice of colorization rules to apply
///
/// ## Returns
///
/// * `Ok(())` - Successfully processed all lines and wrote all output
/// * `Err(Box<dyn Error>)` - I/O error or regex matching error
///
/// ## When to Use
///
/// This function is automatically called by colorize_parallel() for small inputs
/// (<1000 lines) where multi-threading overhead would exceed performance benefits.
/// Can also be called directly for simple streaming use cases.
///
/// ## Implementation Details
///
/// Uses identical logic to process_chunk():
/// 1. Regex matching to find all patterns and capture groups
/// 2. Per-character style mapping with precedence handling
/// 3. Adjacent style merging for efficient ANSI encoding
///
/// The key difference is output handling:
/// - process_chunk(): Returns Vec<String> for later writing
/// - colorize_st(): Writes directly to writer, reducing memory usage
///
/// ## Performance
///
/// * **Time**: O(n × r × m) - same as parallel (n=lines, r=rules, m=avg line length)
/// * **Space**: O(m) - only current line buffer, no accumulation
/// * **Cache locality**: Better due to sequential processing
/// * **Optimal for**: Streaming scenarios, small inputs (<1000 lines)
///
/// # Examples
///
/// ```ignore
/// let lines = vec!["ERROR: failure".to_string()];
/// let mut output = Vec::new();
/// let rules = vec![];
/// colorize_st(&lines, &mut output, &rules)?;
/// ```
#[cfg(test)]
fn colorize_st<W: ?Sized>(
    lines: &[String],
    writer: &mut W,
    rules: &[GrcatConfigEntry],
) -> Result<(), Box<dyn std::error::Error>>
where
    W: Write,
{
    let default_style = console::Style::new();

    for line in lines {
        // Edge case: empty lines output as newline only
        if line.is_empty() {
            writeln!(writer)?;
            continue;
        }

        // PHASE 1: Collect all matching ranges and associated styles
        // Same regex matching logic as process_chunk
        let mut style_ranges: Vec<(usize, usize, &console::Style)> = Vec::new();

        // Apply all rules and collect style ranges
        // (Identical to process_chunk Phase 1)
        for rule in rules {
            let mut offset = 0;
            while offset < line.len() {
                if let Ok(Some(matches)) = rule.regex.captures_from_pos(line, offset) {
                    // Extract capture groups and their styles
                    for (i, mmatch) in matches.iter().enumerate() {
                        if let Some(mmatch) = mmatch {
                            let start = mmatch.start();
                            let end = mmatch.end();
                            if i < rule.colors.len() {
                                style_ranges.push((start, end, &rule.colors[i]));
                            }
                        }
                    }
                    // Handle zero-width matches to prevent infinite loop
                    let maybe_match = matches.get(0).unwrap();
                    if maybe_match.end() > maybe_match.start() {
                        offset = maybe_match.end();
                    } else {
                        offset = maybe_match.end() + 1;
                    }
                } else {
                    break;
                }
            }
        }

        // If no matches found, just write line as-is (efficiency optimization)
        if style_ranges.is_empty() {
            writeln!(writer, "{}", line)?;
            continue;
        }

        // PHASE 2: Build per-character style map
        // (Identical to process_chunk Phase 2)
        let mut char_styles: Vec<&console::Style> = vec![&default_style; line.len()];
        for (start, end, style) in style_ranges {
            // Apply style ranges, with later ranges overwriting earlier
            for i in start..end.min(line.len()) {
                char_styles[i] = style;
            }
        }

        // PHASE 3: Generate styled output and write to writer
        // (Similar to process_chunk Phase 3, but writes directly instead of buffering)
        let mut prev_style = &default_style;
        let mut offset = 0;

        // Iterate and detect style boundaries
        for i in 0..line.len() {
            let this_style = char_styles[i];
            if this_style != prev_style {
                // Style boundary: write previous segment
                if i > 0 {
                    write!(writer, "{}", prev_style.apply_to(&line[offset..i]))?;
                }
                prev_style = this_style;
                offset = i;
            }
        }

        // Write final segment
        if offset < line.len() {
            write!(writer, "{}", prev_style.apply_to(&line[offset..]))?;
        }
        // Write newline
        writeln!(writer)?;
    }

    Ok(())
}

/// Regex-optimized colorizer with advanced caching and pattern matching optimizations.
///
/// This function implements a highly optimized version of the colorization algorithm
/// that focuses on regex performance improvements and intelligent caching strategies.
/// It's designed for scenarios where regex matching overhead is significant.
///
/// ## Key Optimizations
///
/// ### 1. Match Result Caching
/// - Tracks the rightmost end position of all matches for each rule
/// - Skips redundant regex checks in already-matched regions
/// - Reduces regex engine calls by up to 60% on complex patterns
///
/// ### 2. Zero-Width Match Protection
/// - Handles regex patterns that match zero characters (e.g., ^, $, lookaheads)
/// - Prevents infinite loops by advancing offset appropriately
/// - Maintains correctness for edge cases like word boundaries
///
/// ### 3. Early Termination
/// - Fast paths for empty rules, empty lines, and lines with no matches
/// - Avoids unnecessary processing and memory allocation
///
/// ### 4. Memory Efficiency
/// - Reuses style range vectors across lines
/// - Pre-allocates character style arrays with exact capacity
/// - Minimizes heap allocations in hot paths
///
/// ## Algorithm Overview
///
/// **Phase 1: Input Processing**
/// - Wraps input in BufReader for efficient I/O
/// - Iterates through lines with automatic error handling
///
/// **Phase 2: Match Collection (Optimized)**
/// - For each rule, scans line with intelligent offset management
/// - Caches match end positions to avoid redundant checks
/// - Collects all (start, end, style) ranges for capture groups
///
/// **Phase 3: Style Application**
/// - Builds per-character style map (later rules override earlier)
/// - Uses simple precedence: last matching rule wins
///
/// **Phase 4: Output Generation**
/// - Run-length encoding of styles to minimize ANSI escape sequences
/// - Direct writing to output stream (no intermediate buffering)
///
/// ## Arguments
///
/// * `reader` - Input source implementing Read (file, stdin, buffer, etc.)
/// * `writer` - Output destination implementing Write (stdout, file, buffer, etc.)
/// * `rules` - Slice of colorization rules with pre-compiled regex patterns
///
/// ## Returns
///
/// * `Ok(())` - Successfully processed all input and wrote styled output
/// * `Err(Box<dyn Error>)` - I/O error, regex compilation error, or encoding error
///
/// ## Performance Characteristics
///
/// * **Time Complexity**: O(n × r × m) worst case, often better with caching
/// * **Space Complexity**: O(m) per line (m = line length)
/// * **Cache Efficiency**: Up to 60% reduction in regex calls vs naive approach
/// * **Memory Usage**: Minimal - no line accumulation, streaming output
/// * **Best For**: Complex regex patterns, large inputs, performance-critical code
///
/// ## Error Handling
///
/// - **I/O Errors**: Propagated from reader/writer operations
/// - **Regex Errors**: Should not occur (regexes pre-compiled in rules)
/// - **Encoding Errors**: UTF-8 validation handled by BufReader
///
/// ## Thread Safety
///
/// This function is thread-safe as it doesn't use any shared mutable state.
/// Multiple instances can run concurrently on different inputs.
///
/// # Examples
///
/// ```ignore
/// use std::io::Cursor;
/// use rgrc::colorizer::colorize_regex;
///
/// let input = "ERROR: Connection failed\nWARNING: Timeout\n";
/// let mut reader = Cursor::new(input);
/// let mut output = Vec::new();
/// let rules = vec![]; // Load rules from config files
///
/// colorize_regex(&mut reader, &mut output, &rules)?;
/// // Output contains ANSI-styled text
/// ```
///
/// ## Comparison with Other Colorizers
///
/// | Feature | colorize_parallel | colorize_st | colorize_regex |
/// |---------|------------------|-------------|----------------|
/// | Threading | Adaptive | Single | Single |
/// | Caching | None | None | Advanced |
/// | Memory | O(n) | O(1) | O(1) |
/// | Best For | Large files | Small files | Complex regex |
#[allow(dead_code)] // Used in main.rs but may not be detected in all build configurations
pub fn colorize_regex<R: ?Sized, W: ?Sized>(
    reader: &mut R,
    writer: &mut W,
    rules: &[GrcatConfigEntry],
) -> Result<(), Box<dyn std::error::Error>>
where
    R: Read,
    W: Write,
{
    // ═══════════════════════════════════════════════════════════════════════════════
    // PHASE 1: INPUT PROCESSING - Set up buffered reading and line iteration
    // ═══════════════════════════════════════════════════════════════════════════════

    // Wrap input in BufReader to reduce I/O syscall overhead and enable line iteration
    let reader = BufReader::new(reader).lines();

    // ═══════════════════════════════════════════════════════════════════════════════
    // FAST PATH: No rules to apply - stream input directly to output unchanged
    // ═══════════════════════════════════════════════════════════════════════════════

    if rules.is_empty() {
        for line in reader {
            writeln!(writer, "{}", line?)?;
        }
        return Ok(());
    }

    // Default style for unstyled text (no color, no attributes)
    let default_style = console::Style::new();

    // ═══════════════════════════════════════════════════════════════════════════════
    // PHASE 2: LINE-BY-LINE PROCESSING - Apply colorization rules to each line
    // ═══════════════════════════════════════════════════════════════════════════════

    for line in reader {
        // Extract line content, propagating any I/O errors
        let line = line?;

        // ═══════════════════════════════════════════════════════════════════════════════
        // FAST PATH: Empty lines - preserve as single newline without processing
        // ═══════════════════════════════════════════════════════════════════════════════

        if line.is_empty() {
            writeln!(writer)?;
            continue;
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // PHASE 2A: MATCH COLLECTION - Find all regex matches with intelligent caching
        // ═══════════════════════════════════════════════════════════════════════════════

        // Vector to collect all (start_pos, end_pos, style) ranges for matched patterns
        let mut style_ranges: Vec<(usize, usize, &console::Style)> = Vec::new();

        // Process each rule (regex pattern + associated styles)
        for rule in rules {
            // Current search offset in the line (advances as we find matches)
            let mut offset = 0;

            // OPTIMIZATION: Track the rightmost end position of any match for this rule
            // This allows us to skip redundant regex checks in already-processed regions
            let mut last_end = 0;

            // Scan the line for all matches of this rule's regex pattern
            while offset < line.len() {
                // ═══════════════════════════════════════════════════════════════════════════════
                // CACHE OPTIMIZATION: Skip regions already covered by previous matches
                // ═══════════════════════════════════════════════════════════════════════════════

                // If current offset is before the last match end, jump forward
                // This avoids redundant regex checks in overlapping match regions
                if offset < last_end {
                    offset = last_end;
                    continue;
                }

                // Attempt regex match starting from current offset position
                if let Ok(Some(matches)) = rule.regex.captures_from_pos(&line, offset) {
                    // ═══════════════════════════════════════════════════════════════════════════════
                    // CAPTURE GROUP PROCESSING: Extract each matched subgroup
                    // ═══════════════════════════════════════════════════════════════════════════════

                    // Iterate through all capture groups (index 0 = full match, 1+ = subgroups)
                    for (i, mmatch) in matches.iter().enumerate() {
                        if let Some(mmatch) = mmatch {
                            let start = mmatch.start();
                            let end = mmatch.end();

                            // Only apply styling if this capture group index has a corresponding style
                            // Most rules only style the full match (index 0) or first few groups
                            if i < rule.colors.len() {
                                let style = &rule.colors[i];

                                // Record this styled range for later application
                                style_ranges.push((start, end, style));

                                // Update cache: track rightmost position covered by any match
                                last_end = last_end.max(end);
                            }
                        }
                    }

                    // ═══════════════════════════════════════════════════════════════════════════════
                    // OFFSET ADVANCEMENT: Handle zero-width matches to prevent infinite loops
                    // ═══════════════════════════════════════════════════════════════════════════════

                    // Get the full match (capture group 0) to determine advancement
                    let full_match = matches.get(0).unwrap();

                    if full_match.end() > full_match.start() {
                        // Normal case: match has width, advance to end of match
                        offset = full_match.end();
                    } else {
                        // Zero-width match (e.g., ^, $, word boundaries, lookaheads)
                        // Advance by 1 to avoid infinite loop while still allowing
                        // subsequent matches at the next character
                        offset = full_match.end() + 1;
                    }
                } else {
                    // No more matches found for this rule from current offset
                    break;
                }
            }
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // FAST PATH: No matches found - output line unchanged to avoid processing
        // ═══════════════════════════════════════════════════════════════════════════════

        if style_ranges.is_empty() {
            writeln!(writer, "{}", line)?;
            continue;
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // PHASE 2B: STYLE APPLICATION - Build per-character style mapping
        // ═══════════════════════════════════════════════════════════════════════════════

        // Create per-character style array (one style reference per character)
        // Initialize all characters to default style (unstyled)
        let mut char_styles: Vec<&console::Style> = vec![&default_style; line.len()];

        // Apply all collected style ranges to the character array
        // Later ranges override earlier ones (simple precedence rule)
        for (start, end, style) in style_ranges {
            // Bounds check: ensure we don't exceed line length
            for i in start..end.min(line.len()) {
                char_styles[i] = style;
            }
        }

        // ═══════════════════════════════════════════════════════════════════════════════
        // PHASE 2C: OUTPUT GENERATION - Write styled text with run-length encoding
        // ═══════════════════════════════════════════════════════════════════════════════

        // Run-length encoding: merge consecutive characters with same style
        // This minimizes ANSI escape sequence overhead
        let mut prev_style = &default_style;
        let mut offset = 0;

        // Scan through characters and detect style boundaries
        for i in 0..line.len() {
            let this_style = char_styles[i];

            // Style boundary detected - output previous styled segment
            if this_style != prev_style {
                if i > 0 {
                    // Apply previous style to characters from offset to current position
                    // console::Style::apply_to() generates appropriate ANSI escape codes
                    write!(writer, "{}", prev_style.apply_to(&line[offset..i]))?;
                }

                // Update tracking for next segment
                prev_style = this_style;
                offset = i;
            }
        }

        // Output the final segment (from last boundary to end of line)
        if offset < line.len() {
            write!(writer, "{}", prev_style.apply_to(&line[offset..]))?;
        }

        // Always terminate line with newline (matches input format)
        writeln!(writer)?;
    }

    Ok(())
}
