use extrair_chaves_de_44_digitos::{*, Arguments};

use claudiofsr_lib::{BytesExtension, open_file};
use std::{
    collections::BTreeSet, 
    io::{BufRead, BufReader}, 
    time::Instant
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let time = Instant::now();
    let arguments = Arguments::build()?;
    let efd_entries = get_efd_entries(&arguments)?;

    let mut chaves: BTreeSet<String> = BTreeSet::new();

    // Processa os arquivos que foram coletados
    for entry in efd_entries {
        let path = entry.path();
        let file = open_file(path)?;
        let buffer = BufReader::new(file);

        chaves = buffer
            .split(NEWLINE_BYTE)
            .flatten()
            .enumerate()
            .map(|(line_number, vec_bytes)| {
                get_string_utf8(vec_bytes.trim(), line_number + 1, path)
            })
            .map(split_line)
            .filter(|campos| campos.len() >= 2)
            .take_while(|campos| campos[0] != "9999")
            .flat_map(|campos| {
                let mut chaves: Vec<String> = Vec::new();
                for campo in campos {
                    for cap in REGEX_CHAVE44.captures_iter(&campo) {
                        let chave = cap[0].to_string();
                        chaves.push(chave);
                    }
                }
                chaves
            })
            .collect();
    }

    println!("{} chaves: {chaves:#?}", chaves.len());

    if arguments.time {
        eprintln!("\nTotal Execution Time: {:?}", time.elapsed());
    }

    Ok(())
}
