# 0035. Keys go over the wire with both a sharp and a flat display name, in Ableton's own scale names

- **Status:** Accepted — implemented 2026-09-23
- **Recognized:** 2026-09-23, when the mockup had to turn `FSharp` and `HarmonicMinor`
  into something a person could read
- **Decided:** 2026-09-23
- **Recorded:** 2026-09-23
- **Confidence:** decided now. The scale names marked verified below were read from the
  maintainer's own `.als` files; the rest come from Ableton's scale list and are not
  verified
- **Evidence:** `src/models.rs` (`Tonic` and `Scale`, whose `Display` is `Debug`);
  `src/scan/parser.rs` (the `ScaleInformation` name match); a read-only tally of
  `<ScaleInformation>` in 603 of the maintainer's `.als` files on 2026-09-23;
  `src/database/collections.rs` (`most_common_key` built in SQL as
  `tonic || ' ' || scale`)

## Context

Keys reached the API as enum names: `{"tonic": "FSharp", "scale": "HarmonicMinor"}`, and
the statistics routes sent strings built in SQL, such as `"FSharp Minor"`. Every client
would have had to hold its own table of display names.

The maintainer wants a sharp/flat switch like Ableton's: the same key shown as F♯ or G♭,
with the real musical symbols ♯ and ♭, not `#` and `b`.

Looking into display names turned up a parser bug. `src/scan/parser.rs` mapped only
`"Major"` and `"Minor"` and sent every other scale name to `Scale::Empty`, behind a
comment that said "Add other scale mappings as needed". The maintainer's files contain
1,048 clips in other scales, including 510 Mixolydian, 45 Minor Pentatonic and 15
Messiaen 3. All of them were stored as `Empty`. One of them, Kumoi, is not in the
`Scale` enum at all.

## Decision

**On the wire.** A key is sent as four fields:

```json
{ "tonic": "FSharp", "scale": "HarmonicMinor",
  "sharp": "F♯ Harmonic Minor", "flat": "G♭ Harmonic Minor" }
```

`tonic` and `scale` stay, because the list filters take them as input. The frontend
shows `sharp` or `flat` according to its switch and holds no name tables. The switch
position is a UI preference and persists with the others (ADR-0031). The key statistics
use the same shape, with a count, in place of their SQL-built strings.

**Names are Ableton's, in title case, with real symbols.** Tonics are C, C♯/D♭, D,
D♯/E♭, E, F, F♯/G♭, G, G♯/A♭, A, A♯/B♭, B. Scale names are the strings Ableton writes,
with `#` rendered as ♯: "Dorian ♯4", "Minor Pentatonic", "Messiaen 3". The flat form
changes only the tonic. A scale name is the same either way, so "Dorian ♯4" keeps its
sharp in flat mode, as it does in Ableton.

**Names live in Rust.** `Scale` gets a mapping to and from Ableton's name, and a key
gets `sharp_name()` and `flat_name()`. The parser, the HTTP layer and the CLI all use
them.

**The parser keeps every scale.** It maps Ableton's names through that table.
`Scale::Other(String)` keeps any name the table does not know yet, rather than dropping
it to `Empty`. That covers scales added in a later Live version and any spelling below
that turns out wrong.

Verified in the maintainer's files: Major, Minor, Mixolydian, Minor Pentatonic, Major
Pentatonic, Messiaen 3, Hirajoshi, Locrian, Kumoi, Dorian #4, Whole Tone.

## Rejected alternatives

- **Send enum names and let the frontend format them.** Rejected because it spreads the
  name tables into TypeScript, where they must be kept in step with Rust, and the CLI
  would need its own copy too. The frontend is meant to be a thin display.
- **An intermediate representation interpreted on the TypeScript side.** Rejected for the
  same reason: it still needs tonic and scale tables on the client.
- **Send only a display string.** Rejected because the filters need the enum names back,
  so the client would have to parse "F♯ Minor" in reverse.
- **`#` and `b`.** Rejected at the maintainer's request. The data is Unicode throughout,
  and nothing on the path needs ASCII.

## Consequences

Two display strings per key cost a few dozen bytes per project. That is not measurable
here.

Projects parsed before the parser fix keep `Empty` for non-major/minor scales until they
are rescanned. The scanner re-parses only changed files, so recovering them needs a
forced rescan.

`Scale::Other` stores the raw name in the `key_signature_scale` column. It then filters
and groups like any other scale, just without an enum variant of its own.
