# One Million Crabs 🦀

A galaxy simulation backend, developed for the 2025/26 Advanced Programming course @ UniTN.

>Galaxy simulation about explorers travelling around the galaxy to gather and combine resources. Watch as strive to survive and how they perform during the simulation!

omc-galaxy focuses on:
- Handling multi-threaded components
- Accomodating modular and well-structured code
- Extensible features
- Solid protocol implementation 

## Run the project
The recommended way to run the project is through the One Million Crabs GUIs:
- Use [omc-gui](https://github.com/Advance-Programming-2025/omc-gui) for a Bevy-based graphical environment, courtesy of Davide Da Col
- Use [ratatui-gui](https://github.com/Advance-Programming-2025/ratatui-gui) for a Ratatui-based TUI visualizer, courtesy of Marco Adami

The omc-galaxy code is used as a dependency for both projects, which allows for more modular development and better separation of concerns.

## Architecture

The simulation consists of three main components:

- **Orchestrator**: manages the galaxy and global events
- **Planets**: autonomous actors that offer varying combination of recipes and defence
- **Explorers**: autonomous agents that act according to the current state of the galaxy

All components communicate through message-passing channels.

## Initialization file
The galaxy's topology is set through a topology file, which can either be set as a path in an .env file or sent directly to the orchestrator with the appropriate methods.

If you wish to set the .env file for testing purposes, create a new .env file and write in it the variable as in .env.example use the absolute path of the initialization file.

### File format:
the file follows the .csv schema and each row represent a planet, where the first 2 elements are: `planet_id`, `type_id`

The remaining elements are the `planet_id`s to which the planet is connected (Note: It doesn't matter if you define a connection in only one direction or both; the program will always create a bidirectional connection).

(Note: `planet_id` and `type_id` are `u32`)

(Note: the `planet_ids` do not need to be consecutive)

List of possible values of `type_id`:
```
0: BlackAdidasShoe
1: Ciuc
2: HoustonWeHaveABorrow
3: ImmutableCosmicBorrow
4: OneMillionCrabs
5: Rustrelli
6: RustyCrab
7. TheCompilerStrikesBack
_: Random (one type will be chosen at random from among the possible ones)
```

An example of a valid topology would be the following:
```
0, 5, 1, 2, 3, 4
1, 2, 2, 3, 4
2, 3, 3
3, 6
4, 7
```

The corresponding adjacency matrix should look like this:
```
[false, true, true, true, true]
[true, false, true, true, true]
[true, true, false, true, false]
[true, true, true, false, false]
[true, true, false, false, false]
```

Graphically, that would look something like:
```
          1---------+
         /|\        |
        / | \       |
       /  |  \      |
      /   |   \     |
     /    |    \    |
    3-----4     5   |
     \    |    /    |
      \   |   /     |
       \  |  /      |
        \ | /       |
         \|/        |
          2---------+
```

## Explorers
The project features two different explorer implementations:
- "The survivor", which aims to survive as long as possible by dodging dangerous situations, courtesy of Mattia Pistollato
- "The AI-researcher", which aims to create as many AI partners as possible by optimizing its paths and generations, courtesy of Tommaso Ascolani

The documentation and further specific details are available inside the repo, respectively at `src/components/mattia_explorer` and `/src/components/tommy_explorer`.

## Tests
For limitations outside of our control, the standard `cargo test` function will fail after the first test.
This is because certain components of the common crate can only be instantiated once, which means that the standard `cargo test` runner cannot execute the entire test suite reliably (since it relies on a single environment).

To fully test the library, utilize `nextest`; the pre-built binaries are available [here](https://nexte.st/docs/installation/pre-built-binaries/). After installing the program, run the following command:

```bash
cargo nextest run --no-fail-fast
```

# Contributions
- Davide Da Col => UI
- Mattia Pistollato => Explorer
- Tommaso Ascolani => Explorer
- Marco Adami => UI



