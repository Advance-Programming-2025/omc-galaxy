## Tipo di pianeta

| Tipo | Celle Energetiche | Regola di generazione delle risorse | Razzi (Rockets) | Regole di combinazione delle risorse |
|----|----|----|----|----|
| **A** | `Vec<EnergyCell>` (Molte) | Al massimo una | Al massimo uno | No |
| **B** | `EnergyCell` (Una) | Illimitata (Unbounded) | Zero | Una |
| **C** | `EnergyCell` (Una) | Al massimo una | Al massimo uno | Illimitata (Unbounded) |
| **D** | `Vec<EnergyCell>` (Molte) | Illimitata (Unbounded) | Zero | No |

## Tipo di risorse

### Risorse base

| Risorsa di Base |
|-----------------|
| **Hydrogen**    |
| **Oxygen**      |
| **Carbon**      |
| **Silicon**     |

### Risorse complesse

| Risorsa Complessa | Ricetta Immediata (Input 1 + Input 2) | Risorse Base Totali Necessarie | Tot | Dettaglio della Derivazione |
|----|----|----|----|----|
| **Water** | Hydrogen + Oxygen | <font color="#c00000">1</font> $H$, <font color="#c00000">1 $O$ | 2 | H + O (Entrambe risors</font>e base) |
| **Diamond** | Carbon + Carbon | <font color="#c00000">2 $C$ </font> | 2 | C + C (Entrambe risorse base) |
| **Life** | Water + Carbon | <font color="#c00000">1</font> $H$, <font color="#c00000">1</font> $O$, <font color="#c00000">1</font> $C$ | 3 | **Water** (H+O) + C |
| **Robot** | Silicon + Life | <font color="#c00000">1</font> $Si$, <font color="#c00000">1</font> $H$, <font color="#c00000">1</font> $O$, <font color="#c00000">1</font> $C$ | 4 | Si + **Life** (H+O+C) |
| **Dolphin** | Water + Life | <font color="#c00000">2</font> $H$, <font color="#c00000">2</font> $O$, <font color="#c00000">1</font> $C$ | 5 | **Water** (H+O) + **Life** (H+O+C) |
| **AI-Partner** | Robot + Diamond | <font color="#c00000">1</font> $Si$, <font color="#c00000">1</font> $H$, <font color="#c00000">1</font> $O$, <font color="#c00000">3</font> $C$ | 6 | **Robot** (Si+H+ **Life**(O+C)) + **Diamond** (C+C) |

## AI Explorer

#### Legenda

**tipi di pianeta**: A, B, C, D
**Risorse**:

| Simbolo | Risorsa    |
|---------|------------|
| *Si*    | Silicio    |
| *H*     | Idrogeno   |
| *O*     | Ossigeno   |
| *C*     | Carbonio   |
| *W*     | Acqua      |
| *D*     | Diamante   |
| *L*     | Vita       |
| *R*     | Robot      |
| *Do*    | Delfino    |
| *AI*    | AI-Partner |

**Risorse base**: *b*
**Risorse complesse**: *x*
**Razzi**: *r*
\*
**- Explorer che cerca di **sopravvivere il più possibile\*\*
- Massimizzare la quantità di razzi:
- Maggioranza di A, C + eventualmente 1 o 2 D a seconda dell'altra strategia
- Proposta: 1 A (*b*:*C*, *r*), 4 C (*b*, *x*, *r*), 2 D (*b*)
- Scoprire la topologia completa e aggiornata dei pianeti
- Non servono necessariamente tipi specifici però direi che avere una galassia stabile sia un idea per evitare che venga divisa
- Probabilmente qualche A o C
- Collezionare tutte le risorse possibili
- Risorse necessarie:

| Risorsa    | *b* Tot | Combinazioni |
|------------|---------|--------------|
| Carbonio   | 9       |              |
| Idrogeno   | 7       |              |
| Ossigeno   | 7       |              |
| Silicio    | 3       |              |
| Acqua      |         | 3            |
| Vita       |         | 3            |
| Diamante   |         | 2            |
| Robot      |         | 2            |
| Delfino    |         | 1            |
| AI-Partner |         | 1            |

        Pianeti necessari: A (*C*), A (*H*), A (*O*), D (*Si* ...), 3C (*x*, *Si*)

- Massimizzare la quantità di una risorsa specifica (es Ai girlfriend)
  - **Complesse**:
    - Acqua: 2A (*O*) + 2A (*H*) + 3C (*x*:*A*)
      1A (*O*) +1A (*H*) + 2C (*x*:*A*, *b*:*O*)+ 2C (*x*:*A*, *b*:*H*)+ 1C (*x*:*A*, *b*: \_ )
    - Vita: 1A (*O*) + 1A (*H*) + 1A (*C*) +1C (*x*: *A*, *L*, *b*: *O*)+1C (*x*: *A*, *L*, *b*: *H*)+1C (*x*: *A*, *L*, *b*: *C*) +1C (*x*: *A*, *L*, *b*: \_ )
    - Diamante: 2A (*C*) + 5C (*x*: *D*, *b*:*C*)
    - Robot: ~~1A (*Si*)+1A (*H*)+1A (*O*)+1A (*C*)+ 1C (*x*:*L*, *R*, *b*:*H*) +1C (*x*:*L*, *R*, *b*:*O*)+1C (*x*:*L*, *R*, *b*:*C*)~~
      3D (*b*:*Si*, *H*, *O*, *C*)+ 4C (*x*:*L*, *R*, *b*: \_ )
    - Delfino: 1A (*H*)+1A (*O*) + 1C (C, H, O) + 3D (*x*:*W*, *L*, *Do*, *b*:*C*)
    - AI-partner: 1A (*C*)+2 (3?)C(*b*:*Si*, *H*, *O*)+1D (*x*:*R*, *L*, *D*, *b*:*O*)+1D (*x*:*R*, *L*, *D*, *AI*, *b*:*H*)+1D (*x*:*R*, *L*, *D*, *AI*, *b*:*C*)+1D (*x*:*R*, *L*, *D*, *AI*, *b*:*Si*)
- Collezionare tutte le risorse base
  - 1A (*O*)+1A (*H*)+1A (*C*)+1A (*Si*)+3C (*b*:*O*, *H*, *C*, *Si*)
- Collezionare tutte le risorse complesse
  - 1A (*C*)+3C (*b*:*H*, *O*, *C*, *Si*) + 3D (*x*:*A*, *L*, *D*, *R*, *Do*, *AI*)
- Produrre più risorse possibili minimizzando l'attesa:
  - Devono restare più pianeti possibili in grado di accumulare energy cell: 7A
- Consumare completamente tutti i pianeti (energy cell)
  Tutti pianeti con solo una energy cell: B o C
- Identificare i nodi critici del grafo, quelli che se venissero distrutti potrebbero dividere il grafo
  Per questo scopo è necessario avere pianeti forti e pianeti deboli:
  4 (o 5) A (o C) + 3 (o 2) B (o D)
- Massimizzare il throuput delle risorse complesse senza dover continuamente spostarsi tra i vari pianeti
  4C + 3D
- Morire il prima possibile
  7 tra B o D

## Cose da tenere a mente per la scelta

**Ricette Complessa "Collo di Bottiglia"**
**ai del pianeta che privilegia avere sempre missili pronti o preferisce accumulare energy cell per l'explorer**
**robustezza nella concorrenza**
