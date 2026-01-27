use std::sync::{LazyLock, RwLock};

// Viene inizializzato a "".to_string() AUTOMATICAMENTE al primo utilizzo
static SUNRAY_ASTEROID_SEQUENCE: LazyLock<RwLock<String>> =
    LazyLock::new(|| RwLock::new(String::new()));

pub fn get_sunray_asteroid_sequence() -> String {
    let seq = SUNRAY_ASTEROID_SEQUENCE.read().unwrap();
    seq.clone()
}

pub fn set_sunray_asteroid_sequence(new_sequence: String) {
    let mut seq = SUNRAY_ASTEROID_SEQUENCE.write().unwrap();
    *seq = new_sequence;
}

pub fn pop_sunray_asteroid_sequence() -> Option<char> {
    let mut seq = SUNRAY_ASTEROID_SEQUENCE.write().unwrap();
    seq.pop()
}

/// Global variable to store the Sunray spawn probability, default is 100 (100%)
static SUNRAY_SPAWN_PROBABILITY: LazyLock<RwLock<u32>> = LazyLock::new(|| RwLock::new(50));

/// Get the current Sunray spawn probability
pub fn get_sunray_probability() -> u32 {
    let prob = SUNRAY_SPAWN_PROBABILITY.read().unwrap();
    *prob
}

/// Set a new Sunray spawn probability (0-100)%
pub fn set_sunray_probability(new_probability: u32) {
    let mut prob = SUNRAY_SPAWN_PROBABILITY.write().unwrap();
    if new_probability > 100 {
        *prob = 100;
        return;
    }
    *prob = new_probability;
}

/// Determine if a Sunray should spawn based on the current probability
pub fn does_sunray_spawn() -> bool {
    let prob = get_sunray_probability();
    let random_value = rand::random::<u32>() % 100;
    random_value <= prob
}
