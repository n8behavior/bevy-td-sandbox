---
name: review-code
description: Review code from multiple technical perspectives (simplicity, Bevy/ECS idioms, testability, documentation)
argument-hint: [file-or-directory-path]
---

## Multi-Perspective Code Review

You are reviewing code in this project. Your job is to provide four independent expert reviews, then synthesize them.

### Step 1: Determine the review target

If `$ARGUMENTS` is provided, the review target is `$ARGUMENTS` (a file or directory path).
If `$ARGUMENTS` is empty, the review target is `src/` (the entire source tree).

### Step 2: Launch 4 review agents in parallel

Use the Agent tool to spawn all 4 agents in a single message (parallel execution). Each agent should:
- Read `CLAUDE.md` for project conventions
- Read all source files in the review target (if a directory, read all `.rs` files recursively)
- Also read `tests/` directory for context on existing test coverage
- Provide a focused review from its assigned perspective
- Report findings in under 400 words with specific, actionable recommendations

**Agent 1 — Simplicity & Clarity** (subagent_type: Explore, thoroughness: very thorough)
> You are reviewing code for simplicity and clarity. Read CLAUDE.md for project conventions, then read all `.rs` files in the review target: `$ARGUMENTS` (if empty, use `src/`).
> 
> Review for:
> - Dead code, unused imports, or unnecessary abstractions
> - Overly complex logic that could be simplified
> - Duplicated code that should be consolidated
> - Functions or systems that are doing too much (single responsibility)
> - Naming clarity — do names accurately describe what things do?
> - Are there any TODO/FIXME/HACK comments indicating unfinished work?
> 
> Be specific. Reference file paths and line numbers. Report in under 400 words.

**Agent 2 — Bevy/ECS Idioms** (subagent_type: Explore, thoroughness: very thorough)
> You are reviewing code for Bevy 0.18 and ECS correctness. Read CLAUDE.md for project conventions — it contains critical Bevy 0.18 API patterns that MUST be followed. Then read all `.rs` files in the review target: `$ARGUMENTS` (if empty, use `src/`).
> 
> **IMPORTANT:** When verifying API usage, do NOT rely solely on CLAUDE.md or training data. Cross-reference against the actual Bevy source in `~/.cargo/registry/src/` to confirm that APIs, method signatures, and component patterns match the version used by this project. This is the ground truth for what's available.
> 
> Review for:
> - Deprecated Bevy patterns (old bundles, old event API, StateScoped, timer.finished(), etc.)
> - Components vs enums: are enums used for mutually exclusive behavioral states? Are components used for existential membership?
> - Events vs Messages: Events for observer-based reactive triggers (no registration), Messages for buffered per-frame processing (require add_message)
> - Observer vs polling system mismatches — is anything polling that should be reactive, or vice versa?
> - System scheduling: FixedUpdate vs Update usage, ordering dependencies, missing run conditions
> - Resource usage: are resources appropriate, or should some data live on entities?
> - Query filters: any overly broad or overly narrow queries?
> 
> Be specific. Reference file paths and line numbers. Report in under 400 words.

**Agent 3 — Testability** (subagent_type: Explore, thoroughness: very thorough)
> You are reviewing code for testability and regression safety. Read CLAUDE.md for project conventions. Read all `.rs` files in the review target: `$ARGUMENTS` (if empty, use `src/`). Also read the test files in `tests/` and any `#[cfg(test)]` modules.
> 
> Review for:
> - Behavioral contracts that lack test coverage
> - Systems with side effects that are hard to test in isolation
> - Logic that could be extracted into pure functions for easier testing
> - Test infrastructure gaps — missing helpers, setup utilities, or patterns that would make tests easier to write
> - Any existing tests that appear fragile or tightly coupled to implementation details
> - Public API surface: are the right things exposed from lib.rs for testing?
> 
> Be specific. Reference file paths and line numbers. Report in under 400 words.

**Agent 4 — Documentation** (subagent_type: Explore, thoroughness: very thorough)
> You are reviewing code for rustdoc documentation quality. Read CLAUDE.md for project conventions, then read all `.rs` files in the review target: `$ARGUMENTS` (if empty, use `src/`). Also look at any existing doc comments and module-level docs (`//!`).
> 
> Review for:
> - Are public items (modules, structs, enums, functions, methods) documented with rustdoc comments?
> - Do module-level docs (`//!`) explain how the components and systems in that module work together as a cohesive unit? Module docs should discuss concepts, not just list contents.
> - Do docs explain *concepts* and *why*, not just restate the type signature?
> - Are `// ANCHOR:` / `// ANCHOR_END:` tags used so that doc comments and module docs can reference specific code sections with `{{#include}}`?
> - Where a simpler example would clarify a concept, is there a doc test (`/// ```rust` block) that both explains and verifies the example?
> - Are any existing doc comments stale or misleading relative to the current code?
> 
> Be specific. Reference file paths and line numbers. Report in under 400 words.

### Step 3: Present individual reports

After all 4 agents complete, present each agent's full report under its own heading. Do not summarize or truncate — show everything the agent returned:

#### Simplicity & Clarity
> (Agent 1 report verbatim)

#### Bevy/ECS Idioms
> (Agent 2 report verbatim)

#### Testability
> (Agent 3 report verbatim)

#### Documentation
> (Agent 4 report verbatim)

### Step 4: Synthesize

After the individual reports, compile cross-cutting findings into this structure:

**Agreements** — Points where 2+ perspectives align

**Conflicts** — Points where perspectives disagree (e.g., "simpler" vs "more testable")

**Recommendations** — Prioritized list of actionable changes, noting which perspective(s) support each one
