# .github/workflows/modules/package.nu

use config.nu [ hr-line, format_template ]

export def create-archive [config: record] {
    let os      = ($env.OS? | default "")
    let target  = ($env.TARGET? | default "")
    let bin     = (try { $config.metadata.bin } catch { "" })
    let version = (try { $config.metadata.version } catch { "" })
    let src     = ($env.GITHUB_WORKSPACE? | default $env.PWD)
    let dist    = $"($src)/output"

    let USE_UBUNTU = ($os | str starts-with "ubuntu")

    # --- Packaging Artifacts ---
    let suffix = if $os =~ 'windows' { '.exe' } else { '' }
    let executable_pattern = $"target/($target)/release/($bin)*($suffix)"

    cd $src
    mkdir $dist

    # Clean up build artifacts
    rm -rf ...(glob $"target/($target)/release/*.d")

    print $"(char nl)Copying release files..."
    hr-line

    let files_config = (try { $config.metadata.files } catch { { include: ["README.md", "LICENSE"], exclude: [] } })
    let include_globs = (try { $files_config.include } catch { ["README.md", "LICENSE"] })
    let exclude_globs = (try { $files_config.exclude } catch { [] })

    # Start with the executable(s)
    mut assets = (glob $executable_pattern)

    # Add included files
    for pattern in $include_globs {
        let matches = (glob $pattern)
        $assets = ($assets | append $matches)
    }

    $assets = ($assets | uniq)

    # Remove excluded files
    for pattern in $exclude_globs {
        let excluded = (glob $pattern)
        $assets = ($assets | where {|it| $it not-in $excluded })
    }

    $assets | each {|it| if ($it | path exists) { cp -rv $it $dist } } | flatten

    # --- Create Archive ---
    cd $dist
    print $"(char nl)Creating release archive..."
    hr-line

    let release_name = $"($bin)-($version)-($target)"

    if $os in ['macos-latest'] or $USE_UBUNTU {
        let archive = $"($dist)/($release_name).tar.gz"
        mkdir $release_name
        let files_to_archive = (ls | where name != $release_name and name !~ '\.(deb|rpm|apk)$' | get name)
        $files_to_archive | each {|it| mv $it $release_name }
        tar -czf $archive $release_name
        if ("GITHUB_OUTPUT" in $env) {
            echo $"archive=($archive)(char nl)" o>> $env.GITHUB_OUTPUT
        }
    } else if $os =~ 'windows' {
        let archive = $"($dist)/($release_name).zip"
        # Exclude .msi and .zip files to prevent including the installer or the archive itself
        let files = (glob * | where ($it | path parse | get extension | $in not-in ['msi', 'zip']))
        7z a $archive ...$files
        if ($archive | path exists) and ("GITHUB_OUTPUT" in $env) {
            let normalized_archive = ($archive | str replace --all '\' '/')
            echo $"archive=($normalized_archive)(char nl)" o>> $env.GITHUB_OUTPUT
        }

        # Optional: Windows MSI packaging
        let msi_enabled = (try { $config.msi.enable } catch { false })
        let tpl_wxs = $"($src)/.github/workflows/templates/main.template.wxs"
        let tpl_wixproj = $"($src)/.github/workflows/templates/build.template.wixproj"
        let tpl_wxl = $"($src)/.github/workflows/templates/main.template.wxl"

        if $msi_enabled and ($tpl_wxs | path exists) and ($tpl_wixproj | path exists) {
            let can_build_msi = [dotnet wix] | all { (which $in | length) > 0 }
            if $can_build_msi and (try { wix --version | split row . | first | into int } catch { 0 }) >= 6 {
                print $"(char nl)Building MSI package..."
                let wix_dir = $"($src)/wix"
                if not ($wix_dir | path exists) { mkdir $wix_dir }

                let maintainer = (try { $config.metadata.maintainer } catch { "Maintainer" })
                let wxs_content = (open --raw $tpl_wxs | str replace --all "{{maintainer}}" $maintainer)
                $wxs_content | save --force $"($wix_dir)/main.wxs"
                cp $tpl_wixproj $"($wix_dir)/build.wixproj"
                if ($tpl_wxl | path exists) { cp $tpl_wxl $"($wix_dir)/main.wxl" }

                cd $src; cd wix; mkdir $bin
                # Copy only base assets to the target folder, excluding any existing archives or installers
                ls $dist | where type == file | where ($it.name | path parse | get extension | $in not-in ['msi', 'zip']) | each {|it| cp -r $it.name $"($bin)/" }

                # Generate LICENSE.rtf for WiX UI (requires RTF format)
                let license_file = $"($bin)/LICENSE"
                if ($license_file | path exists) {
                    let license_text = (open --raw $license_file | str replace --all "\n" "\\line ")
                    let rtf_content = $"{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0\\fnil\\fcharset0 Arial;}}\\viewkind4\\uc1\\pard\\lang1033\\f0\\fs22 ($license_text)\\par}"
                    $rtf_content | save --force $"($bin)/LICENSE.rtf"
                }

                # Calculate WiX architecture
                let arch = match $target {
                    'x86_64-pc-windows-msvc' | 'x86_64-pc-windows-gnu'  => 'x64'
                    'i686-pc-windows-msvc' | 'i686-pc-windows-gnu'      => 'x86'
                    'aarch64-pc-windows-msvc' | 'aarch64-pc-windows-gnu' => 'arm64'
                    _ => 'x64'
                }

                let _hash = ($bin | hash md5)
                let upgrade_code = $"($_hash | str substring 0..7)-($_hash | str substring 8..11)-($_hash | str substring 12..15)-($_hash | str substring 16..19)-($_hash | str substring 20..31)"

                # Fix execution of dotnet build avoiding dummy executable copy-paste
                with-env { PROJECT_NAME: $bin, PROJECT_VERSION: $version, UPGRADE_CODE: $upgrade_code } {
                    dotnet build -c Release $"-p:Platform=($arch)"
                }

                let wix_msi   = (glob **/*.msi | where $it =~ $bin | get 0)
                let final_msi = $"($dist)/($release_name).msi"
                mv $wix_msi $final_msi
                if ("GITHUB_OUTPUT" in $env) {
                    echo $"msi=($final_msi | str replace --all '\' '/')(char nl)" o>> $env.GITHUB_OUTPUT
                }
            }
        }

        # NuGet packaging
        let nuget_enabled = (try { $config.nuget.enable } catch { false })
        if $nuget_enabled and $target == "x86_64-pc-windows-msvc" {
            let n_template = $"($src)/.github/workflows/templates/Nuspec.template.xml"
            if ($n_template | path exists) {
                print $"(char nl)Building NuGet package..."
                let authors = (try { $config.nuget.authors } catch { (try { $config.metadata.maintainer } catch { "Maintainer" }) })
                let description = (try { $config.metadata.description } catch { "" })
                let repo = (try { $config.metadata.repository } catch { "" })

                let n_content = (open --raw $n_template
                    | str replace --all "{{bin}}" $bin
                    | str replace --all "{{version}}" $version
                    | str replace --all "{{authors}}" $authors
                    | str replace --all "{{description}}" $description
                    | str replace --all "{{repository}}" $repo)
                
                let nuspec_file = $"($dist)/($bin).nuspec"
                $n_content | save --force $nuspec_file
                
                try {
                    ^nuget pack $nuspec_file -OutputDirectory $dist
                    print $"Created NuGet package in ($dist)"
                } catch {
                    print "Failed to create NuGet package. Is nuget.exe available?"
                }
            } else {
                print $"Warning: ($n_template) not found. Skipping NuGet package."
            }
        }
    }
}

export def generate-installers [config: record, dist: string] {
    let installer_enabled = (try { $config.installer.enable } catch { false })
    if $installer_enabled {
        print $"(char nl)[Installer] Generating installer scripts..."
        hr-line
        let bin = (try { $config.metadata.bin } catch { "" })
        let version = (try { $config.metadata.version } catch { "" })
        let repo = (try { $config.metadata.repository } catch { "" })
        let features = (try { $config.installer.features } catch { [] })
        let p_linux = (try { $config.installer.path } catch { "~/.local/bin" })
        let p_win = (try { $config.installer.path-win } catch { "C:/bin" })

        let target_keys = (try { $config.targets | columns } catch { [] })
        let targets_context = ($target_keys | reduce -f {} {|it, acc| $acc | insert $"target.($it)" (try { $config.targets | get $it } catch { false }) })

        if "sh" in $features {
            let tpl_sh = ".github/workflows/templates/installer.template.sh"
            if ($tpl_sh | path exists) {
                let content = (format_template $tpl_sh $targets_context | str replace --all "{{bin}}" $bin | str replace --all "{{version}}" $version | str replace --all "{{repository}}" $repo | str replace --all "{{path}}" $p_linux)
                $content | save --force $"($dist)/install.sh"
                print $"Generated ($dist)/install.sh"
            } else {
                print $"Warning: ($tpl_sh) not found."
            }
        }

        if "ps1" in $features {
            let tpl_ps1 = ".github/workflows/templates/installer.template.ps1"
            if ($tpl_ps1 | path exists) {
                let content = (format_template $tpl_ps1 $targets_context | str replace --all "{{bin}}" $bin | str replace --all "{{version}}" $version | str replace --all "{{repository}}" $repo | str replace --all "{{path-win}}" $p_win)
                $content | save --force $"($dist)/install.ps1"
                print $"Generated ($dist)/install.ps1"
            } else {
                print $"Warning: ($tpl_ps1) not found."
            }
        }
    }
}

export def generate-nfpm [config: record, dist: string] {
    let bin_name = (try { $config.metadata.bin } catch { "" })
    let bin_version = (try { $config.metadata.version } catch { "" })
    let use_nfpm = (try { $config.nfpm.enable } catch { false })
    let root_dir = ($env.GITHUB_WORKSPACE? | default $env.PWD)

    if $use_nfpm and (which nfpm | is-not-empty) {
        print $"(char nl)[nFPM] Building Linux packages from artifacts..."
        hr-line
        
        let tpl_nfpm = ".github/workflows/templates/nfpm.template.yaml"
        if not ($tpl_nfpm | path exists) {
            print $"::error::($tpl_nfpm) not found. Skipping nFPM packaging."
        } else {
            let maintainer = (try { $config.metadata.maintainer } catch { "Maintainer" })
            let description = (try { $config.metadata.description } catch { "" })
            let vendor = (try { $config.metadata.vendor } catch { "CodeTease" })
            let homepage = (try { $config.metadata.homepage } catch { "" })
            let license = (try { $config.metadata.license } catch { "MIT" })
            let repo = (try { $config.metadata.repository } catch { "" })

            let n_context = {
                "nfpm.enable": $use_nfpm,
            }

            let n_content = (format_template $tpl_nfpm $n_context
                | str replace --all "{{bin}}" $bin_name
                | str replace --all "{{maintainer}}" $maintainer
                | str replace --all "{{description}}" $description
                | str replace --all "{{vendor}}" $vendor
                | str replace --all "{{homepage}}" $homepage
                | str replace --all "{{repository}}" $repo
                | str replace --all "{{license}}" $license)
            
            $n_content | save --force $"($root_dir)/nfpm.yaml"
            print $"Generated ($root_dir)/nfpm.yaml"

            let archives = (glob $"($dist)/*.tar.gz" | where {|f| ($f | path basename) =~ "linux" })
            
            for archive in $archives {
                let base = ($archive | path basename | str replace ".tar.gz" "")
                let target_str = ($base | str replace $"($bin_name)-($bin_version)-" "")
                
                let nfpm_arch = match $target_str {
                    'x86_64-unknown-linux-gnu' | 'x86_64-unknown-linux-musl' => 'amd64'
                    'i686-unknown-linux-gnu' => '386'
                    'aarch64-unknown-linux-gnu' | 'aarch64-unknown-linux-musl' => 'arm64'
                    'armv7-unknown-linux-gnueabihf' | 'armv7-unknown-linux-musleabihf' => 'arm7'
                    's390x-unknown-linux-gnu' => 's390x'
                    'powerpc64le-unknown-linux-gnu' => 'ppc64le'
                    _ => ''
                }
                
                if $nfpm_arch != "" {
                    print $"[nFPM] Processing ($target_str) for ($nfpm_arch)..."
                    let tmp_dir = $"($dist)/tmp_($target_str)"
                    mkdir $tmp_dir
                    
                    # Extract the tar.gz exactly into it
                    tar -xzf $archive -C $tmp_dir
                    
                    let bin_path = $"($tmp_dir)/($base)/($bin_name)"
                    
                    if ($bin_path | path exists) {
                        cp -v $bin_path $"($root_dir)/($bin_name)"
                        
                        with-env { ARCH: $nfpm_arch, VERSION: $bin_version } {
                            cd $root_dir
                            
                            let is_musl = ($target_str | str contains "musl")
                            let packagers = if $is_musl {
                                ["apk"]
                            } else {
                                ["deb", "rpm"]
                            }
                            
                            $packagers | each {|packager|
                                let pkg_file = $"($dist)/($bin_name)-($bin_version)-($target_str).($packager)"
                                print $"  -> Packaging ($packager)..."
                                nfpm pkg --packager $packager --target $pkg_file
                            }
                        }
                    }
                    
                    rm -rf $tmp_dir
                    rm -f $"($root_dir)/($bin_name)"
                }
            }
            rm -f $"($root_dir)/nfpm.yaml"
        }
    }
}

export def generate-pkgbuild [config: record, dist: string] {
    let arch_enabled = (try { $config.archlinux.enable } catch { false })
    let p_template = ".github/workflows/templates/PKGBUILD.template"
    if $arch_enabled and ($p_template | path exists) {
        print $"(char nl)[Arch Linux] Generating PKGBUILD and .SRCINFO..."
        hr-line

        let bin_name = (try { $config.metadata.bin } catch { "" })
        let bin_version = (try { $config.metadata.version } catch { "" })
        let repo = (try { $config.metadata.repository } catch { "" })
        let maintainer = (try { $config.metadata.maintainer } catch { "Maintainer" })
        let description = (try { $config.metadata.description } catch { "" })
        let license = (try { $config.metadata.license } catch { "MIT" })

        # Calculate SHA256 of the linux archives
        let x86_archive = $"($dist)/($bin_name)-($bin_version)-x86_64-unknown-linux-gnu.tar.gz"
        let arch_archive = $"($dist)/($bin_name)-($bin_version)-aarch64-unknown-linux-gnu.tar.gz"

        let sha256_x86 = if ($x86_archive | path exists) {
            try { ^sha256sum $x86_archive | split row ' ' | first } catch { "SKIP" }
        } else { "SKIP" }
        
        let sha256_aarch64 = if ($arch_archive | path exists) {
            try { ^sha256sum $arch_archive | split row ' ' | first } catch { "SKIP" }
        } else { "SKIP" }

        let has_x86_64 = (try { $config.targets | get "x86_64-unknown-linux-gnu" } catch { false })
        let has_aarch64 = (try { $config.targets | get "aarch64-unknown-linux-gnu" } catch { false })
        let p_context = {
            "arch.x86_64": $has_x86_64,
            "arch.aarch64": $has_aarch64
        }
        let p_content = (format_template $p_template $p_context 
            | str replace --all "{{bin}}" $bin_name 
            | str replace --all "{{version}}" $bin_version 
            | str replace --all "{{repository}}" $repo
            | str replace --all "{{maintainer}}" $maintainer
            | str replace --all "{{description}}" $description
            | str replace --all "{{license}}" $license
            | str replace --all "{{sha256_x86_64}}" $sha256_x86
            | str replace --all "{{sha256_aarch64}}" $sha256_aarch64)
        
        $p_content | save --force $"($dist)/PKGBUILD"
        print $"Generated ($dist)/PKGBUILD"

        # Generate .SRCINFO
        print "Generating .SRCINFO via Docker..."
        try {
            ^docker run --rm -v $"($dist):/pkg" archlinux /bin/bash -c "HOST_UID=$(stat -c %u /pkg/PKGBUILD); HOST_GID=$(stat -c %g /pkg/PKGBUILD); useradd -m build && pacman -Sy --noconfirm base-devel sudo git && cp /pkg/PKGBUILD /home/build/ && chown -R build:build /home/build && cd /home/build && sudo -u build makepkg --printsrcinfo > .SRCINFO && cp .SRCINFO /pkg/ && chown $HOST_UID:$HOST_GID /pkg/.SRCINFO"
            print $"Generated ($dist)/.SRCINFO"

            let arch_pkg = $"($bin_name)-($bin_version)-archlinux-pkgbuild.tar.gz"
            tar -czf $"($dist)/($arch_pkg)" -C $dist PKGBUILD .SRCINFO
            print $"Generated ($dist)/($arch_pkg)"
        } catch {
            print "Failed to generate .SRCINFO via docker"
        }
    }
}

export def generate-formula [config: record, tag_name: string, dist: string] {
    let brew_enabled = (try { $config.brew.enable } catch { false })
    let f_template = ".github/workflows/templates/Formula.template.rb"
    if $brew_enabled and ($f_template | path exists) {
        print $"(char nl)[Homebrew] Generating Formula..."
        hr-line

        let bin_name = (try { $config.metadata.bin } catch { "" })
        let class_name = ($bin_name | split row '-' | each { |it| $it | str capitalize } | str join '')
        let bin_version = (try { $config.metadata.version } catch { "" })
        let repo = (try { $config.metadata.repository } catch { "" })
        let homepage = (try { $config.metadata.homepage } catch { "" })
        let description = (try { $config.metadata.description } catch { "" })
        let license = (try { $config.metadata.license } catch { "MIT" })

        let url_mac_amd   = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-x86_64-apple-darwin.tar.gz"
        let url_mac_arm   = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-aarch64-apple-darwin.tar.gz"
        let url_linux_amd = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-x86_64-unknown-linux-gnu.tar.gz"
        let url_linux_arm = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-aarch64-unknown-linux-gnu.tar.gz"

        let f_mac_amd = (try { glob $"($dist)/*x86_64*apple-darwin*.tar.gz" | first } catch { "" })
        let hash_mac_amd = if $f_mac_amd != "" { try { ^sha256sum $f_mac_amd | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_mac_arm = (try { glob $"($dist)/*aarch64*apple-darwin*.tar.gz" | first } catch { "" })
        let hash_mac_arm = if $f_mac_arm != "" { try { ^sha256sum $f_mac_arm | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_linux_amd = (try { glob $"($dist)/*x86_64*unknown-linux-gnu*.tar.gz" | first } catch { "" })
        let hash_linux_amd = if $f_linux_amd != "" { try { ^sha256sum $f_linux_amd | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_linux_arm = (try { glob $"($dist)/*aarch64*unknown-linux-gnu*.tar.gz" | first } catch { "" })
        let hash_linux_arm = if $f_linux_arm != "" { try { ^sha256sum $f_linux_arm | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_content = (open --raw $f_template
            | str replace --all "{{class_name}}" $class_name
            | str replace --all "{{description}}" $description
            | str replace --all "{{homepage}}" $homepage
            | str replace --all "{{version}}" $bin_version
            | str replace --all "{{license}}" $license
            | str replace --all "{{bin}}" $bin_name
            | str replace --all "{{url_mac_amd}}" $url_mac_amd
            | str replace --all "{{sha256_mac_amd}}" $hash_mac_amd
            | str replace --all "{{url_mac_arm}}" $url_mac_arm
            | str replace --all "{{sha256_mac_arm}}" $hash_mac_arm
            | str replace --all "{{url_linux_amd}}" $url_linux_amd
            | str replace --all "{{sha256_linux_amd}}" $hash_linux_amd
            | str replace --all "{{url_linux_arm}}" $url_linux_arm
            | str replace --all "{{sha256_linux_arm}}" $hash_linux_arm)
        
        $f_content | save --force $"($dist)/($bin_name).rb"
        print $"Generated ($dist)/($bin_name).rb"
    }
}

export def generate-scoop [config: record, tag_name: string, dist: string] {
    let scoop_enabled = (try { $config.scoop.enable } catch { false })
    let s_template = ".github/workflows/templates/Scoop.template.json"
    if $scoop_enabled and ($s_template | path exists) {
        print $"(char nl)[Scoop] Generating Manifest..."
        hr-line

        let bin_name = (try { $config.metadata.bin } catch { "" })
        let bin_version = (try { $config.metadata.version } catch { "" })
        let repo = (try { $config.metadata.repository } catch { "" })
        let homepage = (try { $config.metadata.homepage } catch { "" })
        let description = (try { $config.metadata.description } catch { "" })
        let license = (try { $config.metadata.license } catch { "MIT" })

        let url_win_x64   = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-x86_64-pc-windows-msvc.zip"
        let url_win_x86   = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-i686-pc-windows-msvc.zip"
        let url_win_arm64 = $"($repo)/releases/download/($tag_name)/($bin_name)-($bin_version)-aarch64-pc-windows-msvc.zip"

        let f_win_x64 = (try { glob $"($dist)/*x86_64-pc-windows-msvc*.zip" | first } catch { "" })
        let hash_win_x64 = if $f_win_x64 != "" { try { ^sha256sum $f_win_x64 | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_win_x86 = (try { glob $"($dist)/*i686-pc-windows-msvc*.zip" | first } catch { "" })
        let hash_win_x86 = if $f_win_x86 != "" { try { ^sha256sum $f_win_x86 | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let f_win_arm64 = (try { glob $"($dist)/*aarch64-pc-windows-msvc*.zip" | first } catch { "" })
        let hash_win_arm64 = if $f_win_arm64 != "" { try { ^sha256sum $f_win_arm64 | split row ' ' | first } catch { "SKIP" } } else { "SKIP" }

        let s_content = (open --raw $s_template
            | str replace --all "{{description}}" $description
            | str replace --all "{{homepage}}" $homepage
            | str replace --all "{{version}}" $bin_version
            | str replace --all "{{license}}" $license
            | str replace --all "{{bin}}" $bin_name
            | str replace --all "{{repository}}" $repo
            | str replace --all "{{url_win_x64}}" $url_win_x64
            | str replace --all "{{sha256_win_x64}}" $hash_win_x64
            | str replace --all "{{url_win_x86}}" $url_win_x86
            | str replace --all "{{sha256_win_x86}}" $hash_win_x86
            | str replace --all "{{url_win_arm64}}" $url_win_arm64
            | str replace --all "{{sha256_win_arm64}}" $hash_win_arm64)
        
        $s_content | save --force $"($dist)/($bin_name).json"
        print $"Generated ($dist)/($bin_name).json"
    }
}

export def generate-registry-docs [config: record, dist: string, is_tag: bool] {
    let cloudsmith_enabled = (try { $config.cloudsmith.enable } catch { false })
    let docs_path = (try { $config.cloudsmith.docs_path } catch { "REGISTRY.md" })
    let docs_file = ($docs_path | path basename)
    let r_template = ".github/workflows/templates/Registry.template.md"
    let root_dir = ($env.GITHUB_WORKSPACE? | default $env.PWD)

    if $cloudsmith_enabled and $docs_path != "" and ($r_template | path exists) {
        print $"(char nl)[Registry] Generating Registry instructions..."
        hr-line

        let bin_name = (try { $config.metadata.bin } catch { "" })
        let bin_version = (try { $config.metadata.version } catch { "" })
        let repo_path = (try { $config.cloudsmith.repo } catch { "" })
        let repo_url = (try { $config.metadata.repository } catch { "" })
        let github_org = (try { $config.docker.github_org } catch {
            ($env.GITHUB_REPOSITORY? | default "codetease/cli-dummy" | split row "/" | first | str downcase)
        })

        let has_docker = (try { $config.docker.enable } catch { false })
        let registries = (try { $config.docker.registries } catch { [] })
        let has_ghcr = "ghcr" in $registries
        let has_cloudsmith = "cloudsmith" in $registries

        let brew_tap = (try { $config.brew.tap } catch { "" })
        let scoop_bucket = (try { $config.scoop.bucket } catch { "" })
        let scoop_bucket_name = if ($scoop_bucket | is-empty) { "" } else { $scoop_bucket | split row "/" | last }

        let cloudsmith_targets = (try { $config.cloudsmith.targets } catch { {} })
        
        let deb_target = (try { $cloudsmith_targets.deb } catch { "ubuntu/any-version" })
        let deb_parts = ($deb_target | split row "/")
        let apt_distro = (try { $deb_parts | first } catch { "ubuntu" })
        let apt_codename = (try { $deb_parts | last } catch { "any-version" })

        let rpm_target = (try { $cloudsmith_targets.rpm } catch { "el/any-version" })
        let rpm_parts = ($rpm_target | split row "/")
        let rpm_distro = (try { $rpm_parts | first } catch { "el" })
        let rpm_codename = (try { $rpm_parts | last } catch { "any-version" })

        let apk_target = (try { $cloudsmith_targets.apk } catch { "alpine/any-version" })
        let apk_parts = ($apk_target | split row "/")
        let apk_distro = (try { $apk_parts | first } catch { "alpine" })
        let apk_codename = (try { $apk_parts | last } catch { "any-version" })

        let context = {
            "cloudsmith.enable": $cloudsmith_enabled,
            "docker.enable": $has_docker,
            "ghcr.enable": ($has_docker and $has_ghcr),
            "docker.cloudsmith.enable": ($has_docker and $has_cloudsmith),
            "ghcr_only": ($has_docker and $has_ghcr and not $has_cloudsmith),
            "cloudsmith_only": ($has_docker and $has_cloudsmith and not $has_ghcr),
            "ghcr_and_cloudsmith": ($has_docker and $has_ghcr and $has_cloudsmith),
            "nuget.enable": (try { $config.nuget.enable } catch { false }),
            "archlinux.enable": (try { $config.archlinux.enable } catch { false }),
            "scoop.enable": (try { $config.scoop.enable } catch { false }),
            "brew.enable": (try { $config.brew.enable } catch { false }),
            "crate.enable": (try { $config.crate.enable } catch { false }),
            "crate.crates_io": (try { $config.crate.enable and ("crates.io" in $config.crate.registries) } catch { false }),
            "crate.cloudsmith": (try { $config.crate.enable and ("cloudsmith" in $config.crate.registries) } catch { false })
        }

        let r_content = (format_template $r_template $context
            | str replace --all "{{bin}}" $bin_name
            | str replace --all "{{version}}" $bin_version
            | str replace --all "{{repo_path}}" $repo_path
            | str replace --all "{{repository}}" $repo_url
            | str replace --all "{{github_org}}" $github_org
            | str replace --all "{{brew_tap}}" $brew_tap
            | str replace --all "{{scoop_bucket}}" $scoop_bucket
            | str replace --all "{{scoop_bucket_name}}" $scoop_bucket_name
            | str replace --all "{{apt_distro}}" $apt_distro
            | str replace --all "{{apt_codename}}" $apt_codename
            | str replace --all "{{rpm_distro}}" $rpm_distro
            | str replace --all "{{rpm_codename}}" $rpm_codename
            | str replace --all "{{apk_distro}}" $apk_distro
            | str replace --all "{{apk_codename}}" $apk_codename)
        
        $r_content | save --force $"($dist)/($docs_file)"
        print $"Generated ($dist)/($docs_file)"

        do {
            let target_docs_path = $"($root_dir)/($docs_path)"
            let target_dir = ($target_docs_path | path dirname)
            if not ($target_dir | path exists) { mkdir $target_dir }
            
            cp -f $"($dist)/($docs_file)" $target_docs_path
            
            print "Checking for changes in registry docs..."
            cd $root_dir
            let actor = "github-actions[bot]"
            let email = "41898282+github-actions[bot]@users.noreply.github.com"
            try {
                git config --local user.name $actor
                git config --local user.email $email
                git add $docs_path
                let status = (git status --porcelain $docs_path)
                if ($status | is-not-empty) {
                    print $"Committing changes for ($docs_path)..."
                    git commit -m $"update registry docs for v($bin_version)"
                    
                    # Fix: detached HEAD push by specifying target branch
                    let target_branch = if $is_tag {
                        # Default to the primary branch for tag-triggered releases
                        (try { ^gh repo view --json defaultBranchRef --template '{{.defaultBranchRef.name}}' } catch { "main" })
                    } else {
                        ($env.GITHUB_REF_NAME? | default ($env.REF? | default "refs/heads/main" | split row "/" | last))
                    }
                    print $"[Git] Pushing changes to ($target_branch)..."
                    git push origin $"HEAD:($target_branch)"
                    print "[Git] Successfully committed and pushed registry docs."
                } else {
                    print "[Git] No changes in registry docs to commit."
                }
            } catch {|err|
                print $"::warning::Failed to commit or push registry docs: ($err.msg)"
            }
        }
    }
}

export def generate-all [config: record] {
    let root_dir = ($env.GITHUB_WORKSPACE? | default $env.PWD)
    let dist = $"($root_dir)/output"
    if not ($dist | path exists) {
        print $"::error::Output directory ($dist) not found. Build is required or artifacts download failed."
        exit 1
    }

    let is_tag = ($env.REF? | default "" | str starts-with "refs/tags/")
    let bin_version = (try { $config.metadata.version } catch { "" })
    let tag_name = if $is_tag { ($env.REF | str replace 'refs/tags/' '') } else { $"v($bin_version)" }

    generate-installers $config $dist
    generate-nfpm $config $dist
    generate-pkgbuild $config $dist
    generate-formula $config $tag_name $dist
    generate-scoop $config $tag_name $dist
    generate-registry-docs $config $dist $is_tag
}
