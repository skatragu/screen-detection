# Screen Detection

Rust project that detects semantic changes on web pages and drives an intelligent agent to interact with them (form filling, clicking, navigating). Uses Playwright (via Node.js subprocess) for DOM extraction and browser interaction.

## Architecture

Layered pipeline — each layer transforms and enriches the previous:

```
Raw DOM (JSON from Playwright/Node.js)
  -> Screen Semantics (forms, inputs, actions, outputs)
    -> Screen State (stable identity IDs per element)
      -> Canonical State (BTreeMap, diff-friendly)
        -> Semantic Diff (high-level signals: ScreenLoaded, FormSubmitted, etc.)
          -> Agent Decision (via Ollama or Mock backend)
            -> Browser Action (via interact.js subprocess)
```

## Module Map

| Module | Key Files | Purpose |
|--------|-----------|---------|
| `browser` | `playwright.rs` | Spawns Node.js (`extract.js` for DOM extraction, `interact.js` for browser actions); defines `SelectorHint`, `BrowserCommand`, `BrowserResult` |
| `screen` | `classifier.rs`, `intent.rs`, `screen_model.rs` | Classifies DOM elements into forms/inputs/actions/outputs; infers form intent |
| `state` | `state_builder.rs`, `state_model.rs`, `identity.rs`, `diff.rs`, `normalize.rs` | Builds screen state with stable semantic element IDs; computes identity-based diffs; defines `ScreenState` and `Outcome` |
| `canonical` | `canonical_model.rs`, `diff.rs` | Canonicalizes state into BTreeMap; derives semantic signals from diffs |
| `agent` | `agent.rs`, `agent_model.rs`, `ai_model.rs`, `budget.rs`, `error.rs` | State machine (Observe->Evaluate->Think->Act->Stop); multi-layer gating; pluggable LLM backends with `DeterministicPolicy` and `HybridPolicy`; typed `AgentError` enum |
| `trace` | `trace.rs`, `logger.rs` | JSONL trace logging with builder-pattern TraceEvent |

## Key Design Decisions

- **Stable identity system**: Elements get semantic IDs like `form:{id}:{kind}:{label}` or `screen:output:{region}:{sha1_hash}` that persist across snapshots for reliable diffing. Volatile outputs fall back to `screen:output:{region}:idx:{n}`.
- **Signal-based semantics**: Low-level diffs are converted to high-level signals (`ScreenLoaded`, `FormSubmitted`, `ResultsAppeared`, `ErrorAppeared`, `NavigationOccurred`, `NoOp`) that drive agent decisions.
- **Multi-layer gating**: Agent actions must pass confidence threshold (0.65), loop suppression (max 2 repeats), budget checks (5 think / 3 retry / 2 loop), and terminal-success gates before execution. Loop budget resets to 5 on signal detection.
- **Pluggable inference**: `ModelBackend` trait with `OllamaBackend` (qwen2.5:1.5b at localhost:11434), `MockBackend` (deterministic, for tests), plus `DeterministicPolicy` (rule-based with `guess_value()` heuristics) and `HybridPolicy` (combines deterministic + model). Agent uses `Box<dyn Policy>` and can be constructed with `Agent::with_deterministic()`, `Agent::with_hybrid()`, `Agent::with_mock()`, `Agent::with_policy()`.
- **Smart form filling**: `guess_value(label, input_type)` derives sensible values from input labels (email → `user@example.com`, password → `TestPass123!`, search/query → `test query`, etc.) with fallback to input_type and final fallback `"test"`. `DeterministicPolicy` uses `FillAndSubmitForm` to fill all inputs + submit in one agent step.
- **Agent state machine**: Deterministic progression Observe -> Evaluate -> Think -> Act -> (back to Observe or Stop).
- **Browser interaction**: Agent actions are translated to `BrowserCommand` structs with `SelectorHint` for element targeting, then executed via `interact.js` Node.js subprocess.

## Constants

- `MIN_CONFIDENCE`: 0.65
- `MAX_THINK_STEPS`: 5
- `MAX_RETRIES`: 3
- `MAX_LOOP_REPEATS`: 2
- Ollama model: `qwen2.5:1.5b` at `http://localhost:11434/api/generate`
- `MAX_ITERATIONS`: 20 (agent loop safety limit in run_app)
- Form intent thresholds: >0.7 = Authentication, >0.4 = User Input
- `TARGET_URL` env var: overrides default URL (google.com) for run_app

## Build & Test

```bash
cargo build
cargo test
```

- Rust edition 2024
- Dependencies: serde (1.0.228 with derive), serde_json (1.0.145), sha1 (0.10.6), reqwest (0.12 with json + blocking)
- Tests use HTML fixtures in `tests/fixtures/` (01-10) loaded via `file://` URLs
- Test helpers in `tests/common/` provide `diff_between_pages()`, `diff_static()`, etc.
- `MockBackend` enables deterministic agent tests without Ollama running
- 30 integration tests covering classification, diffing, signals, agent behavior, browser command serialization, policy logic, guess_value heuristics, and error handling

## External Dependencies

- Node.js + Playwright required for DOM extraction (`../../node/dom-extraction/extract.js` relative to project root)
- Node.js + Playwright required for browser actions (`../../node/dom-extraction/interact.js` relative to project root)
- Ollama running locally for real inference (not needed for tests)

## Agent Actions

`FillInput { form_id, input_label, value, identity }`, `SubmitForm { form_id, action_label, identity }`, `FillAndSubmitForm { form_id, values: Vec<(label, value)>, submit_label }` (composite: fills all inputs then submits), `FormSubmitted { form_id }` (observed, not executed), `ClickAction { label, identity }`, `Wait { reason }`

## Key Types

- `SelectorHint`: ARIA role, name, tag, input_type, form_id — used to locate elements in the browser
- `BrowserCommand`: action, url, value, selector, duration_ms — sent to interact.js
- `BrowserResult`: success, error — returned from interact.js
- `ScreenState`: url, title, forms, standalone_actions, outputs, identities
- `Outcome`: NoChange, FormSubmissionSucceeded, FormSubmissionFailed, Navigation, Unknown
- `OutputRegion`: Header, Main, Results, Footer, Modal, Unknown
- `Volatility`: Stable, Volatile
- `DecisionType`: Act, Wait, Stop
- `BudgetDecision`: Allow, Block
- `AgentError`: SubprocessSpawn, SubprocessFailed, JsonParse, JsonSerialize, BrowserAction, DomStructure, ElementNotFound, MissingState — typed error enum replacing ad-hoc String errors; implements `Display` and `std::error::Error` with source chaining

## File Layout

```
src/
  lib.rs              -- Agent loop orchestration (run_app), snapshot() helper
  main.rs             -- Entry point
  agent/
    mod.rs            -- Module exports
    agent.rs          -- State machine, gating, execution, emit_observed_actions()
    agent_model.rs    -- AgentState, AgentAction (incl. FillAndSubmitForm), AgentMemory, ModelDecision, Policy trait
    ai_model.rs       -- OllamaBackend, MockBackend, DeterministicPolicy, HybridPolicy, guess_value(), prompt construction
    budget.rs         -- Budget check (think/retry/loop)
    error.rs          -- AgentError enum (8 variants), Display, std::error::Error impls
  browser/
    mod.rs            -- Module exports
    playwright.rs     -- DOM extraction (returns Result), SelectorHint, BrowserCommand, BrowserResult, execute_browser_action()
  canonical/
    mod.rs            -- Module exports
    canonical_model.rs -- CanonicalScreenState, canonicalize()
    diff.rs           -- SemanticStateDiff, SemanticSignal, signal derivation
  screen/
    mod.rs            -- Module exports
    classifier.rs     -- classify() DOM -> ScreenSemantics
    intent.rs         -- infer_form_intent() heuristic scoring
    screen_model.rs   -- DomElement, Form, ScreenElement, FormIntent, ElementKind
  state/
    mod.rs            -- Module exports
    state_builder.rs  -- build_state(), resolve_identities()
    state_model.rs    -- ScreenState, Outcome
    identity.rs       -- IdentifiedElement, form_key(), element_key()
    diff.rs           -- StateDiff (added/removed/unchanged)
    normalize.rs      -- Text normalization, region inference (OutputRegion), volatility detection
  trace/
    mod.rs            -- Module exports
    trace.rs          -- TraceEvent builder
    logger.rs         -- JSONL file logger (Mutex<File>), gracefully degrades if file unavailable
tests/
  test.rs             -- 30 integration tests
  common/
    mod.rs            -- Module exports
    utils.rs          -- page() helper, is_terminal_success()
    semantic_diff.rs  -- diff helpers (run_diff_from_raw, diff_between_pages, etc.)
  fixtures/           -- HTML test pages (01-10)
```

## Node.js Scripts (../../node/dom-extraction/)

- `extract.js` — Launches Playwright, navigates to URL, extracts DOM as JSON (elements with tag, id, text, attributes, children)
- `interact.js` — Receives a BrowserCommand JSON arg, launches Playwright, executes fill/click/wait actions using SelectorHint for element targeting
- Playwright dependency: `^1.58.2`
