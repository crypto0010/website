# DCL CUDA Explorer — Web UI Design Spec

**Date:** 2026-03--21
**Status:** Approved
**Scope:** Interactive web application for testing and learning DCL CUDA operations

## Overview

A self-contained web application (`dcl-web/` crate) that provides an interactive browser-based UI for experimenting with Dynamic Coprime Labeling operations. It serves three audiences simultaneously: researchers testing parameters, students learning DCL theory, and paper reviewers verifying published results.

The architecture is **Axum + Embedded HTML**: a lightweight Rust web server exposing REST API endpoints that call dcl-gpu (CUDA) or dcl-core (CPU fallback), serving a single self-contained HTML file embedded via `include_str!`.

## Architecture

### Crate Structure

```
dcl-web/
├── Cargo.toml
├── src/
│   ├── main.rs              # Server startup, router setup
│   ├── api/
│   │   ├── mod.rs
│   │   ├── gcd.rs           # POST /api/gcd
│   │   ├── power_map.rs     # POST /api/power-map
│   │   ├── coprime.rs       # POST /api/coprime-check
│   │   ├── evolve.rs        # POST /api/evolve
│   │   ├── repair.rs        # POST /api/repair
│   │   ├── sieve.rs         # POST /api/prime-sieve
│   │   └── health.rs        # GET /api/health
│   ├── gpu_backend.rs       # GPU/CPU fallback wrapper
│   └── benchmark_data.rs    # Pre-computed brute-force & NIST results
└── static/
    └── index.html           # Single self-contained HTML file
```

### Dependencies

```toml
[dependencies]
dcl-core = { path = "../dcl-core" }
dcl-gpu = { path = "../dcl-gpu" }
dcl-crypto = { path = "../dcl-crypto" }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

## Backend — Axum REST API

### Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Serves embedded index.html |
| GET | `/api/health` | GPU availability, device info |
| POST | `/api/gcd` | Batch GCD computation |
| POST | `/api/power-map` | Power map transformation |
| POST | `/api/coprime-check` | Edge coprimality verification |
| POST | `/api/evolve` | Multi-step DCL evolution |
| POST | `/api/repair` | Coprimality repair |
| POST | `/api/prime-sieve` | Safe prime generation |

### Request/Response Format

All endpoints accept and return JSON.

**Batch GCD:**
```json
// Request
{ "pairs": [[12, 8], [35, 49], [97, 13]] }

// Response
{ "results": [{"a":12,"b":8,"gcd":4,"coprime":false}, ...],
  "backend": "gpu", "time_us": 42 }
```

**Evolve:**
```json
// Request
{ "graph": "P5", "labels": [2,3,5,7,11], "exponent": 2,
  "modulus": null, "steps": 3 }

// Response
{ "steps": [{"t":0,"labels":[2,3,5,7,11],"coprime":true},
            {"t":1,"labels":[4,9,25,49,121],"coprime":true}, ...],
  "backend": "gpu", "time_us": 150 }
```

**Health:**
```json
{ "gpu_available": true, "device": "RTX 3050",
  "cuda_cores": 2048, "cpu_fallback": true }
```

### GPU Fallback Logic (`gpu_backend.rs`)

- Attempt `CudaContext::new()` at startup
- GPU available → use CUDA kernels, report `"backend": "gpu"`
- GPU unavailable → fall back to dcl-core CPU functions, report `"backend": "cpu"`
- Health endpoint exposes active mode

### Graph Input Format

Presets send type + parameter:
```json
{ "graph_type": "path", "n": 5 }
```

Custom graphs send full topology:
```json
{ "graph_type": "custom",
  "vertices": [2, 3, 5, 7, 11],
  "edges": [[0,1], [1,2], [2,3], [3,4]] }
```

Server constructs graphs using `dcl_core::Graph::path()`, `::cycle()`, etc. for presets.

## Frontend — Single HTML Page

### Page Sections

**1. Header/Hero**
- Title: "DCL CUDA Explorer"
- GPU status indicator (green = GPU, yellow = CPU fallback)
- Links to research papers

**2. Learn Section**
- Collapsible theory cards:
  - "What is Coprime Labeling?" — definition, visual example on P₅
  - "The Power Map g(x) = xᵐ" — evolution mechanics, Theorem 2
  - "GPU Acceleration" — why batch operations are embarrassingly parallel
- Each card has a "Try it" button linking to the relevant interactive panel

**3. Interactive Playground (6 panels)**

Each panel has consistent layout: title + help tooltip, brief explanation, input form, "Run" button, output area with timing and backend indicator.

The 6 interactive operations:
- **Batch GCD** — input pairs, output table with gcd and coprime flag
- **Power Map** — input labels/exponent/modulus, output before/after comparison
- **Coprime Check** — input graph + labels, output per-edge gcd with pass/fail highlighting
- **Evolve** — input graph/labels/exponent/steps, output step-by-step evolution table with animation (centerpiece)
- **Coprime Repair** — input non-coprime labels, output repaired labels with diff
- **Prime Sieve** — input target count, output safe primes with statistics

**4. Graph Workspace**
- Preset gallery: P₅, C₆, W₅, K₄, Q₃ with adjustable n
- Visual SVG editor (details in Graph Editor section below)
- Shared graph state feeds into Coprime Check, Evolve, and Repair panels

**5. Benchmark Dashboard (Pre-computed)**
- GPU vs CPU scaling chart (Table IX / Figure 4 from paper)
- Kernel throughput bar chart (Figure 5)
- NIST SP 800-22 results table (Table VII)
- Brute-force search results (Table XIII)
- All data embedded as JSON constants

**6. Footer**
- Repository link, paper citations, "Built with Rust + CUDA + Axum"

### Visual Style

- Dark theme (developer-friendly)
- Monospace font for numbers and code
- Color coding: green = coprime/pass, red = not coprime/fail
- No external CSS/JS dependencies — all inline
- Responsive for desktop (primary) and tablet

### JavaScript Architecture

- Vanilla JS with `fetch()` for API calls
- Each panel is a self-contained module (init, event handlers, render)
- SVG graph rendering via DOM manipulation
- Shared `currentGraph` state object
- Loading spinner overlay per panel during API calls

## Graph Editor & Visualization

### SVG Rendering

Force-directed layout (~50 lines JS spring simulation):
- **Vertices**: circles r=24px, label centered inside
- **Edges**: straight lines connecting vertex centers
- **Colors**:
  - Default vertex: `#3b82f6` (blue)
  - Coprime edge: `#22c55e` (green)
  - Non-coprime edge: `#ef4444` (red, dashed)
  - Selected vertex: `#f59e0b` (amber outline)

### Editor Interactions

- **Add vertex**: click empty canvas → new vertex with next prime label
- **Add edge**: click vertex A, then vertex B → toggle edge
- **Edit label**: double-click vertex → inline number input
- **Delete**: right-click vertex/edge → remove
- **Drag**: mousedown + drag to reposition vertex
- **Presets**: replace graph entirely, auto-compute coprime labels

### Evolution Animation

- Labels update in SVG with fade transition
- Timeline slider to scrub through steps
- Each step shows: step number, all labels, coprimality status
- Labels exceeding display width shown in scientific notation (e.g., `3.4×10¹³`)
- Play/pause button auto-advances at 1-second intervals

## Error Handling

### API Error Format

```json
{
  "error": "Labels must be positive integers (got 0 for vertex 2)",
  "code": "INVALID_INPUT"
}
```

Error codes: `INVALID_INPUT`, `OVERFLOW`, `GPU_ERROR`, `TIMEOUT`

### Input Validation

**Client-side (immediate):**
- Labels ≥ 1 (positive integers)
- Exponent m ≥ 2
- Graph must have ≥ 1 edge for coprime operations
- Batch GCD: max 10,000 pairs
- Evolve: max 100 vertices, max 50 steps

**Server-side (enforced):**
- Same constraints re-validated
- Request size limit: 1 MB
- Computation timeout: 5 seconds

### Graceful Degradation

- No GPU → CPU mode banner with suggestion to install CUDA
- API unreachable → pre-computed benchmarks still visible, interactive panels show "Server offline" with start instructions
- Overflow → stop at overflow step, suggest modular arithmetic with "Retry with modulus" button (pre-fills 65537)

### Educational Error Messages

Errors include learning context:
> "Edge (v₂, v₃) has labels 4 and 6. gcd(4, 6) = 2 ≠ 1, so these are NOT coprime. Try changing one label to a prime number — primes are coprime to most integers!"

## Test Strategy

- **Backend unit tests**: each API endpoint with valid/invalid inputs
- **GPU fallback**: verify CPU produces identical results when GPU unavailable
- **Integration**: end-to-end request/response cycle
- **Pre-computed data**: verify embedded benchmark JSON matches paper tables
- **Frontend**: manual testing of graph editor interactions and evolution animation

## Startup

```
$ cargo run -p dcl-web
🌐 DCL CUDA Explorer running at http://localhost:3000
🔥 GPU: NVIDIA GeForce RTX 3050 (2048 cores)
```
