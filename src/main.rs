#[cfg(feature = "debug-prints")]
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => { println!($($arg)*) };
}

#[cfg(not(feature = "debug-prints"))]
#[macro_export]
macro_rules! debug_println {
    ($($arg:tt)*) => {
        ()
    };
}

mod components;
mod utils_planets;

use components::Orchestrator;
use std::env;

//This main let us terminate in an elegant and simple way, returning the error message
fn main() -> Result<(), String> {
    // Load env
    dotenv::dotenv().ok();
    //Init and check orchestrator
    let mut orchestrator = Orchestrator::new()?;

    //Give the absolute path for the init file
    let file_path = env::var("INPUT_FILE")
        .expect("Imposta INPUT_FILE nel file .env o come variabile d'ambiente");

    orchestrator.initialize_galaxy_by_file(file_path.as_str().trim())?;
    // orchestrator.run_only_planets()?;
    let sequence = "AAAASSS".to_string();
    orchestrator.run_only_planet_sequence(sequence)?;

    Ok(())
}
