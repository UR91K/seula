# 0029. The web frontend is React, TypeScript, Vite and hand-rolled CSS, living in this repository

- **Status:** Accepted in part: the framework is superseded by 0047 (Solid, not React), and the open serving question is settled by 0048
- **Recognized:** 2025-08-14, when `docs/archive/FRONTEND_SPEC.md` was first sketched;
  stack settled 2026-09-16 in conversation
- **Decided:** 2026-09-16
- **Recorded:** 2026-09-16
- **Confidence:** decided now
- **Evidence:** `docs/archive/FRONTEND_SPEC.md` (the sketch, sat uncommitted and
  gitignored for a year before landing in `66dc3f8`); ADR-0024 (the HTTP surface this
  consumes); ADR-0028 (the direction that makes HTTP the only surface);
  `docs/architecture/frontend.md` (the resulting shape)

## Context

ADR-0028 records the maintainer's direction: give up on a native Rust GUI and serve a
web frontend over the HTTP router from ADR-0024. That ADR named React as the likely
choice but decided nothing about it. With the HTTP surface now complete for every
domain, the frontend is the next thing to build, and the first question is what it is
built from and where it goes.

The sketch in `docs/archive/FRONTEND_SPEC.md` describes five views (projects,
collections, plugins, samples, stats) under a shared shell. It was written a year before
this decision, and the maintainer confirmed on 2026-09-16 that it is still fairly
accurate. Static HTML mockups are being made from it before any React is written, so
the layout questions get answered in a medium that is cheap to throw away.

## Decision

**Stack.** React with TypeScript, built by Vite. CSS is written by hand: no component
library, no utility framework, no CSS-in-JS. Icons come from an icon library, never
emoji.

**Placement.** The frontend lives in this repository as a sibling of the Rust crate, in
a `web/` directory, giving a monorepo of one Cargo workspace plus one npm project. The
mockups that precede it live under `docs/mockups/` and are documentation, not a build
input.

**Serving is not decided here.** Whether the built assets are embedded into the daemon
binary and served from `http_port`, or served by something else, is left open. It is a
packaging question that depends on how ADR-0028 and ADR-0030 resolve, and nothing in
the mockup or early React work needs the answer. During development Vite's dev server
proxies API calls to `http_port`; the CORS layer from ADR-0024 already accepts any
localhost origin, so that works without a server change.

## Rejected alternatives

- **A component library (MUI, Radix, shadcn and the like).** Rejected because the
  sketch's design language is Windows Explorer and media-library grids, which every
  library fights against, and because the surface is small enough (five views, one
  shell) that a library's abstraction costs more than the dozen components it would
  save writing. Hand-rolled CSS also means the mockups' stylesheet carries forward
  rather than being rewritten in a library's vocabulary.
- **A separate repository for the frontend.** Rejected because the frontend has exactly
  one consumer of the API, which is itself, and the DTOs in `src/http/dto/` are the
  contract. Keeping both sides in one tree means a DTO change and the TypeScript type
  that mirrors it land in the same commit. The cost is a repo with two toolchains.
- **A framework other than React.** Not seriously weighed. React is what the maintainer
  named in ADR-0028 and has the widest pool of reference material; nothing in the
  sketch needs anything a framework choice would change. Cheap to reverse before the
  first view is written, expensive after.
- **Building the React app straight from the sketch, no mockups.** Rejected because the
  sketch is prose and leaves layout questions (details panel width, where the batch
  toolbar appears, how the plugin hover list renders) that are cheaper to answer in
  static HTML than in components with state.

## Consequences

Two toolchains in one repository: `cargo` and `npm`. CI and contributor setup grow by a
Node install. `tests/docs_tests.rs` does not check paths under `web/` or
`docs/mockups/`, so documentation that names files there is unchecked prose; keep such
references to directories rather than files.

The hand-rolled CSS decision is a commitment to maintain a stylesheet. The mockups'
shared stylesheet is the seed of it.

The DTO layer from ADR-0024 now has a second mirror: proto, DTO, and TypeScript type.
That is the cost ADR-0024 already accepted, extended by one.

## Notes

The serving question is the one thing here that a reader might expect to find and will
not. It is deferred on purpose. When it is decided it gets its own ADR, and the likely
trigger is ADR-0028 firming up, since "one binary is the whole app" only makes sense
once the binary has no CLI.

`docs/archive/FRONTEND_SPEC.md` stays in the archive and is not edited. Its content, as
verified on 2026-09-16, is restated in `docs/architecture/frontend.md`, which is the
mutable home for the shape.
