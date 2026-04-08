---
name: review-plan
description: Review a plan document from multiple technical perspectives (simplicity, Bevy/ECS idioms, testability)
argument-hint: [plan-filepath]
---

## Multi-Perspective Plan Review

You are reviewing the plan at `$ARGUMENTS`. Your job is to provide three independent expert reviews, then synthesize them.

### Step 1: Read the plan

Read the plan file at `$ARGUMENTS` and read `CLAUDE.md` for project conventions. Identify all source files referenced in the plan.

### Step 2: Launch 3 review agents in parallel

Use the Agent tool to spawn all 3 agents in a single message (parallel execution). Each agent should:
- Read the plan file at `$ARGUMENTS`
- Read `CLAUDE.md` for project conventions
- Read all source files referenced in the plan
- Provide a focused review from its assigned perspective
- Report findings in under 300 words with specific, actionable recommendations

**Agent 1 — Simplicity & Clarity** (subagent_type: Explore, thoroughness: very thorough)
> You are reviewing a plan for simplicity. Read the plan at `$ARGUMENTS` and all source files it references. Also read CLAUDE.md for project conventions.
> 
> Review for:
> - Is this the simplest approach that solves the problem?
> - Can any steps, components, or abstractions be removed without losing functionality?
> - Are there existing utilities or patterns in the codebase that could be reused instead of creating new ones?
> - Is the scope appropriate or does it introduce unnecessary changes?
> 
> Be specific. Reference file paths and line numbers. Report in under 300 words.

**Agent 2 — Bevy/ECS Idioms** (subagent_type: Explore, thoroughness: very thorough)
> You are reviewing a plan for Bevy 0.18 and ECS correctness. Read the plan at `$ARGUMENTS` and all source files it references. Also read CLAUDE.md for project conventions — it contains critical Bevy 0.18 API patterns that MUST be followed.
> 
> Review for:
> - Does it use the correct Bevy 0.18 APIs (not deprecated patterns from older versions)?
> - Are components vs enums used appropriately? (enums for mutually exclusive behavioral states within a category, components for existential membership)
> - Are Events vs Messages used correctly? (Events for observer-based reactive triggers, Messages for buffered per-frame processing)
> - Are observers used where they should be, vs polling systems?
> - Does the system scheduling make sense (FixedUpdate vs Update, ordering dependencies)?
> 
> Be specific. Reference file paths and line numbers. Report in under 300 words.

**Agent 3 — Testability** (subagent_type: Explore, thoroughness: very thorough)
> You are reviewing a plan for testability and regression safety. Read the plan at `$ARGUMENTS` and all source files it references. Also read CLAUDE.md and look at the existing test files (tests/systems.rs and any #[cfg(test)] modules).
> 
> Review for:
> - Can the proposed changes be tested with the existing test infrastructure?
> - Are there behavioral contracts that should have tests but the plan doesn't mention them?
> - Will any existing tests break? Does the plan account for updating them?
> - Are the seams in the right places for unit testing (pure functions, clear inputs/outputs)?
> - Is there adequate separation between detection logic and side effects to allow testing each independently?
> 
> Be specific. Reference file paths and line numbers. Report in under 300 words.

### Step 3: Synthesize

After all 3 agents complete, compile their findings into this structure:

**Agreements** — Points where 2+ perspectives align

**Conflicts** — Points where perspectives disagree (e.g., "simpler" vs "more testable")

**Recommendations** — Prioritized list of actionable changes, noting which perspective(s) support each one
