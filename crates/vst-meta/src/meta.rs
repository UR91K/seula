//! A single metadata record that both VST2 and VST3 plugins normalize into.
//!
//! The two formats agree on surprisingly little at the *binary* level, so this type
//! keeps three tiers of field:
//!
//! 1. Common fields, present for every plugin of either format.
//! 2. `Option` fields, present in one format but requiring extra work (or impossible)
//!    in the other -- these become NULLable columns.
//! 3. `extra`, the format-specific leftovers that do not generalize at all.

use serde::{Deserialize, Serialize};

#[cfg(feature = "host")]
use vst::plugin::{Category as Vst2Category, Info as Vst2Info};
#[cfg(feature = "host")]
use vst3_host::discovery::DetailedPluginInfo;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PluginFormat {
    Vst2,
    Vst3,
}

impl PluginFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            PluginFormat::Vst2 => "VST2",
            PluginFormat::Vst3 => "VST3",
        }
    }
}

/// Fields that exist in only one format and have no counterpart in the other.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "format", rename_all = "UPPERCASE")]
pub enum FormatExtra {
    Vst2 {
        /// The four-character code packed into the i32 unique id, e.g. "FQ4p".
        fourcc: String,
        /// Plugin manages its own opaque preset blobs rather than exposing parameters.
        preset_chunks: bool,
        /// Plugin can process f64 buffers.
        f64_precision: bool,
        /// Plugin outputs silence for silent input.
        silent_when_stopped: bool,
        /// Raw declared MIDI channel counts (0 means "the default of 16").
        midi_in_channels: i32,
        midi_out_channels: i32,
    },
    Vst3 {
        /// Raw factory capability flags.
        factory_flags: i32,
        /// Every class the bundle's factory exports (processor, controller, ...).
        classes: Vec<Vst3Class>,
        /// Named buses with their channel counts.
        buses: Vec<Vst3Bus>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vst3Class {
    pub name: String,
    pub category: String,
    pub class_id: String,
    pub cardinality: i32,
    pub version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Vst3Bus {
    pub name: String,
    /// "input" or "output".
    pub direction: String,
    /// "audio" or "event".
    pub media: String,
    pub channel_count: i32,
    pub bus_type: i32,
    pub flags: i32,
}

/// The normalized record. One row in the `plugins` table.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PluginMeta {
    // --- identity: both formats always have these ---
    pub path: String,
    pub format: PluginFormat,
    pub name: String,
    pub vendor: String,
    /// Normalized to dotted decimal. VST2 unpacks its i32 (1283 -> "1.2.8.3");
    /// VST3 reports a string already and is passed through verbatim.
    pub version: String,
    /// Hex. VST2 is a 32-bit id (8 chars), VST3 a 128-bit class id (32 chars).
    /// Unique *within* a format only -- key your table on (format, uid).
    pub uid: String,
    /// VST3-style subcategory string. VST2's enum is mapped onto the same shape,
    /// e.g. Category::RoomFx -> "Fx|Reverb", Category::Synth -> "Instrument".
    pub category: String,
    pub is_instrument: bool,

    // --- I/O ---
    /// VST2 counts channels directly; VST3 channels are summed over its buses.
    pub audio_in_channels: u32,
    pub audio_out_channels: u32,
    /// VST2 has no bus concept, so these are None for it.
    pub audio_in_buses: Option<u32>,
    pub audio_out_buses: Option<u32>,
    pub has_midi_input: bool,
    pub has_midi_output: bool,

    // --- known in one format, or needing instantiation in the other ---
    /// VST3 exposes presets only via the loaded controller, so None there.
    pub presets: Option<i32>,
    /// Likewise: VST3 needs the parameter list from a live instance.
    pub parameters: Option<i32>,
    /// VST3 reports latency only from an initialized processor, so None there.
    pub latency_samples: Option<i32>,
    /// VST2's `Info` does not carry an editor flag, so None there.
    pub has_gui: Option<bool>,
    /// Factory-level vendor contact. VST3 only.
    pub vendor_url: Option<String>,
    pub vendor_email: Option<String>,

    // --- shell plugins (VST2 only) ---
    /// True for a container binary that holds many logical plugins. Its own row is
    /// a placeholder -- the real plugins are the rows that point back at it.
    pub is_shell: bool,
    /// For a plugin unpacked from a shell, the `uid` of the container it came from.
    /// This is the VST3 factory/class relationship, expressed for VST2.
    pub shell_parent_uid: Option<String>,

    pub extra: FormatExtra,
}

#[cfg(feature = "host")]
impl PluginMeta {
    pub fn from_vst2(path: &str, info: &Vst2Info) -> Self {
        let v = info.version;
        let version = format!(
            "{}.{}.{}.{}",
            (v / 1000) % 10,
            (v / 100) % 10,
            (v / 10) % 10,
            v % 10
        );

        let fourcc: String = info
            .unique_id
            .to_be_bytes()
            .iter()
            .map(|&b| if b.is_ascii_graphic() { b as char } else { '?' })
            .collect();

        let is_instrument = matches!(info.category, Vst2Category::Synth);

        PluginMeta {
            path: path.to_string(),
            format: PluginFormat::Vst2,
            name: info.name.clone(),
            vendor: info.vendor.clone(),
            version,
            uid: format!("{:08X}", info.unique_id),
            category: vst2_category_string(&info.category),
            is_instrument,
            audio_in_channels: info.inputs.max(0) as u32,
            audio_out_channels: info.outputs.max(0) as u32,
            audio_in_buses: None,
            audio_out_buses: None,
            // A VST2 declares MIDI channel counts, where 0 means "the default of 16".
            // That makes 0 ambiguous between "none" and "16", so treat a synth as
            // always accepting MIDI and require an explicit count otherwise.
            has_midi_input: info.midi_inputs > 0 || is_instrument,
            has_midi_output: info.midi_outputs > 0,
            presets: Some(info.presets),
            parameters: Some(info.parameters),
            latency_samples: Some(info.initial_delay),
            has_gui: None,
            vendor_url: None,
            vendor_email: None,
            is_shell: matches!(info.category, Vst2Category::Shell),
            // Set by the caller when this record came out of a shell enumeration.
            shell_parent_uid: None,
            extra: FormatExtra::Vst2 {
                fourcc,
                preset_chunks: info.preset_chunks,
                f64_precision: info.f64_precision,
                silent_when_stopped: info.silent_when_stopped,
                midi_in_channels: info.midi_inputs,
                midi_out_channels: info.midi_outputs,
            },
        }
    }

    pub fn from_vst3(detailed: &DetailedPluginInfo) -> Self {
        let info = &detailed.info;
        let factory = &detailed.factory;
        let b = &detailed.buses;

        let sum = |list: &[vst3_host::discovery::BusInfo]| -> u32 {
            list.iter().map(|x| x.channel_count.max(0) as u32).sum()
        };

        let mut buses = Vec::new();
        for (direction, media, list) in [
            ("input", "audio", &b.audio_inputs),
            ("output", "audio", &b.audio_outputs),
            ("input", "event", &b.event_inputs),
            ("output", "event", &b.event_outputs),
        ] {
            for bus in list.iter() {
                buses.push(Vst3Bus {
                    name: bus.name.clone(),
                    direction: direction.to_string(),
                    media: media.to_string(),
                    channel_count: bus.channel_count,
                    bus_type: bus.bus_type,
                    flags: bus.flags,
                });
            }
        }

        let classes = detailed
            .classes
            .iter()
            .map(|c| Vst3Class {
                name: c.name.clone(),
                category: c.category.clone(),
                class_id: c.class_id.clone(),
                cardinality: c.cardinality,
                version: c.version.clone(),
            })
            .collect();

        PluginMeta {
            path: info.path.display().to_string(),
            format: PluginFormat::Vst3,
            name: info.name.clone(),
            // The factory vendor is the authoritative one; fall back to the class vendor.
            vendor: if factory.vendor.is_empty() {
                info.vendor.clone()
            } else {
                factory.vendor.clone()
            },
            version: info.version.clone(),
            uid: info.uid.clone(),
            category: info.category.clone(),
            is_instrument: info.category.to_lowercase().contains("instrument"),
            audio_in_channels: sum(&b.audio_inputs),
            audio_out_channels: sum(&b.audio_outputs),
            audio_in_buses: Some(info.audio_inputs),
            audio_out_buses: Some(info.audio_outputs),
            has_midi_input: info.has_midi_input,
            has_midi_output: info.has_midi_output,
            presets: None,
            parameters: None,
            latency_samples: None,
            has_gui: Some(info.has_gui),
            vendor_url: opt(&factory.url),
            vendor_email: opt(&factory.email),
            // VST3 has no shell concept -- a bundle exporting several classes is the
            // format's native, always-enumerable equivalent.
            is_shell: false,
            shell_parent_uid: None,
            extra: FormatExtra::Vst3 {
                factory_flags: factory.flags,
                classes,
                buses,
            },
        }
    }

    pub fn print(&self) {
        let na = |v: &Option<String>| v.clone().unwrap_or_else(|| "n/a".into());
        let ni = |v: Option<i32>| v.map_or("n/a".to_string(), |x| x.to_string());
        let nu = |v: Option<u32>| v.map_or("n/a".to_string(), |x| x.to_string());
        let nb = |v: Option<bool>| v.map_or("n/a".to_string(), |x| x.to_string());

        println!("--- {} Plugin Information ---", self.format.as_str());
        println!("Path:               {}", self.path);
        println!("Name:               {}", self.name);
        println!("Vendor/Developer:   {}", self.vendor);
        println!("Vendor URL:         {}", na(&self.vendor_url));
        println!("Vendor Email:       {}", na(&self.vendor_email));
        println!("Version:            {}", self.version);
        println!("Unique ID:          {}", self.uid);
        println!("Category:           {}", self.category);
        println!(
            "Type:               {}",
            if self.is_instrument {
                "Instrument (Synthesizer/Sampler)"
            } else {
                "Effect (Audio Processor)"
            }
        );
        println!("Audio In Channels:  {}", self.audio_in_channels);
        println!("Audio Out Channels: {}", self.audio_out_channels);
        println!("Audio In Buses:     {}", nu(self.audio_in_buses));
        println!("Audio Out Buses:    {}", nu(self.audio_out_buses));
        println!("MIDI Input:         {}", self.has_midi_input);
        println!("MIDI Output:        {}", self.has_midi_output);
        println!("Preset Count:       {}", ni(self.presets));
        println!("Parameter Count:    {}", ni(self.parameters));
        println!("Latency (samples):  {}", ni(self.latency_samples));
        println!("Has GUI:            {}", nb(self.has_gui));
        if self.is_shell {
            println!("Shell Container:    yes");
        }
        if let Some(parent) = &self.shell_parent_uid {
            println!("From Shell:         {}", parent);
        }

        match &self.extra {
            FormatExtra::Vst2 {
                fourcc,
                preset_chunks,
                f64_precision,
                silent_when_stopped,
                midi_in_channels,
                midi_out_channels,
            } => {
                println!();
                println!("--- VST2 Specific ---");
                println!("Four-char Code:     {}", fourcc);
                println!("Preset Chunks:      {}", preset_chunks);
                println!("f64 Precision:      {}", f64_precision);
                println!("Silent When Stopped:{}", silent_when_stopped);
                println!(
                    "MIDI Channels:      {} in / {} out (0 = default of 16)",
                    midi_in_channels, midi_out_channels
                );
            }
            FormatExtra::Vst3 {
                factory_flags,
                classes,
                buses,
            } => {
                println!();
                println!("--- VST3 Specific ---");
                println!("Factory Flags:      {}", factory_flags);
                println!();
                println!("Buses ({}):", buses.len());
                for bus in buses {
                    println!(
                        "  [{} {}] {} -- {} channels, type {}, flags {}",
                        bus.media,
                        bus.direction,
                        bus.name,
                        bus.channel_count,
                        bus.bus_type,
                        bus.flags
                    );
                }
                println!();
                println!("Exported Factory Classes ({}):", classes.len());
                for (i, class) in classes.iter().enumerate() {
                    println!("  [{}] {}", i + 1, class.name);
                    println!("      Category:    {}", class.category);
                    println!("      Class ID:    {}", class.class_id);
                    println!("      Cardinality: {}", class.cardinality);
                    println!("      Version:     {}", class.version);
                }
            }
        }
    }
}

/// Map the VST2 category enum onto a VST3-shaped subcategory string, so one column
/// can hold either format's notion of "what kind of plugin is this".
#[cfg(feature = "host")]
fn vst2_category_string(category: &Vst2Category) -> String {
    match category {
        Vst2Category::Synth => "Instrument".into(),
        Vst2Category::Generator => "Instrument|Generator".into(),
        Vst2Category::Effect => "Fx".into(),
        Vst2Category::Analysis => "Fx|Analyzer".into(),
        Vst2Category::Mastering => "Fx|Mastering".into(),
        Vst2Category::Spacializer => "Fx|Spatial".into(),
        Vst2Category::RoomFx => "Fx|Reverb".into(),
        Vst2Category::SurroundFx => "Fx|Surround".into(),
        Vst2Category::Restoration => "Fx|Restoration".into(),
        Vst2Category::OfflineProcess => "Fx|OfflineProcess".into(),
        Vst2Category::Shell => "Shell".into(),
        Vst2Category::Unknown => "Unknown".into(),
    }
}

#[cfg(feature = "host")]
fn opt(s: &str) -> Option<String> {
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

/// Schema for persisting these records. The `plugins` table holds the flat fields;
/// the VST3 lists get child tables keyed by plugin id.
pub const SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS plugins (
    id                  INTEGER PRIMARY KEY,
    path                TEXT    NOT NULL,
    format              TEXT    NOT NULL,
    name                TEXT    NOT NULL,
    vendor              TEXT    NOT NULL,
    version             TEXT    NOT NULL,
    uid                 TEXT    NOT NULL,
    category            TEXT    NOT NULL,
    is_instrument       INTEGER NOT NULL,
    audio_in_channels   INTEGER NOT NULL,
    audio_out_channels  INTEGER NOT NULL,
    audio_in_buses      INTEGER,
    audio_out_buses     INTEGER,
    has_midi_input      INTEGER NOT NULL,
    has_midi_output     INTEGER NOT NULL,
    presets             INTEGER,
    parameters          INTEGER,
    latency_samples     INTEGER,
    has_gui             INTEGER,
    vendor_url          TEXT,
    vendor_email        TEXT,
    is_shell            INTEGER NOT NULL DEFAULT 0,
    shell_parent_uid    TEXT REFERENCES plugins(uid),
    -- VST2 extras
    fourcc              TEXT,
    preset_chunks       INTEGER,
    f64_precision       INTEGER,
    silent_when_stopped INTEGER,
    midi_in_channels    INTEGER,
    midi_out_channels   INTEGER,
    -- VST3 extras
    factory_flags       INTEGER,
    UNIQUE (format, uid)
);

CREATE TABLE IF NOT EXISTS plugin_classes (
    plugin_id   INTEGER NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    name        TEXT    NOT NULL,
    category    TEXT    NOT NULL,
    class_id    TEXT    NOT NULL,
    cardinality INTEGER NOT NULL,
    version     TEXT    NOT NULL
);

CREATE TABLE IF NOT EXISTS plugin_buses (
    plugin_id     INTEGER NOT NULL REFERENCES plugins(id) ON DELETE CASCADE,
    direction     TEXT    NOT NULL,
    media         TEXT    NOT NULL,
    name          TEXT    NOT NULL,
    channel_count INTEGER NOT NULL,
    bus_type      INTEGER NOT NULL,
    flags         INTEGER NOT NULL
);
";
