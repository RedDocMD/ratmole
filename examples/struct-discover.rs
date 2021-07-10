use std::env;

use ratmole::{error::Error, explore::structs_in_crate};

fn main() -> Result<(), Error> {
    let args: Vec<String> = env::args().collect();
    let crate_path = &args[1];
    let structs = structs_in_crate(crate_path)?;
    for st in &structs {
        println!("{}", st);
    }
    Ok(())
}
