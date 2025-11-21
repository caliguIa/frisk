use crate::core::element::Element;
use crate::core::error::Result;
use std::io::Cursor;
use std::time::Duration;

fn download_wordnet_data() -> Result<Vec<(String, String)>> {
    eprintln!("[dictionary daemon] Downloading WordNet 2024 WNDB files...");

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .user_agent("frisk/0.1.0")
        .build()?;

    let url = "https://en-word.net/static/english-wordnet-2024.zip";
    let response = client.get(url).send()?;

    if !response.status().is_success() {
        return Err(crate::core::error::Error::new(format!(
            "Failed to download WordNet data: {}",
            response.status()
        )));
    }

    let bytes = response.bytes()?;
    let cursor = Cursor::new(bytes);

    eprintln!("[dictionary daemon] Extracting WordNet data files...");

    let mut archive = zip::ZipArchive::new(cursor)?;

    let mut files = Vec::new();

    for file_name in &["data.noun", "data.verb", "data.adj", "data.adv"] {
        eprintln!("[dictionary daemon] Extracting {}...", file_name);

        let mut found = false;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            if file.name().ends_with(file_name) {
                let mut contents = String::new();
                std::io::Read::read_to_string(&mut file, &mut contents)?;
                files.push((file_name.to_string(), contents));
                found = true;
                break;
            }
        }

        if !found {
            eprintln!(
                "[dictionary daemon] Warning: {} not found in archive",
                file_name
            );
        }
    }

    Ok(files)
}

/// Parse WordNet data file format
/// Format: Each synset (synonym set) has a gloss (definition)
/// Example line: 00001740 03 n 01 entity 0 003 ~ 00001930 n ...  | that which is perceived ...
fn parse_wordnet_data(content: &str, pos_name: &str) -> Vec<(String, String)> {
    let mut entries = Vec::new();

    for line in content.lines() {
        if line.starts_with("  ") || line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split(" | ").collect();
        if parts.len() < 2 {
            continue;
        }

        let synset_data = parts[0];
        let gloss = parts[1];

        let fields: Vec<&str> = synset_data.split_whitespace().collect();
        if fields.len() < 5 {
            continue;
        }

        // fields[3] is word_count (in hex)
        let word_count = match u32::from_str_radix(fields[3], 16) {
            Ok(n) => n as usize,
            Err(_) => continue,
        };

        let mut words = Vec::new();
        let mut idx = 4;
        for _ in 0..word_count {
            if idx < fields.len() {
                let word = fields[idx].replace('_', " ");
                words.push(word);
                idx += 2; // Skip lex_id
            }
        }

        let definition = gloss.split(';').next().unwrap_or(gloss).trim().to_string();

        for word in words {
            let key = format!("{} ({})", word, pos_name);
            entries.push((key, definition.clone()));
        }
    }

    entries
}

fn fetch_dictionary() -> Result<Vec<Element>> {
    let files = download_wordnet_data()?;

    let mut all_entries = Vec::new();

    for (file_name, content) in files {
        let pos_name = if file_name.contains("noun") {
            "noun"
        } else if file_name.contains("verb") {
            "verb"
        } else if file_name.contains("adj") {
            "adj"
        } else if file_name.contains("adv") {
            "adv"
        } else {
            continue;
        };

        eprintln!("[dictionary daemon] Parsing {}s...", pos_name);
        let entries = parse_wordnet_data(&content, pos_name);
        eprintln!(
            "[dictionary daemon] Parsed {} {} entries",
            entries.len(),
            pos_name
        );
        all_entries.extend(entries);
    }

    let mut seen = std::collections::HashSet::new();
    let mut elements = Vec::new();

    for (word_pos, definition) in all_entries {
        if seen.insert(word_pos.clone()) {
            let display_name = format!("{} - {}", word_pos, definition);
            elements.push(Element::new_dictionary(display_name, definition));
        }
    }

    eprintln!(
        "[dictionary daemon] Created {} unique dictionary entries",
        elements.len()
    );
    Ok(elements)
}

fn save_dictionary(elements: &[Element]) -> Result<()> {
    let cache_path = crate::cache::cache_dir()?.join("dictionary.bin");
    let vec = elements.to_vec();
    crate::cache::save_cache("dictionary.bin", &vec)?;
    eprintln!(
        "[dictionary daemon] Saved {} entries to {:?}",
        elements.len(),
        cache_path
    );
    Ok(())
}

pub fn run() -> Result<()> {
    eprintln!("[dictionary daemon] Starting...");

    match fetch_dictionary() {
        Ok(entries) => {
            save_dictionary(&entries)?;
            eprintln!("[dictionary daemon] Complete");
        }
        Err(e) => {
            eprintln!("[dictionary daemon] Failed to fetch: {}", e);
            return Err(e);
        }
    }

    Ok(())
}
