# 1. Record architecture decisions

Date: 2026-04-18

## Status

Accepted

## Context

We need a way to record important architectural decisions in a way that is persistent and easy for both humans and AI agents to understand. Documentation often rots, but a log of decisions provides context for *why* things are the way they are.

## Decision

We will use Architecture Decision Records (ADRs) to document significant design choices. These will be stored in `docs/adr/` as Markdown files, numbered sequentially.

## Consequences

- Improved transparency for all team members (human and AI).
- Reduced "vibe-based" engineering.
- History of trade-offs becomes searchable.
