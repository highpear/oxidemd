#!/usr/bin/env bash
set -euo pipefail

sample_path="samples/mermaid-evaluation.md"
output_path="${TMPDIR:-/tmp}/oxidemd-mermaid-cli-comparison"
native_output_path="${TMPDIR:-/tmp}/oxidemd-mermaid-native-comparison"
mermaid_cli_command="mmdc"

usage() {
    cat <<'USAGE'
Usage: tools/compare-mermaid-cli.sh [options]

Options:
  --sample PATH          Markdown sample file to read.
  --output PATH          Directory for Mermaid CLI comparison output.
  --native-output PATH   Directory containing OxideMD native SVG output.
  --mmdc COMMAND         Mermaid CLI command or path.
  -h, --help             Show this help text.
USAGE
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --sample)
            sample_path="$2"
            shift 2
            ;;
        --output)
            output_path="$2"
            shift 2
            ;;
        --native-output)
            native_output_path="$2"
            shift 2
            ;;
        --mmdc)
            mermaid_cli_command="$2"
            shift 2
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
if [ "${sample_path#/}" = "$sample_path" ]; then
    sample_path="$repo_root/$sample_path"
fi
resolved_sample_path="$(cd "$(dirname "$sample_path")" && pwd)/$(basename "$sample_path")"
resolved_output_path="$(mkdir -p "$(dirname "$output_path")" && cd "$(dirname "$output_path")" && pwd)/$(basename "$output_path")"
resolved_native_output_path="$(mkdir -p "$(dirname "$native_output_path")" && cd "$(dirname "$native_output_path")" && pwd)/$(basename "$native_output_path")"

if ! command -v "$mermaid_cli_command" >/dev/null 2>&1; then
    echo "Mermaid CLI command '$mermaid_cli_command' was not found. Install Mermaid CLI, or pass --mmdc with the path to mmdc." >&2
    exit 1
fi

mmdc_path="$(command -v "$mermaid_cli_command")"

case "$resolved_output_path" in
    /|/tmp|/private/tmp)
        echo "Refusing to use an unsafe output path: $resolved_output_path" >&2
        exit 1
        ;;
esac

rm -rf "$resolved_output_path"
mkdir -p "$resolved_output_path"

blocks_tsv="$resolved_output_path/blocks.tsv"
awk -v output_dir="$resolved_output_path" -v metadata="$blocks_tsv" '
    function safe_name(value, fallback) {
        value = tolower(value);
        gsub(/[^a-z0-9]+/, "-", value);
        gsub(/^-+/, "", value);
        gsub(/-+$/, "", value);
        if (value == "") {
            value = fallback;
        }
        return value;
    }
    BEGIN {
        title = "";
        in_block = 0;
        count = 0;
        source_path = "";
    }
    /^## / {
        title = substr($0, 4);
        next;
    }
    /^[[:space:]]*```mermaid[[:space:]]*$/ {
        in_block = 1;
        count++;
        block_title = title;
        if (block_title == "") {
            block_title = "Diagram " count;
        }
        block_safe_name = safe_name(block_title, "diagram-" count);
        source_name = sprintf("%02d-%s.mmd", count, block_safe_name);
        source_path = output_dir "/" source_name;
        printf "" > source_path;
        printf "%d\t%s\t%s\n", count, block_title, block_safe_name >> metadata;
        next;
    }
    in_block && /^[[:space:]]*```[[:space:]]*$/ {
        in_block = 0;
        source_path = "";
        next;
    }
    in_block {
        print $0 >> source_path;
    }
' "$resolved_sample_path"

if [ ! -s "$blocks_tsv" ]; then
    echo "No Mermaid blocks found in $resolved_sample_path" >&2
    exit 1
fi

report_path="$resolved_output_path/comparison-report.md"
html_report_path="$resolved_output_path/visual-comparison.html"

{
    echo "# Mermaid CLI Comparison Report"
    echo
    echo "- Sample: $resolved_sample_path"
    echo "- Mermaid CLI: $mmdc_path"
    echo "- Output: $resolved_output_path"
    echo "- Native SVG output: $resolved_native_output_path"
    echo
    echo "| Diagram | Source | OxideMD SVG | Mermaid CLI SVG | CLI Result | Manual Notes |"
    echo "| --- | --- | --- | --- | --- | --- |"
} > "$report_path"

cat > "$html_report_path" <<'HTML'
<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<title>OxideMD Mermaid CLI Comparison</title>
<style>
body { font-family: system-ui, sans-serif; margin: 24px; color: #20242a; }
section { margin: 0 0 32px; }
.pair { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; align-items: start; }
.panel { border: 1px solid #d4d8df; border-radius: 6px; padding: 12px; overflow: auto; }
.panel h3 { font-size: 14px; margin: 0 0 8px; }
img { max-width: 100%; height: auto; background: white; }
code { font-family: ui-monospace, Consolas, monospace; }
</style>
</head>
<body>
<h1>OxideMD Mermaid CLI Comparison</h1>
<p>Left: OxideMD native SVG output. Right: Mermaid CLI SVG output.</p>
HTML

while IFS="$(printf '\t')" read -r index title safe_name; do
    printf -v prefix "%02d-%s" "$index" "$safe_name"
    source_name="$prefix.mmd"
    svg_name="$prefix.svg"
    source_file="$resolved_output_path/$source_name"
    svg_file="$resolved_output_path/$svg_name"
    native_file="$resolved_native_output_path/$svg_name"

    status="ok"
    if ! "$mmdc_path" -i "$source_file" -o "$svg_file" >/dev/null; then
        status="error"
    fi

    if [ ! -f "$svg_file" ]; then
        cli_svg_name="-"
    else
        cli_svg_name="$svg_name"
    fi

    if [ ! -f "$native_file" ]; then
        native_svg_name="-"
    else
        native_svg_name="$svg_name"
    fi

    echo "| $title | \`$source_name\` | \`$native_svg_name\` | \`$cli_svg_name\` | $status |  |" >> "$report_path"

    {
        echo "<section>"
        echo "<h2>$title</h2>"
        echo "<p>Source: <code>$source_name</code>; CLI result: <code>$status</code></p>"
        echo "<div class=\"pair\">"
        echo "<div class=\"panel\"><h3>OxideMD</h3>"
        if [ "$native_svg_name" = "-" ]; then
            echo "<p>No OxideMD SVG found.</p>"
        else
            echo "<img src=\"$native_file\" alt=\"OxideMD $title\">"
        fi
        echo "</div>"
        echo "<div class=\"panel\"><h3>Mermaid CLI</h3>"
        if [ "$cli_svg_name" = "-" ]; then
            echo "<p>No Mermaid CLI SVG generated.</p>"
        else
            echo "<img src=\"$cli_svg_name\" alt=\"Mermaid CLI $title\">"
        fi
        echo "</div>"
        echo "</div>"
        echo "</section>"
    } >> "$html_report_path"
done < "$blocks_tsv"

cat >> "$html_report_path" <<'HTML'
</body>
</html>
HTML

echo "Wrote Mermaid CLI comparison files to $resolved_output_path"
echo "Report: $report_path"
echo "Visual report: $html_report_path"
