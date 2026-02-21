#!/usr/bin/env python3
"""Visualize stopping pattern changes in a PR.

Parses git diff of data/*.csv files and generates a Markdown comment
showing which train types had their stopping patterns changed.

Environment variables:
    BASE_REF: git ref to diff against (e.g. "origin/dev" or "HEAD~1")
"""

import csv
import io
import os
import subprocess
import sys

PASS_SYMBOLS = {
    0: "●",  # 停車
    1: "○",  # 通過
    2: "△",  # 一部通過
    3: "◆",  # 平日停車
    4: "◇",  # 休日停車
    5: "▲",  # 一部停車
}

LEGEND = "● 停車 / ○ 通過 / △ 一部通過 / ◆ 平日停車 / ◇ 休日停車 / ▲ 一部停車"

MAX_GROUPS = 20
MAX_COMMENT_LENGTH = 65000

SST_FILE = "data/5!station_station_types.csv"
STATIONS_FILE = "data/3!stations.csv"
TYPES_FILE = "data/4!types.csv"
LINES_FILE = "data/2!lines.csv"


def git_diff(base_ref: str, file_path: str) -> str:
    result = subprocess.run(
        ["git", "diff", f"{base_ref}...HEAD", "--", file_path],
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        print(f"ERROR: git diff failed for {file_path}: {result.stderr}", file=sys.stderr)
        sys.exit(1)
    return result.stdout


def parse_csv_line(header: list[str], line: str) -> dict[str, str] | None:
    """Parse a single CSV line using csv.reader for proper quoting."""
    if line.startswith("\\ No newline at end of file"):
        return None
    reader = csv.reader(io.StringIO(line))
    for row in reader:
        if len(row) < len(header):
            return None
        return dict(zip(header, row))
    return None


def parse_diff_lines(
    diff_text: str, header: list[str]
) -> tuple[list[dict[str, str]], list[dict[str, str]]]:
    """Parse unified diff and return (added, removed) rows."""
    added = []
    removed = []
    for line in diff_text.splitlines():
        if line.startswith("+++") or line.startswith("---"):
            continue
        if line.startswith("+"):
            row = parse_csv_line(header, line[1:])
            if row and row.get(header[0]) != header[0]:  # skip header
                added.append(row)
        elif line.startswith("-"):
            row = parse_csv_line(header, line[1:])
            if row and row.get(header[0]) != header[0]:  # skip header
                removed.append(row)
    return added, removed


def load_csv(file_path: str) -> tuple[list[str], list[dict[str, str]]]:
    """Load a CSV file, returning (header, rows)."""
    with open(file_path, newline="", encoding="utf-8") as f:
        reader = csv.DictReader(f)
        header = reader.fieldnames or []
        rows = list(reader)
    return header, rows


def get_csv_header(file_path: str) -> list[str]:
    """Read just the header line of a CSV file."""
    with open(file_path, newline="", encoding="utf-8") as f:
        reader = csv.reader(f)
        return next(reader, [])


def build_station_map(
    rows: list[dict[str, str]],
) -> dict[str, dict[str, str]]:
    """station_cd -> station row."""
    return {row["station_cd"]: row for row in rows}


def build_type_map(
    rows: list[dict[str, str]],
) -> dict[str, dict[str, str]]:
    """type_cd -> type row."""
    return {row["type_cd"]: row for row in rows}


def build_line_map(
    rows: list[dict[str, str]],
) -> dict[str, dict[str, str]]:
    """line_cd -> line row."""
    return {row["line_cd"]: row for row in rows}


def get_affected_groups(
    added: list[dict[str, str]], removed: list[dict[str, str]]
) -> set[tuple[str, str]]:
    """Get set of (type_cd, line_group_cd) tuples that were changed."""
    groups = set()
    for row in added + removed:
        groups.add((row["type_cd"], row["line_group_cd"]))
    return groups


def get_group_stations(
    sst_rows: list[dict[str, str]], type_cd: str, line_group_cd: str
) -> list[dict[str, str]]:
    """Get all station_station_type rows for a given group from HEAD."""
    return [
        row
        for row in sst_rows
        if row["type_cd"] == type_cd and row["line_group_cd"] == line_group_cd
    ]


def get_line_names_for_group(
    group_stations: list[dict[str, str]],
    station_map: dict[str, dict[str, str]],
    line_map: dict[str, dict[str, str]],
) -> list[str]:
    """Get ordered unique line names for a group of stations."""
    seen = set()
    names = []
    for sst in group_stations:
        station = station_map.get(sst["station_cd"])
        if not station:
            continue
        line_cd = station["line_cd"]
        if line_cd not in seen:
            seen.add(line_cd)
            line = line_map.get(line_cd)
            if line:
                names.append(line["line_name"])
    return names


def classify_changes(
    added: list[dict[str, str]],
    removed: list[dict[str, str]],
    type_cd: str,
    line_group_cd: str,
) -> dict:
    """Classify changes for a specific group."""
    group_added = [
        r
        for r in added
        if r["type_cd"] == type_cd and r["line_group_cd"] == line_group_cd
    ]
    group_removed = [
        r
        for r in removed
        if r["type_cd"] == type_cd and r["line_group_cd"] == line_group_cd
    ]

    added_scd = {r["station_cd"] for r in group_added}
    removed_scd = {r["station_cd"] for r in group_removed}

    # Stations in both added and removed = pass value changed
    changed_scd = added_scd & removed_scd
    new_scd = added_scd - removed_scd
    deleted_scd = removed_scd - added_scd

    is_new_group = len(group_removed) == 0 and len(group_added) > 0
    is_deleted_group = len(group_added) == 0 and len(group_removed) > 0

    # Build pass change details
    pass_changes = {}
    removed_by_scd = {r["station_cd"]: r for r in group_removed}
    added_by_scd = {r["station_cd"]: r for r in group_added}
    for scd in changed_scd:
        old_pass = int(removed_by_scd[scd]["pass"])
        new_pass = int(added_by_scd[scd]["pass"])
        pass_changes[scd] = (old_pass, new_pass)

    return {
        "is_new": is_new_group,
        "is_deleted": is_deleted_group,
        "changed_stations": changed_scd,
        "new_stations": new_scd,
        "deleted_stations": deleted_scd,
        "pass_changes": pass_changes,
        "all_affected": added_scd | removed_scd,
        "removed_rows": group_removed,
    }


def format_group(
    type_cd: str,
    line_group_cd: str,
    changes: dict,
    group_stations: list[dict[str, str]],
    station_map: dict[str, dict[str, str]],
    type_map: dict[str, dict[str, str]],
    line_map: dict[str, dict[str, str]],
) -> str:
    """Format a single group as Markdown."""
    lines = []

    type_info = type_map.get(type_cd, {})
    type_name = type_info.get("type_name", f"type_{type_cd}")
    type_name_r = type_info.get("type_name_r", "")

    # For deleted groups, resolve line names from removed rows
    if group_stations:
        line_names = get_line_names_for_group(group_stations, station_map, line_map)
    else:
        line_names = get_line_names_for_group(
            changes.get("removed_rows", []), station_map, line_map
        )
    line_str = " / ".join(line_names) if line_names else f"line_group_{line_group_cd}"

    if changes["is_new"]:
        badge = "\U0001f195"
    elif changes["is_deleted"]:
        badge = "\U0001f5d1\ufe0f"
    else:
        badge = "\u270f\ufe0f"
    title_parts = [type_name]
    if type_name_r:
        title_parts.append(f"({type_name_r})")
    title_parts.append(f"- {line_str}")

    lines.append(f"### {badge} {' '.join(title_parts)}")
    lines.append(f"<sub>type_cd={type_cd}, line_group_cd={line_group_cd}</sub>")
    lines.append("")

    # Summary of changes
    summaries = []
    if changes["new_stations"]:
        count = len(changes["new_stations"])
        summaries.append(f"**{count}** 駅追加")
    if changes["deleted_stations"]:
        count = len(changes["deleted_stations"])
        summaries.append(f"**{count}** 駅削除")
    if changes["pass_changes"]:
        for scd, (old_p, new_p) in changes["pass_changes"].items():
            station = station_map.get(scd, {})
            sname = station.get("station_name", scd)
            old_sym = PASS_SYMBOLS.get(old_p, "?")
            new_sym = PASS_SYMBOLS.get(new_p, "?")
            summaries.append(f"{sname}: {old_sym} → {new_sym}")

    if summaries:
        lines.append("変更内容: " + " | ".join(summaries))
        lines.append("")

    # Full station list in details (skip for deleted groups)
    if group_stations:
        num_stations = len(group_stations)
        lines.append("<details>")
        lines.append(f"<summary>全駅リスト ({num_stations}駅)</summary>")
        lines.append("")
        lines.append("| # | 駅名 | Station | 停車 |")
        lines.append("|--:|------|---------|:----:|")

        for idx, sst in enumerate(group_stations, 1):
            scd = sst["station_cd"]
            pass_val = int(sst["pass"])
            symbol = PASS_SYMBOLS.get(pass_val, "?")

            station = station_map.get(scd, {})
            sname = station.get("station_name", scd)
            sname_r = station.get("station_name_r", "")

            is_affected = scd in changes["all_affected"]
            if is_affected:
                sname = f"**{sname}**"
                sname_r = f"**{sname_r}**"

            lines.append(f"| {idx} | {sname} | {sname_r} | {symbol} |")

        lines.append("")
        lines.append("</details>")
        lines.append("")

    return "\n".join(lines)


def set_output(name: str, value: str) -> None:
    """Set a GitHub Actions output variable."""
    github_output = os.environ.get("GITHUB_OUTPUT")
    if github_output:
        with open(github_output, "a") as f:
            f.write(f"{name}={value}\n")


def main() -> None:
    base_ref = os.environ.get("BASE_REF")
    if not base_ref:
        print("ERROR: BASE_REF environment variable is required", file=sys.stderr)
        sys.exit(1)

    # Get diff for station_station_types
    sst_diff = git_diff(base_ref, SST_FILE)
    stations_diff = git_diff(base_ref, STATIONS_FILE)

    if not sst_diff and not stations_diff:
        print("No changes to station stopping patterns.")
        set_output("has_changes", "false")
        return

    # Load reference data from HEAD
    _, station_rows = load_csv(STATIONS_FILE)
    station_map = build_station_map(station_rows)

    _, type_rows = load_csv(TYPES_FILE)
    type_map = build_type_map(type_rows)

    _, line_rows = load_csv(LINES_FILE)
    line_map = build_line_map(line_rows)

    _, sst_rows = load_csv(SST_FILE)

    # Parse SST diff
    sst_header = get_csv_header(SST_FILE)
    sst_added, sst_removed = parse_diff_lines(sst_diff, sst_header) if sst_diff else ([], [])

    if not sst_added and not sst_removed:
        print("No changes to stopping patterns in station_station_types.")
        set_output("has_changes", "false")
        return

    # Get affected groups
    affected_groups = get_affected_groups(sst_added, sst_removed)

    if not affected_groups:
        print("No affected groups found.")
        set_output("has_changes", "false")
        return

    # Classify and format each group
    new_count = 0
    changed_count = 0
    deleted_count = 0
    group_sections = []

    sorted_groups = sorted(affected_groups, key=lambda g: (int(g[0]), int(g[1])))

    for type_cd, line_group_cd in sorted_groups[:MAX_GROUPS]:
        changes = classify_changes(sst_added, sst_removed, type_cd, line_group_cd)
        group_stations = get_group_stations(sst_rows, type_cd, line_group_cd)

        if not group_stations and not changes["deleted_stations"]:
            continue

        if changes["is_new"]:
            new_count += 1
        elif changes["is_deleted"]:
            deleted_count += 1
        else:
            changed_count += 1

        section = format_group(
            type_cd,
            line_group_cd,
            changes,
            group_stations,
            station_map,
            type_map,
            line_map,
        )
        group_sections.append(section)

    if not group_sections:
        print("No visualizable changes found.")
        set_output("has_changes", "false")
        return

    # Build final comment
    parts = []
    parts.append("<!-- station-visualizer -->")
    parts.append("## \U0001f689 停車駅の変更")
    parts.append("")

    summary_parts = []
    if new_count:
        summary_parts.append(f"**{new_count}** 件の新しい停車パターン")
    if changed_count:
        summary_parts.append(f"**{changed_count}** 件の変更された停車パターン")
    if deleted_count:
        summary_parts.append(f"**{deleted_count}** 件の削除された停車パターン")
    parts.append(" | ".join(summary_parts))
    parts.append("")
    parts.append(f"> {LEGEND}")
    parts.append("")

    if len(affected_groups) > MAX_GROUPS:
        parts.append(
            f"> **注意:** {len(affected_groups)} 件中 {MAX_GROUPS} 件のみ表示しています。"
        )
        parts.append("")

    truncation_msg = "\n\n---\n> **注意:** コメントが長すぎるため、一部が省略されました。"
    current_length = len("\n".join(parts))
    truncated = False
    for section in group_sections:
        section_length = len("\n") + len(section)
        if current_length + section_length + len(truncation_msg) > MAX_COMMENT_LENGTH:
            parts.append(truncation_msg)
            truncated = True
            break
        parts.append(section)
        current_length += section_length

    comment = "\n".join(parts)

    # Write output
    output_path = "/tmp/visualization_comment.md"
    with open(output_path, "w", encoding="utf-8") as f:
        f.write(comment)

    print(f"Visualization written to {output_path}")
    print(f"New patterns: {new_count}, Changed patterns: {changed_count}")
    set_output("has_changes", "true")


if __name__ == "__main__":
    main()
