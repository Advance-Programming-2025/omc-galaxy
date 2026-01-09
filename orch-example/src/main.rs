use crossbeam_channel::{Receiver, Sender, unbounded};
use omc_galaxy::settings;
use omc_galaxy::{Game, run_with_ui};
use std::env;
use std::io;
use std::io::Write;
use std::{thread, time};
use ui_messages::{GameToUi, UiToGame};

//This main let us terminate in an elegant and simple way, returning the error message
fn main() -> Result<(), String> {
    // Load env
    dotenv::dotenv().ok();

    //Give the absolute path for the init file
    let file_path = env::var("INPUT_FILE")
        .expect("Imposta INPUT_FILE nel file .env o come variabile d'ambiente");

    // let sequence = "AAAAAAA".to_string();
    // settings::set_sunray_asteroid_sequence(sequence);
    settings::set_sunray_asteroid_sequence("AAAAAAASSS".to_string());
    let sequence = settings::pop_sunray_asteroid_sequence();
    println!("{:?}", sequence);

    println!("{}", settings::get_sunray_asteroid_sequence());

    let (sender_game_ui, receiver_ui_game) = unbounded();
    let (sender_ui_game, receiver_game_ui) = unbounded();

    // thread::spawn(|| -> Result<(), String> {
    //     run_with_ui(file_path, sender_game_ui, receiver_game_ui)
    // });

    let handle = thread::spawn(|| run_with_ui(file_path, sender_game_ui, receiver_game_ui));

    thread::sleep(time::Duration::from_millis(1000));

    loop {
        if handle.is_finished() {
            println!("Errore: Il thread di gioco è terminato inaspettatamente!");
            break;
        }
        println!("Enter command:");
        io::stdout().flush().unwrap();

        let mut command = String::new();
        io::stdin()
            .read_line(&mut command)
            .expect("failed to readline");
        match command.as_str().trim() {
            "start" => {
                sender_ui_game
                    .send(UiToGame::StartGame)
                    .map_err(|_| "Unable to send command to orch")?;
            }
            "stop" => {
                sender_ui_game
                    .send(UiToGame::StopGame)
                    .map_err(|_| "Unable to send command to orch")?;
            }
            "reset" => {
                sender_ui_game
                    .send(UiToGame::ResetGame)
                    .map_err(|_| "Unable to send command to orch")?;
            }
            "end" => {
                sender_ui_game
                    .send(UiToGame::EndGame)
                    .map_err(|_| "Unable to send command to orch")?;
            }
            _ => {
                println!("Invalid input")
            }
        }
    }

    Ok(())
}
