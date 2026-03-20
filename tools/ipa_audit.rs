use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{BufRead, BufReader};

include!("../stationapi/src/domain/ipa.rs");

struct Dataset {
    label: &'static str,
    path: &'static str,
    roman_column: usize,
}

#[derive(Default)]
struct TokenStats {
    count: usize,
    examples: BTreeSet<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let datasets = [
        Dataset {
            label: "lines",
            path: "data/2!lines.csv",
            roman_column: 5,
        },
        Dataset {
            label: "stations",
            path: "data/3!stations.csv",
            roman_column: 4,
        },
        Dataset {
            label: "train_types",
            path: "data/4!types.csv",
            roman_column: 4,
        },
    ];

    for dataset in datasets {
        audit_dataset(&dataset)?;
    }

    Ok(())
}

fn audit_dataset(dataset: &Dataset) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::open(dataset.path)?;
    let reader = BufReader::new(file);
    let mut total_names = 0usize;
    let mut unresolved_names = 0usize;
    let mut unresolved_tokens: BTreeMap<String, TokenStats> = BTreeMap::new();

    for (index, line) in reader.lines().enumerate() {
        let line = line?;
        if index == 0 {
            continue;
        }

        let columns = parse_csv_line(&line);
        let Some(name_roman) = columns.get(dataset.roman_column) else {
            continue;
        };
        let name_roman = name_roman.trim();
        if name_roman.is_empty() {
            continue;
        }

        total_names += 1;
        if romanized_name_to_ipa(name_roman).is_none() {
            unresolved_names += 1;
        }

        for token in extract_tokens(name_roman) {
            if word_to_ipa(&token).is_some() {
                continue;
            }
            let entry = unresolved_tokens.entry(token).or_default();
            entry.count += 1;
            if entry.examples.len() < 3 {
                entry.examples.insert(name_roman.to_string());
            }
        }
    }

    println!(
        "[{}] names: {} total / {} unresolved",
        dataset.label, total_names, unresolved_names
    );

    if unresolved_tokens.is_empty() {
        println!("[{}] unresolved tokens: none", dataset.label);
        println!();
        return Ok(());
    }

    println!("[{}] unresolved tokens:", dataset.label);
    let mut sorted_tokens: Vec<_> = unresolved_tokens.into_iter().collect();
    sorted_tokens.sort_by(|a, b| b.1.count.cmp(&a.1.count).then_with(|| a.0.cmp(&b.0)));

    for (token, stats) in sorted_tokens.into_iter().take(40) {
        let examples = stats.examples.into_iter().collect::<Vec<_>>().join(" / ");
        println!("  {} ({}) [{}]", token, stats.count, examples);
    }
    println!();

    Ok(())
}

fn extract_tokens(input: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();

    for c in input.chars() {
        if is_name_token_char(c) {
            current.push(c);
            continue;
        }

        if !current.is_empty() {
            let token = normalize_name_token(&current);
            if !token.is_empty() {
                tokens.push(token);
            }
            current.clear();
        }
    }

    if !current.is_empty() {
        let token = normalize_name_token(&current);
        if !token.is_empty() {
            tokens.push(token);
        }
    }

    tokens
}

fn parse_csv_line(line: &str) -> Vec<String> {
    let mut output = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = line.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            '"' => {
                if in_quotes && chars.peek() == Some(&'"') {
                    current.push('"');
                    chars.next();
                } else {
                    in_quotes = !in_quotes;
                }
            }
            ',' if !in_quotes => {
                output.push(current);
                current = String::new();
            }
            _ => current.push(c),
        }
    }

    output.push(current);
    output
}
