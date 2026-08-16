# .github/workflows/modules/registries/homebrew.nu

use ../config.nu [ hr-line ]

export def publish-brew [config: record] {
    let brew_enabled = (try { $config.brew.enable } catch { false })
    if not $brew_enabled {
        print "Homebrew publishing is disabled in release.toml"
        return
    }

    let tap_repo = (try { $config.brew.tap } catch { "CodeTease/homebrew-tap" })
    let bin = (try { $config.metadata.bin } catch { "" })
    let token = ($env.BREW_TOKEN? | default "")
    
    if ($token | is-empty) {
        print "Skipping Homebrew publish: BREW_TOKEN secret is not set."
        return
    }
    
    let repo_url = $"https://($token)@github.com/($tap_repo).git"
    let rb_file = $"output/($bin).rb"
    if not ($rb_file | path exists) {
        print $"Error: ($rb_file) missing. Cannot publish to Homebrew."
        return
    }

    print $"(char nl)[Homebrew] Publishing to ($tap_repo)..."
    hr-line

    git clone $repo_url brew-repo
    mkdir "brew-repo/Formula"
    cp $rb_file $"brew-repo/Formula/($bin).rb"
    
    cd brew-repo
    git config user.name "github-actions[bot]"
    git config user.email "github-actions[bot]@users.noreply.github.com"
    git add $"Formula/($bin).rb"
    
    let status = (git status --porcelain)
    if ($status | is-empty) {
        print "No changes to commit for Homebrew Tap."
    } else {
        let ref_name = ($env.GITHUB_REF_NAME? | default "")
        git commit -m $"Update ($bin) to ($ref_name)"
        git pull --rebase
        git push
        print "Successfully published Homebrew Formula."
    }
    cd ..
    rm -rf brew-repo
}

export def publish-scoop [config: record] {
    let scoop_enabled = (try { $config.scoop.enable } catch { false })
    if not $scoop_enabled {
        print "Scoop publishing is disabled in release.toml"
        return
    }

    let bucket = (try { $config.scoop.bucket } catch { "CodeTease/scoop-bucket" })
    let bin = (try { $config.metadata.bin } catch { "" })
    let token = ($env.SCOOP_TOKEN? | default "")
    
    if ($token | is-empty) {
        print "Skipping Scoop publish: SCOOP_TOKEN secret is not set."
        return
    }

    let repo_url = $"https://($token)@github.com/($bucket).git"
    let json_file = $"output/($bin).json"
    if not ($json_file | path exists) {
        print $"Error: ($json_file) missing. Cannot publish to Scoop."
        return
    }

    print $"(char nl)[Scoop] Publishing to ($bucket)..."
    hr-line

    git clone $repo_url scoop-repo
    mkdir "scoop-repo/bucket"
    cp $json_file $"scoop-repo/bucket/($bin).json"
    
    cd scoop-repo
    git config user.name "github-actions[bot]"
    git config user.email "github-actions[bot]@users.noreply.github.com"
    git add $"bucket/($bin).json"
    
    let status = (git status --porcelain)
    if ($status | is-empty) {
        print "No changes to commit for Scoop Bucket."
    } else {
        let ref_name = ($env.GITHUB_REF_NAME? | default "")
        git commit -m $"Update ($bin) to ($ref_name)"
        git pull --rebase
        git push
        print "Successfully published Scoop Manifest."
    }
    cd ..
    rm -rf scoop-repo
}

export def publish [config: record] {
    publish-brew $config
    publish-scoop $config
}
