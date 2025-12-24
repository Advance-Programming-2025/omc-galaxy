# One-million-crabs galaxy game
Galaxy simulation about Explorers travelling around the galaxy to gather resources, combining them and create some complex resources. They try not to die, watch how they performe during the simulation and you can manually interact with the simulation


# Initialization file
Create a new .env file and write in it the variable as in .env.example use the absolute path of the initialization file.
Write in it this topology:
```
0 1 2 3 4
1 2 3 4 
2 3
3
4
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
After that you can use `cargo run` or `cargo run --features debug-prints` to se all the debug messages. 

# Task for each member
- Davide Da Col => UI
- Mattia Pistollato => Explorer
- Tommaso Ascolani => Explorer
- Marco Adami => Explorer



