# Screen Detection

Rust project that detects semantic changes on web pages and drives an intelligent agent to interact with them (form filling, clicking, navigating). Uses Playwright (via Node.js subprocess) for DOM extraction.

## Architecture

Layered pipeline — each layer transforms and enriches the previous:

```
Raw DOM (JSON from Playwright/Node.js)
  -> Screen Semantics (forms, inputs, actions, outputs)
    -> Screen State (stable identity IDs per element)
      -> Canonical State (BTreeMap, diff-friendly)
        -> Semantic Diff (high-level signals: ScreenLoaded, FormSubmitted, etc.)
          -> Agent Decision (via Ollama or Mock backend)
```

## Module Map

| Module | Key Files | Purpose |
|--------|-----------|---------|
| `browser` | `playwright.rs` | Spawns Node.js (`../../node/dom-extraction/extract.js`) to extract DOM JSON |
| `screen` | `classifier.rs`, `intent.rs`, `screen_model.rs` | Classifies DOM elements into forms/inputs/actions/outputs; infers form intent |
| `state` | `state_builder.rs`, `identity.rs`, `diff.rs`, `normalize.rs` | Builds screen state with stable semantic element IDs; computes identity-based diffs |
| `canonical` | `canonical_model.rs`, `diff.rs` | Canonicalizes state into BTreeMap; derives semantic signals from diffs |
| `agent` | `agent.rs`, `agent_model.rs`, `ai_model.rs`, `budget.rs` | State machine (Observe->Evaluate->Think->Act->Stop); multi-layer gating; pluggable LLM backends |
| `trace` | `trace.rs`, `logger.rs` | JSONL trace logging with builder-pattern TraceEvent |

## Key Design Decisions

- **Stable identity system**: Elements get semantic IDs like `form:{id}:{kind}:{label}` or `screen:output:{region}:{sha1_hash}` that persist across snapshots for reliable diffing.
- **Signal-based semantics**: Low-level diffs are converted to high-level signals (`ScreenLoaded`, `FormSubmitted`, `ResultsAppeared`, `ErrorAppeared`, `NavigationOccurred`, `NoOp`) that drive agent decisions.
- **Multi-layer gating**: Agent actions must pass confidence threshold (0.65), loop suppression (max 2 repeats), budget checks (5 think / 3 retry / 2 loop), and terminal-success gates before execution.
- **Pluggable inference**: `ModelBackend` trait with `OllamaBackend` (qwen2.5:1.5b at localhost:11434) and `MockBackend` (deterministic, for tests).
- **Agent state machine**: Deterministic progression Observe -> Evaluate -> Think -> Act -> (back to Observe or Stop).

## Constants

- `MIN_CONFIDENCE`: 0.65
- `MAX_THINK_STEPS`: 5
- `MAX_RETRIES`: 3
- `MAX_LOOP_REPEATS`: 2
- Ollama model: `qwen2.5:1.5b` at `http://localhost:11434/api/generate`
- Form intent thresholds: >0.7 = Authentication, >0.4 = User Input

## Build & Test

```bash
cargo build
cargo test
```

- Rust edition 2024
- Dependencies: serde, serde_json, sha1, reqwest (blocking + json)
- Tests use HTML fixtures in `tests/fixtures/` (01-10) loaded via `file://` URLs
- Test helpers in `tests/common/` provide `diff_between_pages()`, `diff_static()`, etc.
- `MockBackend` enables deterministic agent tests without Ollama running

## External Dependencies

- Node.js + Playwright required for DOM extraction (path: `../../node/dom-extraction/extract.js` relative to project root)
- Ollama running locally for real inference (not needed for tests)

## Agent Actions

`FillInput`, `SubmitForm`, `FormSubmitted` (observed, not executed), `ClickAction`, `Wait`

## File Layout

```
src/
  lib.rs              -- Pipeline orchestration (run_app)
  main.rs             -- Entry point
  agent/
    agent.rs          -- State machine, gating, execution
    agent_model.rs    -- AgentState, AgentAction, AgentMemory, ModelDecision
    ai_model.rs       -- OllamaBackend, MockBackend, prompt construction
    budget.rs         -- Budget check (think/retry/loop)
  browser/
    playwright.rs     -- Node.js subprocess for DOM extraction
  canonical/
    canonical_model.rs -- CanonicalScreenState, canonicalize()
    diff.rs           -- SemanticStateDiff, signal derivation
  screen/
    classifier.rs     -- classify() DOM -> ScreenSemantics
    intent.rs         -- infer_form_intent() heuristic scoring
    screen_model.rs   -- DomElement, Form, ScreenElement, FormIntent
  state/
    state_builder.rs  -- build_state(), resolve_identities()
    identity.rs       -- IdentifiedElement, form_key(), element_key()
    diff.rs           -- StateDiff (added/removed/unchanged)
    normalize.rs      -- Text normalization, region inference, volatility
  trace/
    trace.rs          -- TraceEvent builder
    logger.rs         -- JSONL file logger (Mutex<File>)
tests/
  test.rs             -- 16 integration tests
  common/
    utils.rs          -- page() helper, is_terminal_success()
    semantic_diff.rs  -- diff helpers (run_diff_from_raw, diff_between_pages, etc.)
  fixtures/           -- HTML test pages (01-10)
```
