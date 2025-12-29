// mod components;
// mod utils_planets;

use omc_galaxy::Orchestrator;
use std::env;

//This main let us terminate in an elegant and simple way, returning the error message
fn main() -> Result<(), String> {
    // Load env
    dotenv::dotenv().ok();

    //Give the absolute path for the init file
    let file_path = env::var("INPUT_FILE")
        .expect("Imposta INPUT_FILE nel file .env o come variabile d'ambiente");

    let sequence = "AAAASSS".to_string();
    Orchestrator::run(file_path, sequence)?;

    Ok(())
}
