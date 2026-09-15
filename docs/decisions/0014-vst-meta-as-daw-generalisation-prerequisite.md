# 0014. vst-meta was conceived as the prerequisite for multi-DAW support

- **Status:** Accepted (retrospective)
- **Recognized:** sometime before `vst-meta` was created (undated; the maintainer reread
  `generalisation_plan.md` "over a few months" beforehand)
- **Decided:** predates ADR-0004 through ADR-0013, all recorded 2026-09-15
- **Recorded:** 2026-09-15
- **Confidence:** remembered
- **Evidence:** `docs/archive/generalisation_plan.md` (the plan itself); `crates/vst-meta`
  and ADR-0004/0005/0006/0007/0012 (what it produced); none of those ADRs mention this
  motivation, because they were written the same day as this one, after the fact

## Context

`generalisation_plan.md` — an AI-assisted brainstorming session, later archived — sketched
a generic `Project` model and `DawParser` trait/registry to let Seula support DAWs other
than Ableton Live. It never became an ADR and `docs/archive/README.md` now warns not to
read it for orientation.

What that log did not do is notice its own prerequisite: as long as plugin metadata came
from Ableton's own database (the pre-ADR-0006 design), no other DAW's project files could
be supported, because there was no source of "what plugins exist and are installed" that
didn't already assume Ableton was installed and had scanned recently.

The maintainer saved the log, reread it over a period of months, and reached that
conclusion independently — the log had glossed over exactly this. That realization is
what led to building `crates/vst-meta`, an out-of-process scanner that is its own source
of plugin truth, and to `PluginKey`'s `(format, uid)` identity model, neither of which is
tied to Ableton's database.

The ADRs documenting that work (0004, 0005, 0006, 0007, 0012) were all written on
2026-09-15, well after `vst-meta` itself was built. They give a defensible account of
*why vst-meta is correct on its own terms*, but none of them record that it originated as
step one of the generalisation plan — that context existed only in the maintainer's memory
until this ADR.

## Decision

Recognize `vst-meta` and the `(format, uid)` identity model retroactively as Phase 0 of
DAW generalisation: an independent, DAW-agnostic source of plugin-install truth, built
before — and as a precondition for — any generic project-model work.

## Rejected alternatives

- **Leave this unrecorded**, since `vst-meta` already has its own ADRs and works on its
  own merits regardless of this history. Rejected: the connection is exactly the kind of
  fact that decays fastest — it already existed only in memory once, and surfaced again
  only by chance in conversation. The five existing ADRs are correct but incomplete
  without it: a future reader would have no way to know `vst-meta` was anything more than
  a plugin-metadata quality improvement.
- **Fold this into ADR-0004** (out-of-process plugin scanning) as an added note. Rejected:
  ADR-0004's context is the crash-isolation problem, a different and independently
  sufficient reason to build `vst-meta`. Conflating the two would misstate which one was
  load-bearing at the time the decision was actually made.

## Consequences

The DAW-generalisation effort is not "undocumented and unstarted" — it is one phase in,
with that phase already shipped, tested, and in production. Anyone picking the effort back
up should start from the archived plan's Phase 1 (generic `Project`/`DawParser`/registry),
not from zero, and should not re-derive the plugin-scanning prerequisite the plan itself
missed.

Nothing in the codebase changes as a result of this ADR. It records intent, not code.

## Notes

The archived plan is also, on its own merits, over-scoped past this point: it designs
Reaper/FL Studio parsers and a dynamically-loaded community-parser system before a second
parser exists, and it never addresses how plugin identity generalizes past Ableton or how
a `DawParser` trait would integrate with the hand-rolled worker pool in
`src/scan/parallel.rs` (ADR-0002). A future Phase 1 ADR should scope down to the generic
model/trait/registry for Ableton only, and treat the dynamically-loaded parser-plugin idea
as in tension with ADR-0004's own lesson about third-party code running in-process.
