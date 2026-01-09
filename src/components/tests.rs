// #[cfg(test)]
// use std::sync::Mutex;

// #[cfg(test)]
// use once_cell::sync::Lazy;

#[cfg(test)]
use crate::components::orchestrator::Orchestrator;

#[cfg(test)]
#[test]
fn topology_generation() -> Result<(), String> {
    println!("topology generation");
    let mut _orchestrator = Orchestrator::new()?;
    _orchestrator.initialize_galaxy_by_file("")
}

#[test]
fn is_orch_initialized() -> Result<(), String> {
    let _orchestrator = Orchestrator::new()?;
    Ok(())
}
#[test]
fn is_orch_usable_again() -> Result<(), String> {
    let _orchestrator = Orchestrator::new()?;
    Ok(())
}

#[test]
fn try_galaxy_top_lock() -> Result<(), String> {
    let orchestrator = Orchestrator::new()?;
    match orchestrator.get_topology().try_write() {
        Ok(gtop) => {
            drop(gtop);
            Ok(())
        }
        Err(_e) => Err("try lock failed".to_string()),
    }
}
