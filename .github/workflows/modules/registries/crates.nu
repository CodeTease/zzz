# .github/workflows/modules/registries/crates.nu

use ../config.nu [ hr-line ]

export def publish [config: record] {
    let crate_enabled = (try { $config.crate.enable } catch { false })
    let crate_registries = (try { $config.crate.registries } catch { [] })
    let is_tag = ($env.REF? | default "" | str starts-with "refs/tags/")
    let cloudsmith_enabled = (try { $config.cloudsmith.enable } catch { false })
    let repo = (try { $config.cloudsmith.repo } catch { "codetease/tools" })
    let can_publish = $cloudsmith_enabled and ($env.CLOUDSMITH_API_KEY? | is-not-empty) and $is_tag

    if $crate_enabled and $is_tag {
        print $"(char nl)[Crate] Publishing to registries: ($crate_registries | str join ', ')..."
        hr-line
        
        # crates.io
        if "crates.io" in $crate_registries {
            if ($env.CARGO_REGISTRY_TOKEN? | is-not-empty) {
                print "[Crate] Publishing to crates.io..."
                try {
                    cargo publish --token $env.CARGO_REGISTRY_TOKEN
                    print "[Crate] Successfully published to crates.io"
                } catch {
                    print "::warning::[Crate] Failed to publish to crates.io"
                }
            } else {
                print "[Crate] Skipping crates.io publish: CARGO_REGISTRY_TOKEN not found."
            }
        }

        # cloudsmith
        if "cloudsmith" in $crate_registries {
            if $can_publish {
                print "[Crate] Packaging for Cloudsmith..."
                try {
                    cargo package
                    let crate_files = (glob target/package/*.crate)
                    if ($crate_files | is-not-empty) {
                        let crate_file = $crate_files.0
                        print $"[Crate] Pushing ($crate_file) to Cloudsmith ($repo)..."
                        cloudsmith push cargo $repo ($crate_file | path expand) -k $env.CLOUDSMITH_API_KEY
                    } else {
                        print "::warning::[Crate] No .crate file found in target/package."
                    }
                } catch {|err|
                    print $"::warning::[Crate] Failed to package or push to Cloudsmith: ($err.msg)"
                }
            } else {
                print "[Crate] Skipping cloudsmith publish: API Key not found or not a tag push."
            }
        }
    }
}
