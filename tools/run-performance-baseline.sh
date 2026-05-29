#!/usr/bin/env bash
set -euo pipefail

sizes_mib="1 5"
timeout_seconds=20
skip_build=0
keep_generated_files=0

usage() {
    cat <<'USAGE'
Usage: tools/run-performance-baseline.sh [options]

Options:
  --sizes "1 5"           Space-separated fixture sizes in MiB.
  --timeout SECONDS       Seconds to wait for each perf log line.
  --skip-build            Reuse the existing release executable.
  --keep-generated-files  Keep generated Markdown fixtures.
  -h, --help              Show this help text.
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --sizes)
            sizes_mib="$2"
            shift 2
            ;;
        --timeout)
            timeout_seconds="$2"
            shift 2
            ;;
        --skip-build)
            skip_build=1
            shift
            ;;
        --keep-generated-files)
            keep_generated_files=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
release_exe="$repo_root/target/release/oxidemd"
temp_root="${TMPDIR:-/tmp}"
if [ "$(uname -s)" = "Darwin" ] && [ -d /private/tmp ]; then
    temp_root="/private/tmp"
fi
output_dir="$temp_root/oxidemd-performance"
section_file="$output_dir/large-document-section.md"

if [ "$skip_build" -eq 0 ]; then
    (cd "$repo_root" && cargo build --release)
fi

if [ ! -x "$release_exe" ]; then
    echo "Missing release executable: $release_exe" >&2
    exit 1
fi

mkdir -p "$output_dir"

cat > "$section_file" <<'MARKDOWN'
# Large Document Section

This paragraph gives OxideMD enough ordinary prose to parse and render. It
includes **strong text**, *emphasis*, `inline code`, and a [link](https://example.com).

- First item
- Second item
- Third item

| Area | Status | Notes |
| --- | --- | --- |
| Parser | Active | Repeated table content |
| Renderer | Active | Repeated table content |

````rust
fn main() {
    println!("large document sample");
}
````

MARKDOWN

file_size_bytes() {
    wc -c < "$1" | tr -d ' '
}

new_markdown_fixture() {
    fixture_path="$1"
    size_mib="$2"
    target_bytes=$((size_mib * 1024 * 1024))

    : > "$fixture_path"
    while [ "$(file_size_bytes "$fixture_path")" -lt "$target_bytes" ]; do
        cat "$section_file" >> "$fixture_path"
    done
}

count_perf_lines() {
    log_path="$1"
    pattern="$2"

    if [ ! -f "$log_path" ]; then
        echo 0
        return
    fi

    grep -c "$pattern" "$log_path" || true
}

wait_for_perf_line() {
    log_path="$1"
    pattern="$2"
    previous_count="$3"
    deadline=$((SECONDS + timeout_seconds))

    while [ "$SECONDS" -lt "$deadline" ]; do
        if [ -f "$log_path" ]; then
            match_count="$(count_perf_lines "$log_path" "$pattern")"
            if [ "$match_count" -gt "$previous_count" ]; then
                grep "$pattern" "$log_path" | tail -n 1
                return
            fi
        fi

        sleep 0.25
    done

    echo "Timed out waiting for perf log pattern: $pattern" >&2
    return 1
}

append_reload_change() {
    fixture_path="$1"
    cat >> "$fixture_path" <<'MARKDOWN'

## Reload Check
This line checks reload timing.
MARKDOWN
}

for size_mib in $sizes_mib; do
    fixture_path="$output_dir/oxidemd-large-${size_mib}mib.md"
    stderr_path="$output_dir/oxidemd-${size_mib}mib.stderr.log"
    stdout_path="$output_dir/oxidemd-${size_mib}mib.stdout.log"

    rm -f "$stderr_path" "$stdout_path"
    new_markdown_fixture "$fixture_path" "$size_mib"

    actual_size="$(file_size_bytes "$fixture_path")"
    echo
    echo "== $size_mib MiB target =="
    echo "Fixture: $fixture_path"
    echo "Actual size: $actual_size bytes"

    "$release_exe" "$fixture_path" > "$stdout_path" 2> "$stderr_path" &
    app_pid="$!"

    cleanup() {
        if kill -0 "$app_pid" 2>/dev/null; then
            kill "$app_pid" 2>/dev/null || true
            wait "$app_pid" 2>/dev/null || true
        fi
    }
    trap cleanup EXIT

    initial_load="$(wait_for_perf_line "$stderr_path" "\\[perf\\] initial_load:" 0)"
    render_after_load="$(wait_for_perf_line "$stderr_path" "\\[perf\\] render_after_load:" 0)"

    reload_count="$(count_perf_lines "$stderr_path" "\\[perf\\] reload:")"
    render_after_reload_count="$(count_perf_lines "$stderr_path" "\\[perf\\] render_after_reload:")"
    append_reload_change "$fixture_path"
    reload="$(wait_for_perf_line "$stderr_path" "\\[perf\\] reload:" "$reload_count")"
    render_after_reload="$(wait_for_perf_line "$stderr_path" "\\[perf\\] render_after_reload:" "$render_after_reload_count")"

    skipped_count="$(count_perf_lines "$stderr_path" "\\[perf\\] reload_skipped:")"
    touch "$fixture_path"
    skipped_reload="$(wait_for_perf_line "$stderr_path" "\\[perf\\] reload_skipped:" "$skipped_count")"

    echo "$initial_load"
    echo "$render_after_load"
    echo "$reload"
    echo "$render_after_reload"
    echo "$skipped_reload"
    echo "Log: $stderr_path"

    cleanup
    trap - EXIT

    if [ "$keep_generated_files" -eq 0 ]; then
        rm -f "$fixture_path"
    fi
done

rm -f "$section_file"
