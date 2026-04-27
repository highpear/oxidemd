# Mermaid Rendering Evaluation

Use this file for manual Mermaid rendering checks and perf log sampling.

## Flowchart

```mermaid
flowchart LR
    Open[Open Markdown] --> Parse[Parse document]
    Parse --> Render[Render Mermaid SVG]
    Render --> Cache{Cache hit?}
    Cache -->|Yes| Show[Show cached SVG]
    Cache -->|No| Worker[Background worker]
    Worker --> Show
```

## Sequence

```mermaid
sequenceDiagram
    participant User
    participant OxideMD
    participant Worker
    User->>OxideMD: Open document
    OxideMD->>Worker: Queue diagram render
    Worker-->>OxideMD: SVG result
    OxideMD-->>User: Repaint diagram
```

## Class

```mermaid
classDiagram
    class DiagramRenderCache {
        +prepare()
        +clear()
    }
    class DiagramWorkerResult {
        +source
        +result
    }
    DiagramRenderCache --> DiagramWorkerResult
```

## State

```mermaid
stateDiagram-v2
    [*] --> Pending
    Pending --> Ready: render finished
    Pending --> Error: render failed
    Ready --> [*]
    Error --> [*]
```

## Invalid

```mermaid
flowchart TD
    Broken -->
```

## Larger Flowchart

```mermaid
flowchart TD
    N01[Node 01] --> N02[Node 02]
    N02 --> N03[Node 03]
    N03 --> N04[Node 04]
    N04 --> N05[Node 05]
    N05 --> N06[Node 06]
    N06 --> N07[Node 07]
    N07 --> N08[Node 08]
    N08 --> N09[Node 09]
    N09 --> N10[Node 10]
    N10 --> N11[Node 11]
    N11 --> N12[Node 12]
    N12 --> N13[Node 13]
    N13 --> N14[Node 14]
    N14 --> N15[Node 15]
    N15 --> N16[Node 16]
    N16 --> N17[Node 17]
    N17 --> N18[Node 18]
    N18 --> N19[Node 19]
    N19 --> N20[Node 20]
```
