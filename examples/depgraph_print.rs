use colored::*;
use ratmole::{error::Error, DepGraph};
use std::fs::File;
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
    let depgraph = DepGraph::new(crate_path)?;
    println!("{}", depgraph);
    let crates = depgraph.crates();
    println!("\nIndividual crates:");
    for c in &crates {
        println!("    {}", c);
    }
    let dag = depgraph.dag();
    let dot_path = &args[2];
    let mut dot_file = File::create(dot_path)?;
    dag.dump_graphviz(&mut dot_file)?;
    Ok(())
}
