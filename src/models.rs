//! # Core Data Models
//!
//! This module contains all the core data structures used throughout the Seula.
//! These models represent the various entities found in Ableton Live projects, including projects
//! themselves, plugins, samples, musical metadata, and more.
//!
//! ## Key Types
//!
//! - [`AbletonVersion`]: Represents an Ableton Live version with comparison support
//! - [`Plugin`]: Represents a plugin with installation status and metadata
//! - [`Sample`]: Represents an audio sample with file presence validation
//! - [`KeySignature`]: Musical key information combining tonic and scale
//! - [`TimeSignature`]: Musical time signature with validation
//! - [`PluginFormat`]: Enumeration of supported plugin formats (VST2/VST3)
//!
//! ## Musical Types
//!
//! The module includes comprehensive musical type definitions:
//! - [`Tonic`]: Musical root notes (C, D, E, etc.)
//! - [`Scale`]: Musical scales (Major, Minor, Dorian, etc.)
//!
//! These types support parsing from strings and provide display formatting for UI purposes.

use rand::seq::SliceRandom;
use rand::thread_rng;
use serde::{Serialize, Deserialize};
use std::fmt;
use std::path::PathBuf;
use std::str::{self, FromStr};
use uuid::Uuid;


use crate::error::{SampleError, TimeSignatureError};

/// Unique identifier type for database entities.
///
/// This is a wrapper around `u64` that provides type safety for entity IDs.
/// Currently used internally for database operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id(u64);

/// Represents an Ableton Live version with semantic version comparison support.
///
/// This struct stores version information for Ableton Live projects, including
/// major, minor, and patch versions, as well as beta status. It implements
/// proper version ordering where non-beta versions are considered greater
/// than beta versions of the same number.
///
/// # Examples
///
/// ```rust
/// use seula::models::AbletonVersion;
///
/// let v11_2_0 = AbletonVersion {
///     major: 11,
///     minor: 2,
///     patch: 0,
///     beta: false,
/// };
///
/// let v11_1_0 = AbletonVersion {
///     major: 11,
///     minor: 1,
///     patch: 0,
///     beta: false,
/// };
///
/// assert!(v11_2_0 > v11_1_0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AbletonVersion {
    /// Major version number (e.g., 11 for Ableton Live 11)
    pub major: u32,
    /// Minor version number (e.g., 2 for version 11.2.0)
    pub minor: u32,
    /// Patch version number (e.g., 5 for version 11.2.5)
    pub patch: u32,
    /// Whether this is a beta release
    pub beta: bool,
}

impl Default for AbletonVersion {
    fn default() -> Self {
        Self {
            major: 0,
            minor: 0,
            patch: 0,
            beta: false,
        }
    }
}

impl PartialOrd for AbletonVersion {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Compare major versions first
        match self.major.cmp(&other.major) {
            std::cmp::Ordering::Equal => {
                // If major versions are equal, compare minor versions
                match self.minor.cmp(&other.minor) {
                    std::cmp::Ordering::Equal => {
                        // If minor versions are equal, compare patch versions
                        match self.patch.cmp(&other.patch) {
                            std::cmp::Ordering::Equal => {
                                // If all version numbers are equal, non-beta is greater than beta
                                Some((!self.beta).cmp(&(!other.beta)))
                            }
                            ord => Some(ord),
                        }
                    }
                    ord => Some(ord),
                }
            }
            ord => Some(ord),
        }
    }
}

impl Ord for AbletonVersion {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap()
    }
}

/// Musical scales supported by Ableton Live.
///
/// This enum represents all the musical scales that can be detected in Ableton Live projects.
/// It includes common scales like Major and Minor, as well as more exotic scales and modes.
/// The scales are organized into several categories:
///
/// ## Common Scales
/// - [`Scale::Major`]: Major scale
/// - [`Scale::Minor`]: Natural minor scale
/// - [`Scale::HarmonicMinor`]: Harmonic minor scale
/// - [`Scale::MelodicMinor`]: Melodic minor scale
///
/// ## Modes
/// - [`Scale::Dorian`]: Dorian mode
/// - [`Scale::Mixolydian`]: Mixolydian mode
/// - [`Scale::Aeolian`]: Aeolian mode (natural minor)
/// - [`Scale::Phrygian`]: Phrygian mode
/// - [`Scale::Locrian`]: Locrian mode
///
/// ## Pentatonic Scales
/// - [`Scale::MajorPentatonic`]: Major pentatonic scale
/// - [`Scale::MinorPentatonic`]: Minor pentatonic scale
/// - [`Scale::MinorBlues`]: Minor blues scale
///
/// ## Exotic Scales
/// - [`Scale::WholeTone`]: Whole tone scale
/// - [`Scale::HalfWholeDim`]: Half-whole diminished scale
/// - [`Scale::WholeHalfDim`]: Whole-half diminished scale
/// - [`Scale::Hirajoshi`]: Japanese Hirajoshi scale
/// - [`Scale::Iwato`]: Japanese Iwato scale
/// - [`Scale::PelogSelisir`]: Indonesian Pelog Selisir scale
/// - [`Scale::PelogTembung`]: Indonesian Pelog Tembung scale
///
/// ## Messiaen Modes
/// - [`Scale::Messiaen1`] through [`Scale::Messiaen7`]: Messiaen's modes of limited transposition
///
/// The enum supports parsing from strings and display formatting for UI purposes.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[allow(dead_code)]
pub enum Scale {
    /// Empty/unset scale
    Empty,
    /// Major scale (Ionian mode)
    Major,
    /// Natural minor scale
    Minor,
    /// Dorian mode
    Dorian,
    /// Mixolydian mode
    Mixolydian,
    /// Aeolian mode (natural minor)
    Aeolian,
    /// Phrygian mode
    Phrygian,
    /// Locrian mode
    Locrian,
    /// Whole tone scale
    WholeTone,
    /// Half-whole diminished scale
    HalfWholeDim,
    /// Whole-half diminished scale
    WholeHalfDim,
    /// Minor blues scale
    MinorBlues,
    /// Minor pentatonic scale
    MinorPentatonic,
    /// Major pentatonic scale
    MajorPentatonic,
    /// Harmonic minor scale
    HarmonicMinor,
    /// Melodic minor scale
    MelodicMinor,
    /// Dorian #4 mode
    Dorian4,
    /// Phrygian dominant scale
    PhrygianDominant,
    /// Lydian dominant scale
    LydianDominant,
    /// Lydian augmented scale
    LydianAugmented,
    /// Harmonic major scale
    HarmonicMajor,
    /// Super locrian scale
    SuperLocrian,
    /// Spanish scale
    BToneSpanish,
    /// Hungarian minor scale
    HungarianMinor,
    /// Japanese Hirajoshi scale
    Hirajoshi,
    /// Japanese Iwato scale
    Iwato,
    /// Indonesian Pelog Selisir scale
    PelogSelisir,
    /// Indonesian Pelog Tembung scale
    PelogTembung,
    /// Messiaen mode 1 (whole tone)
    Messiaen1,
    /// Messiaen mode 2 (octatonic)
    Messiaen2,
    /// Messiaen mode 3
    Messiaen3,
    /// Messiaen mode 4
    Messiaen4,
    /// Messiaen mode 5
    Messiaen5,
    /// Messiaen mode 6
    Messiaen6,
    /// Messiaen mode 7
    Messiaen7,
}

/// Musical tonic (root note) for key signatures.
///
/// This enum represents the twelve chromatic pitches that can serve as the root
/// note of a key signature. It includes both natural notes (C, D, E, F, G, A, B)
/// and their sharp variants.
///
/// # Examples
///
/// ```rust
/// use seula::models::Tonic;
///
/// // Create a tonic from MIDI note number
/// let tonic = Tonic::from_midi_note(60); // Middle C
/// assert_eq!(tonic, Tonic::C);
///
/// // Parse from string
/// let tonic: Tonic = "CSharp".parse().unwrap();
/// assert_eq!(tonic, Tonic::CSharp);
/// ```
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
#[allow(dead_code)]
pub enum Tonic {
    /// Empty/unset tonic
    Empty,
    /// C natural
    C,
    /// C sharp / D flat
    CSharp,
    /// D natural
    D,
    /// D sharp / E flat
    DSharp,
    /// E natural
    E,
    /// F natural
    F,
    /// F sharp / G flat
    FSharp,
    /// G natural
    G,
    /// G sharp / A flat
    GSharp,
    /// A natural
    A,
    /// A sharp / B flat
    ASharp,
    /// B natural
    B,
}

impl Tonic {
    /// Creates a tonic from a MIDI note number.
    ///
    /// This method converts a MIDI note number to the corresponding tonic by
    /// using modulo 12 arithmetic. MIDI note 60 corresponds to middle C.
    ///
    /// # Arguments
    ///
    /// * `number` - MIDI note number (0-127, but any i32 is accepted)
    ///
    /// # Returns
    ///
    /// The corresponding [`Tonic`] variant
    ///
    /// # Examples
    ///
    /// ```rust
    /// use seula::models::Tonic;
    ///
    /// assert_eq!(Tonic::from_midi_note(60), Tonic::C);      // Middle C
    /// assert_eq!(Tonic::from_midi_note(61), Tonic::CSharp); // C#
    /// assert_eq!(Tonic::from_midi_note(72), Tonic::C);      // C an octave higher
    /// ```
    pub fn from_midi_note(number: i32) -> Self {
        match number % 12 {
            0 => Tonic::C,
            1 => Tonic::CSharp,
            2 => Tonic::D,
            3 => Tonic::DSharp,
            4 => Tonic::E,
            5 => Tonic::F,
            6 => Tonic::FSharp,
            7 => Tonic::G,
            8 => Tonic::GSharp,
            9 => Tonic::A,
            10 => Tonic::ASharp,
            11 => Tonic::B,
            _ => unreachable!(),
        }
    }
}

impl FromStr for Tonic {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Empty" => Ok(Tonic::Empty),
            "C" => Ok(Tonic::C),
            "CSharp" => Ok(Tonic::CSharp),
            "D" => Ok(Tonic::D),
            "DSharp" => Ok(Tonic::DSharp),
            "E" => Ok(Tonic::E),
            "F" => Ok(Tonic::F),
            "FSharp" => Ok(Tonic::FSharp),
            "G" => Ok(Tonic::G),
            "GSharp" => Ok(Tonic::GSharp),
            "A" => Ok(Tonic::A),
            "ASharp" => Ok(Tonic::ASharp),
            "B" => Ok(Tonic::B),
            _ => Err(format!("Invalid tonic: {}", s)),
        }
    }
}

impl FromStr for Scale {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Empty" => Ok(Scale::Empty),
            "Major" => Ok(Scale::Major),
            "Minor" => Ok(Scale::Minor),
            "Dorian" => Ok(Scale::Dorian),
            "Mixolydian" => Ok(Scale::Mixolydian),
            "Aeolian" => Ok(Scale::Aeolian),
            "Phrygian" => Ok(Scale::Phrygian),
            "Locrian" => Ok(Scale::Locrian),
            "WholeTone" => Ok(Scale::WholeTone),
            "HalfWholeDim" => Ok(Scale::HalfWholeDim),
            "WholeHalfDim" => Ok(Scale::WholeHalfDim),
            "MinorBlues" => Ok(Scale::MinorBlues),
            "MinorPentatonic" => Ok(Scale::MinorPentatonic),
            "MajorPentatonic" => Ok(Scale::MajorPentatonic),
            "HarmonicMinor" => Ok(Scale::HarmonicMinor),
            "MelodicMinor" => Ok(Scale::MelodicMinor),
            "Dorian4" => Ok(Scale::Dorian4),
            "PhrygianDominant" => Ok(Scale::PhrygianDominant),
            "LydianDominant" => Ok(Scale::LydianDominant),
            "LydianAugmented" => Ok(Scale::LydianAugmented),
            "HarmonicMajor" => Ok(Scale::HarmonicMajor),
            "SuperLocrian" => Ok(Scale::SuperLocrian),
            "BToneSpanish" => Ok(Scale::BToneSpanish),
            "HungarianMinor" => Ok(Scale::HungarianMinor),
            "Hirajoshi" => Ok(Scale::Hirajoshi),
            "Iwato" => Ok(Scale::Iwato),
            "PelogSelisir" => Ok(Scale::PelogSelisir),
            "PelogTembung" => Ok(Scale::PelogTembung),
            "Messiaen1" => Ok(Scale::Messiaen1),
            "Messiaen2" => Ok(Scale::Messiaen2),
            "Messiaen3" => Ok(Scale::Messiaen3),
            "Messiaen4" => Ok(Scale::Messiaen4),
            "Messiaen5" => Ok(Scale::Messiaen5),
            "Messiaen6" => Ok(Scale::Messiaen6),
            "Messiaen7" => Ok(Scale::Messiaen7),
            _ => Err(format!("Invalid scale: {}", s)),
        }
    }
}

impl FromStr for PluginFormat {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "VST2Instrument" | "VST2 Instrument" => Ok(PluginFormat::VST2Instrument),
            "VST2AudioFx" | "VST2 Effect" => Ok(PluginFormat::VST2AudioFx),
            "VST3Instrument" | "VST3 Instrument" => Ok(PluginFormat::VST3Instrument),
            "VST3AudioFx" | "VST3 Effect" => Ok(PluginFormat::VST3AudioFx),
            _ => Err(format!("Invalid plugin format: {}", s)),
        }
    }
}

impl fmt::Display for Tonic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for Scale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// Musical key signature combining a tonic and scale.
///
/// This struct represents a complete key signature as used in music theory,
/// combining a root note ([`Tonic`]) with a [`Scale`] to define the key of
/// a musical piece or section.
///
/// # Examples
///
/// ```rust
/// use seula::models::{KeySignature, Tonic, Scale};
///
/// let c_major = KeySignature {
///     tonic: Tonic::C,
///     scale: Scale::Major,
/// };
///
/// let a_minor = KeySignature {
///     tonic: Tonic::A,
///     scale: Scale::Minor,
/// };
///
/// // Display formatting
/// println!("{}", c_major); // "C Major"
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KeySignature {
    /// The root note of the key
    pub tonic: Tonic,
    /// The scale type of the key
    pub scale: Scale,
}

impl fmt::Display for KeySignature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {:?}", self.tonic, self.scale)
    }
}

// PLUGINS

/// Plugin format types supported by Ableton Live.
///
/// This enum represents the different plugin formats that can be used in
/// Ableton Live projects. It distinguishes between VST2 and VST3 formats,
/// as well as between instruments and audio effects.
///
/// # Examples
///
/// ```rust
/// use seula::models::PluginFormat;
///
/// let format = PluginFormat::VST3Instrument;
/// println!("{}", format); // "VST3 Instrument"
///
/// // Get development type and category
/// let (dev_type, category) = format.to_dev_type_and_category();
/// assert_eq!(dev_type, "vst3");
/// assert_eq!(category, "instr");
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginFormat {
    /// VST2 instrument plugin
    VST2Instrument,
    /// VST2 audio effect plugin
    VST2AudioFx,
    /// VST3 instrument plugin
    VST3Instrument,
    /// VST3 audio effect plugin
    VST3AudioFx,
}

impl PluginFormat {
    /// Generates a random plugin format for testing purposes.
    ///
    /// This method is primarily used in testing and development to create
    /// random plugin formats. It selects equally from all four format variants.
    ///
    /// # Returns
    ///
    /// A randomly selected [`PluginFormat`] variant
    pub fn random() -> Self {
        let variants = [
            PluginFormat::VST2Instrument,
            PluginFormat::VST2AudioFx,
            PluginFormat::VST3Instrument,
            PluginFormat::VST3AudioFx,
        ];
        *variants.choose(&mut thread_rng()).unwrap()
    }

    /// Converts the plugin format to development type and category strings.
    ///
    /// This method maps the plugin format to the string representations used
    /// in Ableton's plugin database for development type and category.
    ///
    /// # Returns
    ///
    /// A tuple containing `(dev_type, category)` where:
    /// - `dev_type` is either "vst" or "vst3"
    /// - `category` is either "instr" or "audiofx"
    ///
    /// # Examples
    ///
    /// ```rust
    /// use seula::models::PluginFormat;
    ///
    /// let format = PluginFormat::VST3Instrument;
    /// let (dev_type, category) = format.to_dev_type_and_category();
    /// assert_eq!(dev_type, "vst3");
    /// assert_eq!(category, "instr");
    /// ```
    pub fn to_dev_type_and_category(self) -> (&'static str, &'static str) {
        match self {
            PluginFormat::VST2Instrument => ("vst", "instr"),
            PluginFormat::VST2AudioFx => ("vst", "audiofx"),
            PluginFormat::VST3Instrument => ("vst3", "instr"),
            PluginFormat::VST3AudioFx => ("vst3", "audiofx"),
        }
    }
}

impl fmt::Display for PluginFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginFormat::VST2Instrument => write!(f, "VST2 Instrument"),
            PluginFormat::VST2AudioFx => write!(f, "VST2 Effect"),
            PluginFormat::VST3Instrument => write!(f, "VST3 Instrument"),
            PluginFormat::VST3AudioFx => write!(f, "VST3 Effect"),
        }
    }
}

/// The parts of an Ableton `dev_identifier`.
///
/// ```text
/// device:vst3:audiofx:72c4db71-7a4d-459a-b97e-51745d84b39d
///        kind category id
///
/// device:vst:audiofx:1096184373?n=Altiverb%207
///                    id           name (URL-encoded)
/// ```
pub(crate) struct DevIdentifierParts<'a> {
    /// `vst` or `vst3`.
    pub kind: &'a str,
    /// `instr` or `audiofx` — **Ableton's** classification, which disagrees with the
    /// plugin's own in practice. Never part of identity; see ADR-0005.
    pub category: &'a str,
    /// The plugin's own identifier: decimal `i32` for VST2, dashed hex class ID for
    /// VST3.
    pub id: &'a str,
    /// The `?n=` value, still URL-encoded. Ableton's display name for the plugin.
    pub name: Option<&'a str>,
}

/// Split a `dev_identifier` into its parts, or `None` if it is not one.
///
/// The single place that understands this string's shape. Its two consumers read
/// different fields out of it, deliberately: [`parse_plugin_format`] wants `category`,
/// while [`PluginKey`] has nowhere to put it.
///
/// [`parse_plugin_format`]: crate::utils::plugins::parse_plugin_format
pub(crate) fn split_dev_identifier(dev_identifier: &str) -> Option<DevIdentifierParts<'_>> {
    let rest = dev_identifier.strip_prefix("device:")?;
    let (kind, rest) = rest.split_once(':')?;
    let (category, rest) = rest.split_once(':')?;

    if !matches!(kind, "vst" | "vst3") {
        return None;
    }

    // Everything before `?` is the identifier. The query carries Ableton's display
    // name; treat it as one of possibly several parameters rather than assuming.
    let (id, name) = match rest.split_once('?') {
        Some((id, query)) => (id, query.split('&').find_map(|p| p.strip_prefix("n="))),
        None => (rest, None),
    };

    if id.is_empty() {
        return None;
    }

    Some(DevIdentifierParts {
        kind,
        category,
        id,
        name,
    })
}

/// A plugin's own identity, independent of how any host refers to it.
///
/// This is the join key between a plugin reference in a project file and a binary the
/// scanner found on disk (ADR-0005). Ableton's `dev_identifier` is an *encoding* of
/// this, not a separate namespace: strip its prefix and any `?n=` suffix and what
/// remains is the plugin's native identifier.
///
/// The format lives in the variant rather than in a field, so there is nowhere to put
/// Ableton's `instr`/`audiofx` classification — which must never participate in
/// identity, because it is Ableton's opinion rather than the plugin's and the two
/// disagree in practice. Note that [`PluginFormat`] *does* encode that distinction in
/// its four variants, which is exactly why it is not the key.
///
/// Version is deliberately absent too: a uid is stable across plugin updates, so a
/// project referencing Serum still matches after Serum ships a new build.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginKey {
    /// VST2's 32-bit unique id, usually a packed four-character code.
    Vst2(u32),
    /// VST3's 128-bit class ID.
    Vst3([u8; 16]),
}

impl PluginKey {
    /// Derive the key from Ableton's `dev_identifier`.
    ///
    /// Returns `None` for anything that is not a plugin reference — which is also the
    /// gate the parser uses for "is this device a plugin at all".
    pub fn from_dev_identifier(dev_identifier: &str) -> Option<Self> {
        let parts = split_dev_identifier(dev_identifier)?;

        match parts.kind {
            // Ableton writes the id as decimal without a guaranteed sign convention,
            // and VST2's `unique_id` is a signed i32. Going via i64 accepts both
            // `-1094795586` and `3200171710` — the same 32 bits either way.
            "vst" => Some(PluginKey::Vst2(parts.id.parse::<i64>().ok()? as u32)),
            "vst3" => Self::from_uid_hex(&parts.id.replace('-', "")),
            _ => None,
        }
    }

    /// Derive the key from a uid as the scanner reports it (`PluginMeta::uid`).
    ///
    /// The length disambiguates the format, and is fixed by the formats themselves
    /// rather than being a heuristic: a VST2 id is 32 bits (8 hex digits) and a VST3
    /// class ID is 128 bits (32 hex digits). Case is not significant — Ableton writes
    /// lowercase and the scanner uppercase.
    pub fn from_uid_hex(uid: &str) -> Option<Self> {
        match uid.len() {
            8 => Some(PluginKey::Vst2(u32::from_str_radix(uid, 16).ok()?)),
            32 => {
                let bytes = hex::decode(uid).ok()?;
                Some(PluginKey::Vst3(bytes.try_into().ok()?))
            }
            _ => None,
        }
    }

    /// The canonical lowercase hex form, for storage and comparison.
    pub fn uid_hex(&self) -> String {
        match self {
            PluginKey::Vst2(id) => format!("{:08x}", id),
            PluginKey::Vst3(bytes) => hex::encode(bytes),
        }
    }

    /// `"VST2"` or `"VST3"` — the format half of the stored identity.
    pub fn kind(&self) -> &'static str {
        match self {
            PluginKey::Vst2(_) => "VST2",
            PluginKey::Vst3(_) => "VST3",
        }
    }
}

/// Ableton's display name for a plugin, from a `dev_identifier`'s `?n=` suffix.
///
/// Only VST2 identifiers carry one. It exists as a fallback for the rare project file
/// whose `<Name>` element is blank — the parser warns about those, and recovering
/// "Altiverb 7" from `?n=Altiverb%207` beats storing an empty string.
pub fn dev_identifier_display_name(dev_identifier: &str) -> Option<String> {
    let encoded = split_dev_identifier(dev_identifier)?.name?;
    let decoded = percent_decode(encoded);
    if decoded.trim().is_empty() {
        None
    } else {
        Some(decoded)
    }
}

/// Minimal percent-decoder for the `?n=` suffix.
///
/// Not a general URL decoder: this handles the one field we read, and leaves malformed
/// escapes as literal text rather than failing.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 3 <= bytes.len() => {
                match std::str::from_utf8(&bytes[i + 1..i + 3])
                    .ok()
                    .and_then(|hex| u8::from_str_radix(hex, 16).ok())
                {
                    Some(byte) => {
                        out.push(byte);
                        i += 3;
                    }
                    None => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            byte => {
                out.push(byte);
                i += 1;
            }
        }
    }

    String::from_utf8_lossy(&out).into_owned()
}

impl fmt::Display for PluginKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.kind(), self.uid_hex())
    }
}

/// Represents a plugin used in an Ableton Live project.
///
/// This struct contains comprehensive information about a plugin, including
/// both metadata extracted from the project file and installation status
/// determined by checking against Ableton's plugin database.
///
/// # Plugin Installation Status
///
/// The [`Plugin::installed`] field indicates whether the plugin is installed on this
/// system, as of the last plugin scan. It is written only by that scan — never by
/// parsing a project — and is `None` until a scan has looked.
///
/// # Examples
///
/// ```rust
/// use seula::models::{Plugin, PluginFormat};
/// use uuid::Uuid;
///
/// let plugin = Plugin::new(
///     "Serum".to_string(),
///     "serum_vst".to_string(),
///     PluginFormat::VST3Instrument,
/// );
///
/// // Check if plugin is installed
/// match plugin.installed {
///     Some(true) => println!("Plugin {} is installed", plugin.name),
///     Some(false) => println!("Plugin {} is missing", plugin.name),
///     None => println!("Plugin {} has not been scanned for", plugin.name),
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Plugin {
    /// Unique identifier for our database
    pub id: Uuid,
    /// Developer identifier used to uniquely identify the plugin
    pub dev_identifier: String,
    /// Human-readable plugin name
    pub name: String,
    /// Plugin vendor/manufacturer
    pub vendor: Option<String>,
    /// Plugin version string
    pub version: Option<String>,
    /// The format/type of this plugin
    pub plugin_format: PluginFormat,
    /// Whether the plugin is installed on this system, as of the last plugin scan.
    ///
    /// `None` means no scan has looked yet — which is different from having looked and
    /// not found it. A partial scan (one narrowed with `--paths`, or one whose restart
    /// budget ran out) leaves plugins it did not reach as `None` rather than declaring
    /// them missing.
    pub installed: Option<bool>,
}

/// Plugin data with usage statistics for gRPC responses
pub struct GrpcPlugin {
    /// The base plugin data
    pub plugin: Plugin,
    /// Number of times this plugin is used across all projects
    pub usage_count: i32,
    /// Number of unique projects that use this plugin
    pub project_count: i32,
}

#[allow(dead_code)]
impl Plugin {
    /// Creates a new plugin instance with minimal information.
    ///
    /// This constructor creates a plugin with the provided basic information
    /// and sets all optional fields to `None`. The plugin is initially marked
    /// with `installed` unknown until a plugin scan has looked.
    ///
    /// # Arguments
    ///
    /// * `name` - Human-readable plugin name
    /// * `dev_identifier` - Unique developer identifier for the plugin
    /// * `plugin_format` - The format/type of the plugin
    ///
    /// # Returns
    ///
    /// A new [`Plugin`] instance with a generated UUID
    ///
    /// # Examples
    ///
    /// ```rust
    /// use seula::models::{Plugin, PluginFormat};
    ///
    /// let plugin = Plugin::new(
    ///     "Massive".to_string(),
    ///     "massive_vst".to_string(),
    ///     PluginFormat::VST2Instrument,
    /// );
    ///
    /// assert_eq!(plugin.name, "Massive");
    /// assert_eq!(plugin.installed, None); // No scan has looked yet
    /// ```
    pub fn new(name: String, dev_identifier: String, plugin_format: PluginFormat) -> Self {
        Self {
            id: Uuid::new_v4(),
            dev_identifier,
            name,
            vendor: None,
            version: None,
            plugin_format,
            installed: None,
        }
    }

}

/// What a project file says about a plugin, before anything is resolved.
///
/// Exactly the three things an `.als` yields — and `plugin_format` is derived from
/// `dev_identifier` rather than read, so it is really two.
#[derive(Debug)]
#[allow(dead_code)]
pub struct PluginInfo {
    pub name: String,
    pub dev_identifier: String,
    pub plugin_format: PluginFormat,
}

impl fmt::Display for PluginInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.plugin_format, self.name)
    }
}

// Sample types

/// Represents an audio sample used in an Ableton Live project.
///
/// This struct contains information about audio samples referenced in project files,
/// including their file system location and whether they are currently present
/// on the system. The presence check is performed to identify missing samples
/// that may cause playback issues.
///
/// # Sample Presence
///
/// The [`Sample::is_present`] field indicates whether the sample file exists
/// at the specified path. This is useful for identifying projects with missing
/// samples that may not play correctly.
///
/// # Examples
///
/// ```rust
/// use seula::models::Sample;
/// use std::path::PathBuf;
///
/// let sample = Sample::new(
///     "kick.wav".to_string(),
///     PathBuf::from("/path/to/kick.wav"),
/// );
///
/// if sample.is_present {
///     println!("Sample {} is available", sample.name);
/// } else {
///     println!("Sample {} is missing!", sample.name);
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Sample {
    /// Unique identifier for our database
    pub id: Uuid,
    /// Human-readable sample name (usually the filename)
    pub name: String,
    /// File system path to the sample file
    pub path: PathBuf,
    /// Whether the sample file exists on the system
    pub is_present: bool,
}

#[allow(dead_code)]
impl Sample {
    pub fn new(name: String, path: PathBuf) -> Self {
        let is_present = path.exists();
        Self {
            id: Uuid::new_v4(),
            name,
            path,
            is_present,
        }
    }

    pub fn from_pre_11_data(data: &str) -> Result<Self, SampleError> {
        let cleaned_data = data.replace('\t', "").replace('\n', "");
        let byte_data = hex::decode(&cleaned_data).map_err(SampleError::HexDecodeError)?;

        let utf16_chunks: Vec<u16> = byte_data
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect();

        let path_string = String::from_utf16(&utf16_chunks)
            .map_err(|_| SampleError::InvalidUtf16Encoding)?
            .replace('\0', "");

        let path = PathBuf::from(path_string);

        if !path.exists() {
            return Err(SampleError::FileNotFound(path));
        }

        let name = path
            .file_name()
            .and_then(|osstr| osstr.to_str())
            .map(String::from)
            .unwrap_or_else(|| "Unknown".to_string());

        Ok(Self::new(name, path))
    }

    pub fn from_11_plus_data(path_value: &str) -> Self {
        let path = PathBuf::from(path_value);
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        Self::new(name, path)
    }

    pub fn is_present(&self) -> bool {
        self.is_present
    }

    pub fn update_presence(&mut self) {
        self.is_present = self.path.exists();
    }
}

/// Musical time signature with validation support.
///
/// This struct represents a time signature as used in music theory, consisting
/// of a numerator (beats per measure) and denominator (note value that gets
/// the beat). The struct includes validation to ensure the time signature
/// values are musically valid.
///
/// # Validation Rules
///
/// - Numerator must be between 1 and 99 inclusive
/// - Denominator must be between 1 and 16 inclusive and be a power of 2
///
/// # Examples
///
/// ```rust
/// use seula::models::TimeSignature;
///
/// // Common time signatures
/// let four_four = TimeSignature { numerator: 4, denominator: 4 };
/// let three_four = TimeSignature { numerator: 3, denominator: 4 };
/// let six_eight = TimeSignature { numerator: 6, denominator: 8 };
///
/// assert!(four_four.is_valid());
/// assert!(three_four.is_valid());
/// assert!(six_eight.is_valid());
///
/// // Invalid time signature
/// let invalid = TimeSignature { numerator: 4, denominator: 3 }; // 3 is not a power of 2
/// assert!(!invalid.is_valid());
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct TimeSignature {
    /// Number of beats per measure
    pub numerator: u8,
    /// Note value that gets the beat (must be a power of 2)
    pub denominator: u8,
}

impl TimeSignature {
    pub fn is_valid(&self) -> bool {
        // Check numerator is between 1 and 99
        if !(1..=99).contains(&self.numerator) {
            return false;
        }

        // Check denominator is between 1 and 16 and is a power of 2
        if self.denominator > 16 || self.denominator < 1 {
            return false;
        }

        // Check if denominator is a power of 2
        self.denominator & (self.denominator - 1) == 0
    }

    pub fn from_encoded(encoded_value: i32) -> Result<Self, TimeSignatureError> {
        if encoded_value < 0 || encoded_value > 494 {
            return Err(TimeSignatureError::InvalidEncodedValue(encoded_value));
        }

        let numerator = Self::decode_numerator(encoded_value);
        let denominator = Self::decode_denominator(encoded_value);

        Ok(TimeSignature {
            numerator,
            denominator,
        })
    }

    fn decode_numerator(encoded_value: i32) -> u8 {
        if encoded_value < 0 {
            1
        } else if encoded_value < 99 {
            (encoded_value + 1) as u8
        } else {
            ((encoded_value % 99) + 1) as u8
        }
    }

    fn decode_denominator(encoded_value: i32) -> u8 {
        let multiple = encoded_value / 99 + 1;
        2_u8.pow((multiple - 1) as u32)
    }
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self {
            numerator: 0,
            denominator: 0,
        }
    }
}

impl Default for KeySignature {
    fn default() -> Self {
        KeySignature {
            tonic: Tonic::Empty,
            scale: Scale::Empty,
        }
    }
}

impl fmt::Display for AbletonVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if self.beta {
            write!(f, " beta")?;
        }
        Ok(())
    }
}

impl Default for Id {
    fn default() -> Self {
        Id(0)
    }
}

/// Statistics for a collection (for status bar display)
#[derive(Debug, Clone, Serialize)]
pub struct CollectionStatistics {
    /// Number of projects in the collection
    pub project_count: i32,
    /// Total duration of all projects in seconds
    pub total_duration_seconds: Option<f64>,
    /// Average tempo across all projects
    pub average_tempo: Option<f64>,
    /// Total number of unique plugins used across all projects
    pub total_plugins: i32,
    /// Total number of unique samples used across all projects
    pub total_samples: i32,
    /// Total number of unique tags used across all projects
    pub total_tags: i32,
    /// Most common key signature across all projects
    pub most_common_key: Option<String>,
    /// Most common time signature across all projects
    pub most_common_time_signature: Option<String>,
}

#[cfg(test)]
mod plugin_key_tests {
    use super::*;

    // Every string here is real: the `dev_identifier`s come from the parser fixtures
    // in tests/scan/parser/plugins.rs, and the uids are what `vst-meta` actually
    // reported when scanning this machine. The point of these tests is to hold the
    // two halves of ADR-0005 together — if either side's encoding ever drifts, the
    // round-trip assertions below break rather than matching silently failing.

    const PRO_Q_3_DEV_ID: &str = "device:vst3:audiofx:72c4db71-7a4d-459a-b97e-51745d84b39d";
    const PRO_Q_3_SCANNED_UID: &str = "72C4DB717A4D459AB97E51745D84B39D";

    const ALTIVERB_DEV_ID: &str = "device:vst:audiofx:1096184373?n=Altiverb%207";
    const ALTIVERB_SCANNED_UID: &str = "41567235";

    const SPRINGBOX_DEV_ID: &str = "device:vst3:audiofx:13b117f4-1b21-3a38-7923-ff895d3b3131";

    #[test]
    fn vst3_reference_and_scanned_binary_agree() {
        assert_eq!(
            PluginKey::from_dev_identifier(PRO_Q_3_DEV_ID),
            PluginKey::from_uid_hex(PRO_Q_3_SCANNED_UID),
            "the fixture's dev_identifier and the scanner's uid are the same plugin"
        );
    }

    #[test]
    fn vst2_reference_and_scanned_binary_agree() {
        assert_eq!(
            PluginKey::from_dev_identifier(ALTIVERB_DEV_ID),
            PluginKey::from_uid_hex(ALTIVERB_SCANNED_UID)
        );
    }

    #[test]
    fn vst2_decimal_id_is_the_fourcc() {
        // 1096184373 == 0x41567235 == "AVr5", Altiverb's four-character code.
        let key = PluginKey::from_dev_identifier(ALTIVERB_DEV_ID).unwrap();
        assert_eq!(key, PluginKey::Vst2(0x4156_7235));

        let PluginKey::Vst2(id) = key else {
            panic!("expected a VST2 key")
        };
        assert_eq!(&id.to_be_bytes(), b"AVr5");
    }

    #[test]
    fn vst3_uid_matches_the_als_uid_fields() {
        // The same .als carries the class ID a second time, as four u32s. ADR-0005
        // derives the dev_identifier suffix from them; this asserts that derivation
        // rather than trusting the prose.
        let fields: [u32; 4] = [330_373_108, 455_162_424, 2_032_402_313, 1_564_160_305];
        let mut bytes = Vec::new();
        for field in fields {
            bytes.extend_from_slice(&field.to_be_bytes());
        }

        assert_eq!(
            PluginKey::from_dev_identifier(SPRINGBOX_DEV_ID),
            Some(PluginKey::Vst3(bytes.try_into().unwrap()))
        );
    }

    #[test]
    fn abletons_category_is_not_part_of_identity() {
        // The whole reason the format lives in the variant. Ableton's instr/audiofx
        // call is its own, and disagrees with the plugin's in practice.
        let as_fx = PluginKey::from_dev_identifier(
            "device:vst3:audiofx:72c4db71-7a4d-459a-b97e-51745d84b39d",
        );
        let as_instr = PluginKey::from_dev_identifier(
            "device:vst3:instr:72c4db71-7a4d-459a-b97e-51745d84b39d",
        );

        assert_eq!(as_fx, as_instr);
        assert!(as_fx.is_some());
    }

    #[test]
    fn vst2_ids_are_accepted_in_either_sign_convention() {
        // The same 32 bits: VST2's unique_id is a signed i32 and Ableton writes it as
        // decimal without committing to a convention.
        let negative = PluginKey::from_dev_identifier("device:vst:instr:-1094795586");
        let positive = PluginKey::from_dev_identifier("device:vst:instr:3200171710");

        assert_eq!(negative, positive);
        assert!(negative.is_some());
    }

    #[test]
    fn uid_case_does_not_matter() {
        // Ableton writes lowercase, the scanner uppercase.
        assert_eq!(
            PluginKey::from_uid_hex(PRO_Q_3_SCANNED_UID),
            PluginKey::from_uid_hex(&PRO_Q_3_SCANNED_UID.to_lowercase())
        );
    }

    #[test]
    fn canonical_form_round_trips() {
        let key = PluginKey::from_dev_identifier(PRO_Q_3_DEV_ID).unwrap();

        assert_eq!(key.kind(), "VST3");
        assert_eq!(key.uid_hex(), PRO_Q_3_SCANNED_UID.to_lowercase());
        assert_eq!(PluginKey::from_uid_hex(&key.uid_hex()), Some(key));

        let vst2 = PluginKey::from_dev_identifier(ALTIVERB_DEV_ID).unwrap();
        assert_eq!(vst2.kind(), "VST2");
        assert_eq!(vst2.uid_hex(), "41567235");
        assert_eq!(PluginKey::from_uid_hex(&vst2.uid_hex()), Some(vst2));
    }

    #[test]
    fn non_plugin_identifiers_are_rejected() {
        // This is also the parser's gate for "is this device a plugin at all", so a
        // false positive here would invent plugins out of stock Ableton devices.
        for input in [
            "",
            "device:",
            "device:vst3",
            "device:vst3:audiofx",
            "device:vst3:audiofx:",
            "device:auv3:audiofx:1234",
            "device:drum:instr:1234",
            "vst3:audiofx:72c4db71",
            "query:Everything#Pro-Q%203",
        ] {
            assert_eq!(
                PluginKey::from_dev_identifier(input),
                None,
                "should not parse as a plugin reference: {:?}",
                input
            );
        }
    }

    #[test]
    fn malformed_uids_are_rejected_rather_than_truncated() {
        assert_eq!(PluginKey::from_uid_hex(""), None);
        assert_eq!(PluginKey::from_uid_hex("72c4db71"), Some(PluginKey::Vst2(0x72c4db71)));
        // Right length, not hex.
        assert_eq!(PluginKey::from_uid_hex("zzzzzzzz"), None);
        assert_eq!(PluginKey::from_uid_hex(&"z".repeat(32)), None);
        // Wrong length: a truncated or over-long uid must not silently match.
        assert_eq!(PluginKey::from_uid_hex("72c4db717a4d459ab97e51745d84b39"), None);
        assert_eq!(PluginKey::from_uid_hex("72c4db717a4d459ab97e51745d84b39dff"), None);
    }

    #[test]
    fn the_display_name_suffix_is_split_off_not_included() {
        let parts = split_dev_identifier(ALTIVERB_DEV_ID).unwrap();

        assert_eq!(parts.kind, "vst");
        assert_eq!(parts.category, "audiofx");
        assert_eq!(parts.id, "1096184373");
        assert_eq!(parts.name, Some("Altiverb%207"));
    }

    #[test]
    fn the_display_name_suffix_decodes() {
        assert_eq!(
            dev_identifier_display_name(ALTIVERB_DEV_ID).as_deref(),
            Some("Altiverb 7")
        );
        // VST3 identifiers carry no name.
        assert_eq!(dev_identifier_display_name(PRO_Q_3_DEV_ID), None);
        // A malformed escape stays literal rather than losing the whole name.
        assert_eq!(
            dev_identifier_display_name("device:vst:instr:1?n=Odd%ZZName").as_deref(),
            Some("Odd%ZZName")
        );
        // Truncated escape at the end.
        assert_eq!(
            dev_identifier_display_name("device:vst:instr:1?n=Trail%").as_deref(),
            Some("Trail%")
        );
    }

    #[test]
    fn a_name_suffix_does_not_change_the_key() {
        assert_eq!(
            PluginKey::from_dev_identifier("device:vst:instr:1096184373"),
            PluginKey::from_dev_identifier("device:vst:instr:1096184373?n=Anything%20At%20All")
        );
    }
}
