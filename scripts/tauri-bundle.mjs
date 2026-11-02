import { spawnSync } from "node:child_process";
import process from "node:process";
import { fileURLToPath } from "node:url";

const candidates = process.platform === "win32" ? ["python", "py"] : ["python3", "python"];
const script = fileURLToPath(new URL("./tauri-bundle.py", import.meta.url));

for (const candidate of candidates) {
  const result = spawnSync(candidate, [script.pathname], { stdio: "inherit" });
  if (result.error?.code === "ENOENT") continue;
  process.exit(result.status ?? 1);
}

console.error("Python is required to build the desktop bundle.");
process.exit(1);
