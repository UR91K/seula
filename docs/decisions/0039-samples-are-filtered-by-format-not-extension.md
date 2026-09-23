# 0039. Samples are filtered by format, not extension

- **Status:** Accepted — implemented 2026-09-23
- **Recognized:** 2026-09-23, planning the samples board
- **Decided:** 2026-09-23
- **Recorded:** 2026-09-23
- **Confidence:** decided now
- **Evidence:** `src/database/samples.rs` (the extension buckets, and the filter that
  matched `path LIKE '%.<value>'`); the mock snapshot, where `/api/v1/samples/extensions`
  reported 52 `aiff` samples and filtering by `aiff` returned none

## Context

The samples list offered an extension filter, and `/api/v1/samples/extensions` counted
samples in buckets: `wav`, `aiff` (which took `.aif` and `.aiff`), `mp3`, `flac`, `ogg`,
`m4a` and `other`. The filter did not use those buckets. It matched the value as a
literal extension, so choosing `aiff` from a dropdown built from the buckets missed
every `.aif` file, and choosing `other` matched nothing.

The maintainer's view: the filter is really about format. `.aif`, `.aiff` and `.aifc`
are one format, the Audio Interchange File Format.

## Decision

**One table of formats, used everywhere.** `SAMPLE_FORMATS` in `src/models.rs` lists
each format Live loads with its id, display name and extensions:

| id | name | extensions |
|---|---|---|
| `wav` | WAV | wav, wave |
| `aiff` | AIFF | aif, aiff, aifc |
| `flac` | FLAC | flac |
| `mp3` | MP3 | mp3 |
| `ogg` | Ogg Vorbis | ogg |
| `aac` | AAC | m4a, aac |

Anything else is the format `other`. The SQL that buckets samples for counting and the
SQL that filters them are both generated from this table, so the two cannot drift apart
again.

**The API speaks formats.** The HTTP list and search routes take `format_filter`, which
accepts a format id, one of its extensions (`aif` finds AIFF), or `other`. A value that
is none of these matches nothing rather than being ignored.
`GET /api/v1/samples/formats` replaces `/api/v1/samples/extensions`: every format in
table order with its name, extensions and counts, then `other` if any sample has it.

## Rejected alternatives

- **Keep extensions and fix the bucket mismatch.** It would work, but it would make the
  user pick between `.aif` and `.aiff` when what they mean is AIFF.
- **Derive formats from whatever extensions the library contains.** No names, no grouping,
  and a stray `.WAV` would be its own entry.

## Consequences

The gRPC API keeps its field names (`extension_filter`, `samples_by_extension`), since
ADR-0028 retires it. It shares the service layer, so its filter now takes format ids or
extensions too, and its bucket for `.m4a` is now `aac`.

Adding a format is one row in the table.
