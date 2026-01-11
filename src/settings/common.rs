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
