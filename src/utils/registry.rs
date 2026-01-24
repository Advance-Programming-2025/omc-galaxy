use common_game::components::resource::BasicResourceType;
use once_cell::sync::Lazy;

use super::types::PlanetFactory;
use crate::utils::registry::PlanetType::{
    BlackAdidasShoe, Ciuc, HoustonWeHaveABorrow, ImmutableCosmicBorrow, OneMillionCrabs, RustyCrab, Rustrelli,
};
use rand::seq::IndexedRandom;
use std::{collections::HashMap};
// Importiamo il trait per poter usare .iter()
use strum::IntoEnumIterator;
// Importiamo la macro per il derive
use strum_macros::EnumIter;

#[derive(Debug, EnumIter, Eq, PartialEq, Hash, Clone, Copy)]
pub enum PlanetType {
    BlackAdidasShoe,
    Ciuc,
    HoustonWeHaveABorrow,
    ImmutableCosmicBorrow,
    OneMillionCrabs,
    Rustrelli,
    RustyCrab,
}
impl PlanetType {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let variants: Vec<PlanetType> = PlanetType::iter().collect();
        *variants.choose(&mut rng).unwrap()
    }
}

pub static PLANET_REGISTRY: Lazy<HashMap<PlanetType, PlanetFactory>> = Lazy::new(|| {
    let mut map: HashMap<PlanetType, PlanetFactory> = HashMap::new();
    map.insert(PlanetType::BlackAdidasShoe, Box::new(|rx_o, tx_o, rx_e, planet_id| {
        // Need to be updated
        black_adidas_shoe::planet::create_planet(rx_o, tx_o, rx_e, planet_id)
    }));
    map.insert(PlanetType::Ciuc, Box::new(|rx_o, tx_p, rx_e, id| Ok(ciuc_planet::create_planet(rx_o, tx_p, rx_e, id))));

    map.insert(PlanetType::HoustonWeHaveABorrow, Box::new(|rx_o, tx_o, rx_e, planet_id| {
        let rocket = houston_we_have_a_borrow::RocketStrategy::Default;
        let basic_resource = BasicResourceType::Hydrogen;
        houston_we_have_a_borrow::houston_we_have_a_borrow(
            rx_o,
            tx_o,
            rx_e,
            planet_id,
            rocket,
            Some(basic_resource),
        )
    }));

    map.insert(PlanetType::ImmutableCosmicBorrow, Box::new(|rx_o, tx_o, rx_e, planet_id| {
        one_million_crabs::planet::create_planet(rx_o, tx_o, rx_e, planet_id)
    }));
    map.insert(PlanetType::OneMillionCrabs, Box::new(|rx_o, tx_o, rx_e, planet_id| {
        one_million_crabs::planet::create_planet(rx_o, tx_o, rx_e, planet_id)
    }));
    map.insert(PlanetType::Rustrelli, Box::new(|rx_o, tx_o, rx_e, planet_id| {
        let request_limit=rustrelli::ExplorerRequestLimit::None;
        Ok(rustrelli::create_planet(planet_id, rx_o, tx_o, rx_e, request_limit))
    }));
    map.insert(PlanetType::RustyCrab, Box::new(|rx_o, tx_o, rx_e, planet_id| {
        Ok(rusty_crab::planet::create_planet(rx_o, tx_o, rx_e, planet_id))
    }));    

    map
});
