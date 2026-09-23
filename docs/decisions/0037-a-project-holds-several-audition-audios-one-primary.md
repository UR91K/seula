# 0037. A project holds several audition audios, and one of them is the row's

- **Status:** Accepted — implemented 2026-09-23
- **Recognized:** 2026-09-23, reviewing the inspector's Audition section on the projects
  board
- **Decided:** 2026-09-23
- **Recorded:** 2026-09-23
- **Confidence:** decided now
- **Evidence:** `src/database/schema.sql` (`projects.audio_file_id`, one media file per
  project); `src/services/media.rs` (`store_audio_file` sets the uploaded file as the
  project's audio, replacing any previous one); `src/database/media.rs` (the orphan
  queries, which count only `projects.audio_file_id` as a reference); ADR-0011 (a
  `SCHEMA_VERSION` bump discards the user's database); ADR-0036 (a board cannot show data
  the API does not have)

## Context

A project could carry one audition audio: a bounce of the track, playable from the row.
The maintainer wants several, such as a rough mix, a master and an alternate version,
all listed in the inspector, with a choice of which one the row's play button plays.

The one audio is a single column, `projects.audio_file_id`. Uploading a second replaced
the first, and the replaced file stayed on disk with nothing referencing it, so the next
orphan cleanup deleted it.

## Decision

**A list, plus a primary.** A new table, `project_audio_files`, holds each project's
audios in the order they were added:

```sql
project_audio_files (project_id, media_file_id, position, added_at,
                     PRIMARY KEY (project_id, media_file_id))
```

`projects.audio_file_id` stays, with a narrower meaning: it is the **primary** audio,
the one the row plays. The primary is always one of the project's listed audios.

**Why keep the column.** Replacing it with a flag on the new table would change an
existing table, which needs a `SCHEMA_VERSION` bump, which discards the user's database
(ADR-0011). Keeping it and adding a table costs nothing. Every open of the schema copies
an existing `audio_file_id` into the new table (`INSERT OR IGNORE`), so a database from
before this change ends up with each existing audio listed as its project's only,
primary audio.

**Behaviour:**

- **Upload** adds the file to the list. It becomes the primary only if the project had
  none. Uploading used to replace the audio silently, and replacing now has to be done on
  purpose.
- **Setting the primary** (the existing `PUT /api/v1/projects/:id/audio-file`) also adds
  the file to the list if it is not there already.
- **Clearing the primary** (the existing `DELETE` on the same route) leaves the list
  alone, so the row has nothing to play while the inspector still lists the audios.
- **Removing an audio** from the list (new:
  `DELETE /api/v1/projects/:id/audio-files/:media_file_id`) promotes the next audio in
  list order when the removed one was the primary. It detaches the file only, and the
  media file itself is deleted by the orphan cleanup, as cover art already is.
- **Listing** (new: `GET /api/v1/projects/:id/audio-files`), and every project DTO now
  carries `audio_files`: each audio's metadata and whether it is the primary.
  `audio_file_id` stays in the DTO as the primary's id.
- **"Has audio"** means the list is non-empty, in the list filter and the project
  statistics alike. Before, it meant the primary was set.
- **Orphan detection** counts a file referenced from the list as in use.

## Rejected alternatives

- **Replace `audio_file_id` with an `is_primary` flag on the new table.** Rejected because
  it changes an existing table, and the bump that requires would discard every user's
  tags, notes, tasks and collections for no gain in behaviour.
- **Order as the choice: whichever audio is first plays.** Rejected because choosing the
  row's audio by dragging it to the top is indirect, and it ties two separate decisions
  (the order of the list and what the row plays) together.
- **Upload keeps replacing the audio.** Rejected because with a list, silently throwing
  away the previous audio is the wrong default. It was also how audio got lost before.

## Consequences

The gRPC API is not extended (ADR-0028 retires it). It shares the service layer, so its
upload also stops replacing the audio. Its single-audio view stays accurate, because
`audio_file_id` still names the primary.

Deleting a media file directly (`DELETE /api/v1/media/:id`) removes it from every list
through the foreign key's cascade. If it was a primary, the column's
`ON DELETE SET NULL` leaves that project without one, and no other audio is promoted.
That is an administrative path, not the inspector's, so it is left as is.

Reordering the list is not provided. Nothing needs it yet, and `position` is there if
something does.
