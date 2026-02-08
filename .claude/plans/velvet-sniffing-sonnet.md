# Complete AgentError Integration (Continuation)

## Context
We are partway through implementing a custom `AgentError` enum to replace ad-hoc `String` errors across the codebase. This was approved in the previous session. Three steps are done:
1. ✅ Created `src/agent/error.rs` with 8-variant `AgentError` enum
2. ✅ Added `pub mod error;` to `src/agent/mod.rs`
3. ✅ Updated `src/browser/playwright.rs` — both `extract_screen()` and `execute_browser_action()` now return `Result<_, AgentError>`

Four steps remain to complete the migration.

## Remaining Steps

### Step 1: Update `src/agent/agent.rs` — `execute_action()` return type
**File:** `src/agent/agent.rs`

- Change signature: `pub fn execute_action(...) -> Result<(), String>` → `Result<(), AgentError>`
- Add import: `use crate::agent::error::AgentError;`
- Replace error constructions:
  - Line 205: `"No URL in screen state".to_string()` → `AgentError::MissingState("No URL in screen state".into())`
  - Line 217: `format!("Input '{}' not found in form '{}'", ...)` → `AgentError::ElementNotFound { element: input_label.clone(), context: format!("form '{}'", form_id) }`
  - Line 243-246: `format!("Submit action '{}' not found in form '{}'", ...)` → `AgentError::ElementNotFound { element: action_label.clone(), context: format!("form '{}'", form_id) }`
  - Line 295-299: same pattern for FillAndSubmitForm's submit label
  - Line 323: `format!("Standalone action '{}' not found", label)` → `AgentError::ElementNotFound { element: label.clone(), context: "standalone actions".into() }`
- `execute_browser_action()` calls already return `AgentError`, so the `?` propagation just works.

### Step 2: Update `src/lib.rs` — no code changes needed
`AgentError` implements `std::error::Error` and `Display`, so `?` auto-converts to `Box<dyn Error>` in `snapshot()` and `run_app()`. The `execute_action` result in `run_app()` is matched (not `?`-propagated), so no change there either.

### Step 3: Update `tests/test.rs` — fix 2 tests that inspect error strings
**File:** `tests/test.rs`

- Add import: `screen_detection::agent::error::AgentError` to the use block
- `execute_action_errors_on_missing_url` (line 545): change from `result.unwrap_err().contains("No URL")` to matching on `AgentError::MissingState(_)` via `matches!()` macro
- `execute_action_errors_on_missing_element` (line 571): error type changes from `String` to `AgentError`, but `is_err()` checks still work. Optionally add `matches!(fill.unwrap_err(), AgentError::ElementNotFound { .. })` for stronger assertions.

### Step 4: Update `.claude/CLAUDE.md`
- Add `AgentError` to Key Types section with its 8 variants
- Add `error.rs` to the File Layout with description
- Mention `AgentError` in the Module Map for `agent` row

## Verification
```bash
cargo build
cargo test
```
All 30 tests must pass with zero warnings.
