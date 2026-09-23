"""Seed a mock Seula database and snapshot the real HTTP API over it.

Two steps, both deterministic (fixed seed):

1. ``seed``: build ``seula-mock.db`` from ``src/database/schema.sql`` -- the same schema
   the program compiles in -- and fill it with plausible fake data written exactly the
   way the Rust code writes it (epoch-second datetimes, lowercase uid hex, PluginFormat
   display strings, Tonic/Scale FromStr names).

2. ``snapshot``: start the real ``seula --server`` against that database with a
   throwaway config, GET the read endpoints the frontend uses, and write the responses
   to ``mock-api.json`` plus ``mock-api.js`` (the same data as a global, so the mockups
   work from file:// without a server).

The JSON therefore has the exact shapes the real DTOs produce; nothing here restates
them. Run from anywhere:

    python mockup/data/generate.py            # seed + snapshot
    python mockup/data/generate.py seed       # database only
    python mockup/data/generate.py snapshot   # re-snapshot an existing database
"""

import hashlib
import json
import os
import random
import re
import shutil
import sqlite3
import subprocess
import sys
import time
import urllib.request
import uuid
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
REPO = HERE.parents[1]
SCHEMA = REPO / "src" / "database" / "schema.sql"
CORE_RS = REPO / "src" / "database" / "core.rs"
DB = HERE / "seula-mock.db"
MEDIA_DIR = HERE / "media"
CONFIG = HERE / "mock-config.toml"
OUT_JSON = HERE / "mock-api.json"
OUT_JS = HERE / "mock-api.js"

# Off the defaults (50051/50052) so a running real daemon does not collide.
GRPC_PORT = 50151
HTTP_PORT = 50152

rng = random.Random(20260923)


def uid() -> str:
    return str(uuid.UUID(int=rng.getrandbits(128), version=4))


def epoch(y, m, d, hh=0, mm=0) -> int:
    return int(datetime(y, m, d, hh, mm, tzinfo=timezone.utc).timestamp())


def rand_time(start: int, end: int) -> int:
    return rng.randint(start, end)


def schema_version() -> int:
    m = re.search(r"pub const SCHEMA_VERSION: i32 = (\d+);", CORE_RS.read_text("utf-8"))
    if not m:
        sys.exit("could not read SCHEMA_VERSION from core.rs")
    return int(m.group(1))


# --------------------------------------------------------------------------- vocab

WORDS_A = """smile again sipper garage pec8 hellfire jazzchords casiopea dance game fart
bocbeat thaaa pickup junlge upbell dist metallic acid rat drum band abstract night
glass velvet static humid neon tape rust hollow copper lunar cheap soft broken slow
ferry orbit polar sugar paper basement rooftop cold warm late early dusty""".split()
WORDS_B = """vapor type deus bass chords rude buster music feedback house ambient techno
loop idea sketch groove jam demo riff break tekno dub garage swing funk drift heat
bounce shuffle tape vox keys pad lead wave pulse""".split()
SUFFIX = ["", "", "", " 2", " 3", " 4", " v2", " final", " (old)", " edit", " 1"]

TAGS = ["wip", "finished", "sketch", "mix ready", "needs master", "vocals", "collab",
        "lofi", "techno", "house", "ambient", "dnb", "game audio", "live set", "archive me"]

TASKS = ["fix kick and bass clash", "bounce stems", "automate filter in chorus",
         "replace placeholder vocal", "check mono compatibility", "export for mastering",
         "tighten drum timing", "rewrite bridge", "send to joel", "sidechain pads",
         "resample the lead", "cut intro by 8 bars", "fix clipping on master",
         "add ear candy in drop", "record real bass", "tidy up arrangement",
         "consolidate audio", "find better snare", "reference against mix",
         "reduce reverb tail"]

NOTES = ["Tempo feels slow, try +4 bpm.", "Mix is close. Vocals too loud in verse 2.",
         "Started on the train. Chords from the voice memo.",
         "Joel has the stems from 3 June.", "Keep the tape wobble, it is the whole point.",
         "Missing the Omnisphere patch on the laptop."]

COLLECTIONS = [
    ("night drives", "Slow tracks for the EP, sequenced for a single side.", "cover-01"),
    ("acid + tekno", "Everything with a 303 in it.", "cover-02"),
    ("band with joel", "Shared sessions and bounces.", "cover-03"),
    ("rat music 3", None, "cover-04"),
    ("abstract", "Weird ones. Not for release.", "cover-05"),
    ("fbm", "Tape-saturated beat tape.", "cover-06"),
    ("drum and bass stuff", None, "cover-07"),
    ("game music", "Loops and stingers for the jam entry.", "cover-08"),
    ("transcription", "Covers and learning pieces.", "cover-09-ns"),
    ("unsorted demos", None, None),  # deliberately no cover art
]

# (name, vendor, kind, is_instrument, installed) -- installed None = never scanned.
PLUGINS = [
    ("Pro-Q 3", "FabFilter", "VST3", False, True), ("Pro-C 2", "FabFilter", "VST3", False, True),
    ("Pro-L 2", "FabFilter", "VST3", False, True), ("Saturn 2", "FabFilter", "VST3", False, True),
    ("Pro-R 2", "FabFilter", "VST3", False, True), ("Serum", "Xfer Records", "VST2", True, True),
    ("OTT", "Xfer Records", "VST2", False, True), ("Vital", "Vital Audio", "VST3", True, True),
    ("Diva", "u-he", "VST3", True, True), ("Repro-1", "u-he", "VST3", True, False),
    ("Zebra2", "u-he", "VST2", True, True), ("Omnisphere", "Spectrasonics", "VST2", True, False),
    ("Kontakt 7", "Native Instruments", "VST3", True, True),
    ("Massive X", "Native Instruments", "VST3", True, True),
    ("Raum", "Native Instruments", "VST3", False, True),
    ("ValhallaVintageVerb", "Valhalla DSP", "VST3", False, True),
    ("ValhallaSupermassive", "Valhalla DSP", "VST3", False, True),
    ("ValhallaDelay", "Valhalla DSP", "VST2", False, True),
    ("Decapitator", "Soundtoys", "VST3", False, True), ("EchoBoy", "Soundtoys", "VST3", False, True),
    ("Little AlterBoy", "Soundtoys", "VST3", False, False),
    ("soothe2", "oeksound", "VST3", False, True), ("RC-20 Retro Color", "XLN Audio", "VST3", False, True),
    ("Addictive Drums 2", "XLN Audio", "VST2", True, None),
    ("Pigments", "Arturia", "VST3", True, True), ("Mini V4", "Arturia", "VST3", True, True),
    ("Jup-8 V4", "Arturia", "VST3", True, False),
    ("Ozone 11 Maximizer", "iZotope", "VST3", False, True), ("Neutron 4", "iZotope", "VST3", False, True),
    ("RX 10 De-click", "iZotope", "VST3", False, None),
    ("ShaperBox 3", "Cableguys", "VST3", False, True), ("Surge XT", "Surge Synth Team", "VST3", True, True),
    ("TAL-U-NO-LX", "TAL Software", "VST3", True, True), ("TAL-Sampler", "TAL Software", "VST2", True, True),
    ("Dexed", "Digital Suburban", "VST3", True, True), ("PhaseMistress", "Soundtoys", "VST2", False, False),
    ("Kickstart 2", "Nicky Romero", "VST3", False, True), ("LFOTool", "Xfer Records", "VST2", False, False),
    ("Portal", "Output", "VST3", False, True), ("Arcade", "Output", "VST3", True, None),
    ("Sausage Fattener", "Dada Life", "VST2", False, True), ("CamelCrusher", "Camel Audio", "VST2", False, False),
    ("Tube Screamer", "TSE Audio", "VST2", False, None), ("Youlean Loudness Meter 2", "Youlean", "VST3", False, True),
    ("SPAN", "Voxengo", "VST3", False, True), ("Sonible smart:EQ 4", "sonible", "VST3", False, True),
    ("Kilohearts Disperser", "Kilohearts", "VST3", False, True), ("Phase Plant", "Kilohearts", "VST3", True, True),
    ("Uhbik-T", "u-he", "VST2", False, True), ("Unknown Plugin", None, "VST2", False, None),
]

SAMPLE_DIRS = [
    r"C:\Users\producer\Splice\sounds\packs\{pack}",
    r"C:\Users\producer\Music\Samples\{pack}",
    r"D:\Sample Library\{pack}",
    r"C:\Users\producer\Music\Ableton\User Library\Samples\Recorded",
]
PACKS = ["Lofi Dusty Drums", "Techno Tools Vol 2", "Vocal Chops", "303 Acid Lines",
         "Foley Textures", "Jungle Breaks", "Analog Pads", "One Shots Deluxe",
         "Field Recordings", "Modular Blips"]
SAMPLE_KINDS = ["kick", "snare", "hat", "clap", "perc", "break", "vox", "pad", "fx",
                "bass", "loop", "riser", "texture", "chord", "stab"]
EXTENSIONS = ["wav"] * 8 + ["aif"] * 2 + ["flac", "mp3"]

TONICS = ["C", "CSharp", "D", "DSharp", "E", "F", "FSharp", "G", "GSharp", "A", "ASharp", "B"]
SCALES = ["Major"] * 5 + ["Minor"] * 6 + ["Dorian", "Mixolydian", "Phrygian",
                                          "MinorPentatonic", "HarmonicMinor"]
VERSIONS = [(10, 1, 43), (11, 0, 12), (11, 2, 11), (11, 3, 13), (11, 3, 25),
            (12, 0, 5), (12, 0, 25), (12, 1, 5), (12, 1, 10), (12, 2, 5)]


# --------------------------------------------------------------------------- seed

def project_names(n: int) -> list[str]:
    seen, out = set(), []
    while len(out) < n:
        name = f"{rng.choice(WORDS_A)} {rng.choice(WORDS_B)}{rng.choice(SUFFIX)}"
        if name not in seen:
            seen.add(name)
            out.append(name)
    return out


def seed() -> None:
    if DB.exists():
        DB.unlink()
    for suffix in ("-wal", "-shm"):
        p = Path(str(DB) + suffix)
        if p.exists():
            p.unlink()
    shutil.rmtree(MEDIA_DIR, ignore_errors=True)

    conn = sqlite3.connect(DB)
    conn.execute("PRAGMA foreign_keys = ON")
    conn.executescript(SCHEMA.read_text("utf-8"))
    # Without this the program sees an old-schema database and discards it (ADR-0011).
    conn.execute(f"PRAGMA user_version = {schema_version()}")

    now = epoch(2026, 9, 23, 12)
    t0 = epoch(2021, 3, 1)

    # media: covers and audition bounces
    cover_ids = {}
    for _, _, cover in COLLECTIONS:
        if cover is None:
            continue
        mid = uid()
        cover_ids[cover] = mid
        path = REPO / "mockup" / "images" / "thumbs" / f"{cover}.jpg"
        conn.execute(
            "INSERT INTO media_files VALUES (?,?,?,?,?,?,?,?)",
            (mid, f"{cover}.jpg", "jpg", "cover_art", path.stat().st_size, "image/jpeg",
             rand_time(t0, now), hashlib.sha256(path.read_bytes()).hexdigest()))

    # tags
    tag_ids = {}
    for t in TAGS:
        tag_ids[t] = uid()
        conn.execute("INSERT INTO tags VALUES (?,?,?)", (tag_ids[t], t, rand_time(t0, now)))

    # plugins
    plugins = []
    for name, vendor, kind, instr, installed in PLUGINS:
        pid = uid()
        if kind == "VST3":
            raw = rng.getrandbits(128).to_bytes(16, "big")
            uid_hex = raw.hex()
            dashed = str(uuid.UUID(bytes=raw))
            dev = f"device:vst3:{'instr' if instr else 'audiofx'}:{dashed}"
        else:
            n = rng.getrandbits(31)
            uid_hex = f"{n:08x}"
            dev = f"device:vst:{'instr' if instr else 'audiofx'}:{n}?n={name.replace(' ', '%20')}"
        fmt = f"{kind} {'Instrument' if instr else 'Effect'}"
        scanned = installed is True
        folder = r"C:\Program Files\Common Files\VST3" if kind == "VST3" else r"C:\Program Files\VSTPlugins"
        ext = "vst3" if kind == "VST3" else "dll"
        conn.execute(
            """INSERT INTO plugins (id, plugin_kind, uid, name, format, vendor, version,
                installed, last_scanned_at, dev_identifier, path, category, is_instrument,
                audio_in_channels, audio_out_channels, has_midi_input, presets, parameters,
                has_gui) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
            (pid, kind, uid_hex, name, fmt, vendor,
             f"{rng.randint(1, 4)}.{rng.randint(0, 12)}.{rng.randint(0, 9)}" if vendor else None,
             None if installed is None else int(installed),
             now - 86400 * 2 if installed is not None else None, dev,
             f"{folder}\\{name}.{ext}" if scanned else None,
             ("Instrument|Synth" if instr else "Fx|EQ") if scanned else None,
             int(instr) if scanned else None,
             (0 if instr else 2) if scanned else None, 2 if scanned else None,
             int(instr) if scanned else None,
             rng.randint(0, 900) if scanned else None,
             rng.randint(8, 400) if scanned else None, 1 if scanned else None))
        conn.execute("INSERT INTO plugin_refs VALUES (?,?,?,?,?,?)",
                     (dev, pid, name, "instr" if instr else "audiofx",
                      "uid" if scanned else "created", rand_time(t0, now)))
        plugins.append((pid, instr))
    instr_ids = [p for p, i in plugins if i]
    fx_ids = [p for p, i in plugins if not i]
    # a few plugins carry most of the usage, like real libraries
    fx_weights = [8 if i < 6 else 1 for i in range(len(fx_ids))]

    # samples
    samples = []
    used_paths = set()
    while len(samples) < 420:
        d = rng.choice(SAMPLE_DIRS).format(pack=rng.choice(PACKS))
        kind = rng.choice(SAMPLE_KINDS)
        ext = rng.choice(EXTENSIONS)
        if "Recorded" in d:
            fname = f"{rng.choice(['Audio', 'Vocal', 'Bass', 'Guitar'])} {rng.randint(1, 40)} [{rng.randint(2021, 2026)}-0{rng.randint(1, 9)}-1{rng.randint(0, 9)} {rng.randint(100000, 235959)}].wav"
        else:
            fname = f"{kind}_{rng.choice(['dry', 'wet', 'hard', 'soft', 'tape', 'long', 'short', 'clean'])}_{rng.randint(1, 60):02d}.{ext}"
        path = f"{d}\\{fname}"
        if path in used_paths:
            continue
        used_paths.add(path)
        sid = uid()
        present = rng.random() > 0.09
        conn.execute("INSERT INTO samples VALUES (?,?,?,?)", (sid, fname, path, int(present)))
        samples.append(sid)

    # projects
    names = project_names(240)
    project_ids = []
    for i, name in enumerate(names):
        pid = uid()
        project_ids.append(pid)
        folder_name = name if rng.random() > 0.15 else rng.choice(WORDS_A) + " idea"
        file_name = name if rng.random() > 0.2 else f"{name} {rng.choice(['mixdown', 'v1', 'backup', 'old'])}"
        path = rf"C:\Users\producer\Music\Ableton\Projects\{folder_name} Project\{file_name}.als"
        # display name differs from the file name for some (the rename feature)
        display = name if rng.random() > 0.1 else name.title()
        created = rand_time(t0, now - 86400 * 3)
        # most projects stop being touched within weeks; a few are revisited much later
        modified = created + int((now - created) * rng.random() ** 3)
        major, minor, patch = rng.choice(VERSIONS)
        beta = rng.random() < 0.04
        key = rng.random() < 0.7
        ts = rng.choices([(4, 4), (3, 4), (6, 8), (7, 8), (5, 4)], weights=[85, 6, 4, 3, 2])[0]
        tempo = round(rng.choice([rng.uniform(70, 100), rng.uniform(118, 132), rng.uniform(160, 176)]), rng.choice([0, 0, 0, 2]))
        duration = rng.randint(45, 420) if rng.random() > 0.06 else None
        bar = round(duration * tempo / 60 / ts[0], 2) if duration else None
        audio = None
        if rng.random() < 0.35:
            audio = uid()
            conn.execute("INSERT INTO media_files VALUES (?,?,?,?,?,?,?,?)",
                         (audio, f"{file_name}.wav", "wav", "audio_file",
                          rng.randint(8, 70) * 1_000_000, "audio/wav", modified,
                          hashlib.sha256(pid.encode()).hexdigest()))
        conn.execute(
            """INSERT INTO projects (is_active, id, path, name, hash, notes, created_at,
                modified_at, last_parsed_at, tempo, time_signature_numerator,
                time_signature_denominator, key_signature_tonic, key_signature_scale,
                duration_seconds, furthest_bar, daw_type, daw_version_display, audio_file_id)
               VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)""",
            (int(rng.random() > 0.08), pid, path, display,
             hashlib.sha256(path.encode()).hexdigest(),
             rng.choice(NOTES) if rng.random() < 0.18 else None,
             created, modified, now - rng.randint(0, 86400 * 5), tempo, ts[0], ts[1],
             rng.choice(TONICS) if key else None, rng.choice(SCALES) if key else None,
             duration, bar, "Ableton Live",
             f"{major}.{minor}.{patch}" + (" beta" if beta else ""), audio))
        if audio:
            # ADR-0037: a list of audios, one of them primary. Extras come from their
            # own per-project stream so the main rng, and so the rest of the data, is
            # unchanged by them.
            arng = random.Random(pid)
            listed = [audio]
            for suffix in arng.choices([[], [" (master)"], [" (master)", " alt mix"]], weights=[55, 30, 15])[0]:
                extra = str(uuid.UUID(int=arng.getrandbits(128), version=4))
                conn.execute("INSERT INTO media_files VALUES (?,?,?,?,?,?,?,?)",
                             (extra, f"{file_name}{suffix}.wav", "wav", "audio_file",
                              arng.randint(8, 70) * 1_000_000, "audio/wav", modified,
                              hashlib.sha256((pid + suffix).encode()).hexdigest()))
                listed.append(extra)
            for pos, mid in enumerate(listed):
                conn.execute("INSERT INTO project_audio_files VALUES (?,?,?,?)", (pid, mid, pos, modified))
            if len(listed) > 1 and arng.random() < 0.5:
                # Sometimes the master, not the first bounce, is what the row plays.
                conn.execute("UPDATE projects SET audio_file_id = ? WHERE id = ?", (listed[1], pid))
            if len(listed) > 1 and arng.random() < 0.12:
                # Primary cleared: audios listed, nothing plays in the row (ADR-0037).
                conn.execute("UPDATE projects SET audio_file_id = NULL WHERE id = ?", (pid,))
        conn.execute("INSERT INTO project_ableton_metadata VALUES (?,?,?,?,?)",
                     (pid, major, minor, patch, int(beta)))

        used = set(rng.sample(instr_ids, rng.randint(0, 4)))
        used |= set(rng.choices(fx_ids, weights=fx_weights, k=rng.randint(0, 9)))
        for plug in used:
            conn.execute("INSERT INTO project_plugins VALUES (?,?)", (pid, plug))
        for s in rng.sample(samples, rng.choice([0, 1, 3, 5, 8, 12, 20, 34])):
            conn.execute("INSERT INTO project_samples VALUES (?,?)", (pid, s))
        for t in rng.sample(TAGS, rng.choices([0, 1, 2, 3], weights=[40, 35, 18, 7])[0]):
            conn.execute("INSERT INTO project_tags VALUES (?,?,?)", (pid, tag_ids[t], modified))
        if rng.random() < 0.3:
            for desc in rng.sample(TASKS, rng.randint(1, 6)):
                conn.execute("INSERT INTO project_tasks VALUES (?,?,?,?,?)",
                             (uid(), pid, desc, int(rng.random() < 0.4), rand_time(created, now)))

    # collections
    for name, desc, cover in COLLECTIONS:
        cid = uid()
        created = rand_time(t0, now - 86400 * 10)
        conn.execute("INSERT INTO collections VALUES (?,?,?,?,?,?,?)",
                     (cid, name, desc, None, created, rand_time(created, now),
                      cover_ids.get(cover)))
        for pos, p in enumerate(rng.sample(project_ids, rng.randint(3, 14))):
            conn.execute("INSERT INTO collection_projects VALUES (?,?,?,?)",
                         (cid, p, pos, rand_time(created, now)))

    conn.commit()
    conn.close()
    print(f"seeded {DB.relative_to(REPO)}: {len(names)} projects, {len(PLUGINS)} plugins, "
          f"{len(samples)} samples, {len(COLLECTIONS)} collections")


# --------------------------------------------------------------------------- snapshot

def get(path: str):
    with urllib.request.urlopen(f"http://127.0.0.1:{HTTP_PORT}{path}", timeout=30) as r:
        return json.loads(r.read())


def snapshot() -> None:
    if not DB.exists():
        sys.exit("no database; run `seed` first")
    MEDIA_DIR.mkdir(exist_ok=True)
    toml_path = lambda p: str(p).replace("\\", "/")
    CONFIG.write_text(
        "paths = []\n"
        f"database_path = '{toml_path(DB)}'\n"
        f"media_storage_dir = '{toml_path(MEDIA_DIR)}'\n"
        f"grpc_port = {GRPC_PORT}\nhttp_port = {HTTP_PORT}\n"
        "log_level = 'error'\nvst_search_paths = []\n", "utf-8")

    env = {k: v for k, v in os.environ.items() if not k.startswith("SEULA_")}
    subprocess.run(["cargo", "build", "--quiet"], cwd=REPO, check=True)
    exe = REPO / "target" / "debug" / ("seula.exe" if os.name == "nt" else "seula")
    server = subprocess.Popen([str(exe), "--config", str(CONFIG), "--server"], cwd=REPO, env=env)
    try:
        for _ in range(100):
            try:
                get("/health")
                break
            except OSError:
                if server.poll() is not None:
                    sys.exit(f"seula --server exited with {server.returncode}")
                time.sleep(0.2)
        else:
            sys.exit("server did not come up")

        big = "limit=10000"
        api = {}
        for path in [
            f"/api/v1/projects?{big}", f"/api/v1/projects?scope=deleted&{big}",
            "/api/v1/projects/statistics",
            f"/api/v1/tags?{big}", f"/api/v1/tags/with-usage?{big}", "/api/v1/tags/statistics",
            f"/api/v1/collections?{big}",
            f"/api/v1/plugins?{big}", "/api/v1/plugins/stats", f"/api/v1/plugins/vendors?{big}",
            "/api/v1/plugins/formats",
            f"/api/v1/samples?{big}", "/api/v1/samples/stats", "/api/v1/samples/analytics",
            "/api/v1/samples/extensions",
            f"/api/v1/media?{big}", "/api/v1/tasks/statistics",
            "/api/v1/system/info", "/api/v1/system/statistics", "/api/v1/system/scan-status",
            "/api/v1/config/status",
            # A plain term and one of the search operators, for the projects board's
            # search frames.
            "/api/v1/search?query=techno&limit=200", "/api/v1/search?query=plugin:serum&limit=200",
        ]:
            api[path] = get(path)

        cols = api[f"/api/v1/collections?{big}"]
        cols = cols.get("collections", cols) if isinstance(cols, dict) else cols
        for c in cols:
            cid = c["id"]
            for sub in ("", "/projects", "/tasks", "/statistics"):
                api[f"/api/v1/collections/{cid}{sub}"] = get(f"/api/v1/collections/{cid}{sub}")
    finally:
        server.terminate()
        server.wait(timeout=10)

    OUT_JSON.write_text(json.dumps(api, indent=1), "utf-8")
    OUT_JS.write_text(
        "// Generated by mockup/data/generate.py from the real HTTP API over a seeded\n"
        "// database. Keys are request paths. Do not edit; regenerate.\n"
        f"window.SEULA_API = {json.dumps(api, separators=(',', ':'))};\n", "utf-8")
    print(f"snapshot: {len(api)} responses -> {OUT_JSON.relative_to(REPO)} "
          f"({OUT_JSON.stat().st_size // 1024} KB)")


if __name__ == "__main__":
    step = sys.argv[1] if len(sys.argv) > 1 else "all"
    if step in ("seed", "all"):
        seed()
    if step in ("snapshot", "all"):
        snapshot()
