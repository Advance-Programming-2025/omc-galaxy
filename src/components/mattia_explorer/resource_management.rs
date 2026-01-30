use common_game::components::resource::{BasicResource, ComplexResource, GenericResource};

pub trait ToGeneric{
    fn res_to_generic(self) -> GenericResource;
}
impl ToGeneric for BasicResource {
    fn res_to_generic(self) -> GenericResource {
        match self {
            BasicResource::Oxygen(oxygen) => { oxygen.to_generic() }
            BasicResource::Hydrogen(hydrogen) => { hydrogen.to_generic() }
            BasicResource::Carbon(carbon) => { carbon.to_generic() }
            BasicResource::Silicon(silicon) => { silicon.to_generic()}
        }
    }
}
impl ToGeneric for ComplexResource {
    fn res_to_generic(self) -> GenericResource {
        match self {
            ComplexResource::Diamond(diamond) => { diamond.to_generic() }
            ComplexResource::Water(water) => { water.to_generic() }
            ComplexResource::Life(life) => { life.to_generic() }
            ComplexResource::Robot(robot) => { robot.to_generic() }
            ComplexResource::Dolphin(dolphin) => { dolphin.to_generic() }
            ComplexResource::AIPartner(ai_partner) => { ai_partner.to_generic() }
        }
    }
}