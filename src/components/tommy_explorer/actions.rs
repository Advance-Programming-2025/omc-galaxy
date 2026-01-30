use std::collections::VecDeque;

/// these are the actions that the explorer can perform
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExplorerAction {
    AskNeighbours,
    AskSupportedResources,
    AskSupportedCombinations,
    GenerateOrCombine,
    Move,
}

/// this function sets the action flow by putting in the correct order the explorer actions
pub fn initialize_action_flow() -> VecDeque<ExplorerAction> {
    let mut res = VecDeque::new();
    res.push_back(ExplorerAction::Move);
    res.push_back(ExplorerAction::GenerateOrCombine);
    res.push_back(ExplorerAction::AskSupportedCombinations);
    res.push_back(ExplorerAction::AskSupportedResources);
    res.push_back(ExplorerAction::AskNeighbours);
    res
}

/// struct that manages the action queue for the explorer
pub struct ActionQueue {
    queue: VecDeque<ExplorerAction>,
}

impl ActionQueue {
    /// creates a new ActionQueue with the default action flow
    pub fn new() -> Self {
        Self {
            queue: initialize_action_flow(),
        }
    }

    /// gets the next action from the queue
    pub fn next_action(&mut self) -> Option<ExplorerAction> {
        self.queue.pop_front()
    }

    /// pushes an action back to the end of the queue
    pub fn push_back(&mut self, action: ExplorerAction) {
        self.queue.push_back(action);
    }

    /// rotates the action (pop front, push back)
    pub fn rotate_action(&mut self) -> Option<ExplorerAction> {
        if let Some(action) = self.queue.pop_front() {
            self.queue.push_back(action);
            Some(action)
        } else {
            None
        }
    }

    /// clears the action queue
    pub fn clear(&mut self) {
        self.queue.clear();
    }

    /// resets the queue to the default action flow
    pub fn reset(&mut self) {
        self.queue = initialize_action_flow();
    }

    /// returns the number of actions in the queue
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// checks if the queue is empty
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl Default for ActionQueue {
    fn default() -> Self {
        Self::new()
    }
}
