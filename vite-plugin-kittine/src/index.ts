import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync } from "node:fs";
import path from "node:path";
import type { Plugin, ResolvedConfig, ViteDevServer } from "vite";

export interface KittineOptions {
  /**
   * Path to the `kittine-compiler` executable. Defaults to searching (in
   * order) a `kittine-compiler` on `$PATH`, then release/debug binaries in
   * `../kittine-compiler/target/{release,debug}` relative to the Vite
   * project root, falling back to `cargo run -p kittine-compiler --`.
   */
  compilerBin?: string;
  /**
   * Directory containing the Rust crate's `Cargo.toml` that hosts the
   * `.kitty`-derived component files. Defaults to the Vite project root.
   */
  crateDir?: string;
  /** wasm-bindgen output directory, relative to `crateDir`. Defaults to `pkg`. */
  outDir?: string;
  /** Build the Rust crate in release mode. Defaults to `false` in dev, `true` for `vite build`. */
  release?: boolean;
}

const DEFAULT_OUT_DIR = "pkg";

/** Runs a subprocess synchronously and throws a readable error on failure. */
function run(cmd: string, args: string[], cwd: string, label: string): void {
  const result = spawnSync(cmd, args, { cwd, encoding: "utf-8" });
  if (result.error) {
    throw new Error(
      `[vite-plugin-kittine] failed to run ${label} ('${cmd}'): ${result.error.message}\n` +
        `Make sure it is installed and on your PATH.`
    );
  }
  if (result.status !== 0) {
    throw new Error(
      `[vite-plugin-kittine] ${label} exited with code ${result.status}\n` +
        `--- stdout ---\n${result.stdout}\n--- stderr ---\n${result.stderr}`
    );
  }
}

function findUp(startDir: string, filename: string): string | null {
  let dir = startDir;
  for (;;) {
    if (existsSync(path.join(dir, filename))) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) return null;
    dir = parent;
  }
}

function resolveCompilerBin(root: string, explicit?: string): string[] {
  if (explicit) return [explicit];

  const candidates = [
    path.resolve(root, "../kittine-compiler/target/release/kittine-compiler"),
    path.resolve(root, "../kittine-compiler/target/debug/kittine-compiler"),
  ];
  for (const candidate of candidates) {
    if (existsSync(candidate)) return [candidate];
  }

  const compilerCrate = path.resolve(root, "../kittine-compiler");
  if (existsSync(path.join(compilerCrate, "Cargo.toml"))) {
    // Fresh checkout with no prebuilt binary yet: build-and-run through cargo.
    return ["cargo", "run", "--quiet", "--manifest-path", path.join(compilerCrate, "Cargo.toml"), "--"];
  }

  // Last resort: hope it's on PATH.
  return ["kittine-compiler"];
}

/** Reads the Cargo package name out of a crate's Cargo.toml (`[package] name = "..."`). */
function readCrateName(crateDir: string): string {
  const cargoToml = readFileSync(path.join(crateDir, "Cargo.toml"), "utf-8");
  const match = cargoToml.match(/^\s*name\s*=\s*"([^"]+)"/m);
  if (!match) {
    throw new Error(`[vite-plugin-kittine] could not find [package] name in ${crateDir}/Cargo.toml`);
  }
  return match[1];
}

function newestMtime(dir: string, extensions: string[]): number {
  let newest = 0;
  if (!existsSync(dir)) return newest;
  for (const entry of readdirSync(dir, { withFileTypes: true })) {
    const full = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (entry.name === "target" || entry.name === "node_modules" || entry.name === "pkg") continue;
      newest = Math.max(newest, newestMtime(full, extensions));
    } else if (extensions.some((ext) => entry.name.endsWith(ext))) {
      newest = Math.max(newest, statSync(full).mtimeMs);
    }
  }
  return newest;
}

/**
 * The Vite plugin. On every `.kitty` file transform:
 *
 *  1. Spawns `kittine-compiler build <file>` to produce a sibling `.rs` file.
 *  2. Rebuilds the host Rust crate to `wasm32-unknown-unknown` with `cargo
 *     build`, then runs `wasm-bindgen` over the resulting `.wasm` to produce
 *     browser-ready JS bindings + a processed `.wasm` (skipped if nothing
 *     Rust-relevant changed since the last build).
 *  3. Returns a JS module that re-exports the generated wasm-bindgen glue,
 *     so anything that `import`s the `.kitty` file gets the compiled
 *     component's `init` function and exports directly.
 */
export default function kittine(options: KittineOptions = {}): Plugin {
  let root = process.cwd();
  let isBuild = false;
  let lastBuildKey = "";

  function crateDir(): string {
    return options.crateDir ? path.resolve(root, options.crateDir) : root;
  }

  function outDir(): string {
    return path.resolve(crateDir(), options.outDir ?? DEFAULT_OUT_DIR);
  }

  function compileKitty(file: string): string {
    const [bin, ...baseArgs] = resolveCompilerBin(root, options.compilerBin);
    const rsPath = file.replace(/\.kitty$/, ".rs");
    run(bin, [...baseArgs, "build", file, "--output", rsPath], path.dirname(file), "kittine-compiler");
    return rsPath;
  }

  function buildWasmIfNeeded(): { jsGlue: string; crateName: string } {
    const dir = crateDir();
    const crateName = readCrateName(dir);
    const release = options.release ?? isBuild;
    const profileDir = release ? "release" : "debug";
    // `cargo build` keeps the package name as-is (hyphens included) for a
    // bin target's output file, but conventionally we want an
    // underscore-normalized name for the JS glue module it produces.
    const normalizedName = crateName.replace(/-/g, "_");
    const wasmPath = path.join(dir, "target", "wasm32-unknown-unknown", profileDir, `${crateName}.wasm`);
    const out = outDir();
    const jsGlue = path.join(out, `${normalizedName}.js`);

    const sourceKey = String(newestMtime(dir, [".rs", ".kitty", ".toml"]));
    const alreadyFresh = sourceKey === lastBuildKey && existsSync(jsGlue);
    if (!alreadyFresh) {
      const cargoArgs = ["build", "--target", "wasm32-unknown-unknown"];
      if (release) cargoArgs.push("--release");
      run("cargo", cargoArgs, dir, "cargo build --target wasm32-unknown-unknown");

      mkdirSync(out, { recursive: true });
      run(
        "wasm-bindgen",
        [wasmPath, "--target", "web", "--out-dir", out, "--out-name", normalizedName],
        dir,
        "wasm-bindgen"
      );
      lastBuildKey = sourceKey;
    }

    return { jsGlue, crateName };
  }

  return {
    name: "vite-plugin-kittine",

    config(userConfig) {
      // Don't let Vite's file watcher treat our own build output (the
      // Cargo `target/` dir and the wasm-bindgen `outDir`) as source
      // changes — otherwise the flurry of files written mid-build can
      // trigger a spurious full-page reload while the very first
      // compile is still in flight.
      const dir = options.crateDir ? path.resolve(root, options.crateDir) : root;
      const ignored = [
        path.join(dir, "target", "**"),
        path.join(dir, options.outDir ?? DEFAULT_OUT_DIR, "**"),
      ];
      const existing = userConfig.server?.watch?.ignored;
      const existingArr = Array.isArray(existing) ? existing : existing ? [existing] : [];
      return {
        server: {
          watch: {
            ignored: [...existingArr, ...ignored],
          },
        },
      };
    },

    configResolved(config: ResolvedConfig) {
      root = config.root;
      isBuild = config.command === "build";
    },

    configureServer(server: ViteDevServer) {
      // Serve the wasm-bindgen output (JS glue + processed .wasm binary) as
      // static files so the browser can fetch the wasm without a bundler
      // round-trip in dev mode.
      server.middlewares.use((req, res, next) => {
        const urlPath = req.url?.split("?")[0];
        if (!urlPath?.startsWith(`/${path.basename(outDir())}/`)) return next();
        const filePath = path.join(crateDir(), decodeURIComponent(urlPath));
        if (!existsSync(filePath)) return next();
        if (filePath.endsWith(".wasm")) {
          res.setHeader("Content-Type", "application/wasm");
        } else if (filePath.endsWith(".js")) {
          res.setHeader("Content-Type", "application/javascript");
        }
        res.end(readFileSync(filePath));
      });
    },

    resolveId(source) {
      if (source.endsWith(".kitty")) return source;
      return null;
    },

    transform(_code, id) {
      if (!id.endsWith(".kitty")) return null;

      const kittyFile = id.split("?")[0];
      const crate = findUp(path.dirname(kittyFile), "Cargo.toml");
      if (!crate) {
        this.error(
          `[vite-plugin-kittine] no Cargo.toml found above '${kittyFile}'. ` +
            `.kitty files must live inside a Rust crate.`
        );
      }
      if (options.crateDir === undefined) {
        // Infer crateDir from the file's own crate if the user didn't pin one.
        options.crateDir = crate!;
      }

      compileKitty(kittyFile);
      const { jsGlue, crateName } = buildWasmIfNeeded();
      const publicOutDir = `/${path.basename(outDir())}/${path.basename(jsGlue)}`;

      const code = `
// Auto-generated by vite-plugin-kittine from ${JSON.stringify(path.basename(kittyFile))}.
// Re-exports the wasm-bindgen glue produced by compiling the Kittine
// component to Leptos 0.7 Rust and then to WebAssembly.
import init, * as kittineBindings from ${JSON.stringify(publicOutDir)};

export default init;
export const bindings = kittineBindings;
export const crateName = ${JSON.stringify(crateName)};
export * from ${JSON.stringify(publicOutDir)};
`;
      return { code, map: null };
    },
  };
}
