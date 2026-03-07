# Documentazione Dettagliata - Mattia Explorer

## Indice

1. [Panoramica Architetturale](#1-panoramica-architetturale)
2. [Struttura dei File](#2-struttura-dei-file)
3. [La Struct Explorer (`mod.rs`)](#3-la-struct-explorer)
4. [La Macchina a Stati (`states.rs`)](#4-la-macchina-a-stati)
5. [Il Main Loop (`mod.rs::run()`)](#5-il-main-loop)
6. [Il Sistema di Buffering (`buffers.rs`)](#6-il-sistema-di-buffering)
7. [Handlers dei Messaggi (`handlers.rs`)](#7-handlers-dei-messaggi)
8. [Il Bag - Inventario Risorse (`bag.rs`)](#8-il-bag---inventario-risorse)
9. [Informazioni sui Pianeti (`planet_info.rs`)](#9-informazioni-sui-pianeti)
10. [Funzioni Helper (`helpers.rs`)](#10-funzioni-helper)
11. [Conversione Risorse (`resource_management.rs`)](#11-conversione-risorse)
12. [Il Sistema di Intelligenza Artificiale (`explorer_ai.rs`)](#12-il-sistema-di-intelligenza-artificiale)
13. [Mappa Completa dei Possibili Panic](#13-mappa-completa-dei-possibili-panic)
14. [Flusso dei Messaggi](#14-flusso-dei-messaggi)

---

## 1. Panoramica Architetturale

L'explorer e' un **attore** (actor model) che gira nel proprio thread e comunica con due entita' esterne tramite canali `crossbeam_channel`:

```
[Orchestrator] <--canali bidirezionali--> [Explorer] <--canali bidirezionali--> [Planet]
```

L'explorer e' implementato come una **macchina a stati** che processa messaggi da entrambi i canali simultaneamente. Quando un messaggio non corrisponde allo stato corrente, viene inserito in un buffer e processato successivamente.

L'explorer ha due modalita' operative:
- **Modalita' Manuale** (`manual_mode = true`): l'explorer risponde solo ai comandi dell'orchestrator.
- **Modalita' AI** (`manual_mode = false`): l'explorer prende decisioni autonome quando e' in stato `Idle` e non ci sono messaggi nei canali.

---

## 2. Struttura dei File

```
src/components/mattia_explorer/
├── mod.rs                  -- Struct Explorer, main loop, costruttore
├── explorer_ai.rs          -- Motore AI utility-based (1104 righe)
├── handlers.rs             -- Handler per ogni tipo di messaggio (982 righe)
├── helpers.rs              -- Funzioni di utilita' (gather_info_from_planet)
├── states.rs               -- Enum degli stati e funzioni di matching
├── bag.rs                  -- Inventario risorse dell'explorer
├── buffers.rs              -- Gestione messaggi bufferizzati
├── resource_management.rs  -- Trait ToGeneric per conversione risorse
├── planet_info.rs          -- Dati topologici per pianeta (risorse, energia, tipo)
├── tests.rs                -- Suite di test (2075 righe)
└── test_topology_files/
    └── t0.txt              -- File topologia per i test (9 pianeti)
```

---

## 3. La Struct Explorer

**File:** `mod.rs:37-54`

```rust
pub struct Explorer {
    explorer_id: ID,                    // identificativo univoco
    planet_id: ID,                      // pianeta corrente
    orchestrator_channels: (Receiver, Sender),  // canali verso l'orchestrator
    planet_channels: (Receiver, Sender),         // canali verso il pianeta
    topology_info: HashMap<ID, PlanetInfo>,      // mappa conoscenza topologica
    state: ExplorerState,               // stato corrente della macchina a stati
    bag: Bag,                           // inventario risorse
    buffer_orchestrator_msg: VecDeque<OrchestratorToExplorer>,  // buffer messaggi orchestrator
    buffer_planet_msg: VecDeque<PlanetToExplorer>,              // buffer messaggi pianeta
    time: u64,                          // tick temporale interno
    ai_data: AiData,                    // dati dell'AI (utility, bisogni, ultima azione)
    current_planet_neighbors_update: bool,  // flag per aggiornamento vicini
    manual_mode: bool,                  // true = manuale, false = AI autonoma
}
```

### Costruttore (`new()`)

`mod.rs:58-93`

Alla creazione l'explorer:
- Riceve i canali di comunicazione con orchestrator e pianeta
- Inizializza la topologia con il solo pianeta iniziale
- Parte in stato `Idle`
- Crea un `Bag` vuoto
- Inizializza il tempo a `1`
- Crea `AiData` con valori di default
- Parte in `manual_mode = true`

### Getter

- `id()` (`mod.rs:96`): restituisce l'ID dell'explorer.
- `get_planet_info(planet_id)` (`mod.rs:101`): restituisce un `Option<&PlanetInfo>` per un pianeta dato.
- `get_planet_info_mut(planet_id)` (`mod.rs:104`): versione mutabile.
- `get_current_planet_info()` (`mod.rs:108`): restituisce `Result<&PlanetInfo, &str>` per il pianeta corrente. Errore se il pianeta non e' nella topologia.
- `get_current_planet_info_mut()` (`mod.rs:114`): versione mutabile.

---

## 4. La Macchina a Stati

**File:** `states.rs`

### Stati dell'Explorer

```rust
enum ExplorerState {
    Idle,                                   // pronto a ricevere/eseguire azioni
    WaitingForNeighbours,                   // in attesa della risposta dei vicini dall'orchestrator
    Traveling,                              // in viaggio verso un altro pianeta
    GeneratingResource { orchestrator_response: bool },  // in generazione risorsa base
    CombiningResources { orchestrator_response: bool },  // in combinazione risorsa complessa
    Surveying {                             // in fase di survey del pianeta
        resources: bool,                    // se true: in attesa di risorse base
        combinations: bool,                 // se true: in attesa di combinazioni
        energy_cells: bool,                 // se true: in attesa di celle energetiche
        orch_resource: bool,                // se true: l'orchestrator ha richiesto le risorse
        orch_combination: bool,             // se true: l'orchestrator ha richiesto le combinazioni
    },
    Killed,                                 // explorer terminato
}
```

Il campo `orchestrator_response` in `GeneratingResource` e `CombiningResources` indica se l'azione e' stata avviata dall'orchestrator (in quel caso bisogna inviare la risposta indietro) oppure dall'AI (in quel caso non serve rispondere all'orchestrator).

I campi `orch_resource` e `orch_combination` nel `Surveying` servono per ricordare se l'orchestrator ha richiesto i dati, cosi' da inoltrargli la risposta una volta ricevuta dal pianeta.

### Funzioni di State Matching

- `orch_msg_match_state(state, msg)` (`states.rs:27`): verifica se un messaggio dell'orchestrator e' accettabile nello stato corrente.
  - In `Idle` tutti i messaggi sono accettati.
  - `NeighborsResponse` solo in `WaitingForNeighbours`.
  - `MoveToPlanet` solo in `Traveling`.
  - `KillExplorer` e' **sempre** accettato in qualsiasi stato.

- `planet_msg_match_state(state, msg)` (`states.rs:40`): verifica se un messaggio del pianeta e' accettabile.
  - In `Idle` tutti i messaggi sono accettati.
  - `GenerateResourceResponse` solo in `GeneratingResource`.
  - `CombineResourceResponse` solo in `CombiningResources`.
  - I messaggi di survey solo quando il corrispondente flag `Surveying` e' `true`.

---

## 5. Il Main Loop

**File:** `mod.rs:122-369`

Il metodo `run()` e' il cuore dell'explorer. Funziona cosi':

```
loop {
    1. Incrementa il tick temporale (con wrapping_add per evitare overflow/panic)
    2. select! sui canali:
       a. Messaggio dall'orchestrator:
          - Se il messaggio corrisponde allo stato -> esegui handler
          - Altrimenti -> push nel buffer orchestrator
          - Se il canale e' disconnesso -> ERRORE FATALE, return Err
       b. Messaggio dal pianeta:
          - Se il messaggio corrisponde allo stato -> esegui handler
          - Altrimenti -> push nel buffer pianeta
          - Se il canale e' disconnesso -> LOG errore ma NON termina
            (perche' deve aspettare il KillExplorer dall'orchestrator)
       c. default (nessun messaggio):
          - Se ci sono messaggi nei buffer -> gestisci buffer
          - Se manage_buffer_msg ha impostato stato Killed -> return Ok
          - Altrimenti se non e' in manual_mode e' e' Idle -> chiama ai_core_function()
    3. Sleep di 20ms per ridurre il busy waiting
}
```

### Dettaglio importante sul canale pianeta disconnesso

`mod.rs:230`: *"even if the channel is disconnected we need to wait the kill msg to terminate the execution"*

Quando il canale del pianeta si disconnette (es. il pianeta muore), l'explorer non termina ma logga un errore e continua ad attendere il messaggio `KillExplorer` dall'orchestrator. Questo e' diverso dal canale dell'orchestrator: se quel canale si disconnette, l'explorer termina immediatamente con un errore fatale.

### Gestione errori negli handler

`mod.rs:199-210, 306-317`: Se un handler restituisce `Err`, l'errore viene loggato come warning ma **l'explorer non termina**. Continua il loop normalmente.

### Risposta `BagContentRequest`

`mod.rs:191`: *"IMPORTANTE: restituisce un vettore contenente i resource type e non gli item in se"*. La risposta a `BagContentRequest` invia un `Vec<ResourceType>` e non gli oggetti risorse veri e propri, perche' il bag non puo' cedere l'ownership delle risorse all'orchestrator.

---

## 6. Il Sistema di Buffering

**File:** `buffers.rs`

### `manage_buffer_msg()`

`buffers.rs:21`

Questa funzione gestisce i messaggi messi nei buffer (nello stesso modo in cui l'explorer li gestirebbe normalmente nel main loop).

Funzionamento:
1. Se il buffer orchestrator non e' vuoto:
   - Controlla se il primo messaggio corrisponde allo stato corrente (`orch_msg_match_state`)
   - Se si': lo estrae (`.pop_front()`) e lo processa chiamando l'handler appropriato
2. Se il buffer pianeta non e' vuoto:
   - Stessa logica con `planet_msg_match_state`

**Nota sul `KillExplorer` dal buffer** (`buffers.rs:46`): il commento dice *"I don't think it is possible to arrive here"* - il `KillExplorer` nel buffer e' un caso teoricamente impossibile perche' `KillExplorer` e' sempre accettato (match in qualsiasi stato).

---

## 7. Handlers dei Messaggi

**File:** `handlers.rs`

### 7.1 Gestione Ciclo di Vita AI

#### `start_explorer_ai()` - `handlers.rs:21`
Imposta lo stato a `Idle`, disattiva `manual_mode`, e invia `StartExplorerAIResult` all'orchestrator. Da questo momento l'AI autonoma e' attiva.

#### `stop_explorer_ai()` - `handlers.rs:90`
Attiva `manual_mode = true` e invia `StopExplorerAIResult`. L'explorer torna in modalita' manuale.

#### `reset_explorer_ai()` - `handlers.rs:44`
Resetta completamente l'explorer:
- Svuota `topology_info` e reinserisce solo il pianeta corrente
- Resetta `current_planet_neighbors_update`
- Disattiva `manual_mode` (l'AI riparte)
- Crea un nuovo `AiData` di default
- Invia `ResetExplorerAIResult`

#### `kill_explorer()` - `handlers.rs:129`
Imposta stato a `Killed` e invia `KillExplorerResult`. Nel main loop, dopo `kill_explorer()`, viene fatto `return Ok(())` per terminare il thread.

### 7.2 Movimento

#### `move_to_planet()` - `handlers.rs:168`
Gestisce lo spostamento dell'explorer su un nuovo pianeta.

**Due scenari in caso di morte del pianeta** (`handlers.rs:187-189`):
1. L'orchestrator rifiuta l'operazione di spostamento
2. L'orchestrator uccide anche l'explorer se ha gia' accettato lo spostamento

Se `sender_to_new_planet` e' `Some(sender)`:
- Aggiorna il canale verso il pianeta
- Aggiorna `planet_id`
- Se il pianeta e' nella topologia ma l'explorer non e' in manual mode, avvia survey delle informazioni mancanti
- Se il pianeta NON e' nella topologia, lo aggiunge e imposta `current_planet_neighbors_update = true`
- Se non in manual mode, chiama `gather_info_from_planet()` per avviare il survey
- Invia `MovedToPlanetResult`

Se `sender_to_new_planet` e' `None` (`handlers.rs:248`):
- *"the explorer cannot move, but it is not a problem"*
- Imposta `current_planet_neighbors_update = true` (priorita' assoluta) per forzare un aggiornamento dei vicini al prossimo ciclo AI
- Non viene inviato alcun errore

### 7.3 Query Informazioni

#### `current_planet_request()` - `handlers.rs:266`
Invia l'ID del pianeta corrente all'orchestrator.

#### `supported_resource_request()` - `handlers.rs:308`
Invia le risorse base supportate dal pianeta corrente. Se l'explorer non le conosce, avvia un survey al pianeta e le inoltrera' quando le ricevera' (grazie al flag `orch_resource`).

**Meccanismo di recovery** (`handlers.rs:364-366`): se l'explorer non ha il pianeta corrente nella sua topologia (cosa che non dovrebbe accadere), tenta di recuperare richiedendo tutte le informazioni al pianeta.

#### `supported_combination_request()` - `handlers.rs:405`
Analoga a `supported_resource_request()` ma per le risorse complesse. Usa il flag `orch_combination`.

### 7.4 Generazione e Combinazione Risorse

#### `generate_resource_request()` - `handlers.rs:502`
Invia una `GenerateResourceRequest` al pianeta per generare una risorsa base. Imposta lo stato a `GeneratingResource`. Il parametro `to_orchestrator` indica se la risposta va inoltrata all'orchestrator.

#### `combine_resource_request()` - `handlers.rs:554`
Prepara una `ComplexResourceRequest` estraendo le risorse necessarie dal bag, poi la invia al pianeta. Se le risorse non sono sufficienti, invia un errore all'orchestrator e torna in `Idle`.

**Dettaglio** (`handlers.rs:571`): *"provide the requested resources from the bag for each combination"* - le risorse vengono estratte dal bag prima dell'invio della richiesta.

### 7.5 Gestione Risposte dal Pianeta

#### `manage_supported_resource_response()` - `handlers.rs:682`
Riceve la lista delle risorse base supportate dal pianeta:
- Aggiorna `topology_info` con le risorse
- Se ha anche le risorse complesse, calcola il tipo di pianeta (`calculate_planet_type`)
- Se `orch_resource` e' true, inoltra la risposta all'orchestrator
- Aggiorna lo stato (rimuove il flag `resources` dal `Surveying`)

#### `manage_supported_combination_response()` - `handlers.rs:769`
Analoga alla precedente ma per le combinazioni complesse.

#### `manage_generate_response()` - `handlers.rs:853`
Riceve la risorsa generata dal pianeta:
- Se la generazione ha avuto successo, inserisce la risorsa nel bag
- Se `orchestrator_response` e' true, invia la risposta all'orchestrator
- Torna in stato `Idle`

#### `manage_combine_response()` - `handlers.rs:918`
Riceve il risultato della combinazione:
- Se riuscita: inserisce la risorsa complessa nel bag
- Se fallita: **reinserisce le risorse nel bag** (`handlers.rs:944`). Le risorse `r1` e `r2` tornano indietro all'explorer.
- Se `orchestrator_response` e' true, invia la risposta all'orchestrator
- Torna in stato `Idle`

#### `neighbours_response()` - `handlers.rs:639`
Riceve la lista dei vicini del pianeta corrente:
- Aggiunge eventuali nuovi pianeti alla topologia con `PlanetInfo::new()`
- Aggiorna i vicini del pianeta corrente
- Resetta e ricostruisce `ai_data.ai_action.move_to` con i nuovi vicini (utilita' iniziale 0.0)

---

## 8. Il Bag - Inventario Risorse

**File:** `bag.rs`

### Struttura

Il bag contiene vettori separati per ogni tipo di risorsa (type-safe):

```rust
struct Bag {
    oxygen: Vec<Oxygen>,        // risorse base
    hydrogen: Vec<Hydrogen>,
    carbon: Vec<Carbon>,
    silicon: Vec<Silicon>,
    diamond: Vec<Diamond>,      // risorse complesse
    water: Vec<Water>,
    life: Vec<Life>,
    robot: Vec<Robot>,
    dolphin: Vec<Dolphin>,
    ai_partner: Vec<AIPartner>,
}
```

### Metodi principali

| Metodo | Descrizione |
|--------|-------------|
| `insert(res)` | Inserisce una risorsa generica nel vettore appropriato |
| `take_resource(ty)` | Estrae (pop) una risorsa dal vettore, restituisce `Option` |
| `contains(ty)` | Verifica se esiste almeno una risorsa del tipo dato |
| `count(ty)` | Conta le risorse di un tipo |
| `can_craft(complex_type)` | Verifica se le risorse per una combinazione sono disponibili |
| `to_resource_types()` | Converte il bag in `Vec<ResourceType>` (senza cedere ownership) |

### `can_craft()` - `bag.rs:152`

Restituisce una tupla a 5 elementi:
```
(puo_craftare, tipo_risorsa_1, ha_risorsa_1, tipo_risorsa_2, ha_risorsa_2)
```

### Albero delle Ricette

| Risorsa Complessa | Ingrediente 1 | Ingrediente 2 |
|-------------------|---------------|---------------|
| **Diamond** | Carbon | Carbon |
| **Water** | Hydrogen | Oxygen |
| **Life** | Water | Carbon |
| **Robot** | Silicon | Life |
| **Dolphin** | Water | Life |
| **AIPartner** | Robot | Diamond |

### Metodi `make_*_request()`

`bag.rs:271-373`

Questi metodi (es. `make_diamond_request()`, `make_water_request()`, ecc.):
1. Verificano con `can_craft()` che le risorse siano disponibili
2. Estraggono le risorse dal bag con `take_resource().unwrap()`
3. Convertono al tipo specifico (es. `.to_carbon()`)
4. Costruiscono e restituiscono un `ComplexResourceRequest`

**NOTA IMPORTANTE**: gli `unwrap()` in questi metodi sono **protetti** dal controllo `can_craft()` che precede ogni chiamata (vedi [sezione panic](#13-mappa-completa-dei-possibili-panic)).

### Funzioni standalone

- `put_complex_resource_in_the_bag()` (`bag.rs:377`): inserisce una risorsa complessa nel bag.
- `put_basic_resource_in_the_bag()` (`bag.rs:395`): inserisce una risorsa base nel bag.

---

## 9. Informazioni sui Pianeti

**File:** `planet_info.rs`

### `PlanetClassType`

```rust
enum PlanetClassType { A, B, C, D }
```

| Tipo | Puo' avere Rocket | Max Energy Cells |
|------|-------------------|-----------------|
| A | Si | 5 |
| B | No | 1 |
| C | Si | 1 |
| D | No | 5 |

### `PlanetInfo`

```rust
struct PlanetInfo {
    basic_resources: Option<HashSet<BasicResourceType>>,     // risorse base producibili
    complex_resources: Option<HashSet<ComplexResourceType>>,  // risorse complesse combinabili
    neighbors: Option<HashSet<ID>>,                          // pianeti vicini
    energy_cells: Option<u32>,                               // celle energetiche note
    charge_rate: Option<f32>,                                // tasso di ricarica stimato
    timestamp_neighbors: u64,                                // ultimo aggiornamento vicini
    timestamp_energy: u64,                                   // ultimo aggiornamento energia
    safety_score: Option<f32>,                               // punteggio di sicurezza [0, 1]
    inferred_planet_type: Option<PlanetClassType>,           // tipo di pianeta dedotto
}
```

### `update_charge_rate()` - `planet_info.rs:65`

Calcola il tasso di ricarica energetica del pianeta con una **media mobile esponenziale** (EMA):

1. **Prima visita** (`planet_info.rs:74`): registra solo l'energia corrente e il timestamp. Non puo' calcolare il tasso.
2. **Visite successive**:
   - Calcola `delta_t` = tempo corrente - timestamp precedente
   - Se `delta_t <= 0` (`planet_info.rs:82-84`): evita divisione per zero, aggiorna solo l'energia
   - Calcola `instant_rate = (energia_corrente - energia_precedente) / delta_t`
   - **Media ammortizzata** (`planet_info.rs:92-93`): `new_rate = alpha * instant_rate + (1 - alpha) * old_rate` con `alpha = 0.3`

### `calculate_planet_type()` - `planet_info.rs:105`

Inferisce il tipo di pianeta basandosi sul numero di risorse:
- `complex > 1` -> Tipo **C** (sicuramente)
- `complex == 1` -> Tipo **B** (piu' probabile, potrebbe essere C)
- `complex == 0 && basic > 1` -> Tipo **D**
- `complex == 0 && basic <= 1` -> Tipo **A** (piu' probabile, potrebbe essere D)

Restituisce `Err` se le risorse base o complesse sono `None` (non ancora note).

---

## 10. Funzioni Helper

**File:** `helpers.rs`

### `gather_info_from_planet()` - `helpers.rs:11`

Funzione che, basandosi sullo stato `Surveying` dell'explorer, invia i messaggi appropriati al pianeta:

- Se `resources == true` -> invia `SupportedResourceRequest`
- Se `combinations == true` -> invia `SupportedCombinationRequest`
- Se `energy_cells == true` -> invia `AvailableEnergyCellRequest`

Se l'explorer non e' in stato `Surveying`, logga un warning ma **non genera errore** (restituisce `Ok(())`).

---

## 11. Conversione Risorse

**File:** `resource_management.rs`

### Trait `ToGeneric`

Trait implementato per `BasicResource` e `ComplexResource` che fornisce un unico metodo:

```rust
fn res_to_generic(self) -> GenericResource
```

Converte una risorsa tipizzata (es. `BasicResource::Oxygen(val)`) nel tipo generico `GenericResource` necessario per l'inserimento nel bag.

---

## 12. Il Sistema di Intelligenza Artificiale

**File:** `explorer_ai.rs` (1104 righe)

### 12.1 Panoramica

L'AI dell'explorer e' un sistema **utility-based**: ad ogni ciclo calcola un punteggio di utilita' (valore `f32` in `[0.0, 1.0]`) per ogni azione possibile, poi esegue quella con il punteggio piu' alto.

### 12.2 Costanti di Configurazione

| Costante | Valore | Descrizione |
|----------|--------|-------------|
| `RANDOMNESS_RANGE` | 0.1    | Rumore aggiunto ai calcoli di utilita' |
| `LAMBDA` | 0.005  | Fattore di decadimento per informazioni obsolete |
| `PROPAGATION_FACTOR` | 0.8    | Propagazione bisogno risorse nell'albero dei crafting |
| `SAFETY_CRITICAL` | 0.3    | Soglia di pericolo critico - evacuazione immediata |
| `SAFETY_WARNING` | 0.6    | Soglia di allarme - inizia a cercare pianeti piu' sicuri |
| `ENERGY_CELLS_DEFENSE_THRESHOLD` | 2      | Celle energetiche minime per difesa |
| `MAX_ENERGY_INFO_AGE` | 150    | Tick massimi prima che i dati energetici siano obsoleti |
| `ACTION_HYSTERESIS_MARGIN` | 0.07   | Margine per evitare cambio azione frequente |
| `MIN_ACTIVE_CHARGE_RATE` | 0.05   | Tasso minimo per considerare il pianeta "in ricarica" |
| `MAX_PREDICTION_HORIZON` | 100    | Tick massimi per predizioni energetiche |
| `PERFECT_INFO_MAX_TIME` | 25      | Tick entro cui l'informazione e' considerata perfetta |
| `SAFETY_MIN_DIFF` | 0.07   | Differenza minima di safety per fuga |

### 12.3 Strutture Dati AI

#### `AIActionType` (enum) - `explorer_ai.rs:57`

Le azioni discrete che l'AI puo' intraprendere:

```rust
enum AIActionType {
    Produce(BasicResourceType),     // genera risorsa base
    Combine(ComplexResourceType),   // combina risorsa complessa
    MoveTo(ID),                     // spostati su un pianeta
    SurveyNeighbors,                // aggiorna informazioni vicini
    SurveyEnergy,                   // aggiorna informazioni energia
    Wait,                           // aspetta
    RunAway,                        // fuggi (verso il pianeta piu' sicuro)
}
```

#### `AIAction` (struct) - `explorer_ai.rs:67`

Contiene i punteggi di utilita' per tutte le azioni possibili:

```rust
struct AIAction {
    produce_resource: HashMap<BasicResourceType, f32>,    // utilita' per ogni risorsa base
    combine_resource: HashMap<ComplexResourceType, f32>,   // utilita' per ogni combinazione
    move_to: HashMap<ID, f32>,                            // utilita' per ogni pianeta raggiungibile
    survey_energy_cells: f32,                             // utilita' survey energia
    survey_neighbors: f32,                                // utilita' survey vicini
    wait: f32,                                            // utilita' attesa (default 0.15)
    run_away: f32,                                        // utilita' fuga
}
```

**Nota** (`explorer_ai.rs:68`): il campo `produce_resource` ha il commento *"not sure if this will be useful, because I think it is useless to waste energy cell in making resources"*.

#### `ResourceNeeds` (struct) - `explorer_ai.rs:107`

Traccia i bisogni di ogni tipo di risorsa. Ogni campo e' un `f32` in `[0, 1]`.

**`get_effective_need()`** (`explorer_ai.rs:168`): calcola il bisogno effettivo di una risorsa tenendo conto della **propagazione attraverso l'albero dei crafting**.

L'albero ha 5 livelli:

```
Livello 4: AIPartner
Livello 3: Robot, Dolphin
Livello 2: Life, Diamond
Livello 1: Water
Livello 0: Carbon, Oxygen, Hydrogen, Silicon (risorse base)
```

Esempio: se c'e' bisogno di `AIPartner`, questo propaga (con fattore 0.8) bisogno di `Robot` e `Diamond`, che a loro volta propagano bisogno di `Silicon`, `Life`, `Carbon`, e cosi' via. Il risultato e' clampato a `1.0`.

#### `AiData` (struct) - `explorer_ai.rs:234`

```rust
struct AiData {
    resource_needs: ResourceNeeds,           // bisogni di risorse
    ai_action: AIAction,                     // utilita' correnti
    last_action: Option<AIActionType>,       // ultima azione eseguita (per hysteresis)
    last_action_planet_id: Option<ID>,       // pianeta dell'ultima azione (anti ping-pong)
}
```

### 12.4 Funzioni di Calcolo Utilita'

#### `calculate_time_decay()` - `explorer_ai.rs:251`

Calcola il decadimento temporale delle informazioni:
- Se `timestamp == 0` (pianeta mai visitato): restituisce `0.0` (nessuna informazione attendibile)
- Altrimenti: `e^(-LAMBDA * delta_t)` dove `delta_t = tempo_corrente - timestamp_pianeta`

Il risultato va da `1.0` (informazione fresca) a `~0.0` (informazione molto vecchia).

#### `calculate_max_number_cells()` - `explorer_ai.rs:264`

Stima il numero massimo di celle energetiche basandosi sul tipo di pianeta inferito. Se il tipo non e' noto, assume un valore ottimistico di `3`.

#### `add_noise()` - `explorer_ai.rs:274`

Aggiunge rumore casuale a un valore: moltiplica per un fattore in `[1 - RANDOMNESS_RANGE, 1 + RANDOMNESS_RANGE]` e clampa in `[0, 1]`.

#### `predict_energy_cells()` - `explorer_ai.rs:281`

Predice le celle energetiche future:
- Usa valori di default se mancano dati (`unwrap_or(1)` per energia, `unwrap_or(0.0)` per tasso)
- Limita l'orizzonte di predizione a `MAX_PREDICTION_HORIZON`
- Calcola `energia_guadagnata = tasso * tempo`
- Clampa il risultato tra `0` e `max_cells`

#### `estimate_current_energy()` - `explorer_ai.rs:304`

Stima l'energia corrente e il livello di confidenza:

| Condizione | Confidenza |
|------------|-----------|
| Nessuna informazione energetica | 0.0 |
| Tempo trascorso <= `PERFECT_INFO_MAX_TIME` (10 tick) | 1.0 (perfetta) |
| Tempo trascorso <= `MAX_ENERGY_INFO_AGE` (50 tick) | Da 1.0 a 0.5 (lineare) |
| Tempo trascorso > 50 tick | 0.3 (bassa confidenza) |
| Minimo assoluto | 0.1 |

L'energia predetta e' pesata dalla confidenza: alta confidenza = si fidano della predizione; bassa confidenza = si fidano dell'ultimo valore registrato.

### 12.5 Pipeline di Calcolo Utilita'

#### `calc_utility()` - `explorer_ai.rs:335`

Funzione principale che calcola l'utilita' di TUTTE le azioni:

1. **Aggiorna safety score** per ogni pianeta conosciuto
2. **Produzione risorse base**: per ogni risorsa nel set produzione del pianeta, calcola lo score
3. **Combinazione risorse complesse**: analoga, per ogni combinazione supportata
4. **Movimento**: per ogni vicino conosciuto, calcola lo score di spostamento
5. **Survey energy e neighbors**: calcola utilita' di raccogliere informazioni
6. **Wait**: utilita' base `0.08`, con bonus `+0.1` se il pianeta ha buon charge rate e puo' avere rocket
7. **Run away**: `(1 - safety_score)^2` - diventa molto alto quando la safety e' bassa (reattivita' quadratica)

#### `score_basic_resource_production()` - `explorer_ai.rs:449`

Calcola l'utilita' di produrre una risorsa base:

```
score = bisogno_effettivo
      * (1 / conteggio_in_bag)           -- meno risorse hai, piu' ne vuoi
      * (1 - 1/energy_cells)             -- meno energia, piu' conservativo
      * (charge_rate > 0 ? 1.0 : 0.8)    -- bonus per pianeti in ricarica
      * (affidabilita'*0.2 + 0.8)        -- l'affidabilita' dei dati energetici conta poco
      * noise_factor                      -- rumore [0.95, 1.05]
```

#### `score_complex_resource_production()` - `explorer_ai.rs:485`

Simile al precedente, con l'aggiunta del **readiness factor**:

| Ingredienti disponibili | Fattore |
|------------------------|---------|
| Entrambi | 1.0 |
| Uno solo | 0.666 |
| Nessuno | 0.333 |

#### `calculate_safety_score()` - `explorer_ai.rs:530`

**Funzione molto importante** (commento: *"very important"*).

Calcola il punteggio di sicurezza di un pianeta come combinazione pesata di:

1. **Sustainability** (peso 0.15):
   - Charge rate > `MIN_ACTIVE_CHARGE_RATE`: `1.0` (ricarica attiva)
   - Charge rate > 0: `0.7` (ricarica lenta)
   - Charge rate = 0: `0.5` (nessuna ricarica)

2. **Physical safety** (peso 0.70, moltiplicato per `rocket`):
   - Usa energia predetta pesata per confidenza
   - `>= 2 celle`: `0.6 + (ratio * 0.4)` = range `[0.6, 1.0]`
   - `> 0 celle`: `0.3 + (ratio * 0.3)` = range `[0.3, 0.6]`
   - `0 celle`: `0.2` (baseline minima)
   - Se il pianeta **non puo' avere rocket**: moltiplicatore `0.5` (penalita')

3. **Escape factor** (peso 0.15):
   - Nessun vicino noto: `0.3` (ottimismo)
   - 0 vicini: `0.2`
   - 1 vicino: `0.5`
   - 2 vicini: `0.8`
   - 3+ vicini: `1.0`
   - Aggiustato con affidabilita' dei dati: `(escape * reliability) + (0.15 * (1 - reliability))`

Formula finale: `(sustainability*0.15 + physical_safety*rocket*0.70 + escape*0.15) * noise`

Il risultato e' salvato in `planet_info.safety_score`.

#### `score_survey_neighbors()` - `explorer_ai.rs:609`

```
score = 0.1
      + staleness_component (max 0.7)     -- dati vecchi -> piu' utile aggiornare
      + safety_bonus (0.2 se in pericolo)  -- se non sicuro, vuoi sapere le vie di fuga
      + unknown_bonus (0.3 se nessun vicino noto)
      * noise
```

#### `score_survey_energy()` - `explorer_ai.rs:645`

```
score = (0.15
      + staleness_component (max 0.5)
      + charge_rate_uncertainty (max 0.5)  -- pianeti con ricarica rapida cambiano velocemente
      + no_info_boost (0.3 se nessuna info))
      * threat_multiplier (1.3 se in pericolo su pianeta con rocket)
      * noise
```

#### `score_move_to()` - `explorer_ai.rs:696`

Due modalita' in base alla safety corrente:

**Modalita' emergenza** (safety < `SAFETY_WARNING`):
- Priorita': spostarsi verso pianeti piu' sicuri
- Score = safety_target + bonus_energia + bonus_ricarica

**Modalita' esplorazione** (safety >= `SAFETY_WARNING`):
- Priorita': esplorare pianeti meno conosciuti
- `exploration_value = 1 - reliability` (meno lo conosci, piu' e' interessante)
- Penalizzazione per pianeti pericolosi (`safety_factor`: 0.3 se critico, 0.6 se meno sicuro, 0.8 altrimenti)

### 12.6 Selezione dell'Azione

#### `can_run_away()` - `explorer_ai.rs:759`

Verifica se la fuga ha senso:
- `run_away <= 0` -> no
- Nessuna info sul pianeta corrente -> no
- Nessun vicino -> no
- Almeno un vicino con safety significativamente migliore (`+ SAFETY_MIN_DIFF`) -> si'
- Almeno un vicino con tipo sconosciuto (scenario ottimistico) -> si'
- Safety corrente <= `SAFETY_CRITICAL` (panico) -> si'

#### `action_utility()` - `explorer_ai.rs:787`

Restituisce l'utilita' dell'azione precedente, ma **solo se l'explorer e' ancora sullo stesso pianeta** dove l'aveva eseguita. Se ha cambiato pianeta, restituisce `None` (l'azione precedente non e' piu' rilevante).

#### `find_best_action()` - `explorer_ai.rs:814`

Seleziona l'azione migliore:

1. Scansiona tutte le azioni e trova quella con il valore piu' alto
2. **Anti ping-pong** (`explorer_ai.rs:825`): per `MoveTo`, scarta il pianeta da cui si e' arrivati (`last_action_planet_id`)
3. **Hysteresis** (`explorer_ai.rs:874`): se l'azione precedente ha ancora un'utilita' che, sommata al margine `ACTION_HYSTERESIS_MARGIN` (0.07), e' >= al miglior valore trovato, **mantiene l'azione precedente**. Questo riduce l'oscillazione tra azioni.

### 12.7 Funzione Core AI

#### `ai_core_function()` - `explorer_ai.rs:892`

Chiamata ad ogni ciclo del main loop quando non ci sono messaggi e l'AI e' attiva.

**Flusso:**

```
1. Prima visita su un pianeta?
   ├─ SI: aggiorna vicini (NeighborsRequest) -> return
   └─ NO: continua

2. Mancano info su risorse base o complesse?
   ├─ SI: survey del pianeta -> return
   └─ NO: continua

3. Calcola utilita' (calc_utility)
4. Trova azione migliore (find_best_action)
5. Esegui azione:
   ├─ RunAway: trova il pianeta con max utilita' di spostamento, TravelToPlanetRequest
   ├─ MoveTo(id): TravelToPlanetRequest verso id
   ├─ SurveyNeighbors: NeighborsRequest
   ├─ SurveyEnergy: gather_info_from_planet (solo energia)
   ├─ Produce(res): GenerateResourceRequest al pianeta
   ├─ Combine(res): estrai ingredienti dal bag, CombineResourceRequest al pianeta
   └─ Wait: non fare nulla
```

**Bypass AI per prima visita** (`explorer_ai.rs:904-907`): se `current_planet_neighbors_update` e' true o i vicini sono sconosciuti, l'AI viene bypassata e si aggiornano direttamente i vicini.

**Gestione errori**: ogni invio di messaggio e' wrappato in `match` con gestione dell'errore che resetta lo stato a `Idle` e restituisce l'errore.

---

## 13. Mappa Completa dei Possibili Panic

### 13.1 `unwrap()` nel Bag (SICURI - Protetti da guard)

**File:** `bag.rs`

Tutte le chiamate a `.unwrap()` nei metodi `make_*_request()` sono **protette** dal controllo `can_craft()` che precede la chiamata:

| Riga | Metodo | Protezione |
|------|--------|------------|
| 280 | `make_diamond_request()` | `can_craft(Diamond)` a riga 274 |
| 284 | `make_diamond_request()` | `can_craft(Diamond)` a riga 274 |
| 297 | `make_water_request()` | `can_craft(Water)` a riga 291 |
| 300 | `make_water_request()` | `can_craft(Water)` a riga 291 |
| 314 | `make_life_request()` | `can_craft(Life)` a riga 308 |
| 317 | `make_life_request()` | `can_craft(Life)` a riga 308 |
| 329 | `make_robot_request()` | `can_craft(Robot)` a riga 325 |
| 332 | `make_robot_request()` | `can_craft(Robot)` a riga 325 |
| 348 | `make_dolphin_request()` | `can_craft(Dolphin)` a riga 342 |
| 351 | `make_dolphin_request()` | `can_craft(Dolphin)` a riga 342 |
| 365 | `make_ai_partner_request()` | `can_craft(AIPartner)` a riga 359 |
| 369 | `make_ai_partner_request()` | `can_craft(AIPartner)` a riga 359 |

**Rischio residuo**: se `can_craft()` restituisce `true` ma tra il controllo e l'`unwrap()` un'altra operazione modifica il bag. In un contesto single-threaded (l'explorer gira in un solo thread), questo **non puo' accadere**.

**ATTENZIONE**: dopo `.unwrap()` viene chiamato `.to_carbon()?`, `.to_hydrogen()?` ecc. Questi metodi `.to_*()` restituiscono `Result` e potrebbero fallire se il tipo non corrisponde (es. un `GenericResource::BasicResources(BasicResource::Oxygen)` su cui si chiama `.to_carbon()`). In pratica questo non puo' accadere perche' `take_resource()` restituisce sempre il tipo corretto grazie al pattern matching interno.

### 13.2 `unwrap()` negli Handlers (SICURI - Inserimento appena avvenuto)

**File:** `handlers.rs`

| Riga | Contesto | Commento nel codice | Spiegazione |
|------|----------|---------------------|-------------|
| 674 | `neighbours_response()` | *"this should never panic"* | L'`unwrap()` e' su `topology_info.get_mut(planet_id)` ma **appena prima** (riga 668-669) e' stato fatto `topology_info.insert(planet_id, ...)`. L'elemento appena inserito e' sicuramente presente. |
| 718 | `manage_supported_resource_response()` | *"this should never panic"* | Stessa logica: `insert` alla riga 712, poi `get_mut().unwrap()` alla riga 717. |
| 805 | `manage_supported_combination_response()` | *"this should never panic"* | Stessa logica: `insert` alla riga 800, poi `get_mut().unwrap()` alla riga 804. |

**Rischio residuo**: nessuno in pratica. L'`insert` e' immediatamente prima dell'`unwrap()` sullo stesso thread.

### 13.3 `unwrap()` nei Buffers (SICURI - Controllo is_empty prima)

**File:** `buffers.rs`

| Riga | Contesto | Commento nel codice | Spiegazione |
|------|----------|---------------------|-------------|
| 32 | `buffer_orchestrator_msg.front().unwrap()` | *"this should never panic"* | Protetto dal controllo `!is_empty()` alla riga 28 |
| 34 | `buffer_orchestrator_msg.pop_front().unwrap()` | (implicito) | Protetto dallo stesso controllo. Se `front()` ha avuto successo, `pop_front()` avra' successo |
| 87 | `buffer_planet_msg.front().unwrap()` | *"this should not panic"* | Protetto dal controllo `!is_empty()` alla riga 85 |
| 88 | `buffer_planet_msg.pop_front().unwrap()` | (implicito) | Stessa logica |

### 13.4 `unwrap_or()` nell'AI (SICURI - Con valori di fallback)

**File:** `explorer_ai.rs`

| Riga | Espressione | Valore di fallback |
|------|-------------|-------------------|
| 287 | `current_energy.unwrap_or(1)` | Default ottimistico di 1 cella |
| 288 | `charge_rate.unwrap_or(0.0)` | Default pessimistico: nessuna ricarica |
| 443 | `safety_score.unwrap_or(SAFETY_WARNING)` | Predizione ottimistica |

Questi **non possono causare panic** perche' `unwrap_or` fornisce sempre un valore di default.

### 13.5 Overflow del Tempo (PROTETTO)

**File:** `mod.rs:128-129`

```rust
//this way should not panic
self.time = self.time.wrapping_add(1);
```

Usa `wrapping_add` invece di `+` per evitare un panic da overflow su `u64`. Quando il contatore raggiunge `u64::MAX`, torna a `0` senza panic.

### 13.6 Punti dove "this should not happen" (Situazioni Anomale)

Questi non sono panic ma situazioni logicamente impossibili che vengono gestite con log di warning:

| File | Riga | Situazione |
|------|------|-----------|
| `mod.rs` | 265-266 | `AvailableEnergyCellResponse` ricevuto quando l'explorer non e' in stato `Surveying` |
| `handlers.rs` | 364, 461 | Pianeta corrente non presente nella topologia dell'explorer |
| `handlers.rs` | 349-351, 446-448, 752-754, 837-839, 902-904, 969-971 | Tentativo di processare una risposta in uno stato non corretto |
| `planet_info.rs` | 113-115 | Tentativo di calcolare il tipo di pianeta senza avere info sulle risorse |
| `buffers.rs` | 133-134 | `AvailableEnergyCellResponse` dal buffer in stato non `Surveying` |

---

## 14. Flusso dei Messaggi

### 14.1 Pattern Orchestrator -> Explorer -> Planet -> Explorer -> Orchestrator

Esempio con `SupportedResourceRequest`:

```
1. Orchestrator invia SupportedResourceRequest all'Explorer
2. Explorer controlla se ha gia' le info nella topologia
   a. SI: invia SupportedResourceResult all'Orchestrator
   b. NO:
      - Imposta stato Surveying con orch_resource=true
      - Invia SupportedResourceRequest al Planet
      - (aspetta risposta)
      - Planet invia SupportedResourceResponse
      - Explorer salva le info nella topologia
      - Explorer invia SupportedResourceResult all'Orchestrator
      - Torna in stato Idle
```

### 14.2 Pattern AI-Initiated

```
1. Main loop, branch default, AI attiva, stato Idle
2. ai_core_function() viene chiamata
3. calc_utility() calcola utilita' di tutte le azioni
4. find_best_action() seleziona la migliore
5. Explorer invia messaggio appropriato (al pianeta o orchestrator)
6. Explorer cambia stato (es. Traveling, GeneratingResource, ecc.)
7. (aspetta risposta nel prossimo ciclo del main loop)
```

### 14.3 Protocollo di Viaggio

```
1. Explorer (o AI) decide di viaggiare
2. Explorer invia TravelToPlanetRequest all'Orchestrator
   (con explorer_id, current_planet_id, dst_planet_id)
3. Explorer imposta stato Traveling
4. Orchestrator gestisce IncomingExplorer/OutgoingExplorer con i pianeti
5. Orchestrator invia MoveToPlanet all'Explorer
   (con sender_to_new_planet e planet_id)
6. move_to_planet() aggiorna canali, topologia, e stato
7. Se non in manual mode: avvia survey del nuovo pianeta
```

### 14.4 Buffering

```
1. Messaggio arriva ma non corrisponde allo stato
   (es. BagContentRequest arriva mentre si e' in Traveling)
2. Messaggio viene inserito nel buffer appropriato
3. Quando i canali sono vuoti (branch default del select!):
   - manage_buffer_msg() controlla se il primo messaggio nel buffer
     ora corrisponde allo stato
   - Se si': lo processa normalmente
   - Se no: resta nel buffer
```

### 14.5 Terminazione

```
1. Orchestrator invia KillExplorer (accettato in QUALSIASI stato)
2. kill_explorer() imposta stato Killed, invia KillExplorerResult
3. Nel main loop: return Ok(()) -> il thread termina

Se KillExplorer arriva dal buffer:
1. manage_buffer_msg() processa KillExplorer
2. Stato diventa Killed
3. Nel main loop dopo manage_buffer_msg():
   if self.state == ExplorerState::Killed { return Ok(()) }
```
