#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
project_dir=$(cd -- "$script_dir/.." && pwd)
tag=${1:-${CI_COMMIT_TAG:-}}

if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    printf 'error: release tag must use the vMAJOR.MINOR.PATCH format; got %q\n' "$tag" >&2
    exit 1
fi

package_version() {
    awk '
        $0 == "[package]" {
            in_package = 1
            next
        }
        in_package && /^\[/ {
            exit
        }
        in_package && /^[[:space:]]*version[[:space:]]*=/ {
            version = $0
            sub(/^[^=]*=[[:space:]]*"/, "", version)
            sub(/".*$/, "", version)
            print version
            exit
        }
    ' "$1"
}

tag_version=${tag#v}
figen_version=$(package_version "$project_dir/Cargo.toml")
proc_macro_version=$(package_version "$project_dir/proc-macro/Cargo.toml")
dependency_version=$(
    sed -nE 's/^[[:space:]]*figen-proc-macros[[:space:]]*=.*version[[:space:]]*=[[:space:]]*"=([^"]+)".*/\1/p' \
        "$project_dir/Cargo.toml"
)

check_version() {
    local label=$1
    local actual=$2

    if [[ -z "$actual" ]]; then
        printf 'error: could not read %s version\n' "$label" >&2
        exit 1
    fi

    if [[ "$actual" != "$tag_version" ]]; then
        printf 'error: %s version %s does not match tag version %s\n' \
            "$label" "$actual" "$tag_version" >&2
        exit 1
    fi
}

check_version "figen package" "$figen_version"
check_version "figen-proc-macros package" "$proc_macro_version"
check_version "figen-proc-macros dependency" "$dependency_version"

printf 'Release tag and crate versions match: %s\n' "$tag_version"
