#!/usr/bin/env nu
# Build a release binary inside an Ubuntu 22.04 Docker container and copy it to dist/.

def main [
    --version: string = ""   # Override version tag (default: read from Cargo.toml)
    --push                   # Create a git tag and push a GitHub release after building
] {
    let ver = if $version != "" {
        $version
    } else {
        open Cargo.toml | get package.version
    }

    let archive = $"vellum-($ver)-x86_64-linux.tar.gz"
    let dist = "dist"

    print $"Building vellum ($ver) ..."

    (docker run --rm 
        -v $"($env.PWD):/src"
        -w /src
        rust:1-bookworm
        cargo build --release)

    mkdir $dist
    tar -czf $"($dist)/($archive)" -C target/release vellum

    print $"Created ($dist)/($archive)"

    if $push {
        git tag $"v($ver)"
        git push origin $"v($ver)"
        gh release create $"v($ver)" $"($dist)/($archive)" --title $"v($ver)" --generate-notes
        print $"Released v($ver) on GitHub."
    }
}
