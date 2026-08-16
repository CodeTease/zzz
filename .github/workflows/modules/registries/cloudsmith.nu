# .github/workflows/modules/registries/cloudsmith.nu

use ../config.nu [ hr-line ]

export def publish [config: record] {
    let root_dir = ($env.GITHUB_WORKSPACE? | default $env.PWD)
    let dist = $"($root_dir)/output"
    let is_tag = ($env.REF? | default "" | str starts-with "refs/tags/")
    let cloudsmith_enabled = (try { $config.cloudsmith.enable } catch { false })
    let repo = (try { $config.cloudsmith.repo } catch { "codetease/tools" })
    let targets_mapping = (try { $config.cloudsmith.targets } catch { { deb: "ubuntu/noble", rpm: "el/9", apk: "alpine/any-version" } })

    let can_publish = $cloudsmith_enabled and ($env.CLOUDSMITH_API_KEY? | is-not-empty) and $is_tag

    if $can_publish {
        print $"(char nl)[Cloudsmith] Publishing Packages..."
        hr-line
        
        if (which cloudsmith | is-empty) {
            print "::error::Cloudsmith CLI not found."
            exit 1
        }

        let pkgs = (try { ls $dist | where type == file | get name | where { |f| $f =~ '\.(deb|rpm|apk|nupkg)$' } } catch { [] })

        if ($pkgs | is-not-empty) {
            for pkg in $pkgs {
                let ext = ($pkg | path parse | get extension)

                let fallback_path = $"($repo)/any/version"
                let target_path_suffix = (try { $targets_mapping | get $ext } catch { "" })
                
                let target_path = if $ext == "nupkg" {
                    $repo
                } else if ($target_path_suffix | is-empty) {
                    $fallback_path
                } else {
                    $"($repo)/($target_path_suffix)"
                }

                let pkg_type = if $ext == "apk" { "alpine" } else if $ext == "nupkg" { "nuget" } else { $ext }
                
                print $"[Cloudsmith] Pushing ($ext) to ($target_path)..."
                cloudsmith push $pkg_type $target_path ($pkg | path expand) -k $env.CLOUDSMITH_API_KEY
            }
        } else {
            print "[Cloudsmith] Skipping publish: No linux packages found in output directory."
        }
    } else {
        print "[Cloudsmith] Skipping publish: Conditions not met (disabled, missing API key, or not a tag)."
    }
}
