use std::path::Path;

pub fn is_coconut_file(path: &str) -> bool {
    let path = Path::new(path);

    match path.extension() {
        Some(ext) => ext == "coconut",
        None => false,
    }
}

pub fn read_coconut_file(path: &str) -> Result<String, String> {
    if !is_coconut_file(path) {
        return Err(format!("Error: '{}' is not a .coconut file", path));
    }
    std::fs::read_to_string(path).map_err(|e| format!("Error reading file '{}': {}", path, e))
}
