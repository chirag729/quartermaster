import { describe, it, expect } from "vitest";
import { formatError } from "../../lib/formatError";

describe("formatError", () => {
  it("formats a TauriError with a hint", () => {
    const err = { kind: "IoError", message: "File not found", hint: "Check the file path" };
    expect(formatError(err)).toBe("File not found. Check the file path");
  });

  it("formats a TauriError without a hint", () => {
    const err = { kind: "ConfigError", message: "Invalid config" };
    expect(formatError(err)).toBe("Invalid config");
  });

  it("formats a TauriError with null hint", () => {
    const err = { kind: "ConfigError", message: "Invalid config", hint: null };
    expect(formatError(err)).toBe("Invalid config");
  });

  it("formats a standard Error instance", () => {
    const err = new Error("Something went wrong");
    expect(formatError(err)).toBe("Something went wrong");
  });

  it("formats a plain string", () => {
    expect(formatError("raw error string")).toBe("raw error string");
  });

  it("formats other types by converting to string", () => {
    expect(formatError(42)).toBe("42");
    expect(formatError(null)).toBe("null");
    expect(formatError(undefined)).toBe("undefined");
  });
});
