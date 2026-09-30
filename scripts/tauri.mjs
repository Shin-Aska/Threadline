import { spawnSync } from "node:child_process";
import process from "node:process";

const env = { ...process.env };
if (process.platform === "linux") {
  // These libraries are extension/plugin ABI boundaries. Let the target system
  // load one coherent GLib stack, and never let an older bundled nghttp2 satisfy
  // a newer host curl.
  env.LINUXDEPLOY_EXCLUDED_LIBRARIES = [
    "libglib-2.0.so*",
    "libgobject-2.0.so*",
    "libgio-2.0.so*",
    "libgmodule-2.0.so*",
    "libnghttp2.so*",
  ].join(";");
}

const command = process.platform === "win32" ? "tauri.cmd" : "tauri";
const result = spawnSync(command, process.argv.slice(2), { env, stdio: "inherit", shell: process.platform === "win32" });
if (result.error) throw result.error;
process.exit(result.status ?? 1);
