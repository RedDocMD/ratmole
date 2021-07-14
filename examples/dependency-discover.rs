use cargo::{core::SourceId, sources::SourceConfigMap, Config};
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
    let crates_io_id = SourceId::crates_io(&config)?;
    let config_map = SourceConfigMap::new(&config)?;
    let mut crates_io = config_map.load(crates_io_id, &Default::default())?;
    crates_io.update()?;

    let manifest = parse_cargo(crate_root, &config)?;
    let mut pkgs = Vec::new();
    for dep in manifest.dependencies() {
        println!(
            "{}",
            format!("Downloading {} ...", dep.name_in_toml()).yellow()
        );
        if dep.source_id() == crates_io_id {
            pkgs.push(download_dependency(dep, &mut crates_io, &config)?);
        } else {
            let config_map = SourceConfigMap::new(&config)?;
            let mut src = config_map.load(dep.source_id(), &Default::default())?;
            src.update()?;
            pkgs.push(download_dependency(dep, &mut src, &config)?);
        }
        println!(
            "{}",
            format!(" ... downloaded {}", dep.name_in_toml()).green()
        );
    }
    for pkg in &pkgs {
        println!(
            "{} {}",
            pkg.name(),
            pkg.root().as_os_str().to_str().unwrap()
        );
    }

    Ok(())
}
