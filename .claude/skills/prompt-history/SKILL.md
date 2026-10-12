---
name: prompt-history
description: "Use when a working session ends, or when the user asks to save or record it: save this session into prompts, record the prompts, write the prompt history. Covers the file name and number under docs/prompts/, the skeleton (date, context, outcome, one entry per user turn), what to quote and what to elide, how to name what the session produced, where lessons go instead, and the commit that lands the entry alone. Keywords: prompt history, session log, save the session, record the prompts, save this session into prompts, docs/prompts, evidence, reproducible, session record, how was this built."
---

# Prompt history

`docs/prompts/` holds one file per working session: the user's prompts in order, each with what it
produced. With `tasks/` and the decision log, it is the evidence of how this repo was built — the
path from each request to the change it made stays reproducible.

## The file

- `docs/prompts/NNNN-kebab-case-title.md`: the highest existing number plus one, zero-padded to
  four digits; the first is `0001`.
- The heading carries the same four digits: `# Prompt History NNNN — Title`.
- One file per session, written as the session closes; usually its last turn asks for it.

## The skeleton

```text
# Prompt History NNNN — Title

Date: YYYY-MM-DD

<One paragraph: what the session set out to do, and where it started from.>

**Outcome:** <What changed. A decision is named by its title, never by its number.>

---

### 1. <Turn title>
> <the user's prompt, trimmed; elide with [...]>

<One to three sentences: what the turn produced.>

### N. Save the session
> <the prompt that asked for this file>

<Recorded the session as this entry and committed it.>
```

## What goes in

- **Quote the user only**, one `### N.` per user turn, in order. Trim each prompt to what drove
  the work and mark every cut with `[...]`. Never quote the assistant: the note under the quote
  says what the turn produced.
- **Elide what the repo must not hold.** The file is tracked and follows the same rules as the
  rest of the repo: secrets, credentials, absolute paths, other projects' names and other stacks
  become `[...]`. An external brief or specification is never quoted, restated or paraphrased: a
  prompt that pastes or restates one keeps only its request, and the brief is named neutrally.
- **Name what the session produced.** A decision record is named by its title, never linked and
  never cited by number (`CLAUDE.md` § Comments). A file may be linked relative to
  `docs/prompts/`, as `[app/src/lib.rs](../../app/src/lib.rs)`.
- **Lessons go elsewhere.** A correction the user made is still a turn here, but the rule it
  teaches goes to `tasks/lessons.md`, not into the prompt history.

## Commit

Commit the entry alone:

```text
docs(prompts): add prompt history NNNN (<short title>)

> <the business or technical impact>
```

Related: [[architecture-decisions]] · [[review-and-planning]]
