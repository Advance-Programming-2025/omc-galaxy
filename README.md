# One-million-crabs galaxy game
Galaxy simulation about Explorers travelling around the galaxy to gather resources, combining them and create some complex resources. They try not to die, watch how they performe during the simulation and you can manually interact with the simulation


## Initialization file
Create a new .env file and write in it the variable as in .env.example use the absolute path of the initialization file.

File format:
the file follows csv schema and each row represent a planet, the first 2 elements are: `planet_id`, `type_id`

The remaining elements are the `planet_id`s to which the planet is connected.

(Note: It doesn't matter if you define a connection in only one direction or both; the program will always create a bidirectional connection.)

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
_: Random (one type will be chosen at random from among the possible ones)
```

Write in it this topology:
```
0, 4, 1, 2, 3, 4
1, 4, 2, 3, 4
2, 4, 3
3, 4
4, 4
```
The adjacency matrix should look like this:
```
[false, true, true, true, true]
[true, false, true, true, true]
[true, true, false, true, false]
[true, true, true, false, false]
[true, true, false, false, false]
```

## How to run it (at the moment)
Go in `orch-example`, after that you can use `cargo run` or `cargo run --features omc-galaxy/debug-prints` to se all the debug messages. 

In order to see the logs you have first to set RUST_LOG variable to whatever level you want (Error, Warning, Info, Debug, Trace)

For Windows: `$env:RUST_LOG="debug";`

For Unix: `RUST_LOG=debug`

By default env_logger filters out everything below Info

## Tests
Use `cargo nextest run`

>Tests run with `cargo test` are considered as the same process. Therefore we cannot istanciate orchestrator multiple times in different test.



# Task for each member
- Davide Da Col => UI
- Mattia Pistollato => Explorer
- Tommaso Ascolani => Explorer
- Marco Adami => Explorer



