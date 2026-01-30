use common_game::components::resource::{
    BasicResourceType, ComplexResourceRequest,
    ComplexResourceType, GenericResource, ResourceType,
};

/// the type that is returned to the orchestrator when he asks for the explorer's bag
pub type BagType = Vec<ResourceType>;

/// struct of the bag for explorer's internal use
pub struct Bag {
    resources: Vec<GenericResource>,
}

impl Bag {
    /// creates an empty bag
    pub fn new() -> Self {
        Self {
            resources: Vec::new(),
        }
    }

    /// inserts a resource in the bag
    pub fn insert(&mut self, res: GenericResource) {
        self.resources.push(res);
    }

    /// takes a resource from the bag if it exists
    pub fn take_resource(&mut self, ty: ResourceType) -> Option<GenericResource> {
        let idx = self.resources
            .iter()
            .position(|r| r.get_type() == ty)?;
        Some(self.resources.remove(idx))
    }

    /// tells if a resource is contained in the bag
    pub fn contains(&self, ty: ResourceType) -> bool {
        self.resources
            .iter()
            .any(|r| r.get_type() == ty)
    }

    /// returns a BagType containing all the ResourceType in the bag
    // this is needed because the bag cannot give its ownership to the orchestrator
    // and cannot be passed as a reference
    pub fn to_resource_types(&self) -> Vec<ResourceType> {
        self.resources
            .iter()
            .map(|r| r.get_type())
            .collect()
    }

    /// creates a ComplexResourceRequest based on the desired resource type
    pub fn make_complex_request(
        &mut self,
        resource_type: ComplexResourceType,
    ) -> Result<ComplexResourceRequest, String> {
        match resource_type {
            ComplexResourceType::Diamond => self.make_diamond_request(),
            ComplexResourceType::Water => self.make_water_request(),
            ComplexResourceType::Life => self.make_life_request(),
            ComplexResourceType::Robot => self.make_robot_request(),
            ComplexResourceType::Dolphin => self.make_dolphin_request(),
            ComplexResourceType::AIPartner => self.make_ai_partner_request(),
        }
    }
    

    /// the following methods are the ones to combine resources
    pub(crate) fn make_diamond_request(&mut self) -> Result<ComplexResourceRequest, String> {
        // checks that the explorer has 2 carbons before taking any
        let carbon_count = self.resources
            .iter()
            .filter(|r| r.get_type() == ResourceType::Basic(BasicResourceType::Carbon))
            .count();

        if carbon_count < 2 {
            return Err("Missing resource".to_string());
        }

        let c1 = self
            .take_resource(ResourceType::Basic(BasicResourceType::Carbon))
            .ok_or("Missing resource")?
            .to_carbon()?;

        let c2 = self
            .take_resource(ResourceType::Basic(BasicResourceType::Carbon))
            .ok_or("Missing resource")?
            .to_carbon()?;

        Ok(ComplexResourceRequest::Diamond(c1, c2))
    }

    pub(crate) fn make_water_request(&mut self) -> Result<ComplexResourceRequest, String> {
        let hydrogen_type = ResourceType::Basic(BasicResourceType::Hydrogen);
        let oxygen_type = ResourceType::Basic(BasicResourceType::Oxygen);

        if !(self.contains(hydrogen_type) && self.contains(oxygen_type)) {
            return Err("Missing resource".to_string());
        }

        let h = self
            .take_resource(hydrogen_type)
            .ok_or("Missing resource")?
            .to_hydrogen()?;

        let o = self
            .take_resource(oxygen_type)
            .ok_or("Missing resource")?
            .to_oxygen()?;

        Ok(ComplexResourceRequest::Water(h, o))
    }

    pub(crate) fn make_life_request(&mut self) -> Result<ComplexResourceRequest, String> {
        let water_type = ResourceType::Complex(ComplexResourceType::Water);
        let carbon_type = ResourceType::Basic(BasicResourceType::Carbon);

        if !(self.contains(water_type) && self.contains(carbon_type)) {
            return Err("Missing resource".to_string());
        }

        let w = self
            .take_resource(water_type)
            .ok_or("Missing resource")?
            .to_water()?;

        let c = self
            .take_resource(carbon_type)
            .ok_or("Missing resource")?
            .to_carbon()?;

        Ok(ComplexResourceRequest::Life(w, c))
    }

    pub(crate) fn make_robot_request(&mut self) -> Result<ComplexResourceRequest, String> {
        let silicon_type = ResourceType::Basic(BasicResourceType::Silicon);
        let life_type = ResourceType::Complex(ComplexResourceType::Life);

        if !(self.contains(silicon_type) && self.contains(life_type)) {
            return Err("Missing resource".to_string());
        }

        let s = self
            .take_resource(silicon_type)
            .ok_or("Missing resource")?
            .to_silicon()?;

        let l = self
            .take_resource(life_type)
            .ok_or("Missing resource")?
            .to_life()?;

        Ok(ComplexResourceRequest::Robot(s, l))
    }

    pub(crate) fn make_dolphin_request(&mut self) -> Result<ComplexResourceRequest, String> {
        let water_type = ResourceType::Complex(ComplexResourceType::Water);
        let life_type = ResourceType::Complex(ComplexResourceType::Life);

        if !(self.contains(water_type) && self.contains(life_type)) {
            return Err("Missing resource".to_string());
        }

        let w = self
            .take_resource(water_type)
            .ok_or("Missing resource")?
            .to_water()?;

        let l = self
            .take_resource(life_type)
            .ok_or("Missing resource")?
            .to_life()?;

        Ok(ComplexResourceRequest::Dolphin(w, l))
    }

    pub(crate) fn make_ai_partner_request(&mut self) -> Result<ComplexResourceRequest, String> {
        let robot_type = ResourceType::Complex(ComplexResourceType::Robot);
        let diamond_type = ResourceType::Complex(ComplexResourceType::Diamond);

        if !(self.contains(robot_type) && self.contains(diamond_type)) {
            return Err("Missing resource".to_string());
        }

        let r = self
            .take_resource(robot_type)
            .ok_or("Missing resource")?
            .to_robot()?;

        let d = self
            .take_resource(diamond_type)
            .ok_or("Missing resource")?
            .to_diamond()?;

        Ok(ComplexResourceRequest::AIPartner(r, d))
    }
}

impl Default for Bag {
    fn default() -> Self {
        Self::new()
    }
}
