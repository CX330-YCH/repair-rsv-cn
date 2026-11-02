import { lstat, readdir, unlink } from "node:fs/promises";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));

async function visit(directory) {
  for (const entry of await readdir(directory)) {
    const path = join(directory, entry);
    let details;
    try {
      details = await lstat(path);
    } catch (error) {
      if (error.code === "ENOENT") continue;
      throw error;
    }
    if (details.isSymbolicLink()) continue;
    if (details.isDirectory()) await visit(path);
    else if (entry.startsWith("._")) await unlink(path);
  }
}

await visit(root);
