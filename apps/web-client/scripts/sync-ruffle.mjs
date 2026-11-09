// Copies the runtime files of the @ruffle-rs/ruffle npm package into
// public/ruffle so they can be committed-free served as static assets next to
// the built React app (browsers then load the wasm player same-origin).
import { createRequire } from "node:module";
import { cpSync, existsSync, mkdirSync, readdirSync, statSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const packageRoot = path.resolve(here, "..");

const require = createRequire(path.join(packageRoot, "noop.js"));
const packageJson = require.resolve("@ruffle-rs/ruffle/package.json");
const source = path.dirname(packageJson);
const target = path.join(packageRoot, "public", "ruffle");

if (!existsSync(source)) {
    console.error("@ruffle-rs/ruffle is not installed; run npm install first.");
    process.exit(1);
}

// Skipping sourcemaps and package scaffolding; everything else in the bundle
// (ruffle.js, the two core.ruffle.*.js chunks and the .wasm files) is runtime.
const kept = (name) =>
    statSync(path.join(source, name)).isFile() &&
    !/\.(map|ts)$/.test(name) &&
    name !== "package.json" &&
    !/^README/i.test(name);

mkdirSync(target, { recursive: true });
for (const entry of readdirSync(source)) {
    if (kept(entry)) {
        cpSync(path.join(source, entry), path.join(target, entry));
    }
}
console.log(`synced ruffle runtime files into public/ruffle`);
