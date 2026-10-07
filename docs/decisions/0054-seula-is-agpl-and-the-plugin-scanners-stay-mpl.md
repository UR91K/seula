# 0054. Seula is AGPL-3.0-or-later, and the plugin scanners stay MPL-2.0

- **Status:** Accepted
- **Recognized:** 2026-10-07
- **Decided:** 2026-10-07
- **Recorded:** 2026-10-07
- **Confidence:** decided now
- **Evidence:** `LICENSE` history (MPL-2.0 added in `f125dcd`, 2025-07-23, the only
  licence commit); `git shortlog -sne` (authorship); `cargo metadata` on 2026-10-07
  (licences of every dependency); `crates/vst-meta/Cargo.toml` (the scanner crate, a
  separate process per ADR-0004)

## Context

Seula has been under the Mozilla Public License 2.0 since `f125dcd`. MPL is copyleft
per file. Changes to Seula's own files have to stay open, but those files can be
combined with closed code into a larger work, and that larger work can be distributed
closed.

What the maintainer wants Seula to be:

- **An open-source desktop app that people can trust.** That includes trusting that it
  stays open.
- **Sold on Steam**, as paid builds with the source available to everyone. This is how
  Ardour works, and how Aseprite worked before 2016.
- **Possibly a paid cloud sync service.** People self-hosting sync instead is
  acceptable.
- **"Seula" becomes a trademark.** Forks are allowed, but under their own name.

Plugin scanning is `crates/vst-meta` today. AAX, AU and CLAP scanners are planned as
sibling crates, one per format, each running as a separate process (ADR-0004). The AAX
SDK is Avid's, under Avid's own licence. Whether GPL code may be linked against it was
not checked; the decision below makes that question irrelevant.

Who owns the code, which decides who can relicense it:

- **The maintainer** wrote all but 5 of the commits. That includes the ones by
  `UR91K`, `UR 91000` and the GitHub noreply address, and the 4 by
  `st10438340@vcconnect.edu.za`, which is the maintainer's college account (confirmed
  2026-10-07, now mapped in `.mailmap`).
- **Kramer Campbell** made the other 5 commits, in 2023. They touched `gui.py`,
  `main.py` and `.gitignore` in the Python version. None of that code survived the
  rewrite in Rust (confirmed by the maintainer; both files are gone from the tree).

So the maintainer can relicense alone.

Every dependency is under a permissive licence (MIT, Apache-2.0, BSD, ISC, Zlib, BSL,
Unlicense, CC0) or MPL-2.0. All of these can be combined with AGPLv3.

## Decision

1. **Everything except the plugin scanners is AGPL-3.0-or-later.** That covers the root
   crate, the Tauri shell (`web/solid/src-tauri/`) and the web frontend. The licence
   text is in `LICENSE`.
2. **The plugin scanner crates stay MPL-2.0.** That is `crates/vst-meta` now, and
   `aax-meta`, `au-meta` and `clap-meta` when they exist. Each crate carries its own
   copy of the licence, starting with `crates/vst-meta/LICENSE`.
3. **One additional permission under AGPL section 7: linking with the Steamworks SDK.**
   It's in `LICENSE-EXCEPTIONS.md`, using the wording of the FSF's template for this
   kind of exception. Whether Seula will use Steamworks at all is undecided. The
   permission is added now anyway, because adding it later would need the consent of
   everyone who has contributed by then, while adding it now costs nothing.
4. **No contributor licence agreement.** Contributions come in under the licence of the
   files they change and nothing more.
5. **Code up to and including `e697019` stays available under MPL-2.0.** A relicence
   cannot withdraw a licence already granted, and this one does not try to.

## Rejected alternatives

**Staying on MPL-2.0.** MPL allows Seula's files to be built into a larger closed work
and distributed that way. That does not fit an app whose promise is that it stays open.

**GPLv3 instead of AGPLv3.** For a desktop app the two are close to identical. The only
difference is AGPL section 13: a modified Seula that is offered to people over a network
has to offer them its source. Seula has an HTTP API, so wrapping a modified copy as a
hosted service is the easiest way to take it closed, and only AGPL covers that. The
usual cost of AGPL is that many companies ban it outright. That cost falls on libraries
other software embeds. Seula is an application, and its one reusable part, the scanner,
is not under AGPL (decision 2).

**GPLv2, or any "-only" licence.** Several dependencies are Apache-2.0, which can be
combined with GPLv3 but not GPLv2. "-only" was rejected because, with no CLA, "or later"
is the only way Seula could ever move to a future version of the licence.

**AGPL for the scanners too.** The scanners are the part of Seula other audio tools
would plausibly reuse, and AGPL would mostly stop that. An AAX scanner has to link
Avid's SDK, and under MPL that is allowed whatever Avid's terms say about the GPL. MPL
still keeps changes to the scanners' own files open, which is the part worth protecting.

**A contributor licence agreement.** This is the Aseprite route. Aseprite left the GPL
for a proprietary licence in 2016, which it could do because it held the rights to its
contributors' code (LibreSprite forked the last GPL version). A CLA would keep that
option open for Seula. It is exactly the option users would have to trust the
maintainer never to use. Without a CLA, once outside code lands, nobody can take Seula
closed, the maintainer included. That is the point.

**Rewriting history to replace the college identity.** ADRs are immutable and cite
commit hashes, and a rewrite would change every hash after May 2024. A `.mailmap`
entry shows the right identity in `git log`, `shortlog` and `blame` without changing
anything.

## Consequences

- **Selling on Steam is unaffected.** The obligation is to offer the source with the
  binaries, through a link from the store page or the app's About screen. Anyone may
  also build Seula and give it away or sell it, as with Ardour. What sets official
  builds apart is the trademark. **No trademark policy is written yet.**
- **The sync service is a separate program and is not covered by the client's
  licence.** If it is built from Seula's AGPL crates, it is under AGPL once other
  people's code is in those crates. Since self-hosting is acceptable, that would be
  fine.
- **Once outside contributions land, the licence is fixed.** Changing it, or adding
  another section 7 exception, needs every contributor's consent. This is intended.
- **AGPL section 13 almost never applies to ordinary use.** The HTTP API accepts
  requests only from localhost and Tauri origins. It applies to someone who modifies
  Seula and exposes it to other people over a network.
- **The licence boundary is the crate boundary.** The `seula` binary includes
  `vst-meta`'s wire types. MPL-2.0 section 3.3 allows that, because AGPL is one of its
  "Secondary Licenses", and those files stay MPL inside the combined work. The rule
  this creates: **code moves from `src/` into a scanner crate only if its author
  agrees to MPL for it.** The same applies to the shared protocol crate planned for when
  a second scanner exists. It belongs with the scanners and is MPL.
- **The `license` field in `Cargo.toml` and `package.json` says `AGPL-3.0-or-later`,
  which doesn't mention the Steamworks exception.** SPDX has no identifier for it.
  `LICENSE-EXCEPTIONS.md` is the authoritative statement. The exception only grants
  extra permissions, so the shorter field claims less than the truth, not more.

## Notes

Not reviewed by a lawyer. Two things were not checked: Avid's AAX SDK terms, and
Valve's Steamworks terms beyond the need for an exception.

Reversal is cheap until the first outside contribution lands. After that, any change
needs the consent of everyone who has contributed.
