# one-million-crabs
Galaxy simulation about a silly Explorer travelling around the galaxy to gether resources, combining them and create its AI-companion. He is trying not to die, watch how it permforme during the simulation and you can tweak some parameter to make the game more interesting.


# In depth description

## Orchestrator
### Galaxy initialization
It creates all the galaxy components at the same time it is created.
It needs create communication channels for planets and explorers.

#### Done

#### To do
- Manage the messages of the planet
- Manage the messages of the explorer
- Using a proper data structer to contain the galaxy comunication 
- Using a proper data structer to contain the galaxy topology

### CrabRave Planet
It implement planetAI in order to manage correctly to all external messages that could be:
- Sunray/Asteroid arrival
- Rocket creation/use
- Explorer Request

#### Done

#### To do
- Manage Sunray/Asteroid arrival
- Test Sunray/Asteroid arrival
- Manage Rocket creation and use
- Test Rocket creation and use

### Explorer 
We need to implement to internal AI in order to manage all the possible states and actions:
(We could use a state machine, we talkend in the seminar the 1th Decemeber)
- Greedy Explorer, takes all resources and try to combine them, goes around randomly
- Greedy Better, it makes sure to go an intersting planet for its purpose
- Best path, it map the topology and then it takes the best path to maximize AI-partner
- Best path + purpose changes, if it realize that cannot maximaze AI-partner then it tries to do dolphin



