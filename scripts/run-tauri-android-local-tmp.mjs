#!/usr/bin/env node

import { access, mkdir } from "node:fs/promises";
import path from "node:path";
import { spawn } from "node:child_process";

const mode = process.argv[2];
if (!mode || !["dev", "build"].includes(mode)) {
  console.error("Usage: node scripts/run-tauri-android-local-tmp.mjs <dev|build> [...args]");
  process.exit(1);
}

const repoRoot = process.cwd();
const localTmpDir = path.join(repoRoot, ".tmp", "android-build");
const defaultLlvmBinDir = "C:\\Program Files\\LLVM\\bin";
const defaultCmakeBinDir = "C:\\Program Files\\CMake\\bin";
const defaultNinjaDir = path.join(
  process.env.LOCALAPPDATA ?? "",
  "Microsoft",
  "WinGet",
  "Packages",
  "Ninja-build.Ninja_Microsoft.Winget.Source_8wekyb3d8bbwe",
);
const androidApiLevel = "34";

async function pathExists(targetPath) {
  try {
    await access(targetPath);
    return true;
  } catch {
    return false;
  }
}

function setIfUnset(env, key, value) {
  if (!env[key]) {
    env[key] = value;
  }
}

async function configureAndroidToolchain(env) {
  const sdkRoot = env.ANDROID_HOME ?? env.ANDROID_SDK_ROOT;
  if (!sdkRoot) return;

  const ndkRoot = env.ANDROID_NDK_HOME ?? env.NDK_HOME ?? path.join(sdkRoot, "ndk", "26.3.11579264");
  const ndkBinDir = path.join(ndkRoot, "toolchains", "llvm", "prebuilt", "windows-x86_64", "bin");
  const ndkSysroot = path.join(ndkRoot, "toolchains", "llvm", "prebuilt", "windows-x86_64", "sysroot");
  if (!(await pathExists(ndkBinDir))) return;

  env.ANDROID_NDK_HOME = ndkRoot;
  env.NDK_HOME = ndkRoot;
  env.Path = `${ndkBinDir};${env.Path ?? env.PATH ?? ""}`;
  env.PATH = env.Path;

  const ar = path.join(ndkBinDir, "llvm-ar.exe");
  const ranlib = path.join(ndkBinDir, "llvm-ranlib.exe");
  const triples = [
    ["aarch64_linux_android", "aarch64-linux-android", "aarch64-linux-android"],
    ["armv7_linux_androideabi", "armv7-linux-androideabi", "armv7a-linux-androideabi"],
    ["i686_linux_android", "i686-linux-android", "i686-linux-android"],
    ["x86_64_linux_android", "x86_64-linux-android", "x86_64-linux-android"],
  ];

  for (const [envTriple, rustTriple, clangTriple] of triples) {
    setIfUnset(env, `CC_${envTriple}`, path.join(ndkBinDir, `${clangTriple}${androidApiLevel}-clang.cmd`));
    setIfUnset(env, `CXX_${envTriple}`, path.join(ndkBinDir, `${clangTriple}${androidApiLevel}-clang++.cmd`));
    setIfUnset(env, `AR_${envTriple}`, ar);
    setIfUnset(env, `RANLIB_${envTriple}`, ranlib);

    const bindgenArgs = `--target=${clangTriple}${androidApiLevel} --sysroot=${ndkSysroot}`;
    setIfUnset(env, `BINDGEN_EXTRA_CLANG_ARGS_${envTriple}`, bindgenArgs);
    setIfUnset(env, `BINDGEN_EXTRA_CLANG_ARGS_${rustTriple}`, bindgenArgs);
  }
}

async function runCommand(command, args, env) {
  return new Promise((resolve, reject) => {
    const child = spawn(command, args, {
      stdio: "inherit",
      env,
    });

    child.on("exit", (code, signal) => {
      if (signal) {
        process.kill(process.pid, signal);
        return;
      }
      if (code === 0) {
        resolve();
        return;
      }
      reject(new Error(`${command} ${args.join(" ")} exited with code ${code ?? 1}`));
    });

    child.on("error", reject);
  });
}

async function main() {
  await mkdir(localTmpDir, { recursive: true });

  const envPath = process.env.Path ?? process.env.PATH ?? "";
  const env = {
    ...process.env,
    TMPDIR: localTmpDir,
    TMP: localTmpDir,
    TEMP: localTmpDir,
  };

  if (!env.LIBCLANG_PATH && (await pathExists(path.join(defaultLlvmBinDir, "libclang.dll")))) {
    env.LIBCLANG_PATH = defaultLlvmBinDir;
    env.Path = envPath;
    env.PATH = envPath;
  }

  if (await pathExists(path.join(defaultCmakeBinDir, "cmake.exe"))) {
    env.Path = `${defaultCmakeBinDir};${env.Path ?? env.PATH ?? ""}`;
    env.PATH = env.Path;
  }

  if (await pathExists(path.join(defaultNinjaDir, "ninja.exe"))) {
    env.Path = `${defaultNinjaDir};${env.Path ?? env.PATH ?? ""}`;
    env.PATH = env.Path;
  }

  await configureAndroidToolchain(env);

  console.log(`[android-local-tmp] Using TMPDIR=${localTmpDir}`);
  if (env.LIBCLANG_PATH) {
    console.log(`[android-local-tmp] Using LIBCLANG_PATH=${env.LIBCLANG_PATH}`);
  }

  await runCommand("node", ["scripts/apply-android-overrides.mjs"], env);
  await runCommand("tauri", ["android", mode, ...process.argv.slice(3)], env);
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
