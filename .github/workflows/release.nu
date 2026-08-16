#!/usr/bin/env nu

# Description:
# A generalized script for cross-compiling and packaging Rust projects.
# Originally from NuShell repo, but this script optimized for
# general-purpose and more features.

use modules/config.nu *
use modules/target.nu
use modules/package.nu
use modules/registries

def main [command?: string] {
    let cmd = ($command | default "build")
    match $cmd {
        "build" => { run_build }
        "publish" => { run_publish }
        _ => { print $"::error::Unknown command: ($cmd)"; exit 1 }
    }
}

def run_build [] {
    let config = (load-config)
    target build $config
    package create-archive $config
}

def run_publish [] {
    let config = (load-config)

    # 0. Generate packaging metadata and installer scripts
    package generate-all $config

    # 1. GitHub Release & Docker Images
    registries github publish $config

    # 2. Cloudsmith API Push
    registries cloudsmith publish $config

    # 3. Crate Publish (crates.io & Cloudsmith Cargo)
    registries crates publish $config

    # 4. Homebrew Tap & Scoop Bucket Publish
    registries homebrew publish $config
}
