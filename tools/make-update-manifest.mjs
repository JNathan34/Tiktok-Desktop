import { readFile, writeFile } from "node:fs/promises";
import { join } from "node:path";

const assets = process.argv[2];
if (!assets) throw new Error("Pass the folder containing both installers and their .sig files.");

const { version, plugins } = JSON.parse(await readFile(
  new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8",
));
if (!/^\d+\.\d+\.\d+$/.test(version)) throw new Error("Expected a release version in the Tauri config.");

const endpoint = plugins.updater.endpoints[0];
const releases = endpoint.replace(/\/latest\/download\/latest\.json$/, "");
if (releases === endpoint) throw new Error("Expected a GitHub Releases updater endpoint.");

const platforms = {};
for (const [installerArch, updateArch] of [["arm64", "aarch64"], ["x64", "x86_64"]]) {
  const name = `TikTok-Desktop-${version}-${installerArch}-setup.exe`;
  const installer = await readFile(join(assets, name));
  if (installer.subarray(0, 2).toString() !== "MZ") throw new Error(`Invalid installer: ${name}`);
  const signature = (await readFile(join(assets, name + ".sig"), "utf8")).trim();
  if (!Buffer.from(signature, "base64").toString().startsWith("untrusted comment:")) {
    throw new Error(`Invalid updater signature: ${name}.sig`);
  }
  platforms[`windows-${updateArch}`] = {
    signature,
    url: `${releases}/download/v${version}/${name}`,
  };
}

const manifest = { version, pub_date: new Date().toISOString(), platforms };
await writeFile(join(assets, "latest.json"), JSON.stringify(manifest, null, 2) + "\n");
