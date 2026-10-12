# tasks

Plans and the working state of whatever is in flight. Everything here is tracked: it is the
evidence of how the project was built. Like every tracked file, nothing here restates, quotes or
paraphrases an external brief or specification: refer to it by a neutral name only.

| File | Owner | Purpose |
|---|---|---|
| `todo.md` | the current piece of work | the plan of record, as checkable items, plus a Review section when it closes |
| `<name>-plan.md` | a closed piece of work | its plan, kept verbatim before the next piece of work replaces `todo.md`; every reference that named it as `todo.md`, quoted prompts aside, names this file |
| `lessons.md` | corrections | what was got wrong, why, and the rule that prevents it next time |
| `<ticket>.md` | one ticket | copy `TEMPLATE.md` |

## How a ticket earns its file

One file per ticket, named for the ticket, not for the fix. The file outlives the branch —
it is where the next session learns what was already ruled out.

Phases are checkable. A tick is a plain `- [x]`: the phase is done, and committed where it
changes files.

When the work closes, add a **Review** section: what the framing got wrong, what turned out to
be narrower or wider than the brief, and what was deliberately not done. That section is the
part anyone reads six months later.

## lessons.md

Append when the work closes, one entry per correction. Three headings, in order: **What happened**, **What I got wrong**
(numbered), **Rules for next time**. A lesson that does not end in a rule is a diary entry.

Durable lessons about *how to work* graduate out of here into `.claude/skills/`. Keep in
`lessons.md` only what is specific to this project.
