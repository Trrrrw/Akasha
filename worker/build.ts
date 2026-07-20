import { mkdir, rm, copyFile, chmod } from "node:fs/promises";
import { dirname, join } from "node:path";

const distDir = "dist";

const workers = [
  {
    name: "news",
    entry: "src/news/index.ts",
    outfile: "dist/news/run",
  },
  {
    name: "character",
    entry: "src/character/index.ts",
    outfile: "dist/character/run",
  },
  {
    name: "event",
    entry: "src/event/index.ts",
    outfile: "dist/event/run",
  },
];

await rm(distDir, { recursive: true, force: true });
await mkdir(distDir, { recursive: true });

await copyFile("src/run.sh", join(distDir, "run.sh"));
await chmod(join(distDir, "run.sh"), 0o755);

for (const worker of workers) {
  await mkdir(dirname(worker.outfile), { recursive: true });

  const result = await Bun.build({
    entrypoints: [worker.entry],
    compile: {
      target: "bun-linux-x64-musl",
      outfile: worker.outfile,
    },
    minify: true,
  });

  if (!result.success) {
    console.error(`Failed to build ${worker.name}`);
    for (const log of result.logs) {
      console.error(log);
    }
    process.exit(1);
  }

  await chmod(worker.outfile, 0o755);
  console.log(`Built ${worker.name}: ${worker.outfile}`);
}
