# 0016. Dispatch DAW parsers through a closed enum, not `dyn DawParser`

- **Status:** Accepted
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `docs/archive/generalisation_plan.md` (`DawParser`/`ParserRegistry`, the
  rejected shape); `src/scan/parallel.rs:11,23-30` (`ParseResult`, `ParserWorker::
  process_file` — the concrete call site this replaces); ADR-0002 (why `parallel.rs`'s
  mechanics are not to be touched casually); ADR-0014 (DAW generalisation scope, and its
  rejection of a dynamically-loaded parser-plugin system); ADR-0015 (the sibling decision
  for DAW-specific *storage*, same reasoning applied to *dispatch* here)

## Context

`ParseResult` in `src/scan/parallel.rs:11` is concretely typed to `LiveSet`/`LiveSetError`,
and `ParserWorker::process_file` (line 23) calls `LiveSet::new(path)` directly — the one
remaining piece ADR-0014's Notes flagged as unanswered: how does a generic parser get
called at all, for the one DAW that exists today.

The archived plan's answer was `dyn DawParser` in a `Vec<Box<dyn DawParser<Error =
Box<dyn Error + Send + Sync>>>>`, matched by a `can_parse` loop. Its appeal is low blast
radius: adding a DAW means writing a struct and one `registry.register(...)` call, with
no existing code to touch.

That benefit was weighed against what it costs, and against who it is actually for.

**Who it is for:** third parties adding parsers independently of the maintainer, without
needing to touch or understand existing call sites. The maintainer confirmed this is not
the situation — no anticipated community of external DAW-parser contributors — and
ADR-0014 already rejected the more extreme version of this idea, a dynamically-loaded
parser-plugin system, for reasons adjacent to ADR-0004. Low-blast-radius extensibility is
solving a problem that does not exist here.

**What it costs, concretely:**
- Each implementor's own error type must be boxed to `Box<dyn Error + Send + Sync>`
  before it can sit in the shared `Vec` — a per-implementor step, and one a caller
  catching `ParseError::ParserError(e)` cannot later match on for a specific DAW's error
  variant.
- Forgetting to call `registry.register(...)` for a new parser is a silent runtime miss —
  the file falls through to `UnsupportedFileType` with no compiler complaint — rather
  than a compile error.
- It is the one place that would introduce type erasure into a codebase that has
  consistently chosen the opposite everywhere else it had the option: `PluginKey`,
  `vst-meta`'s `FormatExtra`, and twice in ADR-0015 (rejecting a `daw_specific_data JSON`
  column, and rejecting a string-only `AbletonVersion` in favor of keeping structured,
  queryable columns).

The maintainer's own stated preference is for the compiler to enforce completeness while
adding a parser, not for minimal-touch extension — the same trade already made once,
deliberately, in porting this project from Python to Rust for correctness.

Plain generics were also considered and do not solve the actual problem: the registry
needs to hold several different concrete parser types and pick one at runtime by file
extension, which is a heterogeneous-collection problem. A generic `ParserWorker<P:
DawParser>` only works when the concrete type is already known at the call site, which it
is not here.

## Decision

Dispatch through a closed enum instead:

```rust
enum AnyDawParser {
    Ableton(AbletonParser),
    // Reaper(ReaperParser), added when that DAW is actually built
}

enum ParseError {
    Ableton(LiveSetError),
    // Reaper(ReaperError),
}

impl AnyDawParser {
    fn parse(&self, path: &Path) -> Result<Project, ParseError> {
        match self {
            AnyDawParser::Ableton(p) => p.parse(path).map_err(ParseError::Ableton),
        }
    }
}
```

`src/scan/parallel.rs` stays concretely typed — not made generic — matching ADR-0002's
instruction not to touch its mechanics casually. Only the payload changes: `ParseResult`
becomes `Result<(PathBuf, Project), (PathBuf, ParseError)>`, and `ParserWorker::
process_file` calls the enum's dispatch function instead of `LiveSet::new` directly. The
pool's threading, the channel, and the guard-lifetime-sensitive `drain` function are
untouched.

Adding a DAW means: write its parser, add an enum variant, add a `ParseError` variant, and
fix every match the compiler now flags as non-exhaustive. That last part is the point, not
a cost to minimize.

## Rejected alternatives

- **`dyn DawParser` with a `ParserRegistry`**, as sketched in the archived plan. See
  Context — solves for extensibility by uncoordinated third parties, which is not this
  project's situation, at the cost of the one type-erasure exception in an otherwise
  erasure-averse codebase, plus a silent (not compiler-caught) failure mode for a
  forgotten registration.
- **Generic `ParserWorker<P: DawParser>`.** Does not address the actual requirement —
  runtime dispatch across a heterogeneous set of parsers by file extension — since a
  generic parameter must be fixed at each call site rather than chosen dynamically.

## Consequences

No code changes yet — `parallel.rs` still calls `LiveSet::new` directly until Phase 1
(ADR-0014) implementation begins. This ADR settles the last open mechanism question for
that phase: scope (0014), storage (0015), and now dispatch (this). An implementation plan
can be written without further open design questions on the Ableton-only slice of the
work.

Every future DAW addition touches more call sites than a `dyn`-based registry would have
required — every exhaustive match over `AnyDawParser`/`ParseError` gets a new arm. This is
accepted as correct, not incidental: it is the enforcement mechanism this decision exists
to get.

## Notes

If a future maintainer decides third-party or community parser contributions *are*
wanted after all, that is grounds to revisit this decision (and possibly ADR-0014's
rejection of dynamic loading too) — but it should be reopened deliberately, as a change in
who this project is for, not patched around locally.
