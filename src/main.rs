use claudiofsr_lib::BTreeSetExtension;
use rayon::prelude::*;
use std::{collections::BTreeSet, error::Error, time::Instant};

use extrair_chaves_de_44_digitos::{get_efd_entries, get_map, Arguments};

/*
    cargo fmt
    cargo clippy
    clear && cargo test -- --show-output
    clear && cargo run -- -h
    cargo doc --open
    cargo b -r && cargo install --path=.
*/

fn main() -> Result<(), Box<dyn Error>> {
    let time = Instant::now();
    let arguments = Arguments::build()?;
    let efd_entries = get_efd_entries(&arguments)?;

    // Processar em paralelo arquivos coletados
    let chaves: BTreeSet<String> = efd_entries
        .par_iter() // rayon: parallel iterator
        .flat_map(get_map) // Processar arquivo individualmente
        .flatten()
        .collect(); // Collect into a BTreeSet

    let output = "efd-chaves_eletronicas.txt";
    chaves.write_to_file(output)?;

    if arguments.verbose {
        println!("{} chaves: {chaves:#?}", chaves.len());
    }

    if arguments.time {
        eprintln!("\nTotal Execution Time: {:?}", time.elapsed());
    }

    Ok(())
}
