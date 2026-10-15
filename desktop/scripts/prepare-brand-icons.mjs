// Deterministic packaging only: no artwork edits, runtime dependency or promotion.
// node desktop/scripts/prepare-brand-icons.mjs NEW_OUTPUT_DIRECTORY
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const desktop = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const output = process.argv[2] && resolve(process.argv[2]);
if (!output || process.argv.length !== 3) throw new Error("Provide one fresh output directory");
if (existsSync(output)) throw new Error("Output directory already exists; preserve prior evidence");
if (process.versions.node.split(".")[0] !== "22") throw new Error("Use the project's Node 22 runtime");
const cliDirectory = join(desktop, "node_modules/@tauri-apps/cli");
const cliVersion = JSON.parse(readFileSync(join(cliDirectory, "package.json"), "utf8")).version;
if (cliVersion !== "2.11.4") throw new Error("Icon conversion is pinned to Tauri CLI 2.11.4");
const master = "src/assets/devsweep-icon-master.png";
const bytes = readFileSync(join(desktop, master));
if (bytes.subarray(0, 8).toString("hex") !== "89504e470d0a1a0a"
  || bytes.readUInt32BE(16) !== bytes.readUInt32BE(20) || bytes[25] !== 6) {
  throw new Error("The compact master must be a square RGBA PNG");
}
mkdirSync(output, { recursive: true });
const generated = join(output, "generated"), icons = join(output, "icons");
const run = spawnSync(process.execPath, [join(cliDirectory, "tauri.js"), "icon", join(desktop, master), "--output", generated], {
  windowsHide: true, encoding: "utf8",
});
writeFileSync(join(output, "conversion.log"), (run.stdout ?? "") + (run.stderr ?? ""));
if (run.error) throw run.error;
if (run.status !== 0) throw new Error(`Tauri icon conversion failed: ${run.status}`);
// Tauri's ICNS encoder varies chunk order between runs. Preserve each payload
// byte-for-byte and sort the container records so the exported hash is stable.
const icnsPath = join(generated, "icon.icns"), icns = readFileSync(icnsPath), chunks = [];
if (icns.subarray(0, 4).toString() !== "icns" || icns.readUInt32BE(4) !== icns.length) {
  throw new Error("Unexpected ICNS container");
}
for (let offset = 8; offset < icns.length;) {
  if (offset + 8 > icns.length) throw new Error("Truncated ICNS chunk");
  const size = icns.readUInt32BE(offset + 4);
  if (size < 8 || offset + size > icns.length) throw new Error("Invalid ICNS chunk size");
  chunks.push(icns.subarray(offset, offset + size));
  offset += size;
}
chunks.sort((left, right) => Buffer.compare(left.subarray(0, 4), right.subarray(0, 4)));
writeFileSync(icnsPath, Buffer.concat([icns.subarray(0, 8), ...chunks]));
mkdirSync(icons);
// Preserve the existing desktop family. Mobile outputs stay in ignored staging.
const names = [
  "32x32.png", "128x128.png", "128x128@2x.png", "icon.png", "icon.ico", "icon.icns",
  "Square30x30Logo.png", "Square44x44Logo.png", "Square71x71Logo.png", "Square89x89Logo.png",
  "Square107x107Logo.png", "Square142x142Logo.png", "Square150x150Logo.png",
  "Square284x284Logo.png", "Square310x310Logo.png", "StoreLogo.png",
].sort();
const hash = (data) => createHash("sha256").update(data).digest("hex");
const record = (path, data) => ({ path, bytes: data.length, sha256: hash(data) });
const derived = names.map((name) => {
  const source = join(generated, name);
  copyFileSync(source, join(icons, name));
  const data = readFileSync(source);
  return { ...record(`src-tauri/icons/${name}`, data), ...(name.endsWith(".png")
    ? { width: data.readUInt32BE(16), height: data.readUInt32BE(20) } : {}) };
});
const provenance = ["branding/devsweep-beaver-full.png", "branding/prompts/full.txt",
  "branding/prompts/compact-v1.txt", "branding/prompts/compact-v2.txt"];
const manifest = {
  version: 1,
  generator: { tool: "@tauri-apps/cli icon", version: cliVersion, node: process.versions.node,
    icns_container: "Sort chunk types lexicographically; encoded pixel and alpha payloads unchanged" },
  artwork: { tool: "built-in image_gen", selected: "compact-v2", content_edits_after_generation: false },
  master: { ...record(master, bytes), width: bytes.readUInt32BE(16), height: bytes.readUInt32BE(20) },
  provenance: provenance.map((path) => record(path, readFileSync(join(desktop, path)))),
  derived,
};
writeFileSync(join(output, "manifest.json"), JSON.stringify(manifest, null, 2) + "\n");
console.log(JSON.stringify({ output, icons: derived.length, master_sha256: manifest.master.sha256 }));
