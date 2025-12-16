mod components;

use std::io::{self, Write};

use components::Orchestrator;

//This main let us terminate in an elegant and simple way, returning the error message
fn main() -> Result<(), String> {
    //Init and check orchestrator
    let mut orchestrator = Orchestrator::new()?;
    // let init = orchestrator.initialize_galaxy()?;

    //Give the absolute path for the init file
    let mut path = String::new();
    print!("Enter something: ");
    io::stdout().flush().unwrap(); // ensure prompt prints
    io::stdin()
        .read_line(&mut path)
        .expect("Failed to read line");

    let _init = orchestrator.initialize_galaxy_by_file(path.as_str().trim())?;
    let _running_program = orchestrator.run_example()?;

    Ok(())
}
