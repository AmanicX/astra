import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { describe, expect, it } from "vitest";

const here = dirname(fileURLToPath(import.meta.url));
const turnRs = readFileSync(resolve(here, "../src-tauri/src/turn.rs"), "utf8");
const typesTs = readFileSync(resolve(here, "../src/lib/types.ts"), "utf8");

function snake(name: string): string {
  return name.replace(/([a-z0-9])([A-Z])/g, "$1_$2").toLowerCase();
}

/** Rust enum variant names → snake_case (what serde's rename_all emits). */
function rustVariants(source: string): string[] {
  const block = source.match(/pub enum TurnEvent \{([\s\S]*?)\n\}/);
  if (!block) throw new Error("TurnEvent enum not found in turn.rs");
  return [...block[1].matchAll(/^\s{4}(\w+) \{/gm)].map((m) => snake(m[1]));
}

/** Balanced-brace slice of `{ type: "<name>"; ... }` from the TS union. */
function tsVariantBlock(source: string, name: string): string {
  const needle = `type: "${name}";`;
  const at = source.indexOf(needle);
  if (at < 0) throw new Error(`TS variant ${name} not found`);
  const start = source.lastIndexOf("{", at);
  let depth = 0;
  for (let i = start; i < source.length; i++) {
    if (source[i] === "{") depth++;
    else if (source[i] === "}") {
      depth--;
      if (depth === 0) return source.slice(start, i + 1);
    }
  }
  throw new Error(`unbalanced braces for ${name}`);
}

/** Rust field names for one variant: `VariantName { ... }`. */
function rustFields(source: string, variant: string): string[] {
  const pascal = variant
    .split("_")
    .map((p) => p[0].toUpperCase() + p.slice(1))
    .join("");
  const start = source.indexOf(`${pascal} {`);
  if (start < 0) throw new Error(`Rust variant ${pascal} not found`);
  const open = source.indexOf("{", start);
  let depth = 0;
  let end = open;
  for (let i = open; i < source.length; i++) {
    if (source[i] === "{") depth++;
    else if (source[i] === "}") {
      depth--;
      if (depth === 0) {
        end = i;
        break;
      }
    }
  }
  const body = source.slice(open + 1, end);
  return [...body.matchAll(/^\s*(\w+):/gm)].map((m) => m[1]);
}

describe("TurnEvent contract (Rust ↔ TypeScript)", () => {
  const variants = rustVariants(turnRs);

  it("finds every Rust variant", () => {
    expect(variants.length).toBeGreaterThanOrEqual(8);
    expect(variants).toContain("turn_started");
    expect(variants).toContain("approval_request");
  });

  it("mirrors every Rust variant in the TS union", () => {
    for (const variant of variants) {
      expect(() => tsVariantBlock(typesTs, variant)).not.toThrow();
    }
  });

  it("keeps the serde tag and snake_case renames", () => {
    expect(turnRs).toContain('#[serde(tag = "type", rename_all = "snake_case")]');
  });

  it("carries every field of every variant across to TypeScript", () => {
    for (const variant of variants) {
      const ts = tsVariantBlock(typesTs, variant);
      for (const field of rustFields(turnRs, variant)) {
        expect(ts, `${variant}.${field}`).toContain(field);
      }
    }
  });
});
