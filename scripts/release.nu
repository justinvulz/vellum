#!/usr/bin/env nu
# Vellum release script.
#
# Workflow:
#   1. Sanity-check: running from dev, clean tree, tag doesn't exist,
#      RELEASE_NOTES.md exists.
#   2. Fetch origin and fast-forward main onto dev (--ff-only).
#   3. Read version from Cargo.toml; create an annotated tag on main.
#   4. Build a portable Linux binary inside Docker (Debian Bullseye for
#      old-glibc compatibility) into target-docker/.
#   5. Tarball it as dist/vellum-vX.Y.Z-x86_64-linux.tar.gz.
#   6. Push main + tag and create a GitHub release with the tarball,
#      using RELEASE_NOTES.md as the release body.
#   7. Switch back to dev.
#
# Before running:
#   - Bump `[package].version` in Cargo.toml.
#   - Write the release body to RELEASE_NOTES.md at the repo root.
#
# Usage:
#   nu scripts/release.nu

def main [] {
    let branch = ^git rev-parse --abbrev-ref HEAD | str trim
    if $branch != "dev" {
        error make {msg: $"release must run from dev (currently on ($branch))"}
    }
    let dirty = ^git status --porcelain | str trim
    if $dirty != "" {
        error make {msg: "working tree must be clean"}
    }

    print "→ fetching origin"
    ^git fetch origin

    let version = open Cargo.toml | get package.version
    let tag = $"v($version)"
    print $"→ release tag: ($tag)"

    let existing = ^git tag --list $tag | str trim
    if $existing != "" {
        error make {msg: $"tag ($tag) already exists — bump version in Cargo.toml first"}
    }

    let notes_file = "RELEASE_NOTES.md"
    if not ($notes_file | path exists) {
        error make {msg: $"($notes_file) is missing — write the release body before releasing"}
    }

    try {
        print "→ merging dev → main (fast-forward)"
        ^git checkout main
        ^git merge --ff-only dev

        ^git tag -a $tag -m $"Release ($tag)"

        print "→ building release binary in Docker"
        let dist = "dist"
        mkdir $dist
        let archive_name = $"vellum-($tag)-x86_64-linux.tar.gz"

        # Bullseye = Debian 11 = glibc 2.31. Covers Ubuntu 20.04+,
        # Debian 11+, RHEL 8+, Fedora 35+. Keep build artefacts in a
        # separate target-docker/ so they don't collide with the host
        # (likely Nix-built) ./target.
        let build_script = 'set -euo pipefail
apt-get update -qq
apt-get install -y -qq --no-install-recommends pkg-config libwayland-dev libxkbcommon-dev libfontconfig1-dev libx11-dev libxcb1-dev libxcursor-dev libxrandr-dev libxi-dev
CARGO_TARGET_DIR=/src/target-docker cargo build --release --locked --manifest-path /src/Cargo.toml
'
        (^docker run --rm
            -v $"($env.PWD):/src"
            -w /src
            rust:1-bullseye
            bash -c $build_script)

        ^tar -czf $"($dist)/($archive_name)" -C target-docker/release vellum
        print $"→ archive: ($dist)/($archive_name)"

        print "→ pushing main and tag"
        ^git push origin main
        ^git push origin $tag

        print "→ creating GitHub release"
        (^gh release create $tag $"($dist)/($archive_name)"
            --title $tag
            --notes-file $notes_file)

        ^git checkout dev
        print $"✓ released ($tag)"
    } catch { |e|
        print $"✗ release failed: ($e.msg)"
        ^git checkout dev
        error make {msg: $e.msg}
    }
}
