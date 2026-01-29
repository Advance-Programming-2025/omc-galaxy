use crossbeam_channel::select;
use crossbeam_channel::{Receiver, Sender, select_biased, tick};
use std::time::Duration;

use crate::components::orchestrator::Orchestrator;
use crate::messages::{GameToUi, UiToGame};
use crate::settings;
use crate::utils::GameState;
use logging_utils::debug_println;

struct GameTick {
    ticker: Receiver<std::time::Instant>,
    start_time: std::time::Instant, //Used for debugging
}
impl GameTick {
    pub fn new(tick_duration: Duration) -> Self {
        Self {
            ticker: tick(tick_duration),
            start_time: std::time::Instant::now(),
        }
    }
}

/// Manages the game loop, timing, and state transitions
pub struct Game {
    state: GameState,
    orchestrator: Orchestrator,
    game_tick: GameTick,
    // UI communication
    receiver_game_ui: Receiver<UiToGame>,
    sender_game_ui: Sender<GameToUi>,
}

impl Game {
    pub fn new(
        orchestrator: Orchestrator,
        receiver_game_ui: Receiver<UiToGame>,
        sender_game_ui: Sender<GameToUi>,
    ) -> Self {
        Self {
            state: GameState::WaitingStart,
            game_tick: GameTick::new(Duration::from_millis(1000)),
            orchestrator,
            receiver_game_ui,
            sender_game_ui,
        }
    }

    fn handle_ui_command(&mut self, msg: UiToGame) -> Result<(), String> {
        // debug_println!("The game should start for the first time");
        match (self.state, msg) {
            (_, UiToGame::EndGame) => {
                debug_println!("The game should end now");
                self.orchestrator.send_planet_kill_to_all()?;
                return Err("The game is terminated".to_string());
                // self.orchestrator.stop_all()?;
                // self.notify_ui(GameToUi::GameEnded)?;
                // return Ok(true); // Exit loop
            }
            (GameState::WaitingStart, UiToGame::StartGame) => {
                debug_println!("The game should start for the first time");
                self.game_tick = GameTick::new(Duration::from_millis(1000));
                self.state = GameState::Running;

                //Send the update to UI
                let update = self.orchestrator.get_game_status()?;
                // let handle_err = self.sender_game_ui.send(GameToUi::GameStatusUpdate { galaxy_topology: update.0, planets_status: update.1, explorer_status: update.2 }).map_err(|_|"Unable to send messages to UI");

                // handle case error
                // if let Err(e) = handle_err {
                //     debug_println!("Error sending game status update to UI: {}", e);
                // }
                // self.notify_ui(GameToUi::GameStarted)?;
                self.orchestrator.start_all()?;
            }
            (GameState::Paused, UiToGame::StartGame) /*if state.can_start()*/ => {
                debug_println!("The game should start or restart");
                self.game_tick = GameTick::new(Duration::from_millis(1000));
                self.state = GameState::Running;
                // self.notify_ui(GameToUi::GameStarted)?;
                // self.orchestrator.start_all()?;
            }

            (GameState::Running, UiToGame::StopGame) /*if state.can_pause()*/ => {
                debug_println!("The game should stop");
                self.state = GameState::Paused;
                // self.orchestrator.stop_logic()?;
                // self.state = GameState::Paused;
                // self.notify_ui(GameToUi::GamePaused)?;
            }

            (_, UiToGame::ResetGame) => {
                debug_println!("The game should reset");
                // self.reset_game()?;
            }

            (_state, _msg) => {
                debug_println!("Invalid command {:?} in state {:?}", _msg, _state);
            }
        }

        Ok(())
    }

    fn asteroid_sunray_sender(&mut self) -> Result<(), String> {
        select! {
            recv(self.game_tick.ticker) -> _ => {
                debug_println!("{:?}", self.game_tick.start_time.elapsed());
                self.orchestrator.send_sunray_or_asteroid()?;
            }
            default => {
                // No tick yet
            }
        }
        Ok(())
    }
}

/// Entry point for running the game with UI
pub fn run_with_ui(
    file_path: String,
    sender_game_ui: Sender<GameToUi>,
    receiver_game_ui: Receiver<UiToGame>,
) -> Result<(), String> {
    // Initialize orchestrator
    let mut orchestrator = Orchestrator::new()?;

    orchestrator.initialize_galaxy_by_file(file_path.as_str().trim())?;

    // Create and run game loop
    let game_loop = Game::new(orchestrator, receiver_game_ui, sender_game_ui);

    game_loop.run()
}

/// Core game loop structure
impl Game {
    pub fn run(mut self) -> Result<(), String> {
        loop {
            match self.state {
                GameState::WaitingStart => self.waiting_loop()?,
                GameState::Running => self.running_loop()?,
                GameState::Paused => self.paused_loop()?,
            }
        }
    }

    /// Loop dedicato esclusivamente alla fase di attesa iniziale
    fn waiting_loop(&mut self) -> Result<(), String> {
        // Qui non facciamo calcoli di tempo, aspettiamo solo lo Start
        let msg = self
            .receiver_game_ui
            .recv()
            .map_err(|_| "UI Channel Error")?;

        self.handle_ui_command(msg)?;
        Ok(())
    }

    /// Loop ad alte prestazioni: gestione tick e orchestrator
    fn running_loop(&mut self) -> Result<(), String> {
        self.game_tick = GameTick::new(Duration::from_millis(1000));

        while self.state == GameState::Running {
            select_biased! {
                recv(self.receiver_game_ui) -> msg => {
                    let msg = msg.map_err(|_| "UI Error")?;
                    self.handle_ui_command(msg)?;
                }
                default => {

                    self.asteroid_sunray_sender()?;
                    self.orchestrator.handle_game_messages()?;

                    // Sleep ridotto per massima reattività
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
        }
        Ok(())
    }

    /// Loop di pausa: consuma solo messaggi UI, tempo fermo
    fn paused_loop(&mut self) -> Result<(), String> {
        debug_println!("Game is paused. Waiting for resume...");

        // Qui usiamo una recv() bloccante: non c'è bisogno di loopare a vuoto
        // perché il tempo di gioco è fermo.
        let msg = self.receiver_game_ui.recv().map_err(|_| "UI Error")?;
        self.handle_ui_command(msg)?;

        Ok(())
    }
}
