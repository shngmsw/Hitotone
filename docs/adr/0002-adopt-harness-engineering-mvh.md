# 2. Adopt Harness Engineering (MVH)

Date: 2026-04-18

## Status

Accepted

## Context

To improve the reliability and autonomy of AI agents working on this project, we need standard tools that enforce quality and provide immediate feedback. Without these, AI agents may produce inconsistent results or deviate from project standards.

## Decision

We adopt the "Minimum Viable Harness" (MVH) as described in the Harness Engineering best practices.

1. **Self-Pointer**: Create `AGENTS.md` to guide AI agents.
2. **Deterministic Tools**: 
   - **Frontend**: Biome (Linter/Formatter) for fast feedback loops.
   - **Backend**: Standard Cargo tools (`fmt`, `clippy`).
3. **Guardrails**: Use `Lefthook` to enforce these tools on every commit.
4. **Context Preservation**: Use ADRs to prevent history rot.

## Consequences

- AI agents gain context through `AGENTS.md`.
- Quality is enforced automatically at the commit level.
- Development speed increases due to faster feedback loops (especially Biome).
- "Vibe-based" engineering is replaced by deterministic checks.
