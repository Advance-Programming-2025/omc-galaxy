use common_game::components::resource::{
    BasicResource, BasicResourceType, ComplexResource, ComplexResourceRequest, ComplexResourceType,
    GenericResource, ResourceType,
};
use crossbeam_channel::{Receiver, Sender, select};
use common_game::protocols::planet_explorer::{ExplorerToPlanet, PlanetToExplorer};
