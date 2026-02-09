import { describe, it, expect } from "vitest";
import { getInstallChain, getUninstallChain } from "../../lib/taskDependencies";
import type { TaskInfo, TaskStateInfo } from "../../types/task";
import type { BlueprintTaskEntry } from "../../types/blueprint";

/** Minimal TaskInfo factory — only the fields the dependency functions use. */
function makeTask(id: string, overrides: Partial<TaskInfo> = {}): TaskInfo {
  return {
    id,
    name: id,
    description: "",
    icon: "Box",
    category: "test",
    tags: [],
    privilege_level: "user",
    execution_target: "any",
    depends_on: [],
    config_schema: [],
    status: "not_started",
    supports_uninstall: true,
    ...overrides,
  };
}

function makeEntry(taskId: string, enabled = true, order = 0): BlueprintTaskEntry {
  return { task_id: taskId, enabled, config_overrides: {}, order };
}

function makeStateMap(
  entries: Array<{ id: string; status: string; drifted?: boolean; versionChanged?: boolean }>,
): Map<string, TaskStateInfo> {
  const map = new Map<string, TaskStateInfo>();
  for (const e of entries) {
    map.set(e.id, {
      ...makeTask(e.id, { status: e.status as TaskInfo["status"] }),
      config_drifted: e.drifted ?? false,
      version_changed: e.versionChanged ?? false,
    });
  }
  return map;
}

describe("getInstallChain", () => {
  it("returns single task when it has no dependencies", () => {
    const taskInfoMap = { a: makeTask("a") };
    const stateMap = makeStateMap([{ id: "a", status: "not_started" }]);
    const entries = [makeEntry("a")];

    const chain = getInstallChain("a", taskInfoMap, stateMap, entries);
    expect(chain).toEqual(["a"]);
  });

  it("includes uninstalled dependency in correct order", () => {
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "not_started" },
      { id: "b", status: "not_started" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b")];

    const chain = getInstallChain("b", taskInfoMap, stateMap, entries);
    expect(chain).toEqual(["a", "b"]);
  });

  it("returns full chain for transitive dependencies", () => {
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
      c: makeTask("c", { depends_on: ["b"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "not_started" },
      { id: "b", status: "not_started" },
      { id: "c", status: "not_started" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b"), makeEntry("c")];

    const chain = getInstallChain("c", taskInfoMap, stateMap, entries);
    expect(chain).toEqual(["a", "b", "c"]);
  });

  it("skips already-installed dependencies", () => {
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "completed" },
      { id: "b", status: "not_started" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b")];

    const chain = getInstallChain("b", taskInfoMap, stateMap, entries);
    expect(chain).toEqual(["b"]);
  });

  it("includes drifted dependency (config_drifted)", () => {
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "completed", drifted: true },
      { id: "b", status: "not_started" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b")];

    const chain = getInstallChain("b", taskInfoMap, stateMap, entries);
    expect(chain).toEqual(["a", "b"]);
  });

  it("includes version-changed dependency", () => {
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "completed", versionChanged: true },
      { id: "b", status: "not_started" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b")];

    const chain = getInstallChain("b", taskInfoMap, stateMap, entries);
    expect(chain).toEqual(["a", "b"]);
  });

  it("excludes disabled entries from the chain", () => {
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "not_started" },
      { id: "b", status: "not_started" },
    ]);
    // "a" is disabled in the blueprint
    const entries = [makeEntry("a", false), makeEntry("b")];

    const chain = getInstallChain("b", taskInfoMap, stateMap, entries);
    // "a" is disabled, so it should not appear
    expect(chain).toEqual(["b"]);
  });

  it("handles circular dependencies without infinite loop", () => {
    const taskInfoMap = {
      a: makeTask("a", { depends_on: ["b"] }),
      b: makeTask("b", { depends_on: ["a"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "not_started" },
      { id: "b", status: "not_started" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b")];

    // Should terminate and include both, order may vary
    const chain = getInstallChain("a", taskInfoMap, stateMap, entries);
    expect(chain).toContain("a");
    expect(chain).toContain("b");
    expect(chain.length).toBe(2);
  });

  it("handles diamond dependencies correctly", () => {
    // d -> b, c; b -> a; c -> a
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
      c: makeTask("c", { depends_on: ["a"] }),
      d: makeTask("d", { depends_on: ["b", "c"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "not_started" },
      { id: "b", status: "not_started" },
      { id: "c", status: "not_started" },
      { id: "d", status: "not_started" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b"), makeEntry("c"), makeEntry("d")];

    const chain = getInstallChain("d", taskInfoMap, stateMap, entries);
    // "a" must be before "b" and "c", "d" must be last
    expect(chain.indexOf("a")).toBeLessThan(chain.indexOf("b"));
    expect(chain.indexOf("a")).toBeLessThan(chain.indexOf("c"));
    expect(chain[chain.length - 1]).toBe("d");
    expect(chain.length).toBe(4);
  });
});

describe("getUninstallChain", () => {
  it("returns single task when nothing depends on it", () => {
    const taskInfoMap = { a: makeTask("a") };
    const stateMap = makeStateMap([{ id: "a", status: "completed" }]);
    const entries = [makeEntry("a")];

    const chain = getUninstallChain("a", taskInfoMap, stateMap, entries);
    expect(chain).toEqual(["a"]);
  });

  it("includes installed dependents first", () => {
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "completed" },
      { id: "b", status: "completed" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b")];

    const chain = getUninstallChain("a", taskInfoMap, stateMap, entries);
    // "b" depends on "a", so "b" should be uninstalled first, then "a"
    expect(chain).toEqual(["b", "a"]);
  });

  it("includes transitive reverse dependencies", () => {
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
      c: makeTask("c", { depends_on: ["b"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "completed" },
      { id: "b", status: "completed" },
      { id: "c", status: "completed" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b"), makeEntry("c")];

    const chain = getUninstallChain("a", taskInfoMap, stateMap, entries);
    // c depends on b depends on a: uninstall c first, then b, then a
    expect(chain.indexOf("c")).toBeLessThan(chain.indexOf("b"));
    expect(chain.indexOf("b")).toBeLessThan(chain.indexOf("a"));
    expect(chain.length).toBe(3);
  });

  it("skips uninstalled dependents", () => {
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "completed" },
      { id: "b", status: "not_started" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b")];

    const chain = getUninstallChain("a", taskInfoMap, stateMap, entries);
    expect(chain).toEqual(["a"]);
  });

  it("excludes disabled entries from the chain", () => {
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "completed" },
      { id: "b", status: "completed" },
    ]);
    // "b" is disabled
    const entries = [makeEntry("a"), makeEntry("b", false)];

    const chain = getUninstallChain("a", taskInfoMap, stateMap, entries);
    // "b" is disabled, so it should not be considered a dependent
    expect(chain).toEqual(["a"]);
  });

  it("handles circular reverse dependencies without infinite loop", () => {
    const taskInfoMap = {
      a: makeTask("a", { depends_on: ["b"] }),
      b: makeTask("b", { depends_on: ["a"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "completed" },
      { id: "b", status: "completed" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b")];

    const chain = getUninstallChain("a", taskInfoMap, stateMap, entries);
    expect(chain).toContain("a");
    expect(chain).toContain("b");
    expect(chain.length).toBe(2);
  });

  it("handles diamond reverse dependencies correctly", () => {
    // b -> a; c -> a; d -> b, c
    // Uninstalling a should uninstall d, b, c first
    const taskInfoMap = {
      a: makeTask("a"),
      b: makeTask("b", { depends_on: ["a"] }),
      c: makeTask("c", { depends_on: ["a"] }),
      d: makeTask("d", { depends_on: ["b", "c"] }),
    };
    const stateMap = makeStateMap([
      { id: "a", status: "completed" },
      { id: "b", status: "completed" },
      { id: "c", status: "completed" },
      { id: "d", status: "completed" },
    ]);
    const entries = [makeEntry("a"), makeEntry("b"), makeEntry("c"), makeEntry("d")];

    const chain = getUninstallChain("a", taskInfoMap, stateMap, entries);
    // "a" must be last
    expect(chain[chain.length - 1]).toBe("a");
    // d depends on b and c, so d must come before b and c
    expect(chain.indexOf("d")).toBeLessThan(chain.indexOf("b"));
    expect(chain.indexOf("d")).toBeLessThan(chain.indexOf("c"));
    expect(chain.length).toBe(4);
  });
});
