import java.io.File
import org.apache.tools.ant.taskdefs.condition.Os
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.logging.LogLevel
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.TaskAction

open class BuildTask : DefaultTask() {
    @Input
    var rootDirRel: String? = null
    @Input
    var target: String? = null
    @Input
    var release: Boolean? = null

    @TaskAction
    fun assemble() {
        val executable = """bun""";
        try {
            runTauriCli(executable)
        } catch (e: Exception) {
            if (Os.isFamily(Os.FAMILY_WINDOWS)) {
                // Try different Windows-specific extensions
                val fallbacks = listOf(
                    "$executable.exe",
                    "$executable.cmd",
                    "$executable.bat",
                )

                var lastException: Exception = e
                for (fallback in fallbacks) {
                    try {
                        runTauriCli(fallback)
                        return
                    } catch (fallbackException: Exception) {
                        lastException = fallbackException
                    }
                }
                throw lastException
            } else {
                throw e;
            }
        }
    }

    fun runTauriCli(executable: String) {
        val rootDirRel = rootDirRel ?: throw GradleException("rootDirRel cannot be null")
        val target = target ?: throw GradleException("target cannot be null")
        val release = release ?: throw GradleException("release cannot be null")
        val args = listOf("tauri", "android", "android-studio-script");

        project.exec {
            workingDir(File(project.projectDir, rootDirRel))
            executable(executable)
            args(args)
            if (project.logger.isEnabled(LogLevel.DEBUG)) {
                args("-vv")
            } else if (project.logger.isEnabled(LogLevel.INFO)) {
                args("-v")
            }
            if (release) {
                args("--release")
            }
            args(listOf("--target", target))
            configureAndroidBuildEnvironment()
        }.assertNormalExitValue()
    }

    private fun org.gradle.process.ExecSpec.configureAndroidBuildEnvironment() {
        val pathSeparator = File.pathSeparator
        val currentPath = System.getenv("PATH") ?: System.getenv("Path") ?: ""
        val pathEntries = mutableListOf<String>()

        val llvmBin = File("C:\\Program Files\\LLVM\\bin")
        if (File(llvmBin, "libclang.dll").exists()) {
            environment("LIBCLANG_PATH", llvmBin.absolutePath)
        }

        val cmakeBin = File("C:\\Program Files\\CMake\\bin")
        if (File(cmakeBin, "cmake.exe").exists()) {
            pathEntries.add(cmakeBin.absolutePath)
        }

        val localAppData = System.getenv("LOCALAPPDATA") ?: ""
        val ninjaDir = File(
            localAppData,
            "Microsoft\\WinGet\\Packages\\Ninja-build.Ninja_Microsoft.Winget.Source_8wekyb3d8bbwe",
        )
        if (File(ninjaDir, "ninja.exe").exists()) {
            pathEntries.add(ninjaDir.absolutePath)
        }

        val androidHome = System.getenv("ANDROID_HOME") ?: System.getenv("ANDROID_SDK_ROOT")
        val ndkRoot = System.getenv("ANDROID_NDK_HOME")
            ?: System.getenv("NDK_HOME")
            ?: androidHome?.let { File(it, "ndk\\26.3.11579264").absolutePath }

        if (ndkRoot != null) {
            val ndkBin = File(ndkRoot, "toolchains\\llvm\\prebuilt\\windows-x86_64\\bin")
            val ndkSysroot = File(ndkRoot, "toolchains\\llvm\\prebuilt\\windows-x86_64\\sysroot")
            if (ndkBin.exists()) {
                environment("ANDROID_NDK_HOME", ndkRoot)
                environment("NDK_HOME", ndkRoot)
                pathEntries.add(ndkBin.absolutePath)

                val ar = File(ndkBin, "llvm-ar.exe").absolutePath
                val ranlib = File(ndkBin, "llvm-ranlib.exe").absolutePath
                val triples = listOf(
                    Triple("aarch64_linux_android", "aarch64-linux-android", "aarch64-linux-android"),
                    Triple("armv7_linux_androideabi", "armv7-linux-androideabi", "armv7a-linux-androideabi"),
                    Triple("i686_linux_android", "i686-linux-android", "i686-linux-android"),
                    Triple("x86_64_linux_android", "x86_64-linux-android", "x86_64-linux-android"),
                )

                for ((envTriple, rustTriple, clangTriple) in triples) {
                    environment("CC_$envTriple", File(ndkBin, "${clangTriple}34-clang.cmd").absolutePath)
                    environment("CXX_$envTriple", File(ndkBin, "${clangTriple}34-clang++.cmd").absolutePath)
                    environment("AR_$envTriple", ar)
                    environment("RANLIB_$envTriple", ranlib)

                    val bindgenArgs = "--target=${clangTriple}34 --sysroot=${ndkSysroot.absolutePath}"
                    environment("BINDGEN_EXTRA_CLANG_ARGS_$envTriple", bindgenArgs)
                    environment("BINDGEN_EXTRA_CLANG_ARGS_$rustTriple", bindgenArgs)
                }
            }
        }

        if (pathEntries.isNotEmpty()) {
            val path = (pathEntries + currentPath).joinToString(pathSeparator)
            environment("PATH", path)
            environment("Path", path)
        }
    }
}
