quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: std::io::Error) {
            from()
            source(err)
            display("IO error: {}", err)
        }
        Utf8(msg: &'static str) {
            display("{}", msg)
        }
        Parse(err: syn::Error) {
            from()
            source(err)
            display("Parse error: {}", err)
        }
        Version(err: semver::Error) {
            from()
            source(err)
            display("Failed to parse version requirement: {}", err)
        }
        TomlDeserialize(err: toml::de::Error) {
            from()
            source(err)
            display("Failed to parse Cargo.toml: {}", err)
        }
        Anyhow(err: anyhow::Error) {
            from()
            display("{}", err)
        }
        PackageNotFound(name: String) {
            display("Package not found: {}", name)
        }
    }
}
