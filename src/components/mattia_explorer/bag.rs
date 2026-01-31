use crate::components::mattia_explorer::Explorer;
use common_game::components::resource::{AIPartner, BasicResource, BasicResourceType, Carbon, ComplexResource, ComplexResourceRequest, ComplexResourceType, Diamond, Dolphin, GenericResource, Hydrogen, Life, Oxygen, ResourceType, Robot, Silicon, Water};

pub(crate) struct Bag {
    oxygen: Vec<Oxygen>,
    hydrogen: Vec<Hydrogen>,
    carbon: Vec<Carbon>,
    silicon: Vec<Silicon>,
    diamond: Vec<Diamond>,
    water: Vec<Water>,
    life: Vec<Life>,
    robot: Vec<Robot>,
    dolphin: Vec<Dolphin>,
    ai_partner: Vec<AIPartner>,
}

impl Bag {
    // creates an empty bag
    pub(crate) fn new() -> Self {
        Self {
            oxygen: vec![],
            hydrogen: vec![],
            carbon: vec![],
            silicon: vec![],
            diamond: vec![],
            water: vec![],
            life: vec![],
            robot: vec![],
            dolphin: vec![],
            ai_partner: vec![],
        }
    }

    // inserts a resource in the bag
    pub fn insert(&mut self, res: GenericResource) {
        match res {
            // base
            GenericResource::BasicResources(BasicResource::Oxygen(val)) => self.oxygen.push(val),
            GenericResource::BasicResources(BasicResource::Hydrogen(val)) => self.hydrogen.push(val),
            GenericResource::BasicResources(BasicResource::Carbon(val)) => self.carbon.push(val),
            GenericResource::BasicResources(BasicResource::Silicon(val)) => self.silicon.push(val),
            //complex
            GenericResource::ComplexResources(ComplexResource::Diamond(val)) => self.diamond.push(val),
            GenericResource::ComplexResources(ComplexResource::Water(val)) => self.water.push(val),
            GenericResource::ComplexResources(ComplexResource::Life(val)) => self.life.push(val),
            GenericResource::ComplexResources(ComplexResource::Robot(val)) => self.robot.push(val),
            GenericResource::ComplexResources(ComplexResource::Dolphin(val)) => self.dolphin.push(val),
            GenericResource::ComplexResources(ComplexResource::AIPartner(val)) => self.ai_partner.push(val),
        }
    }

    // takes a resource from the bag if it exists
    pub fn take_resource(&mut self, ty: ResourceType) -> Option<GenericResource> {
        match ty {
            // Basic Resources
            ResourceType::Basic(BasicResourceType::Oxygen) =>self.oxygen.pop().map(|v| GenericResource::BasicResources(BasicResource::Oxygen(v))),
            ResourceType::Basic(BasicResourceType::Hydrogen) =>self.hydrogen.pop().map(|v| GenericResource::BasicResources(BasicResource::Hydrogen(v))),
            ResourceType::Basic(BasicResourceType::Carbon) =>self.carbon.pop().map(|v| GenericResource::BasicResources(BasicResource::Carbon(v))),
            ResourceType::Basic(BasicResourceType::Silicon) =>self.silicon.pop().map(|v| GenericResource::BasicResources(BasicResource::Silicon(v))),
            // Complex Resources
            ResourceType::Complex(ComplexResourceType::Diamond) =>self.diamond.pop().map(|v| GenericResource::ComplexResources(ComplexResource::Diamond(v))),
            ResourceType::Complex(ComplexResourceType::Water) =>self.water.pop().map(|v| GenericResource::ComplexResources(ComplexResource::Water(v))),
            ResourceType::Complex(ComplexResourceType::Life) =>self.life.pop().map(|v| GenericResource::ComplexResources(ComplexResource::Life(v))),
            ResourceType::Complex(ComplexResourceType::Robot) =>self.robot.pop().map(|v| GenericResource::ComplexResources(ComplexResource::Robot(v))),
            ResourceType::Complex(ComplexResourceType::Dolphin) =>self.dolphin.pop().map(|v| GenericResource::ComplexResources(ComplexResource::Dolphin(v))),
            ResourceType::Complex(ComplexResourceType::AIPartner) =>self.ai_partner.pop().map(|v| GenericResource::ComplexResources(ComplexResource::AIPartner(v))),
        }
    }

    // tells if a resource is contained in the bag
    pub fn contains(&self, ty: ResourceType) -> bool {
        match ty {
            // Basic Resources
            ResourceType::Basic(BasicResourceType::Oxygen) => !self.oxygen.is_empty(),
            ResourceType::Basic(BasicResourceType::Hydrogen) => !self.hydrogen.is_empty(),
            ResourceType::Basic(BasicResourceType::Carbon) => !self.carbon.is_empty(),
            ResourceType::Basic(BasicResourceType::Silicon) => !self.silicon.is_empty(),

            // Complex Resources
            ResourceType::Complex(ComplexResourceType::Diamond) => !self.diamond.is_empty(),
            ResourceType::Complex(ComplexResourceType::Water) => !self.water.is_empty(),
            ResourceType::Complex(ComplexResourceType::Life) => !self.life.is_empty(),
            ResourceType::Complex(ComplexResourceType::Robot) => !self.robot.is_empty(),
            ResourceType::Complex(ComplexResourceType::Dolphin) => !self.dolphin.is_empty(),
            ResourceType::Complex(ComplexResourceType::AIPartner) => !self.ai_partner.is_empty(),
        }
    }

    //tells the number of resource of a certain type
    pub fn count(&self, ty:ResourceType) -> usize {
        match ty {
            //basic
            ResourceType::Basic(BasicResourceType::Oxygen) => self.oxygen.len(),
            ResourceType::Basic(BasicResourceType::Hydrogen) => self.hydrogen.len(),
            ResourceType::Basic(BasicResourceType::Carbon) => self.carbon.len(),
            ResourceType::Basic(BasicResourceType::Silicon) => self.silicon.len(),
            //complex
            ResourceType::Complex(ComplexResourceType::Diamond) => self.diamond.len(),
            ResourceType::Complex(ComplexResourceType::Water) => self.water.len(),
            ResourceType::Complex(ComplexResourceType::Life) => self.life.len(),
            ResourceType::Complex(ComplexResourceType::Robot) => self.robot.len(),
            ResourceType::Complex(ComplexResourceType::Dolphin) => self.dolphin.len(),
            ResourceType::Complex(ComplexResourceType::AIPartner) => self.ai_partner.len(),
        }
    }
    pub fn can_craft(&self, complex_type: ComplexResourceType) -> (bool, ResourceType, bool, ResourceType, bool) {
        match complex_type {
            ComplexResourceType::Diamond => {
                let res = ResourceType::Basic(BasicResourceType::Carbon);
                let n = self.count(res);
                (n>=2,res, n >= 1, res, n >= 2)
            }
            ComplexResourceType::Water => {
                let r1 = ResourceType::Basic(BasicResourceType::Hydrogen);
                let r2 = ResourceType::Basic(BasicResourceType::Oxygen);
                (self.contains(r1) && self.contains(r2),r1, self.contains(r1), r2, self.contains(r2))
            }
            ComplexResourceType::Life => {
                let r1 = ResourceType::Complex(ComplexResourceType::Water);
                let r2 = ResourceType::Basic(BasicResourceType::Carbon);
                (self.contains(r1) && self.contains(r2), r1, self.contains(r1), r2, self.contains(r2))
            }
            ComplexResourceType::Robot => {
                let r1 = ResourceType::Basic(BasicResourceType::Silicon);
                let r2 = ResourceType::Complex(ComplexResourceType::Life);
                (self.contains(r1) && self.contains(r2), r1, self.contains(r1), r2, self.contains(r2))
            }
            ComplexResourceType::Dolphin => {
                let r1 = ResourceType::Complex(ComplexResourceType::Water);
                let r2 = ResourceType::Complex(ComplexResourceType::Life);
                (self.contains(r1) && self.contains(r2),r1, self.contains(r1), r2, self.contains(r2))
            }
            ComplexResourceType::AIPartner => {
                let r1 = ResourceType::Complex(ComplexResourceType::Robot);
                let r2 = ResourceType::Complex(ComplexResourceType::Diamond);
                (self.contains(r1) && self.contains(r2), r1, self.contains(r1), r2, self.contains(r2))
            }
        }
    }

    // this is needed because the bag cannot give his ownership to the orchestrator and cannot be passed as a reference
    pub fn to_resource_types(&self) -> Vec<ResourceType> {
        let total_size = self.oxygen.len() + self.hydrogen.len() + self.carbon.len() + self.silicon.len() + self.diamond.len() + self.water.len() + self.life.len() + self.robot.len() + self.dolphin.len() + self.ai_partner.len();
        let mut types = Vec::with_capacity(total_size); //this way the vec is already of the right size
        for _ in 0..self.oxygen.len() { types.push(ResourceType::Basic(BasicResourceType::Oxygen)); }
        for _ in 0..self.hydrogen.len() { types.push(ResourceType::Basic(BasicResourceType::Hydrogen)); }
        for _ in 0..self.carbon.len() { types.push(ResourceType::Basic(BasicResourceType::Carbon)); }
        for _ in 0..self.silicon.len() { types.push(ResourceType::Basic(BasicResourceType::Silicon)); }
        // complex
        for _ in 0..self.diamond.len() { types.push(ResourceType::Complex(ComplexResourceType::Diamond)); }
        for _ in 0..self.water.len() { types.push(ResourceType::Complex(ComplexResourceType::Water)); }
        for _ in 0..self.life.len() { types.push(ResourceType::Complex(ComplexResourceType::Life)); }
        for _ in 0..self.robot.len() { types.push(ResourceType::Complex(ComplexResourceType::Robot)); }
        for _ in 0..self.dolphin.len() { types.push(ResourceType::Complex(ComplexResourceType::Dolphin)); }
        for _ in 0..self.ai_partner.len() { types.push(ResourceType::Complex(ComplexResourceType::AIPartner)); }
        types
    }

    // the following methods are the ones to combine resources
    //they are all used in order to avoid code duplication
    pub fn make_diamond_request(&mut self) -> Result<ComplexResourceRequest, String> {
        if !self.can_craft(ComplexResourceType::Diamond).0 {
            return Err("Missing resources for Diamond".to_string());
        }

        let c1 = self.take_resource(ResourceType::Basic(BasicResourceType::Carbon))
            .unwrap().to_carbon()?;
        let c2 = self.take_resource(ResourceType::Basic(BasicResourceType::Carbon))
            .unwrap().to_carbon()?;

        Ok(ComplexResourceRequest::Diamond(c1, c2))
    }

    pub fn make_water_request(&mut self) -> Result<ComplexResourceRequest, String> {
        if !self.can_craft(ComplexResourceType::Water).0 {
            return Err("Missing resources for Water".to_string());
        }

        let h = self.take_resource(ResourceType::Basic(BasicResourceType::Hydrogen))
            .unwrap().to_hydrogen()?;
        let o = self.take_resource(ResourceType::Basic(BasicResourceType::Oxygen))
            .unwrap().to_oxygen()?;

        Ok(ComplexResourceRequest::Water(h, o))
    }

    pub fn make_life_request(&mut self) -> Result<ComplexResourceRequest, String> {
        if !self.can_craft(ComplexResourceType::Life).0 {
            return Err("Missing resources for Life".to_string());
        }

        let w = self.take_resource(ResourceType::Complex(ComplexResourceType::Water))
            .unwrap().to_water()?;
        let c = self.take_resource(ResourceType::Basic(BasicResourceType::Carbon))
            .unwrap().to_carbon()?;

        Ok(ComplexResourceRequest::Life(w, c))
    }

    pub fn make_robot_request(&mut self) -> Result<ComplexResourceRequest, String> {
        if !self.can_craft(ComplexResourceType::Robot).0 {
            return Err("Missing resources for Robot".to_string());
        }

        let s = self.take_resource(ResourceType::Basic(BasicResourceType::Silicon))
            .unwrap().to_silicon()?;
        let l = self.take_resource(ResourceType::Complex(ComplexResourceType::Life))
            .unwrap().to_life()?;

        Ok(ComplexResourceRequest::Robot(s, l))
    }

    pub fn make_dolphin_request(&mut self) -> Result<ComplexResourceRequest, String> {
        if !self.can_craft(ComplexResourceType::Dolphin).0 {
            return Err("Missing resources for Dolphin".to_string());
        }

        let w = self.take_resource(ResourceType::Complex(ComplexResourceType::Water))
            .unwrap().to_water()?;
        let l = self.take_resource(ResourceType::Complex(ComplexResourceType::Life))
            .unwrap().to_life()?;

        Ok(ComplexResourceRequest::Dolphin(w, l))
    }

    pub fn make_ai_partner_request(&mut self) -> Result<ComplexResourceRequest, String> {
        if !self.can_craft(ComplexResourceType::AIPartner).0 {
            return Err("Missing resources for AIPartner".to_string());
        }

        let r = self.take_resource(ResourceType::Complex(ComplexResourceType::Robot))
            .unwrap().to_robot()?;
        let d = self.take_resource(ResourceType::Complex(ComplexResourceType::Diamond))
            .unwrap().to_diamond()?;

        Ok(ComplexResourceRequest::AIPartner(r, d))
    }
}

// this function puts a complex resource in the explorer bag
pub fn put_complex_resource_in_the_bag(
    explorer: &mut Explorer,
    complex_response: Result<ComplexResource, (String, GenericResource, GenericResource)>,
) {
    if let Ok(complex_resource) = complex_response {
        let new_resource = match complex_resource {
            ComplexResource::Diamond(diamond) => diamond.to_generic(),
            ComplexResource::Water(water) => water.to_generic(),
            ComplexResource::Life(life) => life.to_generic(),
            ComplexResource::Robot(robot) => robot.to_generic(),
            ComplexResource::Dolphin(dolphin) => dolphin.to_generic(),
            ComplexResource::AIPartner(ai_partner) => ai_partner.to_generic(),
        };
        explorer.bag.insert(new_resource);
    }
}


// this function puts a basic resource in the explorer bag
pub fn put_basic_resource_in_the_bag(explorer: &mut Explorer, resource: Option<BasicResource>) {
    if let Some(resource) = resource {
        let new_resource = match resource {
            BasicResource::Oxygen(oxygen) => oxygen.to_generic(),
            BasicResource::Hydrogen(hydrogen) => hydrogen.to_generic(),
            BasicResource::Carbon(carbon) => carbon.to_generic(),
            BasicResource::Silicon(silicon) => silicon.to_generic(),
        };
        explorer.bag.insert(new_resource);
    }
}