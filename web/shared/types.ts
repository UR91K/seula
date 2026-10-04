// The slice of the HTTP DTOs (src/http/dto/) that the projects screen reads. Hand-written
// to match what the server sends, as ADR-0029 expects; no generated code.

export interface Key { tonic: string; scale: string; sharp: string; flat: string }
export interface TimeSignature { numerator: number; denominator: number }
export interface AbletonVersion { major: number; minor: number; patch: number; beta: boolean }
export interface Tag { id: string; name: string; created_at: number }
export interface Plugin {
  id: string; name: string; format: string; vendor: string | null; version: string | null;
  installed: boolean | null;  // null: no scan has looked (ADR-0012)
}
export interface Sample { id: string; name: string; path: string; is_present: boolean }
export interface Task { id: string; description: string; completed: boolean }

export interface Project {
  id: string;
  is_active: boolean;
  name: string;
  path: string;
  notes: string;
  created_at: number;
  modified_at: number;
  tempo: number;
  time_signature: TimeSignature;
  key_signature: Key | null;
  duration_seconds: number | null;
  ableton_version: AbletonVersion;
  plugins: Plugin[];
  samples: Sample[];
  tags: Tag[];
  tasks: Task[];
  collection_ids: string[];
  audio_file_id: string | null;
}

export interface Collection { id: string; name: string; project_count: number }

export interface SystemInfo { version: string; watch_paths: string[]; watcher_active: boolean; uptime_seconds: number }

/** One event of a scan's SSE stream (ScanProgressDto). */
export interface ScanProgress {
  completed: number; total: number; progress: number; message: string; status: string;
  /** The plugin scan's final event carries its counts; the project scan sends none. */
  result?: unknown;
}

/** Which scan a stream belongs to. Only one runs at a time, of any kind (ADR-0038). */
export type ScanKind = "projects" | "plugins" | "simulated";

// ---------------------------------------------------------------- plugins

/** A plugin as the list routes send it (PluginDto). `installed` is tri-state (ADR-0012):
 *  null means no scan has looked, which is not the same as looked and absent. */
export interface PluginRow {
  id: string; dev_identifier: string; name: string; format: string;
  installed: boolean | null; vendor: string | null; version: string | null;
  project_count: number;
}

/** GET /plugins/vendors and /plugins/formats: one rollup row per vendor or format. */
export interface Rollup {
  plugin_count: number; installed_plugins: number; missing_plugins: number; unknown_plugins: number;
  unique_projects_using: number;
}
export interface VendorRollup extends Rollup { vendor: string }
export interface FormatRollup extends Rollup { format: string }

export interface PluginClass { name: string; category: string }
export interface PluginBus { name: string; direction: "input" | "output"; media: string; channel_count: number; bus_type: number }
export interface PluginReference {
  dev_identifier: string; ableton_name: string | null; ableton_format: string; resolved_via: string;
}

/** GET /plugins/:id `details`. Everything a scan records is null until one has looked. */
export interface PluginDetails {
  plugin_kind: "VST2" | "VST3"; uid: string; last_scanned_at: number | null; path: string | null;
  category: string | null; is_instrument: boolean | null;
  audio_in_channels: number | null; audio_out_channels: number | null;
  audio_in_buses: number | null; audio_out_buses: number | null;
  has_midi_input: boolean | null; has_midi_output: boolean | null;
  presets: number | null; parameters: number | null; latency_samples: number | null;
  has_gui: boolean | null; vendor_url: string | null;
  fourcc: string | null; preset_chunks: boolean | null; f64_precision: boolean | null;
  silent_when_stopped: boolean | null; midi_in_channels: number | null;
  classes: PluginClass[]; buses: PluginBus[]; references: PluginReference[];
}

export type Scope = "active" | "archived";
export type KeySpelling = "sharp" | "flat";
export interface Sort { col: string; desc: boolean }
