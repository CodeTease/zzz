# .github/workflows/modules/registries/github.nu

use ../config.nu [ hr-line ]

export def publish [config: record] {
    let root_dir = ($env.GITHUB_WORKSPACE? | default $env.PWD)
    let dist = $"($root_dir)/output"
    let is_tag = ($env.REF? | default "" | str starts-with "refs/tags/")
    let bin_version = (try { $config.metadata.version } catch { "" })
    let tag_name = if $is_tag { ($env.REF | str replace 'refs/tags/' '') } else { $"v($bin_version)" }
    let is_prerelease = ($tag_name | str contains -i "beta") or ($tag_name | str contains -i "rc")
    let clean_version = ($tag_name | str replace --regex '^v' '')
    let docs_path = (try { $config.cloudsmith.docs_path } catch { "REGISTRY.md" })
    let docs_file = ($docs_path | path basename)
    let installer_enabled = (try { $config.installer.enable } catch { false })

    # Docker Build and Push
    let docker_enabled = (try { $config.docker.enable } catch { false })
    let registries = (try { $config.docker.registries } catch { [] })
    let templates = (try { $config.docker.templates } catch { [] })
    mut docker_release_notes = []

    if $docker_enabled and ($templates | is-not-empty) and ($registries | is-not-empty) {
        print $"(char nl)[Docker] Building and Pushing Images..."
        hr-line

        let bin_name = (try { $config.metadata.bin } catch { "" })
        let image_name = (try { $config.docker.image_name } catch { $bin_name })

        let has_ghcr = "ghcr" in $registries
        let has_cloudsmith = "cloudsmith" in $registries

        if $has_ghcr {
            if ($env.GITHUB_TOKEN? | is-not-empty) and ($env.GITHUB_ACTOR? | is-not-empty) {
                print "Logging into ghcr.io..."
                $env.GITHUB_TOKEN | docker login ghcr.io -u $env.GITHUB_ACTOR --password-stdin
            } else {
                print "Warning: GITHUB_TOKEN or GITHUB_ACTOR is missing. GHCR login skipped."
            }
        }

        if $has_cloudsmith {
            if ($env.CLOUDSMITH_API_KEY? | is-not-empty) {
                let cloudsmith_docker_username = ($config.docker.cloudsmith_docker_username | default "")
                print "Logging into docker.cloudsmith.io..."
                $env.CLOUDSMITH_API_KEY | docker login docker.cloudsmith.io -u $cloudsmith_docker_username --password-stdin
            } else {
                 print "Warning: CLOUDSMITH_API_KEY is missing. Cloudsmith login skipped."
            }
        }

        $docker_release_notes = ($docker_release_notes | append "### 🐳 Docker Images")
        $docker_release_notes = ($docker_release_notes | append "")
        $docker_release_notes = ($docker_release_notes | append "Multi-architecture Docker images are available in the following registries:")
        $docker_release_notes = ($docker_release_notes | append "")

        for tpl in $templates {
            print $"Preparing context for ($tpl)..."
            let target_dir = $"($dist)/docker_build_($tpl)"
            mkdir $"($target_dir)/amd64"
            mkdir $"($target_dir)/arm64"
            
            let is_alpine = $tpl == "alpine"
            let suffix = if $is_alpine { "musl" } else { "gnu" }
            
            let linux_amd64_tar = $"($dist)/($bin_name)-($bin_version)-x86_64-unknown-linux-($suffix).tar.gz"
            let linux_arm64_tar = $"($dist)/($bin_name)-($bin_version)-aarch64-unknown-linux-($suffix).tar.gz"
            
            mut available_platforms = []
            
            if ($linux_amd64_tar | path exists) {
                tar -xzf $linux_amd64_tar -C $"($target_dir)/amd64"
                let extracted_dir = $"($bin_name)-($bin_version)-x86_64-unknown-linux-($suffix)"
                mv $"($target_dir)/amd64/($extracted_dir)/($bin_name)" $"($target_dir)/amd64/($bin_name)"
                $available_platforms = ($available_platforms | append "linux/amd64")
            }
            if ($linux_arm64_tar | path exists) {
                tar -xzf $linux_arm64_tar -C $"($target_dir)/arm64"
                let extracted_dir = $"($bin_name)-($bin_version)-aarch64-unknown-linux-($suffix)"
                mv $"($target_dir)/arm64/($extracted_dir)/($bin_name)" $"($target_dir)/arm64/($bin_name)"
                $available_platforms = ($available_platforms | append "linux/arm64")
            }
            
            if ($available_platforms | is-empty) {
                print "Warning: No linux archives found to build Docker image."
                continue
            }
            
            let platforms = ($available_platforms | str join ",")
            
            let d_template = $".github/workflows/templates/Dockerfile.($tpl).template"
            let d_content = (open --raw $d_template | str replace --all "{{bin}}" $bin_name | str replace --all "{{version}}" $bin_version)
            let d_file = $"($target_dir)/Dockerfile"
            $d_content | save --force $d_file
            
            mut build_args = ["buildx" "build" "--push" "--platform" $platforms "-f" $d_file "--provenance=false" "--sbom=false"]
            
            $docker_release_notes = ($docker_release_notes | append $"**Variant:** `($tpl)`")
            $docker_release_notes = ($docker_release_notes | append "```bash")

            for reg in $registries {
                let full_image = if $reg == "ghcr" {
                    let repo_owner = ($env.GITHUB_REPOSITORY? | default "codetease/cli-dummy" | split row "/" | first | str downcase)
                    $"ghcr.io/($repo_owner)/($image_name)"
                } else if $reg == "cloudsmith" {
                    let repo_path = (try { $config.cloudsmith.repo } catch { "codetease/tools" })
                    $"docker.cloudsmith.io/($repo_path)/($image_name)"
                } else {
                    $image_name
                }
                
                let variant_tags = if $is_prerelease {
                    match $tpl {
                        "alpine" => [$clean_version, $"($clean_version)-alpine"],
                        "debian-slim" => [$"($clean_version)-bookworm"],
                        _ => [$"($clean_version)-($tpl)"]
                    }
                } else {
                    match $tpl {
                        "alpine" => ["latest", $clean_version, "alpine", $"($clean_version)-alpine"],
                        "debian-slim" => ["latest-bookworm", $"($clean_version)-bookworm"],
                        _ => [$tpl, $"($clean_version)-($tpl)"]
                    }
                }
                
                for t in $variant_tags {
                    $build_args = ($build_args | append ["-t" $"($full_image):($t)"])
                }

                # Use the versioned tag for release notes
                let note_tag = ($variant_tags | where {|it| $it == $clean_version or $it == $"($clean_version)-bookworm" or $it == $"($clean_version)-($tpl)" } | first)
                $docker_release_notes = ($docker_release_notes | append $"docker pull ($full_image):($note_tag)")
            }
            
            $docker_release_notes = ($docker_release_notes | append "```")
            $docker_release_notes = ($docker_release_notes | append "")

            $build_args = ($build_args | append $target_dir)
            
            print $"Running docker ($build_args | str join ' ')"
            try {
                ^docker ...$build_args
                print $"Successfully built and pushed ($tpl) image."
                rm -rf $target_dir
            } catch {
                print $"Error: Docker build/push failed for ($tpl)"
                rm -rf $target_dir
            }
        }
    }

    # GitHub Release
    if $is_tag {
        print $"(char nl)[GitHub] Creating Release & Uploading Assets..."
        hr-line

        let bin = (try { $config.metadata.bin } catch { "" })
        let version = (try { $config.metadata.version } catch { "" })
        let features = (try { $config.installer.features } catch { [] })

        let matrix_str = ($env.MATRIX? | default "[]")
        let target_names = if ($matrix_str == "[]" or ($matrix_str | is-empty)) {
            {}
        } else {
            let matrix_items = ($matrix_str | from json)
            $matrix_items | reduce -f {} {|it, acc| 
                let dname = (try { $it.display_name } catch { $it.target })
                $acc | insert $it.target $dname 
            }
        }

        let assets_for_table = (ls $dist | where type == file | get name | where {|f|
            let base = ($f | path basename)
            not ($base in ['install.sh', 'install.ps1', 'PKGBUILD', '.SRCINFO']) and not ($base | str ends-with ".sha256")
        })

        mut rows = ["| Operating System & Architecture | Format | Checksum (SHA256) |", "|---|---|---|"]
        
        mut has_i686 = false
        mut has_s390x = false

        for file in $assets_for_table {
            let base = ($file | path basename)
            let sha = (try { ^sha256sum $file | split row ' ' | first } catch { "UNKNOWN" })
            
            let ext = if ($base | str ends-with ".tar.gz") {
                ".tar.gz"
            } else if ($base | str ends-with ".zip") {
                ".zip"
            } else if ($base | str ends-with ".msi") {
                ".msi"
            } else if ($base | str ends-with ".deb") {
                ".deb"
            } else if ($base | str ends-with ".rpm") {
                ".rpm"
            } else if ($base | str ends-with ".apk") {
                ".apk"
            } else {
                ""
            }

            if $ext != "" {
                let without_prefix = ($base | str replace $"($bin)-($version)-" "")
                let target_str = ($without_prefix | str replace $ext "")
                
                if ($target_str | str starts-with "i686") { $has_i686 = true }
                if ($target_str | str starts-with "s390x") { $has_s390x = true }

                let os_arch = (try { $target_names | get $target_str } catch { $target_str })
                $rows = ($rows | append $"| ($os_arch) | `($ext)` | `($sha)` |")
            }
        }
        
        mut notes_lines = []
        $notes_lines = ($notes_lines | append "### 📦 Target List")
        $notes_lines = ($notes_lines | append "")
        $notes_lines = ($notes_lines | append $rows)
        $notes_lines = ($notes_lines | append "")

        if ($docker_release_notes | length) > 0 {
            $notes_lines = ($notes_lines | append $docker_release_notes)
        }

        if $installer_enabled {
            let github_repo = ($env.GITHUB_REPOSITORY? | default "OWNER/REPO")
            $notes_lines = ($notes_lines | append "### 🚀 Quick Installer Guide")
            $notes_lines = ($notes_lines | append "")
            
            if "ps1" in $features {
                $notes_lines = ($notes_lines | append "**Windows**:")
                $notes_lines = ($notes_lines | append "Instructions on using the command to execute `install.ps1`. This script automatically handles decompression and checks the CPU architecture (AMD64/ARM64).")
                $notes_lines = ($notes_lines | append "```powershell")
                $notes_lines = ($notes_lines | append $"irm 'https://github.com/($github_repo)/releases/latest/download/install.ps1' | iex")
                $notes_lines = ($notes_lines | append "```")
                $notes_lines = ($notes_lines | append "")
            }
            if "sh" in $features {
                $notes_lines = ($notes_lines | append "**Linux/macOS**:")
                $notes_lines = ($notes_lines | append "Instructions to execute the `install.sh`. This script automatically detects the OS (Linux/Darwin) and architecture to load the correct assets from the repository.")
                $notes_lines = ($notes_lines | append "```bash")
                $notes_lines = ($notes_lines | append $"curl -fsSL 'https://github.com/($github_repo)/releases/latest/download/install.sh' | bash")
                $notes_lines = ($notes_lines | append "```")
                $notes_lines = ($notes_lines | append "")
            }
        }

        if ($dist | path join $docs_file | path exists) {
            let github_repo = ($env.GITHUB_REPOSITORY? | default "OWNER/REPO")
            $notes_lines = ($notes_lines | append $"To install via package managers \(APT, RPM, APK, NuGet\), please download [($docs_file)]\(https://github.com/($github_repo)/releases/download/($tag_name)/($docs_file)\) to view the instructions.")
            $notes_lines = ($notes_lines | append "")
        }

        if $has_i686 or $has_s390x {
            $notes_lines = ($notes_lines | append "### ⚠️ Additional Information")
            $notes_lines = ($notes_lines | append "")
            if $has_i686 {
                $notes_lines = ($notes_lines | append "> **Note on `i686`**: This is an older legacy architecture. Support might be limited or deprecated in the future.")
                $notes_lines = ($notes_lines | append "")
            }
            if $has_s390x {
                $notes_lines = ($notes_lines | append "> **Note on `s390x`**: This is a Big Endian risk architecture. Proceed with caution as some libraries may assume Little Endian.")
                $notes_lines = ($notes_lines | append "")
            }
        }

        let notes_file = $"($dist)/RELEASE_NOTES.md"
        ($notes_lines | str join (char nl)) | save --force $notes_file

        # Check if release exists
        let release_exists = (try { gh release view $tag_name | complete } catch { {exit_code: 1} })
        let prerelease_flag = if $is_prerelease { ["--prerelease"] } else { ["--latest"] }

        if $release_exists.exit_code != 0 {
            gh release create $tag_name --title $"($tag_name)" --notes-file $notes_file ...$prerelease_flag
        } else {
            gh release edit $tag_name --notes-file $notes_file ...$prerelease_flag
        }
        
        # Upload all assets in dist (files only)
        let assets = (ls $dist | where type == file | get name | where {|f| not ($f | path basename | $in in ["RELEASE_NOTES.md", "PKGBUILD", ".SRCINFO"])})
        if ($assets | is-not-empty) {
            gh release upload $tag_name ...$assets --clobber
        } else {
             print "No assets found to upload to GitHub."
        }
    } else {
        print "Not a tag push. Skipping GitHub Release."
    }
}
