fn main() {
    println!("cargo::rerun-if-changed=src/proxy/builtin_filters.toml");
    println!("cargo::rerun-if-changed=.opencode/");
}
