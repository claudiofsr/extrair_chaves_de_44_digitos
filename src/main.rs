use extrair_chaves_de_44_digitos::{Arguments, get_efd_entries, get_map};

use rayon::prelude::*;
use std::{
    collections::BTreeSet,
    time::Instant,
};

/**
    cargo fmt
    cargo clippy
    clear && cargo test -- --show-output
    clear && cargo run -- -h
    cargo doc --open
    cargo b -r && cargo install --path=.
*/

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let time = Instant::now();
    let arguments = Arguments::build()?;
    let efd_entries = get_efd_entries(&arguments)?;

    // Processar arquivos coletados em paralelo
    // Utilizar o procedimento Map Reduce
    let chaves: BTreeSet<String> = efd_entries
        .par_iter() // rayon: parallel iterator
        .map(|entry| {
            // Processar arquivo individualmente
            get_map(entry).unwrap_or_default()           
        })
        .reduce(BTreeSet::new, |mut map_a, map_b| {
            map_a.extend(map_b);
            map_a
        });

    println!("{} chaves: {chaves:#?}", chaves.len());

    if arguments.time {
        eprintln!("\nTotal Execution Time: {:?}", time.elapsed());
    }

    Ok(())
}
