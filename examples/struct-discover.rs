use colored::*;
use ratmole::{error::Error, explore::crate_info};
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
    let crate_path = &args[1];
    let info = crate_info(crate_path)?;
    println!("{}", info);
    Ok(())
}
