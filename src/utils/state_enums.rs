#[derive(PartialEq, Debug)]
pub enum Status {
    Running,
    Paused,
    Dead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    WaitingStart,
    Running,
    Paused,
}
