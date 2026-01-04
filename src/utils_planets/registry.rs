use common_game::components::resource::{BasicResourceType};
use once_cell::sync::Lazy;

use super::types::PlanetFactory;
use std::{collections::HashMap, time::Duration};
use crate::utils_planets::registry::PlanetType::{BlackAdidasShoe, Ciuc, HoustonWeHaveABorrow, ImmutableCosmicBorrow, OneMillionCrabs};
use rand::seq::{IndexedRandom};
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
    RustyCrab
}
impl PlanetType {
    pub fn random() -> Self {
        let mut rng = rand::rng();
        let variants: Vec<PlanetType> = PlanetType::iter().collect();
        *variants.choose(&mut rng).unwrap()
    }
}

pub static PLANET_REGISTRY: Lazy<HashMap<PlanetType, PlanetFactory>> = Lazy::new(|| {
    HashMap::from([
        
        //black_adidas_shoe: Type D
        (
            BlackAdidasShoe,
            Box::new(|rx_o, tx_o, rx_e, planet_id| {
                black_adidas_shoe::planet::create_planet(rx_o, tx_o, rx_e, planet_id)
            }) as PlanetFactory,
        ),
        //ciuc: Type A
        (
            Ciuc,
            Box::new(|rx_o, tx_p, rx_e, id| Ok(ciuc_planet::create_planet(rx_o, tx_p, rx_e, id)))
                as PlanetFactory,
        ),
        
        //houston_we_have_a_borrow: Type C
        (
            HoustonWeHaveABorrow,
            Box::new(|rx_o, tx_o, rx_e, planet_id| {
                let rocket = houston_we_have_a_borrow::RocketStrategy::Default;
                let basic_resource = BasicResourceType::Hydrogen;
                houston_we_have_a_borrow::houston_we_have_a_borrow(rx_o, tx_o, rx_e, planet_id, rocket, Some(basic_resource))
            }) as PlanetFactory,
        ),
        //immutable_cosmic_borrow: Type C
        (
            ImmutableCosmicBorrow,
            Box::new(|rx_o, tx_p, rx_e, id| {
                let duration = Duration::from_secs(10);
                let ai = immutable_cosmic_borrow::Ai::new(true, 0.0, 0.0, duration.clone() , duration.clone());
                immutable_cosmic_borrow::create_planet(ai, id, (rx_o, tx_p), rx_e)
            }) as PlanetFactory,
        ),
        // one_million_crabs: Type D
        (
            OneMillionCrabs,
            Box::new(|rx_o, tx_o, rx_e, planet_id| {
                one_million_crabs::planet::create_planet(rx_o, tx_o, rx_e, planet_id)
            }) as PlanetFactory,

        ),
        // TODO manca planet_id
        // (
        //     Rustrelli,
        //     Box::new(|rx_o, tx_o, rx_e, planet_id| {
        //         let request_limit=rustrelli::ExplorerRequestLimit::None;
        //         Ok(rustrelli::create_planet(rx_o, tx_o, rx_e, request_limit))
        //     }) as PlanetFactory,
        // ),
        // TODO manca planet_id
        // (
        //     RustyCrab,
        //     Box::new(|rx_o, tx_o, rx_e, planet_id| {
        //         Ok(rusty_crab::planet::create_planet(rx_o, tx_o, rx_e, BasicResourceType::Carbon))
        //     }) as PlanetFactory,
        // ),
    ])
});
