use crossbeam_channel::select;
use crossbeam_channel::{Receiver, Sender, select_biased, tick};
use std::time::Duration;
use ui_messages::{GameToUi, UiToGame};

use crate::components::orchestrator::Orchestrator;
use crate::debug_println;
use crate::settings;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    WaitingStart,
    Running,
    Paused,
}

impl GameState {
    pub fn can_start(&self) -> bool {
        matches!(self, GameState::Paused)
    }

    pub fn can_pause(&self) -> bool {
        matches!(self, GameState::Running)
    }

    pub fn is_running(&self) -> bool {
        matches!(self, GameState::Running)
    }
}

struct GameTick {
    ticker: Receiver<std::time::Instant>,
    start_time: std::time::Instant,
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
    clock: GameTick,
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
            clock: GameTick::new(Duration::from_millis(1000)),
            orchestrator,
            receiver_game_ui,
            sender_game_ui,
        }
    }

    /// Main game loop
    pub fn run(mut self) -> Result<(), String> {
        /*We want this priority of the messages:
            1. UI messages: they are specific messages to control the game so they have to come first
            2. Game messages: for the other messages is preferred a fair selection in that way no object can block the game
        */
        loop {
            select_biased! {
                // Priority 1: UI commands
                recv(self.receiver_game_ui) -> msg => {
                    let msg = msg.map_err(|_| "Cannot receive UI message")?;
                    self.handle_ui_command(msg)?;
                }

                // Priority 2: Process game
                default => {
                    if self.state.is_running() {
                        // If ticker is ready sent asteroids/sunrays
                        self.game_tick()?;
                        self.orchestrator.handle_game_messages()?;
                    }
                    //REVIEW Avoid busy-waiting
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }

        // Ok(())
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
                self.clock = GameTick::new(Duration::from_millis(1000));
                self.state = GameState::Running;
                // self.notify_ui(GameToUi::GameStarted)?;
                self.orchestrator.start_all()?;
            }
            (GameState::Paused, UiToGame::StartGame) /*if state.can_start()*/ => {
                debug_println!("The game should start or restart");
                self.clock = GameTick::new(Duration::from_millis(1000));
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

            (state, msg) => {
                debug_println!("Invalid command {:?} in state {:?}", msg, state);
            }
        }

        Ok(())
    }

    fn game_tick(&mut self) -> Result<(), String> {
        select! {
            recv(self.clock.ticker) -> _ => {
                debug_println!("{:?}", self.clock.start_time.elapsed());
                self.process_game_events()?;
            }
            default => {
                // No tick yet
            }
        }
        Ok(())
    }

    fn process_game_events(&mut self) -> Result<(), String> {
        // debug_println!("{:?}", self.ticker);
        match settings::pop_sunray_asteroid_sequence() {
            Some('S') => {
                self.orchestrator.send_sunray_to_all()?;
            }
            Some('A') => {
                self.orchestrator.send_asteroid_to_all()?;
            }
            Some('$') => {
                // End of sequence
                // self.orchestrator.stop_logic()?;
                self.state = GameState::Paused;
                // self.notify_ui(GameToUi::SequenceComplete)?;
            }
            msg => {
                // Probability mode
                println!("{:?}", msg);
                self.orchestrator.send_sunray_to_all()?;
            }
        }
        Ok(())
    }

    fn reset_game(&mut self) -> Result<(), String> {
        // self.orchestrator.stop_all()?;

        // TODO: Re-initialize from file
        // self.orchestrator.initialize_galaxy_by_file(&file_path)?;

        // self.state = GameState::WaitingToStart;
        // self.notify_ui(OrchestratorToUi::GameReset)?;
        Ok(())
    }

    fn notify_ui(&self, msg: GameToUi) -> Result<(), String> {
        // self.sender_to_ui
        //     .send(msg)
        //     .map_err(|_| "Failed to send to UI".to_string())
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

// Placeholder - replace with your actual function
fn pop_sunray_asteroid_sequence() -> Option<char> {
    None
}
