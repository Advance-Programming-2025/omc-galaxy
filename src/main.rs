mod components;

use components::Orchestrator;

//This main let us terminate in an elegant and simple way, returning the error message
fn main() -> Result<(), String> {
    //Init and check orchestrator
    let mut orchestrator = Orchestrator::new()?;
    let running_program = orchestrator.run()?;

    Ok(())
}