- What: struct-audit is a Rust CLI tool that analyzes binary memory layouts to detect padding inefficiencies, perform layout diffs, and integrate with CI systems—with a planned SaaS layer for historical tracking and
  team dashboards.
- Why: It solves the "Memory Wall" problem by making memory layout optimization measurable and trackable, targeting HFT (latency), embedded/IoT (RAM costs), and gaming (frame stability) markets.
- How: CLI parses DWARF debug info via gimli + object crates with clap for CLI, while SaaS uses Axum/PostgreSQL/Next.js; development follows phased roadmap from core CLI (v0.1.0) → diff/CI mode (v0.2.0) → optional SaaS
  MVP (v1.0.0).

- This is a Solo developer project done as a side project, with plans to enlarge when success will arrive, no need to waste time, memory, place etc on onboarding and large docs, in the code and external to the code. NEVER summarize a task in a new file, unless explicitly ordered.
- When you start a new task, start by gathering the whole context relevant for the task, understand how the task related to the project as a whole, not only the files that are the location of the changes
- In any interaction with the user - avoid answering cheering answers like "you are absolutely right", avoid unnecessary updates wording like adding "Great, now i understand", be focused, concise, and elaborate on what relevant. Dont add garbage context or social interactions.
- When it is unclear to you what is the task, goal, implementation detail, etc. ask. Take personal decision just when instructed.
- When you have the needed context and you have many subtasks, always prefer using subagents that you manage while holding the real context and directing them. Always validate their work, dont assume it is correct.
- Never assume that your implementation is simply correct, always validate logic and run tests to verify.
- When context is about to end, there is no need to start wrapping up the task prematurely. before the context end you will get context compacted and you'll be able to keep working. never stop a task in the middle or stub implementation because you see the context get empty.
- NEVER leave stubs, todos, dummy implementation etc. unless explicitly instructed
- After each task when you dont need to fix more changes commit before the next step, have as many checkouts as possible
- Rust edition 2024, toolchain version 1.91
- Before push - cargo fmt, cargo clippy