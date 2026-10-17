#!/usr/bin/env node
/* Fail the build if an em dash or en dash sneaks into the site.
   Astra's house rule is a plain hyphen and nothing wider. */

import { readdirSync, readFileSync, statSync } from "node:fs";
import { join, extname, relative } from "node:path";

const ROOT = process.argv[2] || "website";
const EXT = new Set([
  ".html",
  ".htm",
  ".css",
  ".js",
  ".mjs",
  ".cjs",
  ".ts",
  ".tsx",
  ".jsx",
  ".json",
  ".svg",
  ".xml",
  ".txt",
  ".md",
  ".webmanifest",
]);

const BAD = [
  { name: "em dash (U+2014)", ch: "\u2014" },
  { name: "en dash (U+2013)", ch: "\u2013" },
  { name: "horizontal bar (U+2015)", ch: "\u2015" },
  { name: "figure dash (U+2012)", ch: "\u2012" },
];

const SKIP = new Set(["node_modules", ".git", ".taste-skill", ".impeccable"]);

const hits = [];

function walk(dir) {
  for (const entry of readdirSync(dir)) {
    if (SKIP.has(entry)) continue;
    const full = join(dir, entry);
    const info = statSync(full);
    if (info.isDirectory()) {
      walk(full);
      continue;
    }
    if (!EXT.has(extname(entry).toLowerCase())) continue;
    scan(full);
  }
}

function scan(file) {
  const text = readFileSync(file, "utf8");
  const lines = text.split(/\r?\n/);
  lines.forEach((line, index) => {
    for (const bad of BAD) {
      let at = line.indexOf(bad.ch);
      while (at !== -1) {
        hits.push({
          file: relative(process.cwd(), file),
          line: index + 1,
          col: at + 1,
          what: bad.name,
          context: line.trim().slice(0, 90),
        });
        at = line.indexOf(bad.ch, at + 1);
      }
    }
  });
}

walk(ROOT);

if (hits.length) {
  console.error(`\nDash check failed: ${hits.length} violation(s).\n`);
  for (const hit of hits) {
    console.error(`  ${hit.file}:${hit.line}:${hit.col}  ${hit.what}`);
    console.error(`    ${hit.context}`);
  }
  console.error("\nUse a plain hyphen, a comma, a colon, or a period instead.\n");
  process.exit(1);
}

console.log("Dash check passed: no em or en dashes in the site.");
