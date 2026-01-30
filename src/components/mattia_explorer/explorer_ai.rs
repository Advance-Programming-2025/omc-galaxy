use common_game::components::resource::{BasicResource, BasicResourceType, ComplexResourceType};
use common_game::utils::ID;

pub enum AIAction{
    ProduceResource(BasicResourceType), //not sure if this will be useful, because I think it is useless to waste energy cell in making resources
    CombineResource(ComplexResourceType),
    MoveTo(ID),
    SurveyPlanet,
    Wait,
}
pub struct topology_data{
    global_sunray_rate: f32,
    global_asteroid_rate: f32,
}

pub fn calc_produce_resource_utility()->f32{
    todo!()
}