use claudiofsr_lib::BTreeSetExtension;
use std::{collections::BTreeSet, process, time::Instant};

use extrair_chaves_de_44_digitos::{
    get_efd_entries, process_all_efd_files_parallel, Arguments, MyResult,
};

/*
    cargo fmt
    cargo clippy
    clear && cargo test -- --show-output
    clear && cargo run -- -h
    cargo doc --open
    cargo b -r && cargo install --path=.
*/

fn main() {
    // Call the separate function that contains the main logic and can return Result
    let run_result = run();

    // Now handle the result returned by the 'run' function
    match run_result {
        Ok(_) => {
            process::exit(0); // Explicitly exit with success code
        }
        Err(error) => {
            eprintln!("Operation failed:");
            eprintln!("Error: {}", error); // Using Display prints the #[error] message
            process::exit(1); // Explicitly exit with failure code
        }
    }
}

/// Contains the core logic of the application.
/// It parses arguments, finds files, processes them in parallel,
/// writes the results, and handles verbose output/timing.
///
/// # Returns
/// A `MyResult` indicating success (`Ok(())`) or an error (`Err(MyError)`).
fn run() -> MyResult<()> {
    let time = Instant::now(); // Record start time for performance measurement
    let arguments = Arguments::build()?; // Parse command-line arguments, propagating errors
    let efd_entries = get_efd_entries(&arguments)?; // Get a list of EFD files, propagating errors

    // Process all EFD files in parallel to extract unique 44-digit keys.
    // This leverages Rayon for efficiency and collects results into a single BTreeSet.
    let chaves: BTreeSet<String> = process_all_efd_files_parallel(&efd_entries)?;

    let output_filename = "efd-chaves_eletronicas.txt"; // Define the output file name

    // Write the collected keys to the specified file.
    chaves.write_to_file(output_filename)?;

    // Print collected keys if verbose mode is enabled.
    if arguments.verbose && !chaves.is_empty() {
        println!("{} chaves: {chaves:#?}", chaves.len());
    }

    // Print total execution time if time tracking is enabled.
    if arguments.time {
        eprintln!("\nTotal Execution Time: {:?}", time.elapsed());
    }

    Ok(()) // Indicate successful execution
}
