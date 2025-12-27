#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
use once_cell::sync::Lazy;

#[cfg(test)]
use crate::components::Orchestrator;

#[cfg(test)]

// pub static ORCHESTRATOR:Lazy<Mutex<Orchestrator>> = Lazy::new(||{
//     let orch = Orchestrator::new().expect("Failed to init orchestrator");
//     Mutex::new(orch)
// }); 

#[test]
fn is_orch_initialized()->Result<(),String>{
    let mut orchestrator = Orchestrator::new()?;
    Ok(())
}
#[test]
fn is_orch_usable_again()->Result<(),String>{
    let mut orchestrator = Orchestrator::new()?;
    Ok(())
}