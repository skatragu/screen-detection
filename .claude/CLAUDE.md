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
| `spec` | `spec_model.rs`, `context.rs`, `runner.rs`, `runner_config.rs` | Test spec data model (TestSpec, TestStep, AssertionSpec); TestRunner executes specs via BrowserSession with retry/screenshot resilience (RunnerConfig); TestContext tracks assertion results |
| `explorer` | `app_map.rs`, `explorer.rs`, `flow_detector.rs`, `test_generator.rs` | Autonomous exploration and test generation; AppMap page graph (PageNode, Transition, TransitionKind); form-aware BFS crawling via BrowserSession; flow detection across FormSubmission chains; TestSpec generation from PageModel (smoke + form + flow tests) |
| `report` | `report_model.rs`, `console.rs`, `html.rs`, `junit.rs` | Test suite reporting; `TestSuiteReport` aggregation; console output (pass/fail markers), self-contained HTML report, JUnit XML for CI |
| `cli` | `config.rs`, `commands.rs` | CLI argument parsing (clap derive) with 3 subcommands (explore, run, generate); YAML config file support; wires all modules into usable command-line tool |
| `trace` | `trace.rs`, `logger.rs` | JSONL trace logging with builder-pattern TraceEvent |

## Key Design Decisions

- **Stable identity system**: Elements get semantic IDs like `form:{id}:{kind}:{label}` or `screen:output:{region}:{sha1_hash}` that persist across snapshots for reliable diffing. Volatile outputs fall back to `screen:output:{region}:idx:{n}`.
- **Signal-based semantics**: Low-level diffs are converted to high-level signals (`ScreenLoaded`, `FormSubmitted`, `ResultsAppeared`, `ErrorAppeared`, `NavigationOccurred`, `NoOp`) that drive agent decisions.
- **Multi-layer gating**: Agent actions must pass confidence threshold (0.65), loop suppression (max 2 repeats), budget checks (5 think / 3 retry / 2 loop), and terminal-success gates before execution. Loop budget resets to 5 on signal detection.
- **Pluggable inference**: `ModelBackend` trait with `OllamaBackend` (qwen2.5:1.5b at localhost:11434), `MockBackend` (deterministic, for tests), plus `DeterministicPolicy` (rule-based with `guess_value()` heuristics) and `HybridPolicy` (combines deterministic + model). Agent uses `Box<dyn Policy>` and can be constructed with `Agent::with_deterministic()`, `Agent::with_hybrid()`, `Agent::with_mock()`, `Agent::with_policy()`.
- **AI page understanding (hybrid enrichment)**: `PageAnalyzer` trait with `MockPageAnalyzer` (rule-based) and `LlmPageAnalyzer` (LLM-backed via `TextInference` trait). `LlmPageAnalyzer` uses a simplified prompt (~120 words) asking for only `purpose` and `category`, then builds the full `PageModel` deterministically via `MockPageAnalyzer`. LLM output is parsed via `try_parse_llm_response()` with 4-stage JSON recovery (direct parse → strip markdown fences → fix trailing commas → extract JSON substring). Category strings are normalized via `map_llm_category()` (case-insensitive with aliases). Never fails — always produces a valid `PageModel`.
- **Richer DOM extraction**: `DomElement` has 6 additional optional fields (id, name, placeholder, href, value, options) plus 6 constraint fields (pattern, minlength, maxlength, min, max, readonly); `ScreenElement` has 5 additional fields (required, placeholder, id, href, options) plus 5 constraint fields (name, value, maxlength, minlength, readonly). All new fields use `#[serde(default)]` for backward compatibility. `SelectOption { value, text }` type for `<select>` dropdown options. `label_for()` uses placeholder as fallback after aria_label and text.
- **Disabled/readonly filtering**: `is_input()` in `classifier.rs` rejects disabled elements upfront (`if el.disabled { return false; }`). `MockPageAnalyzer::analyze()` filters readonly inputs (`.filter(|input| !input.readonly)`) before building FieldModels — readonly fields carry data but shouldn't be filled.
- **Field type classification**: `classify_field_type(input_type, label, tag)` maps HTML input types, label keywords, and tag names to semantic `FieldType` (15 variants: Text, Email, Password, Number, Date, Tel, Url, Select, Checkbox, Radio, Textarea, Search, Hidden, Time, Other). Tag parameter enables `<textarea>` and `<select>` detection. Used by `MockPageAnalyzer` to enrich form analysis.
- **Constraint-aware value generation**: `constrained_value(label, input_type, category, maxlength, minlength)` wraps `guess_value()` and enforces HTML constraints: truncates to `maxlength`, pads with `'x'` to `minlength`. `MockPageAnalyzer` uses it when filling form fields, ensuring generated values are always valid for the field's declared constraints.
- **Smart dropdown selection**: `smart_select_option(options)` skips placeholder-like entries (empty value, text starting with "select"/"choose"/"--"/"please", text "none"/"n/a") and returns the first real option. Falls back to `options.first()` if all are placeholders. Replaces blind first-option selection in `MockPageAnalyzer`.
- **Output semantic classification**: `classify_output_semantic(text)` assigns `OutputSemantic` (Error, Success, Warning, Navigation, Info) to page output text via keyword matching. `OutputModel` gains a `semantic` field (`#[serde(default)]` → `Info`) so consumers can distinguish errors from success messages without re-parsing text.
- **Field-type-aware SelectorHint**: `build_selector_for_field(field, form_id)` maps `FieldType` to the correct ARIA role + HTML tag + input_type for Playwright element targeting (e.g., Select → role="combobox"/tag="select", Textarea → role="textbox"/tag="textarea", Number → role="spinbutton"). `submit_form_in_session()` uses it instead of hardcoding role="textbox" for all inputs. Hidden fields are skipped; Radio uses `session.check()`.
- **Smart form ranking**: `rank_form(form)` scores forms by input count (up to 0.5), primary action presence (0.3), and intent confidence (up to 0.3). Returns 0.0 for empty forms. `select_best_form(forms)` picks the highest-ranked non-empty form. `DeterministicPolicy::decide()` uses it instead of blind `forms.first()`, ensuring multi-form pages target the main interactive form.
- **Category-aware filling**: `infer_page_category(screen)` in `ai_model.rs` derives `PageCategory` from form intents + title keywords (avoids circular import with `page_analyzer.rs`). `DeterministicPolicy::decide()` passes `Some(&category)` to `guess_value()`, so Login pages get existing-user credentials, Registration pages get new-user values, etc.
- **Smart form filling**: `guess_value(label, input_type, category)` derives sensible values from ~26 label patterns (email, password, name, address, phone, card number, CVV, city, state, country, company, birthday, etc.) with input_type fallbacks (search, time, month, week, color, range, checkbox, radio) and final fallback `"test"`. When `PageCategory` is provided, returns context-aware values: Login → existing user credentials, Registration → new user values, Search → meaningful queries, Checkout → billing data, Settings → update values. `DeterministicPolicy` uses `FillAndSubmitForm` to fill all inputs + submit in one agent step.
- **Enhanced browser interaction**: `select_option`, `check`, `uncheck` commands in the NDJSON browser protocol enable dropdown selection, checkbox toggling, and radio button interaction. `submit_form_in_session()` dispatches to the appropriate interaction method based on `FieldType` (Select → select_option, Checkbox → check, others → fill).
- **Agent state machine**: Deterministic progression Observe -> Evaluate -> Think -> Act -> (back to Observe or Stop).
- **Dual browser modes**: Subprocess mode (`execute_action` via interact.js, fresh browser per action) for backward compatibility; Session mode (`execute_action_session` via `BrowserSession`/browser_server.js, persistent browser) for multi-page flows.
- **NDJSON session protocol**: `BrowserSession` communicates with `browser_server.js` via newline-delimited JSON over stdin/stdout. Commands: navigate, extract, action (fill/click/wait/select/check/uncheck), screenshot, current_url, query_text, query_visible, query_count, quit.
- **CSS selector queries**: Element assertions (ElementText, ElementVisible, ElementCount) use raw CSS selectors (`#id`, `.class`, `tag`) via browser query commands, separate from the semantic `SelectorHint` used for agent interactions.
- **Autonomous exploration**: `ExplorerConfig` controls BFS crawling (max_pages, max_depth, same_origin_only, explore_forms, max_forms_per_page). `explore()` for offline single-page analysis, `explore_live()` for multi-page BFS via `BrowserSession`. Builds `AppMap` graph of `PageNode`s + `Transition`s. Form-aware: fills/submits forms during crawling to discover post-submit pages (dashboards, search results).
- **Transition tracking**: `TransitionKind` enum (`Link` | `FormSubmission { form_id, values }`) tracks how each page transition was triggered. Defaults to `Link` for backward compatibility via `#[serde(default)]`.
- **Flow detection**: `detect_flows(app_map)` walks `FormSubmission` transition chains to discover multi-step user flows. Starts from "origin" pages (not targets of other form submissions). Names flows from `PageCategory` (e.g., "Flow: Login -> Dashboard").
- **Test generation**: `generate_test_plan(app_map, value_overrides)` produces `Vec<TestSpec>` from explored pages. Per page: smoke test (wait + assert suggested_assertions + URL path assertion) + per-form test (fill_and_submit with AI-suggested or user-overridden values + post-submit assertions from `ExpectedOutcome`). Plus flow tests from `detect_flows()`. `map_suggested_assertion()` bridges `SuggestedAssertion` → `AssertionSpec`.
- **Resilient test runner**: `TestRunner::run_with_config(spec, session, config)` adds assertion retry (configurable max retries + delay), screenshot on failure (base64-embedded in HTML reports), and per-test duration tracking. `RunnerConfig` controls retry/screenshot settings. `run()` delegates to `run_with_config()` with defaults for backward compatibility.
- **Expected outcomes**: `ExpectedOutcome` struct captures post-action verification signals: `url_contains`, `url_not_contains`, `success_text`, `error_indicators`. Attached to both `FormModel` and `PageModel`. `infer_form_outcome(category)` derives rule-based defaults per `PageCategory` (Login → url_not_contains /login + error indicators; Search → success_text "results"; etc.). `LlmPageAnalyzer` merges LLM-provided indicators with dedup check.
- **Authentication support**: `AuthConfig` (login_url, credentials HashMap, submit_label, success_url_contains) drives `perform_login()` in `explorer.rs`. Auto-login runs before BFS exploration if credentials are configured. `explore_live()` accepts `Option<&AuthConfig>`.
- **Configuration system**: `AuthConfig`, `ValueConfig`, `ExclusionConfig` added to `AppConfig` with `#[serde(default)]`. `ValueConfig.resolve(label, category)` does case-insensitive label lookup with category-scoped override priority. `ExclusionConfig.should_skip(url)` matches URL substrings; `include_urls` seeds forced pages into BFS queue.
- **Value overrides**: `explore_live()` accepts `Option<&ValueConfig>`; form field values use `value_overrides.resolve()` with fallback to `suggested_test_value`. `generate_test_plan()` accepts `Option<&ValueConfig>` and applies overrides during test spec generation.
- **URL resolution**: `resolve_url(base, candidate)` handles absolute URLs, protocol-relative (`//cdn.example.com`), root-relative (`/path`), and relative paths. Filters `javascript:`, `mailto:`, `tel:`, empty strings, bare `#`. `extract_origin(url)` returns `Option<String>` (scheme + host). `NavigationTarget.href` carries the actual link href for resolution (was previously dropped).

## Constants

- `MIN_CONFIDENCE`: 0.65
- `MAX_THINK_STEPS`: 5
- `MAX_RETRIES`: 3
- `MAX_LOOP_REPEATS`: 2
- Ollama model: `qwen2.5:1.5b` at `http://localhost:11434/api/generate` (configurable via `OLLAMA_MODEL` and `OLLAMA_ENDPOINT` env vars)
- Ollama generation params: `temperature: 0.1`, `top_p: 0.9`, `top_k: 40` (optimized for JSON output from small models)
- `MAX_ITERATIONS`: 20 (agent loop safety limit in run_app)
- Form intent thresholds: >0.7 = Authentication, >0.4 = User Input
- `TARGET_URL` env var: overrides default URL (google.com) for run_app/run_app_session
- `OLLAMA_ENDPOINT` env var: overrides Ollama API endpoint (default: `http://localhost:11434/api/generate`)
- `OLLAMA_MODEL` env var: overrides Ollama model name (default: `qwen2.5:1.5b`)

## Build & Test

```bash
cargo build
cargo test                       # 336 offline tests (fast, no browser needed)
cargo test -- --ignored          # 28 integration tests (requires Node.js + Playwright; 5 LLM tests need Ollama)
cargo test -- --include-ignored  # all 364 tests
```

- Rust edition 2024
- Dependencies: serde (1.0.228 with derive), serde_json (1.0.145), serde_yaml (0.9), sha1 (0.10.6), reqwest (0.12 with json + blocking), clap (4 with derive)
- Tests use HTML fixtures in `tests/fixtures/` (01-10) loaded via `file://` URLs
- Test helpers in `tests/common/` provide `diff_between_pages()`, `diff_static()`, etc.
- `MockBackend` enables deterministic agent tests without Ollama running
- 364 tests total: 336 offline + 28 integration (real browser via `#[ignore]`)
- 336 offline tests split across module-based test files:
  - `tests/agent_tests.rs` (59) — agent behavior, policy logic, guess_value heuristics (generic + intent-based), budget gating, error handling, form ranking, constraint-aware values, page category inference, disabled input filtering
  - `tests/browser_tests.rs` (32) — BrowserRequest/BrowserResponse serialization, session error variants, NavigateTo action, query protocol, select/check/uncheck serialization, DomElement/SelectOption serde, DOM constraint fields
  - `tests/canonical_tests.rs` (12) — classification, diffing, signals
  - `tests/explorer_tests.rs` (69) — ExplorerConfig, AppMap data model, TransitionKind/FlowStep/Flow serde, explore() offline, test generation (smoke/form/flow + value overrides + exclusion filtering + expected outcome assertions), flow detection, map_suggested_assertion, is_same_origin, resolve_url (absolute/root-relative/relative/protocol-relative/filtered), extract_origin, field-type SelectorHint builder
  - `tests/page_model_tests.rs` (78) — PageModel/PageCategory/FieldType serde roundtrips, MockPageAnalyzer (required, select options, login values, readonly filtering, output semantics, richer assertions), LlmPageAnalyzer hybrid enrichment + field_values override + success/error indicators, JSON recovery (try_parse), category mapping, prompt construction, field classification (input types, tags, labels), navigation targets (href serde + backward compat), smart dropdown selection, output semantic classification, ExpectedOutcome (default/serde/infer_form_outcome per category), LLM indicator merging + dedup
  - `tests/report_tests.rs` (15) — TestSuiteReport aggregation, console/HTML/JUnit formatting, JSON roundtrip
  - `tests/runner_tests.rs` (19) — RunnerConfig defaults/serde, TestResult new fields (duration_ms, screenshots, retry_attempts) serde + backward compat, console/HTML/JUnit output with timing/retry/screenshot, screenshot embedding, UrlNotContains report output
  - `tests/spec_tests.rs` (16) — TestSpec YAML/JSON roundtrips, TestContext tracking, TestResult serialization, assertion roundtrips, UrlNotContains YAML roundtrip
  - `tests/state_tests.rs` (5) — normalize edge cases, form intent scoring, ScreenElement constraint fields
  - `tests/cli_tests.rs` (31) — CLI argument parsing (explore/run/generate subcommands), config file loading/defaults/YAML roundtrip, ExplorerConfig builder, sanitize_filename, load_specs, AuthConfig/ValueConfig/ExclusionConfig serde + resolve logic + wiring
- 28 integration tests (real browser, `#[ignore]`):
  - `tests/integration_tests.rs` (28) — session lifecycle, DOM pipeline, queries, form interaction, TestRunner, navigation, canonical pipeline, form-aware exploration, flow detection, LLM page analysis (5 tests, requires Ollama)

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
- `BrowserSession`: child, stdin, reader, current_url — persistent browser session backed by browser_server.js; methods: navigate, extract, fill, click, wait_idle, screenshot, current_url, query_text, query_visible, query_count, select_option, check, uncheck, quit
- `SelectOption`: value, text — dropdown option for `<select>` elements
- `ScreenState`: url, title, forms, standalone_actions, outputs, identities
- `Outcome`: NoChange, FormSubmissionSucceeded, FormSubmissionFailed, Navigation, Unknown
- `OutputRegion`: Header, Main, Results, Footer, Modal, Unknown
- `Volatility`: Stable, Volatile
- `DecisionType`: Act, Wait, Stop
- `BudgetDecision`: Allow, Block
- `AgentError`: SubprocessSpawn, SubprocessFailed, JsonParse, JsonSerialize, BrowserAction, DomStructure, ElementNotFound, MissingState, SessionIO, SessionProtocol — typed error enum (10 variants); implements `Display` and `std::error::Error` with source chaining
- `TestSpec`: name, start_url, steps (Vec<TestStep>) — complete test specification
- `TestStep`: FillForm, FillAndSubmit, Click, Navigate, Wait, Assert — serde-tagged enum (`#[serde(tag = "action")]`)
- `AssertionSpec`: UrlContains, UrlEquals, UrlNotContains, TitleContains, TextPresent, TextAbsent, ElementText, ElementVisible, ElementCount — serde-tagged enum (`#[serde(tag = "type", rename_all = "snake_case")]`); `UrlNotContains { expected }` passes when current URL does NOT contain expected substring
- `AssertionResult`: step_index, spec, passed, actual, message — single assertion outcome
- `TestResult`: spec_name, passed, steps_run, assertion_results, error, duration_ms (Option<u128>), screenshots (Vec<String>), retry_attempts (usize) — complete test run result; new fields use `#[serde(default)]` for backward compat
- `TestContext`: current_step, assertion_results — execution state tracker with pass/fail counting
- `TestRunner`: stateless executor — takes TestSpec + BrowserSession, returns TestResult; `run()` uses defaults; `run_with_config()` adds retry/screenshot/timing
- `RunnerConfig`: max_assertion_retries (2), retry_delay_ms (500), screenshot_on_failure (true), screenshot_dir ("screenshots") — controls test runner resilience; separate from TestSpec (runner-level not spec-level concerns)
- `OutputSemantic`: Success, Error, Warning, Info, Data, Navigation — semantic classification of page output text; `OutputModel.semantic` defaults to `Info` via `#[serde(default)]`
- `PageModel`: purpose, category (PageCategory), forms (Vec<FormModel>), outputs, suggested_assertions, navigation_targets — AI understanding of a web page
- `PageCategory`: Login, Registration, Search, Dashboard, Settings, Listing, Detail, Checkout, Error, Other — page classification enum
- `FormModel`: form_id, purpose, fields (Vec<FieldModel>), submit_label — AI understanding of a form
- `FieldModel`: label, field_type (FieldType), required, suggested_test_value — single form field with AI-suggested test value
- `FieldType`: Text, Email, Password, Number, Date, Tel, Url, Select, Checkbox, Radio, Textarea, Search, Hidden, Time, Other — semantic field classification (15 variants)
- `PageAnalyzer` trait: `analyze(&self, screen: &ScreenState) -> Result<PageModel, AgentError>` — pluggable page analysis
- `TextInference` trait: `infer_text(&self, prompt: &str) -> Option<String>` — generic text-in/text-out LLM interface
- `MockTextInference`: canned response for deterministic testing of LlmPageAnalyzer
- `LlmPageResponse`: purpose (Option), category (Option), field_values (Option<HashMap<String, String>>), success_indicators (Option<Vec<String>>), error_indicators (Option<Vec<String>>) — lightweight struct for parsing LLM output; used by `try_parse_llm_response()`; field_values enables LLM-suggested form values that override deterministic defaults; success/error indicators merge into `ExpectedOutcome` with dedup
- `ExpectedOutcome`: url_contains, url_not_contains, success_text (Vec<String>), error_indicators (Vec<String>) — post-action verification signals; attached to `FormModel` and `PageModel` with `#[serde(default)]`; `infer_form_outcome(category)` derives rule-based defaults per PageCategory
- `AuthConfig`: login_url (Option), credentials (HashMap<String, String>), submit_label (Option), success_url_contains (Option) — drives `perform_login()` before BFS; `has_credentials()` returns true when credentials non-empty
- `ValueConfig`: fields (HashMap<String, String>), categories (HashMap<String, HashMap<String, String>>) — user field value overrides; `resolve(label, category)` does case-insensitive lookup with category-scoped priority over global fields
- `ExclusionConfig`: skip_urls (Vec<String>), include_urls (Vec<String>) — `should_skip(url)` matches any substring in skip_urls; include_urls seeded into BFS queue before main loop
- `AppConfig`: explore (ExploreConfig), run (RunConfig), ollama (OllamaConfig), auth (AuthConfig), values (ValueConfig), exclusions (ExclusionConfig) — optional YAML config file model; all fields `#[serde(default)]` for backward compat
- `ExplorerConfig`: start_url, max_pages (10), max_depth (3), same_origin_only (true), explore_forms (true), max_forms_per_page (3) — controls autonomous BFS crawling with form-aware exploration
- `PageNode`: url, title, depth, page_model (PageModel) — single discovered page in the AppMap
- `TransitionKind`: Link | FormSubmission { form_id, values } — how a page transition was triggered; defaults to Link via serde
- `Transition`: from_url, to_url, label, kind (TransitionKind) — directed edge between pages with transition tracking
- `FlowStep`: Navigate { url } | FillAndSubmit { url, form_id, values, submit_label } — single step in a multi-page flow
- `Flow`: name, steps (Vec<FlowStep>) — detected multi-step user journey
- `AppMap`: pages (HashMap<String, PageNode>), transitions (Vec<Transition>) — graph of discovered pages; has add_page(), add_transition(), page_count(), has_page()
- `TestSuiteReport`: suite_name, total, passed, failed, duration_ms, test_results — aggregated report; from_results(), with_duration(), all_passed()
- `Cli`: clap-derived CLI parser with global args (verbose, ollama_endpoint, ollama_model, config) and `Commands` subcommand enum
- `Commands`: Explore { url, max_pages, max_depth, explore_forms, max_forms_per_page, analyzer } | Run { spec, format, output } | Generate { url, output_dir, ... } — CLI subcommands
- `NavigationTarget`: label, likely_destination, href (Option<String>) — `href` carries the actual link URL for resolution; propagated by `MockPageAnalyzer` from `ScreenElement.href`; used by `explore_live()` as `target.href.as_deref().unwrap_or(&target.label)`

## CLI Usage

```bash
# Explore a website
cargo run -- explore --url https://example.com --max-pages 5 -v

# Generate test specs from exploration
cargo run -- generate --url https://example.com -o ./generated-specs -v

# Run test specs
cargo run -- run --spec ./generated-specs --format console
cargo run -- run --spec test.yaml --format html -o report.html
cargo run -- run --spec test.yaml --format junit -o report.xml

# With LLM analyzer
cargo run -- explore --url https://example.com --analyzer llm --ollama-model llama3

# With config file (including auth, values, exclusions)
cargo run -- --config my-config.yaml explore --url https://example.com
```

### Example Config YAML (screen-detection.yaml)
```yaml
auth:
  login_url: "https://myapp.com/login"
  credentials:
    Email: "admin@staging.com"
    Password: "StagingPass2024!"
  success_url_contains: "/dashboard"

values:
  fields:
    Email: "tester@mycompany.com"
    Phone: "555-0199"
  categories:
    Checkout:
      Card Number: "4000056655665556"
      CVV: "314"

exclusions:
  skip_urls:
    - "/logout"
    - "/admin"
  include_urls:
    - "https://myapp.com/settings"

explore:
  max_pages: 20
  max_depth: 4
```

Exit code: 0 if all tests pass, 1 if any fail (for CI integration).

## File Layout

```
src/
  lib.rs              -- Agent loop orchestration (run_app, run_app_session), snapshot(), snapshot_session()
  main.rs             -- CLI entry point (clap parsing, subcommand dispatch)
  agent/
    mod.rs            -- Module exports
    agent.rs          -- State machine, gating, execute_action(), execute_action_session(), emit_observed_actions()
    agent_model.rs    -- AgentState, AgentAction (incl. FillAndSubmitForm, NavigateTo), AgentMemory, ModelDecision, Policy trait
    ai_model.rs       -- OllamaBackend (env-var config, temperature/top_p/top_k), MockBackend, DeterministicPolicy, HybridPolicy, guess_value(label, input_type, category) with ~26 patterns + PageCategory-aware filling, constrained_value() (maxlength/minlength enforcement), rank_form(), select_best_form(), infer_page_category(), prompt construction, TextInference trait, MockTextInference
    budget.rs         -- Budget check (think/retry/loop)
    error.rs          -- AgentError enum (10 variants), Display, std::error::Error impls
    page_model.rs     -- PageModel (with expected_outcome), PageCategory, FormModel (with expected_outcome), FieldModel, FieldType, ExpectedOutcome, OutputModel (with OutputSemantic), OutputSemantic, SuggestedAssertion, NavigationTarget (with href)
    page_analyzer.rs  -- PageAnalyzer trait, MockPageAnalyzer (rule-based, with readonly filter + constrained_value + smart_select_option + classify_output_semantic + infer_form_outcome), LlmPageAnalyzer (hybrid enrichment: LLM purpose+category+success/error indicators + deterministic details), LlmPageResponse (with success_indicators/error_indicators), try_parse_llm_response() (4-stage JSON recovery), map_llm_category(), classify_field_type(), smart_select_option(), classify_output_semantic(), infer_form_outcome()
  browser/
    mod.rs            -- Module exports
    playwright.rs     -- DOM extraction (returns Result), SelectorHint, BrowserCommand, BrowserResult, execute_browser_action()
    session.rs        -- BrowserSession (persistent browser via NDJSON), BrowserRequest, BrowserResponse, query_text/query_visible/query_count for element assertions, select_option/check/uncheck for form interaction
  canonical/
    mod.rs            -- Module exports
    canonical_model.rs -- CanonicalScreenState, canonicalize()
    diff.rs           -- SemanticStateDiff, SemanticSignal, signal derivation
  screen/
    mod.rs            -- Module exports
    classifier.rs     -- classify() DOM -> ScreenSemantics
    intent.rs         -- infer_form_intent() heuristic scoring
    screen_model.rs   -- DomElement (with id, name, placeholder, href, value, options, pattern, minlength, maxlength, min, max, readonly), SelectOption, Form, ScreenElement (with required, placeholder, id, href, options, name, value, maxlength, minlength, readonly), FormIntent, ElementKind
  state/
    mod.rs            -- Module exports
    state_builder.rs  -- build_state(), resolve_identities()
    state_model.rs    -- ScreenState, Outcome
    identity.rs       -- IdentifiedElement, form_key(), element_key()
    diff.rs           -- StateDiff (added/removed/unchanged)
    normalize.rs      -- Text normalization, region inference (OutputRegion), volatility detection
  explorer/
    mod.rs            -- Module exports
    app_map.rs        -- ExplorerConfig, PageNode, TransitionKind, Transition, FlowStep, Flow, AppMap (page graph data model)
    explorer.rs       -- explore() (offline single-page), explore_live() (6-param: config, session, analyzer, auth?, exclusions?, value_overrides?), perform_login() (navigates to login URL, fills credentials, submits), submit_form_in_session() (field-type-aware, skips Hidden, handles Radio), build_selector_for_field() (FieldType → SelectorHint), is_same_origin(), resolve_url(), extract_origin()
    flow_detector.rs  -- detect_flows() (walks FormSubmission chains), build_flow_step(), name_flow()
    test_generator.rs -- generate_test_plan(app_map, value_overrides?), generate_smoke_test() (URL path assertion + error absence), generate_form_test() (post-submit assertions from ExpectedOutcome), generate_form_test_with_overrides() (internal, applies ValueConfig), build_form_test_spec() (shared builder), generate_flow_tests(), map_suggested_assertion()
  spec/
    mod.rs            -- Module exports
    spec_model.rs     -- TestSpec, TestStep, AssertionSpec (with UrlNotContains), AssertionResult, TestResult (with duration_ms, screenshots, retry_attempts)
    context.rs        -- TestContext: step tracking, assertion result collection, pass/fail counting
    runner.rs         -- TestRunner: run() and run_with_config(); execute_step_with_retry() for Assert steps; maybe_screenshot() on failure
    runner_config.rs  -- RunnerConfig: max_assertion_retries, retry_delay_ms, screenshot_on_failure, screenshot_dir
  report/
    mod.rs            -- Module exports
    report_model.rs   -- TestSuiteReport: aggregation of Vec<TestResult>, from_results(), with_duration(), all_passed()
    console.rs        -- format_console_report(): terminal output with pass/fail markers and failure details
    html.rs           -- generate_html_report(): self-contained HTML with inline CSS, green/red header, per-test duration, base64-embedded screenshots on failure
    junit.rs          -- generate_junit_xml(): standard JUnit XML for CI (Jenkins, GitHub Actions), escape_xml()
  cli/
    mod.rs            -- Module exports
    config.rs         -- Cli, Commands (clap derive), AppConfig (YAML config model with auth/values/exclusions), AuthConfig, ValueConfig, ExclusionConfig, load_config(), build_explorer_config()
    commands.rs       -- cmd_explore(), cmd_run(), cmd_generate(), build_analyzer(), load_specs(), sanitize_filename()
  trace/
    mod.rs            -- Module exports
    trace.rs          -- TraceEvent builder
    logger.rs         -- JSONL file logger (Mutex<File>), gracefully degrades if file unavailable
tests/
  agent_tests.rs      -- 59 tests: agent behavior, policy, guess_value (generic + intent-based), budget, errors, form ranking (rank_form/select_best_form), constrained_value, infer_page_category, disabled input filtering
  browser_tests.rs    -- 32 tests: BrowserRequest/Response serde, session errors, query protocol, select/check/uncheck, DomElement/SelectOption, DOM constraint field deserialization
  canonical_tests.rs  -- 12 tests: classification, diffing, signals
  runner_tests.rs     -- 19 tests: RunnerConfig defaults/serde, TestResult new fields serde + backward compat, console/HTML/JUnit output with timing/retry/screenshot, screenshot embedding, UrlNotContains report output
  explorer_tests.rs   -- 69 tests: ExplorerConfig, AppMap, TransitionKind/FlowStep/Flow serde, explore(), flow detection, test generation (smoke/form/flow + value overrides + exclusion filtering + expected outcome assertions), URL utilities (resolve_url, extract_origin, is_same_origin), build_selector_for_field (FieldType → SelectorHint)
  page_model_tests.rs -- 78 tests: PageModel serde, MockPageAnalyzer (required, select options, login values, readonly filtering, output semantics, richer assertions via expected_outcome), LlmPageAnalyzer hybrid enrichment + field_values + success/error indicators (merge + dedup), JSON recovery, category mapping, prompt construction, field classification (types, tags, labels), smart_select_option, classify_output_semantic, output_semantic serde, ExpectedOutcome (default/serde/infer_form_outcome), NavigationTarget href serde + backward compat
  report_tests.rs     -- 15 tests: TestSuiteReport aggregation, console/HTML/JUnit formatting, JSON roundtrip
  spec_tests.rs       -- 16 tests: TestSpec YAML/JSON, TestContext, TestResult, assertions, UrlNotContains YAML roundtrip
  state_tests.rs      -- 5 tests: normalize, form intent scoring, ScreenElement constraint fields (maxlength/minlength)
  cli_tests.rs        -- 31 tests: CLI arg parsing (explore/run/generate), config loading/defaults/YAML, builder, sanitize_filename, load_specs, AuthConfig/ValueConfig/ExclusionConfig serde + resolve logic + app_config wiring
  integration_tests.rs -- 28 tests (real browser, #[ignore]): session, DOM, queries, forms, runner, nav, canonical, form-aware exploration, flow detection, LLM page analysis
  common/
    mod.rs            -- Module exports
    utils.rs          -- page() helper, is_terminal_success()
    semantic_diff.rs  -- diff helpers (run_diff_from_raw, diff_between_pages, etc.)
  fixtures/           -- HTML test pages (01-10)
```

## Node.js Scripts (../../node/dom-extraction/)

- `extract.js` — Launches Playwright, navigates to URL, extracts DOM as JSON (elements with tag, id, text, attributes, children). Stateless: fresh browser per call.
- `interact.js` — Receives a BrowserCommand JSON arg, launches Playwright, executes fill/click/wait actions using SelectorHint for element targeting. Stateless: fresh browser per call.
- `browser_server.js` — Long-lived NDJSON server. Launches Playwright once, accepts commands over stdin, returns responses to stdout. Supports: navigate, extract, action (fill/click/wait/select/check/uncheck), screenshot, current_url, query_text, query_visible, query_count, quit. Extracts DOM with 14 fields per element (tag, text, role, type, ariaLabel, disabled, required, formId, id, name, placeholder, href, value, options). Used by `BrowserSession` in Rust for persistent multi-page sessions.
- Playwright dependency: `^1.58.2`
