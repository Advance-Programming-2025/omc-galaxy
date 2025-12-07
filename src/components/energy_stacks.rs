use crate::components::energy_stacks::stacks::{CHARGED_CELL_STACK, FREE_CELL_STACK};

pub const N_CELLS: usize = 5; // TODO da cambiare in base al pianeta
pub mod stacks {
    use std::sync::Mutex;
    use crate::components::energy_stacks::N_CELLS;

    pub(crate) static FREE_CELL_STACK: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    pub(crate) static CHARGED_CELL_STACK: Mutex<Vec<u32>> = Mutex::new(Vec::new());
    pub fn initialize_free_cell_stack(){

        //initialize the free cell stack with all of the possible indexes
        let free_cell_stack = FREE_CELL_STACK.lock();
        match free_cell_stack {
            Ok(mut vec) => {
                //empty previous values in case of reset
                vec.clear();
                for i in 0..N_CELLS {
                    vec.push(i as u32);
                }
                //put the indexes in the correct orientation
                vec.reverse();
            }
            Err(err) => {
                println!("{}", err);
            }
        }

        //same thing as above but we just make sure that the vector is empty
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        match charged_cell_stack {
            Ok(mut vec) => {
                vec.clear();
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    pub fn get_free_cell_index() -> Option<u32> {
        let free_cell_stack = FREE_CELL_STACK.lock();
        match free_cell_stack {
            Ok(mut vec) => {
                vec.pop()
            }
            Err(err) => {
                println!("{}", err);
                None
            }
        }
    }

    pub fn get_charged_cell_index() -> Option<u32> {
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        match charged_cell_stack {
            Ok(mut vec) => {
                vec.pop()
            }
            Err(err) => {
                println!("{}", err);
                None
            }
        }
    }
    pub fn push_free_cell(index: u32) {
        let free_cell_stack = FREE_CELL_STACK.lock();
        match free_cell_stack {
            Ok(mut vec) => {
                vec.push(index);
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }
    pub fn push_charged_cell(index: u32) {
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        match charged_cell_stack {
            Ok(mut vec) => {
                vec.push(index);
            }
            Err(err) => {
                println!("{}", err);
            }
        }
    }

    pub fn peek_charged_cell_index() -> Option<u32> {
        let charged_cell_stack = CHARGED_CELL_STACK.lock();
        match charged_cell_stack {
            Ok(vec) => {
                vec.last().copied()
            }
            Err(err) => {
                println!("{}", err);
                None
            }
        }
    }
}



