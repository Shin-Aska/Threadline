import { createHash } from "node:crypto";
import { appendFileSync, copyFileSync, existsSync, mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";

const json = file => JSON.parse(readFileSync(file, "utf8"));
const version = json("package.json").version;
const formats = {
  "linux-x86_64": ["deb", "AppImage"],
  "windows-x86_64": ["exe", "msi"],
  "macos-aarch64": ["dmg"],
  "macos-x86_64": ["dmg"],
};
const filename = (platform, extension) => `Threadline_${version}_${platform}.${extension}`;
const ensure = (condition, message) => { if (!condition) throw new Error(message); };
ensure(/^\d+\.\d+\.\d+$/.test(version), "Release version must be X.Y.Z");

switch (process.argv[2]) {
  case "validate": {
    const cargoVersion = readFileSync("src-tauri/Cargo.toml", "utf8").match(/^version = "([^"]+)"/m)?.[1];
    ensure([json("src-tauri/tauri.conf.json").version, json("package-lock.json").version, json("package-lock.json").packages[""].version, cargoVersion].every(value => value === version), "Package versions disagree");
    ensure(existsSync(`docs/releases/v${version}.md`), "Release notes are missing");
    if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `version=${version}\n`);
    process.stdout.write(`Release v${version}\n`);
    break;
  }
  case "collect": {
    const platform = process.env.RELEASE_PLATFORM;
    ensure(Object.hasOwn(formats, platform ?? ""), "Unknown release platform");
    const root = "src-tauri/target/release/bundle";
    const files = readdirSync(root, { recursive: true, withFileTypes: true }).filter(entry => entry.isFile()).map(entry => path.join(entry.parentPath, entry.name));
    mkdirSync("release-assets", { recursive: true });
    for (const extension of formats[platform]) {
      const matches = files.filter(file => file.endsWith(`.${extension}`));
      ensure(matches.length === 1, `Expected one ${extension} installer, found ${matches.length}`);
      copyFileSync(matches[0], path.join("release-assets", filename(platform, extension)));
    }
    break;
  }
  case "checksums": {
    const expected = Object.entries(formats).flatMap(([platform, extensions]) => extensions.map(extension => filename(platform, extension))).sort();
    const actual = readdirSync("release-assets").sort();
    ensure(JSON.stringify(actual) === JSON.stringify(expected), "Missing or unexpected release assets");
    const sums = expected.map(name => `${createHash("sha256").update(readFileSync(path.join("release-assets", name))).digest("hex")}  ${name}`);
    writeFileSync("release-assets/SHA256SUMS.txt", `${sums.join("\n")}\n`);
    break;
  }
  default:
    throw new Error("Usage: node scripts/release.mjs validate|collect|checksums");
}
