#!/usr/bin/env bash

set -euo pipefail

script_dir=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
validator="$script_dir/check-release-version.sh"

expect_success() {
    local description=$1
    shift

    local output
    if ! output=$("$@" 2>&1); then
        printf 'FAIL: %s\n%s\n' "$description" "$output" >&2
        exit 1
    fi

    if [[ "$output" != *"0.1.0"* ]]; then
        printf 'FAIL: %s did not confirm version 0.1.0\n%s\n' "$description" "$output" >&2
        exit 1
    fi
}

expect_failure() {
    local description=$1
    local expected_message=$2
    shift 2

    local output
    if output=$("$@" 2>&1); then
        printf 'FAIL: %s unexpectedly succeeded\n%s\n' "$description" "$output" >&2
        exit 1
    fi

    if [[ "$output" != *"$expected_message"* ]]; then
        printf 'FAIL: %s did not contain %q\n%s\n' "$description" "$expected_message" "$output" >&2
        exit 1
    fi
}

expect_success "matching argument" "$validator" v0.1.0
expect_success "CI_COMMIT_TAG fallback" env CI_COMMIT_TAG=v0.1.0 "$validator"
expect_failure "mismatching tag" "does not match" "$validator" v0.1.1
expect_failure "missing v prefix" "vMAJOR.MINOR.PATCH" "$validator" 0.1.0
expect_failure "incomplete version" "vMAJOR.MINOR.PATCH" "$validator" v0.1

printf 'All release-version checks passed.\n'
