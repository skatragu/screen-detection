# Screen Detection

Rust project that detects semantic changes on web pages and drives an intelligent agent to interact with them (form filling, clicking, navigating). Uses Playwright (via Node.js subprocess) for DOM extraction and browser interaction. Evolving into an AI-powered autonomous UI testing tool.

## Architecture

Layered pipeline — each layer transforms and enriches the previous:

```
Raw DOM (JSON from Playwright/Node.js)
  -> Screen Semantics (forms, inputs, actions, outputs)
    -> Screen State (stable identity IDs per element)
      -> Canonical State (BTreeMap, diff-friendly)
        -> Semantic Diff (high-level signals: ScreenLoaded, FormSubmitted, etc.)
          -> Agent Decision (via Ollama or Mock backend)
            -> Browser Action (via interact.js subprocess OR persistent BrowserSession)
```

## Module Map

| Module | Key Files | Purpose |
|--------|-----------|---------|
| `browser` | `playwright.rs`, `session.rs` | Spawns Node.js for browser interaction; `playwright.rs` = per-action subprocesses (extract.js, interact.js); `session.rs` = persistent session via browser_server.js NDJSON protocol |
| `screen` | `classifier.rs`, `intent.rs`, `screen_model.rs` | Classifies DOM elements into forms/inputs/actions/outputs; infers form intent |
| `state` | `state_builder.rs`, `state_model.rs`, `identity.rs`, `diff.rs`, `normalize.rs` | Builds screen state with stable semantic element IDs; computes identity-based diffs; defines `ScreenState` and `Outcome` |
| `canonical` | `canonical_model.rs`, `diff.rs` | Canonicalizes state into BTreeMap; derives semantic signals from diffs |
| `agent` | `agent.rs`, `agent_model.rs`, `ai_model.rs`, `budget.rs`, `error.rs`, `page_model.rs`, `page_analyzer.rs` | State machine (Observe->Evaluate->Think->Act->Stop); multi-layer gating; pluggable LLM backends with `DeterministicPolicy` and `HybridPolicy`; typed `AgentError` enum; both subprocess and session execution; AI page understanding via `PageAnalyzer` trait |
| `spec` | `spec_model.rs`, `context.rs`, `runner.rs` | Test spec data model (TestSpec, TestStep, AssertionSpec); TestRunner executes specs via BrowserSession; TestContext tracks assertion results |
| `explorer` | `app_map.rs`, `explorer.rs`, `test_generator.rs` | Autonomous exploration and test generation; AppMap page graph (PageNode, Transition); BFS multi-page crawling via BrowserSession; TestSpec generation from PageModel (smoke + form tests) |
| `trace` | `trace.rs`, `logger.rs` | JSONL trace logging with builder-pattern TraceEvent |

## Key Design Decisions

- **Stable identity system**: Elements get semantic IDs like `form:{id}:{kind}:{label}` or `screen:output:{region}:{sha1_hash}` that persist across snapshots for reliable diffing. Volatile outputs fall back to `screen:output:{region}:idx:{n}`.
- **Signal-based semantics**: Low-level diffs are converted to high-level signals (`ScreenLoaded`, `FormSubmitted`, `ResultsAppeared`, `ErrorAppeared`, `NavigationOccurred`, `NoOp`) that drive agent decisions.
- **Multi-layer gating**: Agent actions must pass confidence threshold (0.65), loop suppression (max 2 repeats), budget checks (5 think / 3 retry / 2 loop), and terminal-success gates before execution. Loop budget resets to 5 on signal detection.
- **Pluggable inference**: `ModelBackend` trait with `OllamaBackend` (qwen2.5:1.5b at localhost:11434), `MockBackend` (deterministic, for tests), plus `DeterministicPolicy` (rule-based with `guess_value()` heuristics) and `HybridPolicy` (combines deterministic + model). Agent uses `Box<dyn Policy>` and can be constructed with `Agent::with_deterministic()`, `Agent::with_hybrid()`, `Agent::with_mock()`, `Agent::with_policy()`.
- **AI page understanding**: `PageAnalyzer` trait with `MockPageAnalyzer` (rule-based) and `LlmPageAnalyzer` (LLM-backed via `TextInference` trait). Analyzes `ScreenState` → `PageModel` with page category, form field classification, suggested test values, suggested assertions, and navigation targets. `LlmPageAnalyzer` falls back to `MockPageAnalyzer` on parse failure.
- **Field type classification**: `classify_field_type(input_type, label)` maps HTML input types and label keywords to semantic `FieldType` (Email, Password, Number, Date, Tel, Url, Checkbox, Radio, Select, Text). Used by `MockPageAnalyzer` to enrich form analysis.
- **Smart form filling**: `guess_value(label, input_type)` derives sensible values from input labels (email → `user@example.com`, password → `TestPass123!`, search/query → `test query`, etc.) with fallback to input_type and final fallback `"test"`. `DeterministicPolicy` uses `FillAndSubmitForm` to fill all inputs + submit in one agent step.
- **Agent state machine**: Deterministic progression Observe -> Evaluate -> Think -> Act -> (back to Observe or Stop).
- **Dual browser modes**: Subprocess mode (`execute_action` via interact.js, fresh browser per action) for backward compatibility; Session mode (`execute_action_session` via `BrowserSession`/browser_server.js, persistent browser) for multi-page flows.
- **NDJSON session protocol**: `BrowserSession` communicates with `browser_server.js` via newline-delimited JSON over stdin/stdout. Commands: navigate, extract, action (fill/click/wait), screenshot, current_url, query_text, query_visible, query_count, quit.
- **CSS selector queries**: Element assertions (ElementText, ElementVisible, ElementCount) use raw CSS selectors (`#id`, `.class`, `tag`) via browser query commands, separate from the semantic `SelectorHint` used for agent interactions.
- **Autonomous exploration**: `ExplorerConfig` controls BFS crawling (max_pages, max_depth, same_origin_only). `explore()` for offline single-page analysis, `explore_live()` for multi-page BFS via `BrowserSession`. Builds `AppMap` graph of `PageNode`s + `Transition`s.
- **Test generation**: `generate_test_plan(app_map)` produces `Vec<TestSpec>` from explored pages. Per page: smoke test (wait + assert suggested_assertions) + per-form test (fill_and_submit with AI-suggested values). `map_suggested_assertion()` bridges `SuggestedAssertion` → `AssertionSpec`.

## Constants

- `MIN_CONFIDENCE`: 0.65
- `MAX_THINK_STEPS`: 5
- `MAX_RETRIES`: 3
- `MAX_LOOP_REPEATS`: 2
- Ollama model: `qwen2.5:1.5b` at `http://localhost:11434/api/generate`
- `MAX_ITERATIONS`: 20 (agent loop safety limit in run_app)
- Form intent thresholds: >0.7 = Authentication, >0.4 = User Input
- `TARGET_URL` env var: overrides default URL (google.com) for run_app/run_app_session

## Build & Test

```bash
cargo build
cargo test
```

- Rust edition 2024
- Dependencies: serde (1.0.228 with derive), serde_json (1.0.145), serde_yaml (0.9), sha1 (0.10.6), reqwest (0.12 with json + blocking)
- Tests use HTML fixtures in `tests/fixtures/` (01-10) loaded via `file://` URLs
- Test helpers in `tests/common/` provide `diff_between_pages()`, `diff_static()`, etc.
- `MockBackend` enables deterministic agent tests without Ollama running
- 111 integration tests split across module-based test files:
  - `tests/agent_tests.rs` (30) — agent behavior, policy logic, guess_value heuristics, budget gating, error handling
  - `tests/browser_tests.rs` (24) — BrowserRequest/BrowserResponse serialization, session error variants, NavigateTo action, query protocol
  - `tests/canonical_tests.rs` (12) — classification, diffing, signals
  - `tests/explorer_tests.rs` (15) — ExplorerConfig, AppMap data model, explore() offline, test generation (smoke/form), map_suggested_assertion, is_same_origin, resolve_url
  - `tests/page_model_tests.rs` (12) — PageModel/PageCategory/FieldType serde roundtrips, MockPageAnalyzer, LlmPageAnalyzer with mock/fallback, field classification, navigation targets
  - `tests/spec_tests.rs` (15) — TestSpec YAML/JSON roundtrips, TestContext tracking, TestResult serialization, assertion roundtrips
  - `tests/state_tests.rs` (3) — normalize edge cases, form intent scoring

## External Dependencies

- Node.js + Playwright required for DOM extraction (`../../node/dom-extraction/extract.js` relative to project root)
- Node.js + Playwright required for browser actions (`../../node/dom-extraction/interact.js` relative to project root)
- Node.js + Playwright required for persistent sessions (`../../node/dom-extraction/browser_server.js` relative to project root)
- Ollama running locally for real inference (not needed for tests)

## Agent Actions

`FillInput { form_id, input_label, value, identity }`, `SubmitForm { form_id, action_label, identity }`, `FillAndSubmitForm { form_id, values: Vec<(label, value)>, submit_label }` (composite: fills all inputs then submits), `FormSubmitted { form_id }` (observed, not executed), `ClickAction { label, identity }`, `Wait { reason }`, `NavigateTo { url, reason }` (session-only, no-op in subprocess mode)

## Key Types

- `SelectorHint`: ARIA role, name, tag, input_type, form_id — used to locate elements in the browser
- `BrowserCommand`: action, url, value, selector, duration_ms — sent to interact.js (subprocess mode)
- `BrowserResult`: success, error — returned from interact.js (subprocess mode)
- `BrowserRequest`: Navigate, Extract, Action, Screenshot, CurrentUrl, QueryText, QueryVisible, QueryCount, Quit — NDJSON commands to browser_server.js (session mode)
- `BrowserResponse`: ok, error, data, url, ready, text, visible, count — NDJSON responses from browser_server.js (session mode)
- `BrowserSession`: child, stdin, reader, current_url — persistent browser session backed by browser_server.js
- `ScreenState`: url, title, forms, standalone_actions, outputs, identities
- `Outcome`: NoChange, FormSubmissionSucceeded, FormSubmissionFailed, Navigation, Unknown
- `OutputRegion`: Header, Main, Results, Footer, Modal, Unknown
- `Volatility`: Stable, Volatile
- `DecisionType`: Act, Wait, Stop
- `BudgetDecision`: Allow, Block
- `AgentError`: SubprocessSpawn, SubprocessFailed, JsonParse, JsonSerialize, BrowserAction, DomStructure, ElementNotFound, MissingState, SessionIO, SessionProtocol — typed error enum (10 variants); implements `Display` and `std::error::Error` with source chaining
- `TestSpec`: name, start_url, steps (Vec<TestStep>) — complete test specification
- `TestStep`: FillForm, FillAndSubmit, Click, Navigate, Wait, Assert — serde-tagged enum (`#[serde(tag = "action")]`)
- `AssertionSpec`: UrlContains, UrlEquals, TitleContains, TextPresent, TextAbsent, ElementText, ElementVisible, ElementCount — serde-tagged enum (`#[serde(tag = "type")]`)
- `AssertionResult`: step_index, spec, passed, actual, message — single assertion outcome
- `TestResult`: spec_name, passed, steps_run, assertion_results, error — complete test run result
- `TestContext`: current_step, assertion_results — execution state tracker with pass/fail counting
- `TestRunner`: stateless executor — takes TestSpec + BrowserSession, returns TestResult
- `PageModel`: purpose, category (PageCategory), forms (Vec<FormModel>), outputs, suggested_assertions, navigation_targets — AI understanding of a web page
- `PageCategory`: Login, Registration, Search, Dashboard, Settings, Listing, Detail, Checkout, Error, Other — page classification enum
- `FormModel`: form_id, purpose, fields (Vec<FieldModel>), submit_label — AI understanding of a form
- `FieldModel`: label, field_type (FieldType), required, suggested_test_value — single form field with AI-suggested test value
- `FieldType`: Text, Email, Password, Number, Date, Tel, Url, Select, Checkbox, Radio, Other — semantic field classification
- `PageAnalyzer` trait: `analyze(&self, screen: &ScreenState) -> Result<PageModel, AgentError>` — pluggable page analysis
- `TextInference` trait: `infer_text(&self, prompt: &str) -> Option<String>` — generic text-in/text-out LLM interface
- `MockTextInference`: canned response for deterministic testing of LlmPageAnalyzer
- `ExplorerConfig`: start_url, max_pages (10), max_depth (3), same_origin_only (true) — controls autonomous BFS crawling
- `PageNode`: url, title, depth, page_model (PageModel) — single discovered page in the AppMap
- `Transition`: from_url, to_url, label — directed edge between pages
- `AppMap`: pages (HashMap<String, PageNode>), transitions (Vec<Transition>) — graph of discovered pages; has add_page(), add_transition(), page_count(), has_page()

## File Layout

```
src/
  lib.rs              -- Agent loop orchestration (run_app, run_app_session), snapshot(), snapshot_session()
  main.rs             -- Entry point
  agent/
    mod.rs            -- Module exports
    agent.rs          -- State machine, gating, execute_action(), execute_action_session(), emit_observed_actions()
    agent_model.rs    -- AgentState, AgentAction (incl. FillAndSubmitForm, NavigateTo), AgentMemory, ModelDecision, Policy trait
    ai_model.rs       -- OllamaBackend, MockBackend, DeterministicPolicy, HybridPolicy, guess_value(), prompt construction, TextInference trait, MockTextInference
    budget.rs         -- Budget check (think/retry/loop)
    error.rs          -- AgentError enum (10 variants), Display, std::error::Error impls
    page_model.rs     -- PageModel, PageCategory, FormModel, FieldModel, FieldType, OutputModel, SuggestedAssertion, NavigationTarget
    page_analyzer.rs  -- PageAnalyzer trait, MockPageAnalyzer (rule-based), LlmPageAnalyzer (LLM-backed with fallback), classify_field_type()
  browser/
    mod.rs            -- Module exports
    playwright.rs     -- DOM extraction (returns Result), SelectorHint, BrowserCommand, BrowserResult, execute_browser_action()
    session.rs        -- BrowserSession (persistent browser via NDJSON), BrowserRequest, BrowserResponse, query_text/query_visible/query_count for element assertions
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
  explorer/
    mod.rs            -- Module exports
    app_map.rs        -- ExplorerConfig, PageNode, Transition, AppMap (page graph data model)
    explorer.rs       -- explore() (offline single-page), explore_live() (BFS multi-page via BrowserSession), is_same_origin(), resolve_url()
    test_generator.rs -- generate_test_plan(), generate_smoke_test(), generate_form_test(), map_suggested_assertion()
  spec/
    mod.rs            -- Module exports
    spec_model.rs     -- TestSpec, TestStep, AssertionSpec, AssertionResult, TestResult
    context.rs        -- TestContext: step tracking, assertion result collection, pass/fail counting
    runner.rs         -- TestRunner: executes TestSpec via BrowserSession, evaluates assertions
  trace/
    mod.rs            -- Module exports
    trace.rs          -- TraceEvent builder
    logger.rs         -- JSONL file logger (Mutex<File>), gracefully degrades if file unavailable
tests/
  agent_tests.rs      -- 30 tests: agent behavior, policy, guess_value, budget, errors
  browser_tests.rs    -- 24 tests: BrowserRequest/Response serde, session errors, query protocol
  canonical_tests.rs  -- 12 tests: classification, diffing, signals
  explorer_tests.rs   -- 15 tests: ExplorerConfig, AppMap, explore(), test generation, assertion mapping, URL utilities
  page_model_tests.rs -- 12 tests: PageModel serde, MockPageAnalyzer, LlmPageAnalyzer, field classification
  spec_tests.rs       -- 15 tests: TestSpec YAML/JSON, TestContext, TestResult, assertions
  state_tests.rs      -- 3 tests: normalize, form intent scoring
  common/
    mod.rs            -- Module exports
    utils.rs          -- page() helper, is_terminal_success()
    semantic_diff.rs  -- diff helpers (run_diff_from_raw, diff_between_pages, etc.)
  fixtures/           -- HTML test pages (01-10)
```

## Node.js Scripts (../../node/dom-extraction/)

- `extract.js` — Launches Playwright, navigates to URL, extracts DOM as JSON (elements with tag, id, text, attributes, children). Stateless: fresh browser per call.
- `interact.js` — Receives a BrowserCommand JSON arg, launches Playwright, executes fill/click/wait actions using SelectorHint for element targeting. Stateless: fresh browser per call.
- `browser_server.js` — Long-lived NDJSON server. Launches Playwright once, accepts commands over stdin, returns responses to stdout. Supports: navigate, extract, action (fill/click/wait), screenshot, current_url, query_text, query_visible, query_count, quit. Used by `BrowserSession` in Rust for persistent multi-page sessions.
- Playwright dependency: `^1.58.2`
