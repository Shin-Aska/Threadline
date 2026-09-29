import { chmod, copyFile, mkdir } from "node:fs/promises";
import { homedir, platform } from "node:os";
import { dirname, join } from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

if (platform() === "linux") {
  // Vendored from tauri-apps/linuxdeploy-plugin-gtk revision
  // dda522bce37387f1b853d9095713bfaa924c8423. The only functional change
  // removes its forced GObject/GIO additions; dependency exclusions then keep
  // the target's GLib/GIO and modules an ABI-matched set.
  const here = dirname(fileURLToPath(import.meta.url));
  const source = join(here, "vendor", "linuxdeploy-plugin-gtk.sh");
  const directory = join(process.env.XDG_CACHE_HOME || join(homedir(), ".cache"), "tauri");
  const destination = join(directory, "linuxdeploy-plugin-gtk.sh");
  await mkdir(directory, { recursive: true });
  await copyFile(source, destination);
  await chmod(destination, 0o755);

  // Vendored from tauri-apps/linuxdeploy-plugin-gstreamer revision
  // 2a2e67491c32995a3f279ad0ecbe77abd512b42a. Its final cleanup supports
  // older linuxdeploy binaries that do not honour the exclusion environment.
  const gstreamerSource = join(here, "vendor", "linuxdeploy-plugin-gstreamer.sh");
  const gstreamerDestination = join(directory, "linuxdeploy-plugin-gstreamer.sh");
  await copyFile(gstreamerSource, gstreamerDestination);
  await chmod(gstreamerDestination, 0o755);
}
