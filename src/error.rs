quick_error! {
    #[derive(Debug)]
    pub enum Error {
        Io(err: std::io::Error) {
            from()
            source(err)
            display("IO error: {}", err)
        }
    }
}
