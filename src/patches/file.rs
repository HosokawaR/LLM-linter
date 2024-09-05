use std::fs::read_to_string;

use crate::core::Patch;

pub fn from_file(path: &str) -> Patch {
    let content = read_to_string(path).unwrap();
    let re = regex::Regex::new(r"\+\+\+ b/(.*)").unwrap();

    Patch {
        path: re
            .captures(&content)
            .unwrap_or_else(|| panic!("Path not found in patch file"))
            .get(1)
            .unwrap_or_else(|| panic!("Failed to extract path"))
            .as_str()
            .to_string(),
        content,
        // TODO
        start_line: 0,
        end_line: 0,
    }
}
