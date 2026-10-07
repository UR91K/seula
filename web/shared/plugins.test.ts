import { describe, expect, it } from "vitest";
import { fileRow, pluginRows, pluginStats, stateOf } from "./plugins";
import type { PluginRow } from "./types";

const plugin = (id: string, installed: boolean | null, scan_error: string | null = null): PluginRow => ({
  id, dev_identifier: id, name: id, format: "VST3 Effect", installed, vendor: null, version: null,
  project_count: 0, scan_error,
});

describe("the Failed status (ADR-0067)", () => {
  it("outranks the installed flag, which stays true for a plugin whose file is there", () => {
    expect(stateOf(plugin("a", true, "crashed"))).toBe("failed");
    expect(stateOf(plugin("b", true))).toBe("installed");
    expect(stateOf(plugin("c", false))).toBe("absent");
    expect(stateOf(plugin("d", null))).toBe("unscanned");
  });

  it("names a file that never loaded after its file, from a Windows path", () => {
    const row = fileRow({
      path: "C:\\Program Files\\Common Files\\VST3\\Hangs Forever.vst3", error_type: "timeout",
      error_message: null, scanned_at: 0,
    });
    expect(row.name).toBe("Hangs Forever");
    expect(row.format).toBe("VST3");
    expect(row.id).toBe("file:C:\\Program Files\\Common Files\\VST3\\Hangs Forever.vst3");
    expect(stateOf(row)).toBe("failed");
    expect(fileRow({ path: "D:\\VstPlugins\\Old.dll", error_type: "crashed", error_message: null, scanned_at: 0 }).format)
      .toBe("VST2");
  });

  it("is counted and filtered apart from Installed", () => {
    const rows = [plugin("a", true, "crashed"), plugin("b", true), plugin("c", false), plugin("d", null)];
    expect(pluginStats(rows)).toMatchObject({ installed: 1, missing: 1, failed: 1, unscanned: 1 });
    const failed = pluginRows(rows, { vendor: null, format: null, states: ["failed"], group: null, sort: null });
    expect(failed.map((p) => p.id)).toEqual(["a"]);
  });
});
