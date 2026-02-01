use std::collections::VecDeque;

/// These are the actions that the explorer can perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerAction {
    AskNeighbours,
    AskSupportedResources,
    AskSupportedCombinations,
    AskFreeCells,
    GenerateOrCombine,
    Move,
}

/// This function sets the action flow by putting in the correct order the explorer actions.
pub fn initialize_action_flow() -> VecDeque<ExplorerAction> {
    let mut res = VecDeque::new();
    res.push_back(ExplorerAction::AskNeighbours);
    res.push_back(ExplorerAction::AskSupportedResources);
    res.push_back(ExplorerAction::AskSupportedCombinations);
    res.push_back(ExplorerAction::AskFreeCells);
    res.push_back(ExplorerAction::GenerateOrCombine);
    res.push_back(ExplorerAction::Move);
    res
}

/// Struct that manages the action queue for the explorer.
pub struct ActionQueue {
    queue: VecDeque<ExplorerAction>,
}

impl ActionQueue {
    /// Creates a new ActionQueue with the default action flow.
    pub fn new() -> Self {
        Self {
            queue: initialize_action_flow(),
        }
    }

    /// Gets the next action from the queue.
    pub fn next_action(&mut self) -> Option<ExplorerAction> {
        self.queue.pop_front()
    }

    /// Pushes an action back to the end of the queue.
    pub fn push_back(&mut self, action: ExplorerAction) {
        self.queue.push_back(action);
    }

    /// Pushes an action to the front of the queue.
    pub fn push_front(&mut self, action: ExplorerAction) {
        self.queue.push_front(action);
    }

    /// Clears the action queue.
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// Resets the queue to the default action flow.
    pub fn reset(&mut self) {
        self.queue = initialize_action_flow();
    }

    /// Returns the number of actions in the queue.
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Checks if the queue is empty:
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl Default for ActionQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Struct that manages the moves that the explorer has to do.
/// It contains all the planet of the chosen path in order.
pub struct MoveQueue {
    move_queue: VecDeque<u32>,
}

impl MoveQueue {
    /// Creates a new empty MoveQueue.
    pub fn new() -> Self {
        MoveQueue {
            move_queue: VecDeque::new(),
        }
    }

    /// Gets the next move in the queue.
    pub fn next_move(&mut self) -> Option<u32> {
        self.move_queue.pop_front()
    }

    /// Push a move back to the end of the queue.
    pub fn push_back(&mut self, x: u32) {
        self.move_queue.push_back(x);
    }

    /// Replace the content of the queue with the given path.
    pub fn push_path(&mut self, path: VecDeque<u32>) {
        self.move_queue = path;
    }

    /// Checks if the queue is empty.
    pub fn is_empty(&self) -> bool {
        self.move_queue.is_empty()
    }

    /// Clears the queue.
    pub fn clear(&mut self) {
        self.move_queue.clear();
    }
}
