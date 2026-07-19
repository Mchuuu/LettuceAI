import { access, mkdir, readFile, readdir, rename, rm, writeFile } from "node:fs/promises";
import path from "node:path";

const repoRoot = process.cwd();
const androidAppDir = path.join(repoRoot, "src-tauri", "gen", "android", "app");
const androidOverridesDir = path.join(repoRoot, "src-tauri", "android-overrides");
const tauriConfigPath = path.join(repoRoot, "src-tauri", "tauri.conf.json");
const javaRootDir = path.join(androidAppDir, "src", "main", "java");
const manifestPath = path.join(androidAppDir, "src", "main", "AndroidManifest.xml");
const stringsPath = path.join(androidAppDir, "src", "main", "res", "values", "strings.xml");
const valuesDir = path.join(androidAppDir, "src", "main", "res", "values");
const valuesNightDir = path.join(androidAppDir, "src", "main", "res", "values-night");
const drawableDir = path.join(androidAppDir, "src", "main", "res", "drawable");
const proguardRulesPath = path.join(androidAppDir, "proguard-rules.pro");
const buildGradlePath = path.join(androidAppDir, "build.gradle.kts");
const sherpaVersion = "1.13.4";
const sherpaAarName = `sherpa-onnx-${sherpaVersion}.aar`;

async function main() {
  const packageName = await readBasePackageName(tauriConfigPath);
  const packagePath = packageName.split(".").join(path.sep);
  const targetJavaDir = path.join(javaRootDir, packagePath);

  await mkdir(targetJavaDir, { recursive: true });
  await deleteStaleClassFiles(javaRootDir, targetJavaDir, [
    "MainActivity.kt",
    "CrashMonitorService.kt",
    "KokoroPhonemizerBridge.kt",
    "PcmAudioTrackBridge.kt",
    "SenseVoiceBridge.kt",
    "ZipformerCtcBridge.kt",
  ]);
  await ensureSherpaAndroidRuntime();

  await applyTemplate("MainActivity.kt.template", path.join(targetJavaDir, "MainActivity.kt"), {
    __PACKAGE__: packageName,
  });
  await applyTemplate(
    "CrashMonitorService.kt.template",
    path.join(targetJavaDir, "CrashMonitorService.kt"),
    {
      __PACKAGE__: packageName,
    },
  );
  await applyTemplate(
    "KokoroPhonemizerBridge.kt.template",
    path.join(targetJavaDir, "KokoroPhonemizerBridge.kt"),
    {
      __PACKAGE__: packageName,
    },
  );
  await applyTemplate(
    "PcmAudioTrackBridge.kt.template",
    path.join(targetJavaDir, "PcmAudioTrackBridge.kt"),
    {
      __PACKAGE__: packageName,
    },
  );
  await applyTemplate(
    "SenseVoiceBridge.kt.template",
    path.join(targetJavaDir, "SenseVoiceBridge.kt"),
    {
      __PACKAGE__: packageName,
    },
  );
  await applyTemplate(
    "ZipformerCtcBridge.kt.template",
    path.join(targetJavaDir, "ZipformerCtcBridge.kt"),
    {
      __PACKAGE__: packageName,
    },
  );
  await applyTemplate("AndroidManifest.xml", manifestPath, {
    __PACKAGE__: packageName,
  });
  await applyTemplate("proguard-rules.pro", proguardRulesPath, {
    __PACKAGE__: packageName,
  });
  await applyTemplate("strings.xml", stringsPath, {});
  await applyTemplate("themes.xml", path.join(valuesDir, "themes.xml"), {});
  await applyTemplate("themes-night.xml", path.join(valuesNightDir, "themes.xml"), {});
  await applyTemplate(
    "transparent_splash_icon.xml",
    path.join(drawableDir, "transparent_splash_icon.xml"),
    {},
  );
  await applyTemplate("app.build.gradle.kts.template", buildGradlePath, {
    __PACKAGE__: packageName,
  });

  console.log(
    `Applied Android overrides for package ${packageName} (applicationId override via LETTUCE_ANDROID_APPLICATION_ID)`,
  );
}

async function ensureSherpaAndroidRuntime() {
  const libsDir = path.join(androidAppDir, "libs");
  const aarPath = path.join(libsDir, sherpaAarName);
  await mkdir(libsDir, { recursive: true });
  if (!(await pathExists(aarPath))) {
    const url = `https://github.com/k2-fsa/sherpa-onnx/releases/download/v${sherpaVersion}/${sherpaAarName}`;
    const tempPath = `${aarPath}.part`;
    console.log(`Downloading sherpa-onnx Android runtime ${sherpaVersion}...`);
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`Failed to download ${url}: HTTP ${response.status}`);
    }
    await writeFile(tempPath, Buffer.from(await response.arrayBuffer()));
    await rename(tempPath, aarPath);
  }

  // The AAR owns ONNX Runtime on Android. A second copy from jniLibs causes
  // duplicate packaging and can load an ABI-incompatible runtime.
  for (const abi of ["arm64-v8a", "armeabi-v7a", "x86", "x86_64"]) {
    await rm(path.join(androidAppDir, "src", "main", "jniLibs", abi, "libonnxruntime.so"), {
      force: true,
    });
  }
}

async function pathExists(targetPath) {
  try {
    await access(targetPath);
    return true;
  } catch {
    return false;
  }
}

async function readBasePackageName(filePath) {
  const content = await readFile(filePath, "utf8");
  const config = JSON.parse(content);
  const packageName = config?.identifier;

  if (typeof packageName !== "string" || packageName.length === 0) {
    throw new Error(`Failed to resolve Android package name from ${filePath}`);
  }

  return packageName;
}

async function applyTemplate(templateName, targetPath, replacements) {
  const templatePath = path.join(androidOverridesDir, templateName);
  let content = await readFile(templatePath, "utf8");
  for (const [needle, value] of Object.entries(replacements)) {
    content = content.replaceAll(needle, value);
  }
  await writeFile(targetPath, content);
}

async function deleteStaleClassFiles(javaDir, targetJavaDir, fileNames) {
  const entries = await walk(javaDir);
  const targetFiles = new Set(fileNames.map((name) => path.join(targetJavaDir, name)));

  await Promise.all(
    entries
      .filter((entry) => fileNames.includes(path.basename(entry)))
      .filter((entry) => !targetFiles.has(entry))
      .map((entry) => rm(entry, { force: true })),
  );
}

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const paths = await Promise.all(
    entries.map(async (entry) => {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        return walk(fullPath);
      }
      return [fullPath];
    }),
  );
  return paths.flat();
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exitCode = 1;
});
