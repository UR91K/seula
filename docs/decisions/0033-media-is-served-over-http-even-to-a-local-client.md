# 0033. Media is served over HTTP even when the client is on the same machine

- **Status:** Reaffirmed — considered and deliberately not changed
- **Recognized:** 2026-09-23, while checking which API routes the web frontend needs
- **Decided:** 2026-09-23
- **Recorded:** 2026-09-23
- **Confidence:** decided now
- **Evidence:** `src/http/handlers/media.rs` (`download_media`, mounted as
  `GET /api/v1/media/:id` in `src/http/server.rs`); ADR-0024 (which planned media download
  as a plain chunked response); ADR-0030 (the browser and Tauri targets)

## Context

Cover art and audition audio are files in `media_storage_dir`, on the same disk as the
daemon. Today the frontend is a page on the same machine. Sending a file from the local
disk through an HTTP response to a client on the same machine, instead of letting the
client read the file directly, looked roundabout to the maintainer when it came up.

The route already exists and works: `GET /api/v1/media/:id` returns the bytes with the
stored `Content-Type`. The question was whether to replace it with something local, such
as `file://` URLs, a Tauri asset protocol, or the API handing out filesystem paths.

## Decision

Keep serving media over HTTP. The frontend builds image and audio URLs from the
`cover_art_id` and `audio_file_id` it already receives, and never sees a filesystem path
for stored media.

## Rejected alternatives

- **Filesystem paths and `file://` URLs.** Rejected because a browser page on
  `http://localhost` cannot load `file://` resources, so this would work in Tauri and fail
  in the browser target (ADR-0030). It also ties the client to the daemon's disk.
- **A Tauri asset protocol for media.** Rejected for the same split: one code path per
  target for something that has nothing to do with the OS bridge ADR-0030 limits Tauri to.

## Consequences

The cost is one loopback copy per file, which is negligible for cover art and small for
audition audio.

What it buys is that the client does not have to be local. A frontend on another machine
or a phone needs no change to show media. That is not planned, but this keeps it
possible for free.

Two gaps in the route were fixed alongside this record, by serving the file through
`tower-http`'s `ServeFile`:

- **`Range` support.** Without it Chrome cannot seek in an `<audio>` element, so the
  audition audio would play but not scrub.
- **Streaming.** The handler read the whole file into memory before responding, where
  ADR-0024 planned a chunked response. That was harmless for cover art and wasteful for
  audio.

It also sends `Content-Disposition: attachment`, which browsers ignore for `<img>` and
`<audio>`. It stays, so that opening the URL directly downloads the file.
