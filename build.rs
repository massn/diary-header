use std::fs;

fn main() {
    // Read version from package.json
    let version = fs::read_to_string("package.json")
        .ok()
        .and_then(|content| {
            // Simple parsing without serde_json
            content
                .lines()
                .find(|line| line.contains("\"version\""))
                .and_then(|line| line.split('"').nth(3).map(|v| v.to_string()))
        })
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());

    println!("cargo:rustc-env=GIT_VERSION={}", version);
    println!("cargo:rerun-if-changed=package.json");
}
