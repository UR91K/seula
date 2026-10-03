# 0051. A project is named its file's name, extension included, until the user renames it

- **Status:** Accepted (retrospective)
- **Recognized:** 2026-10-04, when a real scan of the maintainer's projects returned names
  like `TAG.als` and the question was asked whether that was intended
- **Decided:** before 2026-10-04, no commit or date recovered
- **Recorded:** 2026-10-04
- **Confidence:** remembered (the maintainer's account, given 2026-10-04)
- **Evidence:** a real scan on 2026-10-04 over five `.als` files, whose `name` values were
  `TAG.als`, `JoelInMyHead.als`, `Sands 110 ambient.als` and so on, each equal to the file's
  name; the mockup data (`mockup/data/generate.py`) names projects without the extension,
  which the real scanner does not

## Context

A project's `name` is what the library shows and what the user can rename (`PUT
/api/v1/projects/:id/name`). Ableton has no project name of its own: the file name is all
there is. So a freshly scanned project has to be called something, and the question is
whether its name should be the file name as it is on disk or a tidied form of it.

## Decision

**A project's default name is its file's name with the `.als` extension kept.** The
extension is the signal: a name that still ends in `.als` is the raw file name and has not
been touched by the user. A name without it has been chosen.

## Rejected alternatives

None are recorded; the maintainer gave only the reason above. The obvious one is to strip
the extension for display, which would make a default name and a renamed one look the
same.

## Consequences

The user can tell at a glance which projects they have named and which they have not,
without a separate flag or column.

A UI must not assume the name and the file name differ. The grey file-name hint beside a
project's name is shown only when the two are not equal, so a default-named project shows
none. The mockups assumed a stem comparison, because their data has no extension; the
frontend compares the whole file name (`web/shared/format.ts`, `fileName`).

Limits that follow from the signal being the extension (inferred from the code, not
confirmed): a user who renames a project to exactly its file name, extension included, is
indistinguishable from one who has not; and the signal says nothing about a project
renamed to the same text with `.als` added by hand.

The mock data generator and the mockups do not follow this ADR. Their projects are named
without the extension. They are static reference material and were left as they are.

## Notes

Reversal is cheap in the database (a one-off update of names that equal their file name)
but is a visible behaviour change, and it would remove the only record of which names the
user chose.
