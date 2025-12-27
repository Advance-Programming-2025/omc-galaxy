use common_game::components::resource::{BasicResourceType};
use once_cell::sync::Lazy;

use super::types::PlanetFactory;
use std::{collections::HashMap, time::Duration};


pub static PLANET_REGISTRY: Lazy<HashMap<u8, PlanetFactory>> = Lazy::new(|| {
    HashMap::from([
        
        //Type D
        (
            0,
            Box::new(|rx_o, tx_o, rx_e, planet_id| {
                black_adidas_shoe::planet::create_planet(rx_o, tx_o, rx_e, planet_id)
            }) as PlanetFactory,
        ),
        //Type A
        (
            1,
            Box::new(|rx_o, tx_p, rx_e, id| Ok(ciuc_planet::create_planet(rx_o, tx_p, rx_e, id)))
                as PlanetFactory,
        ),
        
        // Type C planet
        (
            2,
            Box::new(|rx_o, tx_o, rx_e, planet_id| {
                let rocket = houston_we_have_a_borrow::RocketStrategy::Default;
                let basic_resource = BasicResourceType::Hydrogen;
                houston_we_have_a_borrow::houston_we_have_a_borrow(rx_o, tx_o, rx_e, planet_id, rocket, Some(basic_resource))
            }) as PlanetFactory,
        ),
        // Type C
        (
            3,
            Box::new(|rx_o, tx_p, rx_e, id| {
                let duration = Duration::from_secs(10);
                let ai = immutable_cosmic_borrow::Ai::new(true, 0.0, 0.0, duration.clone() , duration.clone());
                immutable_cosmic_borrow::create_planet(ai, id, (rx_o, tx_p), rx_e)
            }) as PlanetFactory,
        ),
        (
            4,
            Box::new(|rx_o, tx_o, rx_e, planet_id| {
                one_million_crabs::planet::create_planet(rx_o, tx_o, rx_e, planet_id)
            }) as PlanetFactory,

        ),
        // TODO manca planet_id
        // (
        //     5,
        //     Box::new(|rx_o, tx_o, rx_e, planet_id| {
        //         let request_limit=rustrelli::ExplorerRequestLimit::None;
        //         Ok(rustrelli::create_planet(rx_o, tx_o, rx_e, request_limit))
        //     }) as PlanetFactory,
        // ),
        // TODO manca planet_id
        // (
        //     6,
        //     Box::new(|rx_o, tx_o, rx_e, planet_id| {
        //         Ok(rusty_crab::planet::create_planet(rx_o, tx_o, rx_e, BasicResourceType::Carbon))
        //     }) as PlanetFactory,
        // ),
    ])
});
