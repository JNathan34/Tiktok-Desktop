import { spawnSync } from "node:child_process";
import { mkdirSync, copyFileSync } from "node:fs";
const result = spawnSync(process.execPath, ["node_modules/typescript/bin/tsc"], { stdio: "inherit" });
if (result.status !== 0) process.exit(result.status ?? 1);
mkdirSync("dist", { recursive: true });
for (const name of ["index.html", "style.css"]) copyFileSync(`src/${name}`, `dist/${name}`);
