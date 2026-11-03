import { spawnSync } from "node:child_process";
import process from "node:process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const tooling = resolve(root, ".tooling");
const cargoHome = process.env.CARGO_HOME ?? resolve(tooling, "cargo");
const candidates = process.platform === "win32" ? ["cargo.exe", "cargo"] : ["cargo"];
const env = {
  ...process.env,
  CARGO_HOME: cargoHome,
  RUSTUP_HOME: process.env.RUSTUP_HOME ?? resolve(tooling, "rustup"),
  CARGO_TARGET_DIR: process.env.CARGO_TARGET_DIR ?? resolve(tooling, "tauri-target"),
  PATH: [resolve(tooling, "bin"), resolve(cargoHome, "bin"), process.env.PATH ?? ""].join(process.platform === "win32" ? ";" : ":"),
};

for (const cargo of candidates) {
  const result = spawnSync(cargo, ["check", "--locked", "--manifest-path", resolve(root, "app/src-tauri/Cargo.toml")], {
    cwd: root,
    env,
    stdio: "inherit",
  });
  if (result.error?.code === "ENOENT") continue;
  process.exit(result.status ?? 1);
}

console.error("Cargo is required. Source scripts/local-env.sh or install Rust into .tooling first.");
process.exit(1);
