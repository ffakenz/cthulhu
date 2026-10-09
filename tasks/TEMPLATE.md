# <ticket id> — <one line: the outcome, not the activity>

Plan of record: `<path to the plan, or the PR>`

## Context

Why this exists. The problem, who hit it, and what changes when it is done. Two or three
sentences — the detail belongs in the phases.

## Phase 0 — evidence (read-only)

- [ ] `<the exact command>` — what it must show for the next phase to be worth starting.

Nothing is written until this phase answers. A phase that assumes its own premise is how a
whole ticket ends up fixing something that was never broken.

## Phase 1 — <name>

- [ ] Red: `<test name>` — fails with `<observed>` / expected `<expected>`.
- [ ] Green: `<the smallest change>`
- [ ] Verify: `<lint>` · `<build>` · `<tests, with counts>`

## Phase 2 — <name>

- [ ] …

## Blocked / handed off

- [ ] `[owner]` — what they must do, why it cannot be done here, and how to roll it back.

## Review

*Filled in when the work closes.*

What the brief got wrong. What was narrower or wider than expected. What was deliberately not
done, and why that was the right call.
