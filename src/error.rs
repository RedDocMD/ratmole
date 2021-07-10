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
    }
}
