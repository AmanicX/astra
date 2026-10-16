import { describe, expect, it } from "vitest";
import {
  cn,
  dayKey,
  riskOf,
  titleFromText,
  providerLabel,
  credentialAccount,
  RISK_LABEL,
} from "../src/lib/format";

describe("cn", () => {
  it("joins truthy class names and drops falsy ones", () => {
    expect(cn("a", false, undefined, null, "b")).toBe("a b");
    expect(cn()).toBe("");
  });
});

describe("titleFromText", () => {
  it("uses the first line and strips markdown prefixes", () => {
    expect(titleFromText("# Hello there\nsecond line")).toBe("Hello there");
    expect(titleFromText(">  quote")).toBe("quote");
    expect(titleFromText("- bullet list")).toBe("bullet list");
  });

  it("truncates long titles with an ellipsis", () => {
    const long = "x".repeat(80);
    const title = titleFromText(long);
    expect(title.length).toBeLessThanOrEqual(53);
    expect(title.endsWith("…")).toBe(true);
  });

  it("falls back to a default when blank", () => {
    expect(titleFromText("   \n\n")).toBe("New conversation");
  });
});

describe("dayKey", () => {
  it("labels today and yesterday", () => {
    expect(dayKey(new Date().toISOString())).toBe("Today");
    const yesterday = new Date(Date.now() - 86_400_000);
    expect(dayKey(yesterday.toISOString())).toBe("Yesterday");
  });
});

describe("riskOf", () => {
  it("normalizes risk strings", () => {
    expect(riskOf("high")).toBe("high");
    expect(riskOf("HIGH")).toBe("high");
    expect(riskOf("moderate")).toBe("moderate");
    expect(riskOf("anything-else")).toBe("low");
    expect(riskOf("")).toBe("low");
  });

  it("has a label for every risk", () => {
    for (const r of ["low", "moderate", "high"] as const) {
      expect(RISK_LABEL[r]).toBeTruthy();
    }
  });
});

describe("provider helpers", () => {
  it("labels known providers", () => {
    expect(providerLabel("groq")).toBe("Groq");
    expect(providerLabel("ollama")).toBe("Ollama (local)");
    expect(providerLabel("mock")).toBe("Offline test provider");
    expect(providerLabel("unknown")).toBe("unknown");
  });

  it("builds the credential account used by the keyring", () => {
    expect(credentialAccount("groq")).toBe("api_key/groq");
    expect(credentialAccount("custom")).toBe("api_key/custom");
  });
});
