fn main() {
    println!("cargo::rerun-if-changed=src/proxy/filters/");
    println!("cargo::rerun-if-changed=config/synapsec.toml");

    // Validate TOML filter files at compile time
    let filters_dir = std::path::Path::new("src/proxy/filters");
    if filters_dir.exists() {
        for entry in std::fs::read_dir(filters_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "toml") {
                let content = std::fs::read_to_string(&path).unwrap();
                content.parse::<toml::Table>().unwrap_or_else(|e| {
                    panic!("Invalid TOML in filter {:?}: {}", path, e);
                });
            }
        }
    }
}
