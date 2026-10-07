# 0065. Bugs are one line each in one file, logged with `just bug`

- **Status:** Accepted
- **Recognized:** 2026-10-07, when bug write-ups had piled up in `docs/status.md`
- **Decided:** 2026-10-07, by the maintainer
- **Recorded:** 2026-10-07
- **Confidence:** decided now
- **Evidence:** `docs/bugs.md`; the `bug` and `bugs` recipes in `justfile`;
  `.gitattributes`; `bug_list_entries_are_well_formed` in `tests/docs_tests.rs`. The
  union merge was tried in a scratch repository on 2026-10-07: two branches that each
  appended a bug merged cleanly with the attribute and conflicted without it

## Context

Bugs had no home of their own. They were written into `docs/status.md`, open ones as rows
of the Known issues table and fixed ones as paragraphs under Resolved, 32 by 2026-10-07.
That mixed what state something is in with what happened, and the table also held
accepted limitations that will never be fixed.

The maintainer finds most bugs while reviewing, and wants to write one down in a sentence
without stopping. Asked whether ADRs have an equivalent for bugs: not really. ADRs spread
from one essay and a file convention, and nothing comparable caught on for bugs. Most
teams use an external tracker. The in-repo tools that exist either store issues outside
the tree as git objects (git-bug), which nothing in `docs/` can link to or check, or are
abandoned.

## Decision

**`docs/bugs.md` is a Markdown checklist, one line per bug.**

- An entry is ``- [ ] 2026-10-07 `name` description``, with optional indented lines
  under it. The name is short kebab-case, and commits and `//` comments cite it.
- **`just bug <name> <description>`** appends an entry with today's date, refusing a
  name that is not kebab-case or is already used. `just bugs` lists the open ones.
- **Closing a bug** means ticking it and adding `Fixed <commit>.`, or `Not a bug: …` /
  `Won't fix: …`. Why it broke and how it was fixed go in the commit message, not here.
- **`merge=union`** in `.gitattributes`, so two branches that each log a bug both keep
  their lines instead of conflicting. `eol=lf`, so the recipe appends the same bytes on
  every platform.
- **A docs test** checks each entry's shape, that names are unique, and that a ticked
  entry says how it closed. Like the rest of `docs/`, every source path an entry names
  must exist.

`docs/status.md` keeps accepted limitations and noise under Known issues. Its Resolved
write-ups stay as they are, as the record from before this list. The two failing-test
rows of its table moved here.

## Rejected alternatives

- **One file per bug, in the ADR template's style.** Rejected by the maintainer: too
  heavy to write mid-review. An ADR earns its structure by being read years later; most
  bugs need one sentence until someone fixes them.
- **Numbered entries.** Rejected: two branches would pick the same next number. That
  happened to the ADRs on 2026-10-07, when `tauri-app` and `main` both wrote 0052 to 0054.
  A chosen name collides only if both sides pick the same words, and the test catches
  that.
- **git-bug or a similar tool.** Rejected: its issues are git objects, not files in the
  tree, so ADRs and comments cannot link to them and the docs tests cannot check them.
- **GitHub Issues.** Not rejected on the merits, just not what was asked for: the record
  should travel with the code and be readable offline, as the ADRs are.
- **Keep using `docs/status.md`.** Rejected: it mixed state with history, and accepted
  limitations with things to fix.

## Consequences

Logging a bug is one command. The list is greppable, reviewable in a diff, and merges
without conflicts.

The costs: an entry's story is short by design, so the reasoning lives in commit
messages, which need to be written with that in mind. A union merge keeps both sides
even when both edited the same line, so ticking the same bug on two branches can leave
two copies of it. The docs test reports that as a reused name. An entry that names a
source path breaks the docs test when that file moves, as everywhere else in `docs/`.

## Notes

Cheap to reverse: the list is plain Markdown. If it ever grows unwieldy, ticked entries
can be cut, because git keeps them.
