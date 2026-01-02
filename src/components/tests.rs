// #[cfg(test)]
// use std::sync::Mutex;

// #[cfg(test)]
// use once_cell::sync::Lazy;

#[cfg(test)]
use crate::components::Orchestrator;

#[cfg(test)]

// pub static ORCHESTRATOR:Lazy<Mutex<Orchestrator>> = Lazy::new(||{
//     let orch = Orchestrator::new().expect("Failed to init orchestrator");
//     Mutex::new(orch)
// }); 

#[test]
fn is_orch_initialized()->Result<(),String>{
    let _orchestrator = Orchestrator::new()?;
    Ok(())
}
#[test]
fn is_orch_usable_again()->Result<(),String>{
    let _orchestrator = Orchestrator::new()?;
    Ok(())
}

#[test]
fn try_galaxy_top_lock()->Result<(),String>{
    let orchestrator = Orchestrator::new()?;
    match orchestrator.get_topology().try_write() {
        Ok(gtop) => {
            drop(gtop);
            Ok(())
        },
        Err(_e) => Err(
            "try lock failed".to_string()
        ) 
    }
}