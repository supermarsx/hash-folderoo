# Roadmap & Issues (short-term priorities)

This roadmap converts the high-level TODO list into actionable issues and milestones.
Use it to guide short-term work and handoff to maintainers.

## Milestone: Reliable, reviewable operations (high priority)

1. Resumable copy planner & execution (important for large bulk ops)
   - Add plan persistence with transactional writes (atomic replace / tmp file + rename).
   - Add `--resume` to `copydiff` which will read a plan and continue from `done: false` ops.
   - Make `CopyOp` include a `status` enum (pending, in-progress, done, failed) for clearer resumability.
   - Add tests for partial-run + resume and for crash-safe plan replacement.
   - Consider locking semantics / user prompt when resuming live plans.

2. Precise multi-hunk unified diffs for `--git-diff-body` (improve UX and reviewability)
   - Replace current full-file body with minimal hunks using a diff algorithm (difflib/SequenceMatcher).
   - Add `--git-diff-context <n>` to configure context lines (default 3).
   - Add tests for multi-hunk cases and binary file detection (skip bodies for binaries).

3. Add `--git-diff-*` sanity and output options
   - `--git-diff-body` (already implemented) — add context and partial-hunks.
   - `--git-diff-output` (already implemented) — ensure multiple subcommands append in a structured way (optionally add `--git-diff-reset`).

## Milestone: Reliability and observability (mid priority)

4. Enhancements to memory/booster mode
   - Per-thread buffer limits, pressure metrics, and optional health checks.
   - Add CI stress test harness to validate memory plans under limited budgets.

5. Reporting improvements
   - Add slowest files reporting (use current pipeline timings) and add a CLI flag `--top-slowest` for report.
   - Add more detailed duplicate group reports (show candidate dedup plans) and JSON schema for reports.

6. Config schemas and docs
   - Produce JSON Schema or examples for `config.toml/config.yaml/config.json` and validate in CI.

## Milestone: Platform & extensions (low priority)

7. Hash algorithm coverage
   - Integrate MeowHash if needed for benchmarking.
   - Expand benchmarking suite to include new algorithms and produce comparative output.

8. Desktop GUI prototype
   - Minimal Tauri/egui prototype to mirror core CLI flows: hashmap, compare, copydiff preview.

---

If you want, I can:
- Start implementing item 2 (precise multi-hunk unified diffs) next — this improves reviewer experience immediately.
- Or implement item 1 (resumable copy planner) if robustness is top priority for you.

Tell me which you'd like first and I'll start coding or create tracked issue files for the chosen scope.