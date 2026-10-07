# 0058. Cover art is uploaded as bytes from a file input, and the daemon keeps its own copy

- **Status:** Accepted — implemented on branch `tauri-app`
- **Recognized:** 2026-10-05, planning the collections view
- **Decided:** 2026-10-05, agreed by the maintainer
- **Recorded:** 2026-10-05
- **Confidence:** decided now; the daemon's behaviour was read from `store_file` in the
  media module and the upload handler, then exercised against a scratch daemon on
  2026-10-05: a PNG chosen in the file input was stored, shown on the card from
  `GET /api/v1/media/:id`, and removed again
- **Evidence:** ADR-0033 (media is served over HTTP); ADR-0048 (OS calls are Tauri
  commands only); `POST /api/v1/media/cover-art`, `PUT /api/v1/collections/:id/cover-art`
  and `GET /api/v1/media/:id`; the media module's `store_file`, which validates the bytes,
  checksums them and writes a copy into the media storage directory under a generated id

## Context

A user sets a collection's cover from a file on disk. The question was whether the
frontend should tell the daemon the file's path, or send the file.

## Decision

**The frontend sends the bytes.** The cover area takes a click, which opens a file input
(the native picker), or a file dropped onto it. The frontend reads the file, posts it to
the upload route, then sets the returned media id on the collection. The daemon stores its
own copy and serves it by id from then on (ADR-0033).

No path ever crosses the API. A file input and a drop are web-platform features, not OS
calls, so ADR-0048 is not engaged and no Tauri command is added.

## Rejected alternatives

- **Send a path and let the daemon read it.** Rejected: the upload route and the media
  store are built around bytes and a stored copy; a path would need a new route and would
  make the cover depend on a file the user can move or delete. It would also let any
  client name any file on the daemon's disk.
- **A Tauri command that opens the picker.** Rejected: a second code path for something
  the webview already does (ADR-0033's reasoning for media applies).

## Consequences

The size limit `max_cover_art_size_mb` and the file-type validation are enforced by the
daemon; the frontend shows its error rather than checking ahead. Replacing a cover leaves
the old file orphaned until the media cleanup removes it.
