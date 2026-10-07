# 0053. A similarity measure for Ableton projects

- **Status:** Proposed — not scheduled
- **Recognized:** long before 2026-09-24 (a standing idea of the maintainer's); written
  down while deciding project identity (ADR-0052)
- **Decided:** not yet
- **Recorded:** 2026-09-24
- **Confidence:** decided now (that it is worth recording, not that it will be built)
- **Evidence:** ADR-0052 (the Save As suggestion heuristic, which needs a notion of
  "internal similarity"); ADR-0001 (the parser is one pass, so any new data is extracted
  there); ADR-0015 (DAW-specific data stays out of the generic core)

## Context

ADR-0052 suggests links between files that look like the same song after a Save As, and
the strongest signal it names is internal similarity: track count, track names and
colours in order, tempo, plugins, samples. That heuristic can start as a handful of
comparisons. The maintainer's longer-standing idea is a proper measure: something like
Levenshtein distance, fuzzy hashing or locality-sensitive hashing, but built for what is
inside an Ableton set rather than for bytes or text.

Other uses beyond ADR-0052:

- Finding near-duplicates and forgotten copies across a library.
- Grouping the versions of one song (*v1*, *v2*, *final*) that were never linked.
- "Projects like this one": sets that share a template, a sound palette or a structure.

Generic tools do not fit the data:

- **Byte-level fuzzy hashes** (ssdeep, TLSH) on the `.als` file see gzip output, where
  one changed byte early on changes everything after it.
- **The same hashes on the decompressed XML** are dominated by churn that means nothing
  musically: view state, zoom and scroll positions, selection, internal ids renumbered
  on save.
- **Plain text distance** on the XML has the same problem, and is too slow for files of
  tens of megabytes.

A useful measure compares what the user made, and ignores what Live records about the
session.

## Proposal

**Extract a project fingerprint in the parser's single pass, then compare fingerprints
facet by facet.**

Candidate facets, roughly in order of cost:

| Facet | Shape | Compared by |
|---|---|---|
| Tempo, time signature, key, length | Scalars | Closeness, with tolerances |
| Tracks in order: type, normalised name, colour | Sequence | Edit distance over tracks as tokens, where two tracks cost less to substitute the more alike they are |
| Plugins (by `PluginKey`, ADR-0005) and native devices | Set | Jaccard |
| Samples (by file name as well as path, so a moved sample pack still matches) | Set | Jaccard |
| Clip names, scene names, locators | Set | Jaccard |
| MIDI content: note n-grams by interval and rhythm, so a transposed part still matches | Set of shingles | Jaccard, estimated with MinHash |

The score is a weighted combination of the facets. Each facet also reports on its own, so
a suggestion can say why ("same 14 tracks in the same order, 90% of samples shared")
rather than show a bare number.

**Retrieval comes after comparison, and only if needed.** Comparing a few projects is
cheap: ADR-0052 only compares newcomers with orphans from the same scan, which is a
handful. Even a full library of a few thousand projects is a few million pair
comparisons of small fingerprints. Locality-sensitive hashing (MinHash signatures with
banding, or SimHash for the weighted facets) earns its place only when a library-wide
"find similar" is too slow by brute force. It narrows the candidates and never replaces
the scoring.

**User decisions calibrate the weights.** Every suggestion from ADR-0052 that the user
confirms or rejects is a labelled pair: two sets the user says are, or are not, the same
song. Those pairs are the data for tuning the weights and the threshold, instead of
guessing them.

**The fingerprint is Ableton-specific data** and is stored like Ableton's version
metadata (ADR-0015): in its own side table, keyed by project. A new table needs no
`SCHEMA_VERSION` bump.

## Open questions

- Which facets carry the most signal per unit of cost. Tracks in order and the plugin and
  sample sets are the likely first pass; MIDI shingles are the most expensive to extract
  and store, and the most distinctive.
- Normalising track names: Live appends numbers to duplicates (*Audio 2*), and many
  tracks keep default names. Default names probably carry little weight.
- Whether colour is compared as Live's palette index or as a colour distance.
- How big a stored fingerprint may get, and whether MIDI shingles are kept whole or only
  as a MinHash signature.
- What "similar" should mean for the library view as opposed to the Save As check: the
  same song, or the same template and sound palette. They may need different weights.
- Whether a fingerprint is computed for Live's own `Backup` copies, which are the
  clearest near-duplicates there are, or whether those stay out of the library.

## Consequences

None until built. ADR-0052's suggestion heuristic starts with direct comparisons, and
this measure would replace it if it proves better on the labelled pairs.
