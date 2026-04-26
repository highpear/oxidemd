# Mermaid Diagram Fallback

This file checks Mermaid fenced block handling before SVG rendering is available.

```mermaid
graph TD
    Start[Open Markdown] --> Parse[Parse Mermaid block]
    Parse --> Fallback[Show readable source]
    Fallback --> Copy[Copy source]
```

Text after the diagram should keep normal document spacing.

```mmd
sequenceDiagram
    participant User
    participant OxideMD
    User->>OxideMD: Open file
    OxideMD-->>User: Show fallback
```
