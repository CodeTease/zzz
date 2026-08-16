# .github/workflows/modules/target.nu

use config.nu [ hr-line ]

export def build [config: record] {
    let bin     = (try { $config.metadata.bin } catch { "" })
    let version = (try { $config.metadata.version } catch { "" })

    if ($bin | is-empty) or ($version | is-empty) {
        print "::error::'metadata.bin' or 'metadata.version' is missing or empty in release.toml"
        exit 1
    }

    let os      = $env.OS
    let target  = $env.TARGET

    # Target Early Exit
    let is_target_enabled = (try { $config.targets | get $target } catch { false })
    if $is_target_enabled != true {
        print $"Target ($target) is not enabled in release.toml. Skipping build."
        exit 0
    }

    let src     = $env.GITHUB_WORKSPACE
    let dist    = $"($env.GITHUB_WORKSPACE)/output"

    print "Debugging info:"
    print { project: $bin, version: $version, os: $os, target: $target, src: $src, dist: $dist }
    hr-line -b

    let USE_UBUNTU = ($os | str starts-with "ubuntu")

    print $"(char nl)Packaging ($bin) v($version) for ($target)..."
    hr-line -b

    if not ('Cargo.lock' | path exists) {
        cargo generate-lockfile
    }

    let cargo_build_project = {
        print $"Building ($bin) for ($target)..."
        cargo build --release --all --target $target
    }

    # --- Build Environment Setup ---
    if $os in ['macos-latest'] or $USE_UBUNTU {
        if $USE_UBUNTU {
            sudo apt update
            # Generic dependencies for compilation
        }

        match $target {
            'aarch64-unknown-linux-gnu' => {
                if ($os | str ends-with "-arm") {
                    do $cargo_build_project
                } else {
                    sudo apt-get install gcc-aarch64-linux-gnu -y
                    $env.CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER = 'aarch64-linux-gnu-gcc'
                    do $cargo_build_project
                }
            }

            'riscv64gc-unknown-linux-gnu' => {
                sudo apt-get install gcc-riscv64-linux-gnu -y
                $env.CARGO_TARGET_RISCV64GC_UNKNOWN_LINUX_GNU_LINKER = 'riscv64-linux-gnu-gcc'
                do $cargo_build_project
            }

            's390x-unknown-linux-gnu' => {
                sudo apt-get install gcc-s390x-linux-gnu -y
                $env.CARGO_TARGET_S390X_UNKNOWN_LINUX_GNU_LINKER = 's390x-linux-gnu-gcc'
                do $cargo_build_project
            }

            'powerpc64le-unknown-linux-gnu' => {
                sudo apt-get install gcc-powerpc64le-linux-gnu -y
                $env.CARGO_TARGET_POWERPC64LE_UNKNOWN_LINUX_GNU_LINKER = 'powerpc64le-linux-gnu-gcc'
                do $cargo_build_project
            }

            'aarch64-unknown-linux-musl' => {
                if ($os | str ends-with "-arm") {
                    sudo apt-get install musl-tools -y
                    do $cargo_build_project
                } else {
                    aria2c https://github.com/nushell/integrations/releases/download/build-tools/aarch64-linux-musl-cross.tgz
                    tar -xf aarch64-linux-musl-cross.tgz -C $env.HOME
                    $env.PATH = ($env.PATH | split row (char esep) | prepend $"($env.HOME)/aarch64-linux-musl-cross/bin")
                    $env.CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER = 'aarch64-linux-musl-gcc'
                    do $cargo_build_project
                }
            }

            'loongarch64-unknown-linux-gnu' => {
                aria2c https://github.com/loongson/build-tools/releases/download/2025.08.08/x86_64-cross-tools-loongarch64-binutils_2.45-gcc_15.1.0-glibc_2.42.tar.xz
                tar xf x86_64-cross-tools-loongarch64-*.tar.xz
                $env.PATH = ($env.PATH | split row (char esep) | prepend $"($env.PWD)/cross-tools/bin")
                $env.CARGO_TARGET_LOONGARCH64_UNKNOWN_LINUX_GNU_LINKER = 'loongarch64-unknown-linux-gnu-gcc'
                $env.RUSTFLAGS = "-C target-feature=+crt-static"
                do $cargo_build_project
            }

            'loongarch64-unknown-linux-musl' => {
                aria2c https://github.com/LoongsonLab/oscomp-toolchains-for-oskernel/releases/download/loongarch64-linux-musl-cross-novec/loongarch64-linux-musl-cross-novec.tgz
                tar -xf loongarch64-linux-musl-cross-novec.tgz
                $env.PATH = ($env.PATH | split row (char esep) | prepend $'($env.PWD)/loongarch64-linux-musl-cross-novec/bin')
                $env.CARGO_TARGET_LOONGARCH64_UNKNOWN_LINUX_MUSL_LINKER = "loongarch64-linux-musl-gcc"
                # Workaround for Rust 1.87 TLS issues: abort strategy to bypass TLS-dependent panic handling
                $env.RUSTFLAGS = "-C panic=abort -C target-feature=+crt-static"
                do $cargo_build_project
            }

            'armv7-unknown-linux-gnueabihf' => {
                sudo apt-get install pkg-config gcc-arm-linux-gnueabihf -y
                $env.CARGO_TARGET_ARMV7_UNKNOWN_LINUX_GNUEABIHF_LINKER = 'arm-linux-gnueabihf-gcc'
                do $cargo_build_project
            }

            'armv7-unknown-linux-musleabihf' => {
                aria2c https://github.com/nushell/integrations/releases/download/build-tools/armv7r-linux-musleabihf-cross.tgz
                tar -xf armv7r-linux-musleabihf-cross.tgz -C $env.HOME
                $env.PATH = ($env.PATH | split row (char esep) | prepend $'($env.HOME)/armv7r-linux-musleabihf-cross/bin')
                $env.CARGO_TARGET_ARMV7_UNKNOWN_LINUX_MUSLEABIHF_LINKER = 'armv7r-linux-musleabihf-gcc'
                do $cargo_build_project
            }

            'i686-unknown-linux-gnu' => {
                sudo apt-get install gcc-multilib -y
                do $cargo_build_project
            }

            _ => {
                if $USE_UBUNTU { sudo apt install musl-tools -y }
                do $cargo_build_project
            }
        }
    }

    # --- Windows Build ---
    if $os =~ 'windows' {
        match $target {
            'x86_64-pc-windows-gnu' => {
                print "Downloading MinGW toolchain..."
                curl.exe -L -o mingw.7z "https://github.com/niXman/mingw-builds-binaries/releases/download/15.2.0-rt_v13-rev1/x86_64-15.2.0-release-posix-seh-ucrt-rt_v13-rev1.7z"
                print "Extracting MinGW toolchain..."
                7z x mingw.7z -y -omingw | ignore
                $env.PATH = ($env.PATH | split row (char esep) | prepend $"($env.PWD)/mingw/mingw64/bin")
                $env.CARGO_TARGET_X86_64_PC_WINDOWS_GNU_LINKER = 'gcc'
                do $cargo_build_project
            }
            _ => {
                do $cargo_build_project
            }
        }
    }
}
