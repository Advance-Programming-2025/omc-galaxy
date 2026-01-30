mod components;
pub mod messages;
pub mod settings;
pub mod utils;

//Orchestrator example
pub use components::Game;
pub use components::game_loop::run_with_ui;

pub use utils::{ExplorerStatus, GalaxyTopology, PlanetInfoMap, PlanetStatus};

//Both GUIs
pub use components::orchestrator::Orchestrator;

//Bevy-GUI
pub use components::orchestrator::OrchestratorEvent;
pub use utils::GalaxySnapshot;
//Ratatui-GUI
pub use utils::{ExplorerStatusNotLock, PlanetStatusNotLock, Status};
