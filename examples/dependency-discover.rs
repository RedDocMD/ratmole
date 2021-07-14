use cargo::{sources::SourceConfigMap, Config};
use colored::*;
use ratmole::{
    cargo::{download_dependency, parse_cargo},
    error::Error,
};
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

    let config = Config::default()?;
    let _lock = config.acquire_package_cache_lock()?;

    let manifest = parse_cargo(crate_root, &config)?;

    let dep = &manifest.dependencies()[0];
    let map = SourceConfigMap::new(&config)?;
    let mut src = map.load(dep.source_id(), &Default::default())?;

    src.update()?;
    let pkg = download_dependency(dep, &mut src, &config)?;
    println!("{}", pkg.root().as_os_str().to_str().unwrap());
    Ok(())
}
