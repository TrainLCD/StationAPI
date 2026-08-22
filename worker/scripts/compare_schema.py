#!/usr/bin/env python3
"""Worker が生成した SDL と BFF のスキーマを突き合わせる。

async-graphql はコードファーストなので、Rust の型を変えると SDL が変わる。
クライアントが壊れる変更に気付けるよう、CI でこの比較を行う。

型・フィールドは順序を無視した集合として、enum は順序込みで比較する。
"""
import re
import sys


def parse(path):
    text = open(path, encoding="utf-8").read()
    out = {}
    for m in re.finditer(r"^(type|enum|input) (\w+) \{\n(.*?)^\}", text, re.S | re.M):
        kind, name, body = m.group(1), m.group(2), m.group(3)
        fields = [
            re.sub(r"\s+", " ", line.strip())
            for line in body.strip().split("\n")
            if line.strip() and not line.strip().startswith("#")
        ]
        out[name] = (kind, fields)
    for m in re.finditer(r"^scalar (\w+)", text, re.M):
        out[m.group(1)] = ("scalar", [])
    return out


def main():
    if len(sys.argv) != 3:
        print("usage: compare_schema.py <expected.graphql> <actual.graphql>", file=sys.stderr)
        return 2

    expected, actual = parse(sys.argv[1]), parse(sys.argv[2])
    diffs = []
    for name in sorted(set(expected) | set(actual)):
        if name not in expected:
            diffs.append(f"Worker にのみ存在する型: {name}")
            continue
        if name not in actual:
            diffs.append(f"BFF にのみ存在する型: {name}")
            continue
        exp_kind, exp_fields = expected[name]
        act_kind, act_fields = actual[name]
        if exp_kind != act_kind:
            diffs.append(f"{name}: 種別が違う ({exp_kind} vs {act_kind})")
        if exp_kind == "enum":
            # enum は値の順序も含めて一致させる
            if exp_fields != act_fields:
                diffs.append(f"{name}: enum の値が違う / BFF={exp_fields} Worker={act_fields}")
        else:
            for f in exp_fields:
                if f not in act_fields:
                    diffs.append(f"{name}: BFF にあって Worker に無い -> {f}")
            for f in act_fields:
                if f not in exp_fields:
                    diffs.append(f"{name}: Worker にあって BFF に無い -> {f}")

    if diffs:
        print(f"スキーマに {len(diffs)} 件の差分があります:")
        for d in diffs:
            print(f"  {d}")
        return 1

    print(f"スキーマは一致しています ({len(expected)} 型)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
