use colored::*;
use ratmole::{cargo::crate_dependencies, error::Error};
use std::{env, io::Write};

fn main() -> Result<(), Error> {
    env_logger::builder()
        .format(|buf, rec| {
            let line = rec
                .line()
                .map_or(String::new(), |line| format!(":{}", line));
            let file = rec
                .file()
                .map_or(String::new(), |file| format!(" {}", file));
            let prelude = format!("[{}{}{}]", rec.level(), file, line);
            writeln!(buf, "{} {}", prelude.cyan(), rec.args())
        })
        .write_style(env_logger::WriteStyle::Always)
        .init();
    let args: Vec<String> = env::args().collect();
    let crate_root = &args[1];
    let deps = crate_dependencies(crate_root)?;
    for dep in &deps {
        println!("{}", dep);
    }
    Ok(())
}
