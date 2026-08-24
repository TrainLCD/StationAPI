#!/usr/bin/env python3
"""`bench.py` (GraphQL ベンチマークスキル) のユニットテスト。

依存は標準ライブラリのみ (プロジェクトに pytest 等は導入されていないため `unittest` を使う)。
ネットワークや `wrangler`/`npx` の実行は一切行わない。`Tail` のプロセス管理だけは
`sys.executable` を使った軽量なダミーコマンドで実プロセスのライフサイクルを検証する。

実行方法:
    python3 -m unittest .claude/skills/benchmark-gql/test_bench.py -v
"""

from __future__ import annotations

import importlib.util
import json
import math
import statistics
import subprocess
import sys
import tempfile
import types
import unittest
from datetime import datetime, timedelta
from pathlib import Path
from unittest import mock

SKILL_DIR = Path(__file__).resolve().parent

_spec = importlib.util.spec_from_file_location("bench_under_test", SKILL_DIR / "bench.py")
bench = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(bench)


# --------------------------------------------------------------------------- root_query_fields


class TestRootQueryFields(unittest.TestCase):
    """`bench._LEXER_CASES` に書かれている取りこぼしケースを、通常のテストとしても走らせる。

    `--self-test` を明示的に実行しない限り検証されないため、通常のテストスイートに
    組み込んでおくことで CI 相当の場でも壊れたら気付けるようにする。
    """

    def test_lexer_cases_from_module(self):
        for label, document, expected in bench._LEXER_CASES:
            with self.subTest(label=label):
                self.assertEqual(bench.root_query_fields(document), expected)

    def test_ignores_operation_level_variable_definitions(self):
        doc = "query Q($x: Int!) { station(id: $x) { name } }"
        self.assertEqual(bench.root_query_fields(doc), {"station"})

    def test_multiple_aliased_root_fields(self):
        doc = "query Q { a: station(id: 1) { name } b: lines(lineIds: [1]) { id } }"
        self.assertEqual(bench.root_query_fields(doc), {"station", "lines"})

    def test_empty_document_returns_empty_set(self):
        self.assertEqual(bench.root_query_fields(""), set())

    def test_document_without_braces_returns_empty_set(self):
        self.assertEqual(bench.root_query_fields("not a query"), set())

    def test_bare_scalar_field_without_selection_set(self):
        self.assertEqual(bench.root_query_fields("query Q { ping }"), {"ping"})

    def test_fields_after_the_operation_body_are_not_collected(self):
        # 深さが 0 に戻った後の文字列 (2 つ目のオペレーションなど) は対象外。
        doc = "query Q { station(id: 1) { name } } query R { lines(lineIds: [1]) { id } }"
        self.assertEqual(bench.root_query_fields(doc), {"station"})


# --------------------------------------------------------------------------- uncovered_query_fields


class TestUncoveredQueryFields(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        self._patcher = mock.patch.object(bench, "REPO_ROOT", self.tmp_path)
        self._patcher.start()
        self.addCleanup(self._patcher.stop)

    def _write_schema(self, body: str) -> None:
        schema_dir = self.tmp_path / "schema"
        schema_dir.mkdir(parents=True, exist_ok=True)
        (schema_dir / "public.graphql").write_text(
            "type Query {\n" + body + "\n}\n", encoding="utf-8"
        )

    def test_returns_empty_list_when_schema_file_is_missing(self):
        self.assertEqual(bench.uncovered_query_fields([]), [])

    def test_finds_the_field_no_case_touches(self):
        self._write_schema(
            "\tfieldA(x: Int): String\n\tfieldB(y: Int): String\n\tfieldC: String\n"
        )
        cases = [
            {"query": "query Q { fieldA(x: 1) }"},
            {"query": "query Q { fieldC }"},
        ]
        self.assertEqual(bench.uncovered_query_fields(cases), ["fieldB"])

    def test_returns_empty_list_when_every_field_is_covered(self):
        self._write_schema("\tfieldA(x: Int): String\n\tfieldB: String\n")
        cases = [{"query": "query Q { fieldA(x: 1) fieldB }"}]
        self.assertEqual(bench.uncovered_query_fields(cases), [])

    def test_http_cases_without_query_do_not_count_as_coverage(self):
        self._write_schema("\tfieldA: String\n")
        cases = [{"kind": "http", "method": "GET", "path": "/__ping"}]
        self.assertEqual(bench.uncovered_query_fields(cases), ["fieldA"])

    def test_real_repo_queries_cover_the_real_schema(self):
        # 統合テスト: 実際の schema/public.graphql と queries.json を使う (self-test と同じ主張)。
        self._patcher.stop()
        real_cases = json.loads((SKILL_DIR / "queries.json").read_text(encoding="utf-8"))["cases"]
        self.assertEqual(bench.uncovered_query_fields(real_cases), [])
        self._patcher.start()  # tearDown で再度 stop されても安全なように戻しておく


# --------------------------------------------------------------------------- load_cases


class TestLoadCases(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        (self.tmp_path / "schema").mkdir()  # public.graphql を置かないので uncovered は常に []
        self._patcher = mock.patch.object(bench, "REPO_ROOT", self.tmp_path)
        self._patcher.start()
        self.addCleanup(self._patcher.stop)

        self.queries_path = self.tmp_path / "queries.json"
        self._write_queries(
            {
                "fragments": {"Frag": "fragment Frag on X { id }"},
                "cases": [
                    {
                        "name": "ping",
                        "kind": "http",
                        "method": "GET",
                        "path": "/__ping",
                        "weight": "baseline",
                    },
                    {
                        "name": "foo",
                        "weight": "light",
                        "uses": ["Frag"],
                        "query": "query Q { foo { id } }",
                    },
                    {
                        "name": "bar",
                        "weight": "medium",
                        "query": "query Q { bar { id } }",
                    },
                ],
            }
        )

    def _write_queries(self, doc: dict) -> None:
        self.queries_path.write_text(json.dumps(doc), encoding="utf-8")

    def test_loads_all_cases_and_builds_document_with_fragment(self):
        cases, uncovered = bench.load_cases(self.queries_path, None, False)
        self.assertEqual(uncovered, [])
        self.assertEqual([c["name"] for c in cases], ["ping", "foo", "bar"])
        foo = next(c for c in cases if c["name"] == "foo")
        self.assertIn("query Q { foo { id } }", foo["_document"])
        self.assertIn("fragment Frag on X", foo["_document"])
        ping = next(c for c in cases if c["name"] == "ping")
        self.assertNotIn("_document", ping)

    def test_skip_baseline_removes_baseline_cases(self):
        cases, _ = bench.load_cases(self.queries_path, None, True)
        self.assertNotIn("ping", [c["name"] for c in cases])
        self.assertEqual({c["name"] for c in cases}, {"foo", "bar"})

    def test_only_filters_to_named_cases(self):
        cases, _ = bench.load_cases(self.queries_path, ["foo"], False)
        self.assertEqual([c["name"] for c in cases], ["foo"])

    def test_only_unknown_case_name_exits(self):
        with self.assertRaises(SystemExit):
            bench.load_cases(self.queries_path, ["does-not-exist"], False)

    def test_only_with_skip_baseline_excludes_skipped_case_without_error(self):
        # "ping" is a known case name (checked before filtering), but --skip-baseline removes
        # it first, so asking for it via --only yields an empty result, not an "unknown" error.
        cases, _ = bench.load_cases(self.queries_path, ["ping"], True)
        self.assertEqual(cases, [])

    def test_missing_fragment_reference_exits(self):
        self._write_queries(
            {
                "fragments": {},
                "cases": [
                    {
                        "name": "foo",
                        "weight": "light",
                        "uses": ["Missing"],
                        "query": "query Q { foo { id } }",
                    }
                ],
            }
        )
        with self.assertRaises(SystemExit):
            bench.load_cases(self.queries_path, None, False)


# --------------------------------------------------------------------------- wrangler_argv


class TestWranglerArgv(unittest.TestCase):
    def test_returns_bare_wrangler_without_makefile(self):
        with tempfile.TemporaryDirectory() as d:
            with mock.patch.object(bench, "REPO_ROOT", Path(d)):
                self.assertEqual(bench.wrangler_argv(), ["npx", "--yes", "wrangler"])

    def test_reads_version_from_makefile(self):
        with tempfile.TemporaryDirectory() as d:
            (Path(d) / "Makefile").write_text(
                "WRANGLER_VERSION := 4.99.1\nWRANGLER := npx --yes wrangler@$(WRANGLER_VERSION)\n",
                encoding="utf-8",
            )
            with mock.patch.object(bench, "REPO_ROOT", Path(d)):
                self.assertEqual(bench.wrangler_argv(), ["npx", "--yes", "wrangler@4.99.1"])

    def test_ignores_commented_out_version_lines(self):
        with tempfile.TemporaryDirectory() as d:
            (Path(d) / "Makefile").write_text(
                "SOME_OTHER := 1.0\n# WRANGLER_VERSION := 9.9.9 (disabled)\n",
                encoding="utf-8",
            )
            with mock.patch.object(bench, "REPO_ROOT", Path(d)):
                self.assertEqual(bench.wrangler_argv(), ["npx", "--yes", "wrangler"])

    def test_matches_the_real_repository_makefile(self):
        # AGENTS.md の方針: bench.py と Makefile の WRANGLER_VERSION はずれてはいけない。
        makefile_text = (bench.REPO_ROOT / "Makefile").read_text(encoding="utf-8")
        expected_version = None
        for line in makefile_text.splitlines():
            if line.startswith("WRANGLER_VERSION"):
                expected_version = line.split(":=", 1)[-1].strip()
                break
        self.assertIsNotNone(expected_version, "Makefile に WRANGLER_VERSION が無い")
        self.assertEqual(bench.wrangler_argv(), ["npx", "--yes", f"wrangler@{expected_version}"])


# --------------------------------------------------------------------------- Client


class FakeGraphQLResponse:
    def __init__(self, status=200, headers=None, body=b'{"data": {}}'):
        self.status = status
        self._headers = headers or {}
        self._body = body

    def getheader(self, name):
        return self._headers.get(name)

    def read(self):
        return self._body


class TestClient(unittest.TestCase):
    def test_rejects_non_https_origin(self):
        with self.assertRaises(AssertionError):
            bench.Client("http://example.com", "run1", 5.0)

    def test_request_sends_expected_headers_and_parses_cf_ray(self):
        conn = mock.MagicMock()
        conn.getresponse.return_value = FakeGraphQLResponse(
            headers={"cf-ray": "abcdef0123456789-NRT"}
        )
        with mock.patch.object(bench, "HTTPSConnection", return_value=conn):
            client = bench.Client("https://example.com", "run-42", 5.0)
            result = client.request("POST", "/graphql", {"query": "{ ping }"})

        self.assertEqual(result["status"], 200)
        self.assertEqual(result["ray"], "abcdef0123456789")
        self.assertEqual(result["colo"], "NRT")

        args, kwargs = conn.request.call_args
        self.assertEqual(args[0], "POST")
        self.assertEqual(args[1], "/graphql")
        self.assertEqual(json.loads(kwargs["body"]), {"query": "{ ping }"})
        self.assertEqual(kwargs["headers"]["x-stationapi-bench"], "run-42")
        self.assertEqual(kwargs["headers"]["content-type"], "application/json")
        self.assertIn("stationapi-bench", kwargs["headers"]["user-agent"])

    def test_get_request_without_payload_omits_content_type_and_body(self):
        conn = mock.MagicMock()
        conn.getresponse.return_value = FakeGraphQLResponse()
        with mock.patch.object(bench, "HTTPSConnection", return_value=conn):
            client = bench.Client("https://example.com", "run-1", 5.0)
            client.request("GET", "/__ping", None)

        _, kwargs = conn.request.call_args
        self.assertNotIn("content-type", kwargs["headers"])
        self.assertIsNone(kwargs["body"])

    def test_missing_cf_ray_header_yields_empty_ray_and_colo(self):
        conn = mock.MagicMock()
        conn.getresponse.return_value = FakeGraphQLResponse(headers={})
        with mock.patch.object(bench, "HTTPSConnection", return_value=conn):
            client = bench.Client("https://example.com", "run-1", 5.0)
            result = client.request("GET", "/__ping", None)
        self.assertEqual(result["ray"], "")
        self.assertEqual(result["colo"], "")

    def test_retries_once_after_a_transient_failure(self):
        bad_conn = mock.MagicMock()
        bad_conn.request.side_effect = OSError("connection reset")
        good_conn = mock.MagicMock()
        good_conn.getresponse.return_value = FakeGraphQLResponse()
        factory = mock.MagicMock(side_effect=[bad_conn, good_conn])

        with mock.patch.object(bench, "HTTPSConnection", factory):
            client = bench.Client("https://example.com", "run-1", 5.0)
            result = client.request("GET", "/__ping", None)

        self.assertEqual(result["status"], 200)
        self.assertEqual(factory.call_count, 2)

    def test_raises_after_two_consecutive_failures(self):
        bad_conn_1, bad_conn_2 = mock.MagicMock(), mock.MagicMock()
        bad_conn_1.request.side_effect = OSError("connection reset")
        bad_conn_2.request.side_effect = OSError("connection reset again")
        factory = mock.MagicMock(side_effect=[bad_conn_1, bad_conn_2])

        with mock.patch.object(bench, "HTTPSConnection", factory):
            client = bench.Client("https://example.com", "run-1", 5.0)
            with self.assertRaises(OSError):
                client.request("GET", "/__ping", None)

    def test_close_forces_a_fresh_connection_on_next_request(self):
        conn1 = mock.MagicMock()
        conn1.getresponse.return_value = FakeGraphQLResponse()
        conn2 = mock.MagicMock()
        conn2.getresponse.return_value = FakeGraphQLResponse()
        factory = mock.MagicMock(side_effect=[conn1, conn2])

        with mock.patch.object(bench, "HTTPSConnection", factory):
            client = bench.Client("https://example.com", "run-1", 5.0)
            client.request("GET", "/__ping", None)
            client.close()
            client.request("GET", "/__ping", None)

        self.assertEqual(factory.call_count, 2)
        conn1.close.assert_called_once()


# --------------------------------------------------------------------------- check_response / graphql_errors


class TestGraphqlErrorsAndCheckResponse(unittest.TestCase):
    def test_graphql_errors_returns_none_when_absent(self):
        self.assertIsNone(bench.graphql_errors(b'{"data": {"ping": true}}'))

    def test_graphql_errors_returns_list_when_present(self):
        errs = bench.graphql_errors(b'{"errors": [{"message": "boom"}]}')
        self.assertEqual(errs, [{"message": "boom"}])

    def test_graphql_errors_handles_invalid_json(self):
        errs = bench.graphql_errors(b"not json")
        self.assertEqual(len(errs), 1)
        self.assertIn("JSON", errs[0]["message"])

    def test_check_response_passes_for_ok_http_only_case(self):
        bench.check_response(
            {"name": "ping"}, {"key": "production"}, {"status": 200, "body": b""}, None
        )

    def test_check_response_exits_on_non_200(self):
        with self.assertRaises(SystemExit):
            bench.check_response(
                {"name": "station"},
                {"key": "production"},
                {"status": 500, "body": b""},
                {"query": "{ station }"},
            )

    def test_check_response_exits_on_graphql_errors(self):
        with self.assertRaises(SystemExit):
            bench.check_response(
                {"name": "station"},
                {"key": "production"},
                {"status": 200, "body": b'{"errors":[{"message":"bad"}]}'},
                {"query": "{ station }"},
            )

    def test_check_response_passes_when_response_has_no_errors(self):
        bench.check_response(
            {"name": "station"},
            {"key": "production"},
            {"status": 200, "body": b'{"data": {"station": null}}'},
            {"query": "{ station }"},
        )


# --------------------------------------------------------------------------- Tail


class TestTail(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.log_dir = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)

    def test_events_parses_concatenated_json_objects_and_ignores_truncated_tail(self):
        tail = bench.Tail("myscript", "run1", self.log_dir)
        event1 = {
            "cpuTime": 12,
            "wallTime": 15,
            "outcome": "ok",
            "scriptVersion": {"id": "v1"},
            "event": {"request": {"headers": {"cf-ray": "abc123-NRT"}}},
        }
        event2 = {
            "cpuTime": 20,
            "wallTime": 22,
            "outcome": "ok",
            "event": {"request": {"headers": {"cf-ray": "def456-HND"}}},
        }
        # 末尾に途中で切れた 1 件を混ぜても壊れない。
        content = json.dumps(event1) + "\n" + json.dumps(event2) + "\n" + '{"trunca'
        tail.out_path.write_text(content, encoding="utf-8")

        events = tail.events()

        self.assertEqual(set(events), {"abc123", "def456"})
        self.assertEqual(events["abc123"]["cpu_ms"], 12)
        self.assertEqual(events["abc123"]["wall_ms"], 15)
        self.assertEqual(events["abc123"]["version_id"], "v1")
        self.assertIsNone(events["def456"]["version_id"])

    def test_events_ignores_entries_without_a_fetch_request(self):
        tail = bench.Tail("myscript", "run1", self.log_dir)
        # cron トリガーなど、fetch 以外のイベントは event が null になりうる。
        tail.out_path.write_text(json.dumps({"cpuTime": 5, "event": None}), encoding="utf-8")
        self.assertEqual(tail.events(), {})

    def test_events_returns_empty_dict_when_log_file_does_not_exist(self):
        tail = bench.Tail("myscript", "run1", self.log_dir)
        self.assertEqual(tail.events(), {})

    def test_died_returns_none_before_start(self):
        tail = bench.Tail("myscript", "run1", self.log_dir)
        self.assertIsNone(tail.died())

    def test_stop_before_start_is_a_safe_no_op(self):
        tail = bench.Tail("myscript", "run1", self.log_dir)
        tail.stop()  # 例外にならないこと

    def test_start_and_stop_terminates_the_underlying_process(self):
        fake_cmd = [sys.executable, "-c", "import time; time.sleep(30)"]
        with mock.patch.object(bench, "wrangler_argv", return_value=fake_cmd):
            tail = bench.Tail("dummy-script", "run-1", self.log_dir)
            tail.start()
            self.addCleanup(tail.stop)
            try:
                self.assertIsNone(tail.died())  # まだ生きている
                tail.stop()
                self.assertIsNotNone(tail.proc.poll())  # 終了している
                tail.stop()  # 二度呼んでも安全
            finally:
                tail.stop()

    def test_died_reports_stderr_after_process_exits(self):
        code = "import sys; sys.stderr.write('boom'); sys.exit(1)"
        with mock.patch.object(bench, "wrangler_argv", return_value=[sys.executable, "-c", code]):
            tail = bench.Tail("dummy-script", "run-1", self.log_dir)
            tail.start()
            try:
                tail.proc.wait(timeout=10)
                self.assertIn("boom", tail.died())
            finally:
                tail.stop()


# --------------------------------------------------------------------------- wait_for_tail


class _FakeTail:
    def __init__(self, events=None, dead_error=None):
        self._events = events or {}
        self._dead_error = dead_error
        self.script = "fake"

    def died(self):
        return self._dead_error

    def events(self):
        return self._events


class _FakeClient:
    def __init__(self, ray):
        self._ray = ray

    def request(self, method, path, payload):
        return {"ray": self._ray}


class TestWaitForTail(unittest.TestCase):
    def test_returns_true_once_ping_ray_appears_in_events(self):
        tails = {
            "production": _FakeTail(events={"ray-prod": {}}),
            "staging": _FakeTail(events={"ray-stg": {}}),
        }
        clients = {"production": _FakeClient("ray-prod"), "staging": _FakeClient("ray-stg")}
        with mock.patch.object(bench.time, "sleep", return_value=None):
            self.assertTrue(bench.wait_for_tail(tails, clients, timeout=5.0))

    def test_returns_false_immediately_when_a_tail_process_died(self):
        tails = {
            "production": _FakeTail(dead_error="wrangler crashed"),
            "staging": _FakeTail(),
        }
        clients = {"production": _FakeClient("r1"), "staging": _FakeClient("r2")}
        with mock.patch.object(bench.time, "sleep", return_value=None):
            self.assertFalse(bench.wait_for_tail(tails, clients, timeout=5.0))

    def test_returns_false_after_timeout_when_no_event_ever_matches(self):
        tails = {"production": _FakeTail(events={}), "staging": _FakeTail(events={})}
        clients = {"production": _FakeClient("never-seen"), "staging": _FakeClient("never-seen")}
        with mock.patch.object(bench.time, "sleep", return_value=None):
            self.assertFalse(bench.wait_for_tail(tails, clients, timeout=0.01))


# --------------------------------------------------------------------------- percentile


class TestPercentile(unittest.TestCase):
    def test_empty_list_returns_nan(self):
        self.assertTrue(math.isnan(bench.percentile([], 0.5)))

    def test_single_value_returns_that_value_for_any_quantile(self):
        self.assertEqual(bench.percentile([42.0], 0.9), 42.0)

    def test_median_of_odd_length_list(self):
        self.assertEqual(bench.percentile([1, 2, 3, 4, 5], 0.5), 3)

    def test_p95_interpolates_between_two_values(self):
        values = list(range(1, 11))  # 1..10
        self.assertAlmostEqual(bench.percentile(values, 0.95), 9.55)

    def test_unsorted_input_is_sorted_before_computing(self):
        self.assertEqual(bench.percentile([5, 1, 3], 0.5), 3)


# --------------------------------------------------------------------------- cold_cutoff / ratio


class TestColdCutoff(unittest.TestCase):
    def test_empty_returns_infinity(self):
        self.assertEqual(bench.cold_cutoff([]), float("inf"))

    def test_uses_the_factor_when_it_dominates(self):
        # median=100 -> factor*median=250, median+margin=250 -> 両方等しい場合は max がそのまま
        self.assertEqual(bench.cold_cutoff([100, 100, 100]), 250)

    def test_uses_the_margin_when_values_are_small(self):
        # median=1 -> factor*median=2.5, median+margin=151 -> margin 側が勝つ
        self.assertEqual(bench.cold_cutoff([1, 1, 1]), 151)


class TestRatio(unittest.TestCase):
    def test_none_production_value_returns_none(self):
        self.assertIsNone(bench.ratio(None, 10))

    def test_zero_production_value_returns_none(self):
        self.assertIsNone(bench.ratio(0, 10))

    def test_none_staging_value_returns_none(self):
        self.assertIsNone(bench.ratio(10, None))

    def test_normal_division(self):
        self.assertEqual(bench.ratio(10, 5), 0.5)


# --------------------------------------------------------------------------- fmt / fmt_delta / verdict


class TestFormatting(unittest.TestCase):
    def test_fmt_none_is_em_dash(self):
        self.assertEqual(bench.fmt(None), "—")

    def test_fmt_nan_is_em_dash(self):
        self.assertEqual(bench.fmt(float("nan")), "—")

    def test_fmt_with_digits_and_unit(self):
        self.assertEqual(bench.fmt(2.567, 1, " ms"), "2.6 ms")

    def test_fmt_default_digits(self):
        self.assertEqual(bench.fmt(1.5), "1.50")

    def test_fmt_delta_none_is_em_dash(self):
        self.assertEqual(bench.fmt_delta(None), "—")

    def test_fmt_delta_positive_has_plus_sign(self):
        self.assertEqual(bench.fmt_delta(1.2), "+20.0%")

    def test_fmt_delta_negative_has_minus_sign(self):
        self.assertEqual(bench.fmt_delta(0.75), "-25.0%")

    def test_fmt_delta_zero_percent_still_has_plus_sign(self):
        self.assertEqual(bench.fmt_delta(1.0), "+0.0%")


class TestVerdict(unittest.TestCase):
    def test_none_ratio_is_em_dash(self):
        self.assertEqual(bench.verdict(None), "—")

    def test_within_threshold_is_same(self):
        self.assertEqual(bench.verdict(1.05), "同等")
        self.assertEqual(bench.verdict(0.95), "同等")

    def test_small_absolute_diff_downgrades_large_ratio_to_small_label(self):
        self.assertEqual(bench.verdict(2.0, abs_diff=0.5, min_diff=1.5), "微差")

    def test_large_ratio_with_large_diff_is_staging_slower(self):
        self.assertEqual(bench.verdict(2.0, abs_diff=5.0, min_diff=1.5), "stg 遅い")

    def test_small_ratio_with_large_diff_is_staging_faster(self):
        self.assertEqual(bench.verdict(0.4, abs_diff=-5.0, min_diff=1.5), "stg 速い")

    def test_custom_small_label_is_honored(self):
        self.assertEqual(
            bench.verdict(3.0, abs_diff=0.1, min_diff=2.0, small_label="誤差内"), "誤差内"
        )


class TestBelowResolution(unittest.TestCase):
    def test_all_none_is_false(self):
        self.assertFalse(bench.below_resolution(None, None))

    def test_true_when_max_is_below_threshold(self):
        self.assertTrue(bench.below_resolution(0.5, 0.9))

    def test_false_when_max_meets_or_exceeds_threshold(self):
        self.assertFalse(bench.below_resolution(0.5, 1.0))

    def test_ignores_none_values_when_computing_max(self):
        self.assertTrue(bench.below_resolution(None, 0.3))


# --------------------------------------------------------------------------- summarize


class TestSummarize(unittest.TestCase):
    def test_computes_client_and_cpu_stats_and_splits_cold_start(self):
        cases = [{"name": "station", "weight": "light", "note": "x"}]
        cpu_values = [10, 11, 9, 10, 400]  # 400 は明確なコールドスタート
        samples = [
            {
                "case": "station",
                "target": "production",
                "iteration": i,
                "client_ms": 20 + i,
                "bytes": 1000 + i,
                "cpu_ms": cpu,
                "worker_wall_ms": cpu + 1,
            }
            for i, cpu in enumerate(cpu_values)
        ]

        rows = bench.summarize(samples, cases)

        self.assertEqual(len(rows), 1)
        prod = rows[0]["targets"]["production"]
        self.assertEqual(prod["n"], 5)
        self.assertEqual(prod["cold_n"], 1)
        self.assertEqual(prod["cold_max"], 400)
        self.assertEqual(prod["cpu_n"], 4)
        self.assertAlmostEqual(prod["cpu_mean"], statistics.fmean([10, 11, 9, 10]))
        self.assertNotIn("staging", rows[0]["targets"])

    def test_missing_cpu_samples_yield_none_cpu_stats(self):
        cases = [{"name": "ping", "weight": "baseline", "note": ""}]
        samples = [
            {"case": "ping", "target": "production", "iteration": 0, "client_ms": 5.0, "bytes": 10}
        ]
        rows = bench.summarize(samples, cases)
        prod = rows[0]["targets"]["production"]
        self.assertIsNone(prod["cpu_mean"])
        self.assertEqual(prod["cpu_n"], 0)
        self.assertIsNone(prod["worker_wall_mean"])
        self.assertIsNone(prod["cold_cutoff"])

    def test_row_order_follows_the_case_list_not_the_samples(self):
        cases = [{"name": "b", "weight": "", "note": ""}, {"name": "a", "weight": "", "note": ""}]
        samples = [
            {"case": "a", "target": "production", "client_ms": 1.0, "bytes": 1},
            {"case": "b", "target": "production", "client_ms": 1.0, "bytes": 1},
        ]
        rows = bench.summarize(samples, cases)
        self.assertEqual([r["case"] for r in rows], ["b", "a"])


# --------------------------------------------------------------------------- render_markdown


class TestRenderMarkdown(unittest.TestCase):
    def setUp(self):
        self.started = datetime(2026, 1, 2, 3, 4, 5, tzinfo=bench.JST)
        self.finished = self.started + timedelta(seconds=42)
        self.args = types.SimpleNamespace(repeat=5, warmup=2)

    def _rows_and_samples(self):
        cases = [
            {"name": "ping", "weight": "baseline", "note": "疎通"},
            {"name": "station", "weight": "light", "note": "駅取得"},
        ]
        samples = []
        for i in range(6):
            samples.append(
                {"case": "ping", "target": "production", "client_ms": 5.0 + i * 0.1, "bytes": 2}
            )
            samples.append(
                {"case": "ping", "target": "staging", "client_ms": 6.0 + i * 0.1, "bytes": 2}
            )
            samples.append(
                {
                    "case": "station",
                    "target": "production",
                    "client_ms": 30.0,
                    "bytes": 500,
                    "cpu_ms": 10,
                    "worker_wall_ms": 11,
                }
            )
            samples.append(
                {
                    "case": "station",
                    "target": "staging",
                    "client_ms": 60.0,
                    "bytes": 500,
                    "cpu_ms": 20,
                    "worker_wall_ms": 21,
                }
            )
        return bench.summarize(samples, cases), samples

    def test_render_includes_expected_sections_and_values(self):
        rows, samples = self._rows_and_samples()
        meta = {
            "cpu_time_available": True,
            "tail_matched": len(samples),
            "tail_total": len(samples),
            "versions": {},
            "uncovered_query_fields": [],
        }
        markdown = bench.render_markdown(
            rows, samples, meta, self.args, "20260102-030405", self.started, self.finished
        )
        self.assertIn("# GraphQL ベンチマーク 2026-01-02 03:04 JST", markdown)
        self.assertIn("## CPU Time (Cloudflare Worker)", markdown)
        self.assertIn("## クライアント応答時間", markdown)
        self.assertIn("## コールドスタート", markdown)
        self.assertIn("## ケース一覧", markdown)
        self.assertIn("## 所見", markdown)
        self.assertIn("## 測り方の限界", markdown)
        self.assertIn("| 反復数 | 5 (計測前に全ケースを 2 巡して破棄) |", markdown)
        self.assertIn("`station`", markdown)
        # staging の CPU が本番の 2 倍、絶対差も 1.5ms 以上 -> 遅い判定になるはず
        self.assertIn("stg 遅い", markdown)

    def test_render_reports_missing_cpu_time_when_unavailable(self):
        rows, samples = self._rows_and_samples()
        for row in rows:
            for stat in row["targets"].values():
                stat["cpu_mean"] = None
        meta = {"cpu_time_available": False, "uncovered_query_fields": []}
        markdown = bench.render_markdown(
            rows, samples, meta, self.args, "run", self.started, self.finished
        )
        self.assertIn("この実行では CPU Time を集めていない", markdown)

    def test_render_warns_about_uncovered_query_fields(self):
        rows, samples = self._rows_and_samples()
        meta = {"cpu_time_available": True, "uncovered_query_fields": ["mysteryField"]}
        markdown = bench.render_markdown(
            rows, samples, meta, self.args, "run", self.started, self.finished
        )
        self.assertIn("mysteryField", markdown)
        self.assertIn("[!WARNING]", markdown)

    def test_render_warns_about_tail_note(self):
        rows, samples = self._rows_and_samples()
        meta = {
            "cpu_time_available": False,
            "tail_note": "接続できませんでした",
            "uncovered_query_fields": [],
        }
        markdown = bench.render_markdown(
            rows, samples, meta, self.args, "run", self.started, self.finished
        )
        self.assertIn("接続できませんでした", markdown)


# --------------------------------------------------------------------------- update_index


class TestUpdateIndex(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.index_path = Path(self._tmp.name) / "index.md"
        self.addCleanup(self._tmp.cleanup)
        self.started = datetime(2026, 1, 2, 3, 4, 5, tzinfo=bench.JST)
        self.args = types.SimpleNamespace(repeat=10)

    @staticmethod
    def _rows(prod_cpu, stg_cpu, weight="light"):
        return [
            {
                "case": "station",
                "weight": weight,
                "targets": {
                    "production": {"cpu_mean": prod_cpu},
                    "staging": {"cpu_mean": stg_cpu},
                },
            }
        ]

    def test_creates_index_with_header_and_summary_row(self):
        bench.update_index(self.index_path, "20260102-030405", self.started, self._rows(10.0, 8.0), {}, self.args)
        content = self.index_path.read_text(encoding="utf-8")
        self.assertIn("# 実行履歴", content)
        self.assertIn("[2026-01-02 03:04](./20260102-030405.md)", content)
        self.assertIn("CPU 中央比", content)

    def test_baseline_rows_are_excluded_from_the_summary(self):
        rows = self._rows(10.0, 8.0, weight="baseline")
        bench.update_index(self.index_path, "run1", self.started, rows, {}, self.args)
        content = self.index_path.read_text(encoding="utf-8")
        self.assertIn("CPU Time 欠測", content)

    def test_missing_cpu_time_is_reported_as_such(self):
        rows = [
            {
                "case": "ping",
                "weight": "baseline",
                "targets": {"production": {"cpu_mean": None}, "staging": {"cpu_mean": None}},
            }
        ]
        bench.update_index(self.index_path, "run1", self.started, rows, {}, self.args)
        self.assertIn("CPU Time 欠測", self.index_path.read_text(encoding="utf-8"))

    def test_rerunning_the_same_run_id_replaces_the_line_instead_of_duplicating(self):
        bench.update_index(self.index_path, "runA", self.started, self._rows(10.0, 8.0), {}, self.args)
        bench.update_index(self.index_path, "runA", self.started, self._rows(10.0, 40.0), {}, self.args)
        content = self.index_path.read_text(encoding="utf-8")
        self.assertEqual(content.count("runA.md"), 1)
        self.assertIn("最遅", content)


# --------------------------------------------------------------------------- rerender


class TestRerender(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.tmp_path = Path(self._tmp.name)
        self.addCleanup(self._tmp.cleanup)
        (self.tmp_path / "schema").mkdir()
        self._patcher = mock.patch.object(bench, "REPO_ROOT", self.tmp_path)
        self._patcher.start()
        self.addCleanup(self._patcher.stop)

        self.queries_path = self.tmp_path / "queries.json"
        self.queries_path.write_text(
            json.dumps(
                {
                    "fragments": {},
                    "cases": [
                        {
                            "name": "ping",
                            "kind": "http",
                            "method": "GET",
                            "path": "/__ping",
                            "weight": "baseline",
                            "note": "疎通",
                        },
                        {
                            "name": "station",
                            "weight": "light",
                            "query": "query Q { station(id: 1) { id } }",
                            "note": "駅取得",
                        },
                    ],
                }
            ),
            encoding="utf-8",
        )

        self.out_dir = self.tmp_path / "benchmarks"
        self.raw_dir = self.out_dir / "raw"
        self.raw_dir.mkdir(parents=True)
        self.run_id = "20260102-030405"
        self.raw_path = self.raw_dir / f"{self.run_id}.json"

        started = datetime(2026, 1, 2, 3, 4, 5, tzinfo=bench.JST)
        finished = started + timedelta(seconds=10)
        samples = [
            {"case": "ping", "target": "production", "client_ms": 5.0, "bytes": 2},
            {"case": "ping", "target": "staging", "client_ms": 6.0, "bytes": 2},
            {
                "case": "station",
                "target": "production",
                "client_ms": 30.0,
                "bytes": 500,
                "cpu_ms": 10,
                "worker_wall_ms": 11,
            },
            {
                "case": "station",
                "target": "staging",
                "client_ms": 60.0,
                "bytes": 500,
                "cpu_ms": 20,
                "worker_wall_ms": 21,
            },
        ]
        self.raw_path.write_text(
            json.dumps(
                {
                    "run_id": self.run_id,
                    "started_at": started.isoformat(),
                    "finished_at": finished.isoformat(),
                    "args": {
                        "repeat": 10,
                        "warmup": 2,
                        "note": "",
                        "queries": str(self.queries_path),
                    },
                    "targets": bench.TARGETS,
                    "meta": {"cpu_time_available": True, "uncovered_query_fields": []},
                    "samples": samples,
                    "summary": [],
                },
                ensure_ascii=False,
            ),
            encoding="utf-8",
        )

    def test_rerender_writes_report_and_index_without_making_requests(self):
        args = types.SimpleNamespace(rerender=self.raw_path)
        result = bench.rerender(args)

        self.assertEqual(result, 0)
        report = self.out_dir / f"{self.run_id}.md"
        self.assertTrue(report.exists())
        content = report.read_text(encoding="utf-8")
        self.assertIn("`station`", content)
        self.assertIn("## 所見", content)
        self.assertTrue((self.out_dir / "index.md").exists())

    def test_rerender_preserves_hand_written_findings_on_a_second_run(self):
        args = types.SimpleNamespace(rerender=self.raw_path)
        bench.rerender(args)
        report = self.out_dir / f"{self.run_id}.md"
        original = report.read_text(encoding="utf-8")
        custom_note = "調査の結果、trainRoute の実装差が原因と判明した。"
        updated = original.replace(
            "<!-- 実行者が記入する。差が出たクエリについて、実装のどこが効いているかを書く。 -->",
            custom_note,
        )
        report.write_text(updated, encoding="utf-8")

        bench.rerender(args)  # 生データは変えずに集計だけやり直す

        final = report.read_text(encoding="utf-8")
        self.assertIn(custom_note, final)

    def test_rerender_rejects_a_path_outside_the_raw_directory(self):
        bad_path = self.tmp_path / "elsewhere" / f"{self.run_id}.json"
        bad_path.parent.mkdir(parents=True)
        bad_path.write_text("{}", encoding="utf-8")
        args = types.SimpleNamespace(rerender=bad_path)
        with self.assertRaises(SystemExit):
            bench.rerender(args)


# --------------------------------------------------------------------------- self_test


class TestSelfTest(unittest.TestCase):
    def test_passes_against_the_real_repository_queries_and_schema(self):
        result = bench.self_test(SKILL_DIR / "queries.json")
        self.assertEqual(result, 0)

    def test_fails_when_the_schema_file_is_missing(self):
        with tempfile.TemporaryDirectory() as d:
            tmp_path = Path(d)
            (tmp_path / "schema").mkdir()
            with mock.patch.object(bench, "REPO_ROOT", tmp_path):
                result = bench.self_test(SKILL_DIR / "queries.json")
        self.assertEqual(result, 1)

    def test_fails_when_the_queries_file_does_not_cover_the_schema(self):
        with tempfile.TemporaryDirectory() as d:
            tmp_path = Path(d)
            schema_dir = tmp_path / "schema"
            schema_dir.mkdir()
            (schema_dir / "public.graphql").write_text(
                "type Query {\n\tfieldA(x: Int): String\n\tfieldB(y: Int): String\n}\n",
                encoding="utf-8",
            )
            incomplete_queries = tmp_path / "queries.json"
            incomplete_queries.write_text(
                json.dumps(
                    {
                        "fragments": {},
                        "cases": [
                            {
                                "name": "onlyA",
                                "weight": "light",
                                "query": "query Q { fieldA(x: 1) }",
                            }
                        ],
                    }
                ),
                encoding="utf-8",
            )
            with mock.patch.object(bench, "REPO_ROOT", tmp_path):
                result = bench.self_test(incomplete_queries)
        self.assertEqual(result, 1)


# --------------------------------------------------------------------------- CLI wiring


class TestCliSelfTestInvocation(unittest.TestCase):
    def test_self_test_flag_exits_zero_via_the_real_cli(self):
        result = subprocess.run(
            [sys.executable, str(SKILL_DIR / "bench.py"), "--self-test"],
            cwd=str(bench.REPO_ROOT),
            capture_output=True,
            text=True,
            timeout=30,
        )
        self.assertEqual(result.returncode, 0, result.stderr)


# --------------------------------------------------------------------------- queries.json fixture


class TestQueriesJsonFixture(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.doc = json.loads((SKILL_DIR / "queries.json").read_text(encoding="utf-8"))

    def test_has_fragments_and_cases(self):
        self.assertIn("fragments", self.doc)
        self.assertIn("cases", self.doc)
        self.assertGreater(len(self.doc["cases"]), 0)

    def test_case_names_are_unique(self):
        names = [c["name"] for c in self.doc["cases"]]
        self.assertEqual(len(names), len(set(names)), "重複したケース名がある")

    def test_every_non_http_case_only_uses_defined_fragments(self):
        fragments = self.doc["fragments"]
        for case in self.doc["cases"]:
            if case.get("kind") == "http":
                continue
            for used in case.get("uses", []):
                with self.subTest(case=case["name"], fragment=used):
                    self.assertIn(used, fragments)

    def test_every_case_declares_a_known_weight(self):
        allowed = {"baseline", "light", "medium", "heavy"}
        for case in self.doc["cases"]:
            with self.subTest(case=case["name"]):
                self.assertIn(case.get("weight"), allowed)

    def test_non_http_cases_define_a_non_empty_query_string(self):
        for case in self.doc["cases"]:
            if case.get("kind") == "http":
                continue
            with self.subTest(case=case["name"]):
                self.assertIn("query", case)
                self.assertTrue(case["query"].strip())

    def test_file_loads_cleanly_through_load_cases(self):
        cases, uncovered = bench.load_cases(SKILL_DIR / "queries.json", None, False)
        self.assertEqual(uncovered, [])
        self.assertEqual(len(cases), len(self.doc["cases"]))


# --------------------------------------------------------------------------- Makefile / .gitignore wiring


class TestMakefileBenchTarget(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.makefile = (bench.REPO_ROOT / "Makefile").read_text(encoding="utf-8")

    def test_bench_is_declared_phony(self):
        phony_line = next(line for line in self.makefile.splitlines() if line.startswith(".PHONY"))
        self.assertIn("bench", phony_line.split())

    def test_bench_target_invokes_the_skill_script(self):
        self.assertIn("bench:", self.makefile)
        idx = self.makefile.index("bench:")
        block = self.makefile[idx : idx + 600]
        self.assertIn(".claude/skills/benchmark-gql/bench.py", block)
        self.assertIn("$(BENCH_ARGS)", block)

    def test_help_target_documents_bench(self):
        help_lines = [line for line in self.makefile.splitlines() if "@echo" in line and "bench" in line]
        self.assertTrue(
            any("Compare production vs staging GraphQL performance" in line for line in help_lines)
        )


class TestGitignoreCoversBenchLogs(unittest.TestCase):
    def test_gitignore_excludes_wrangler_tail_logs(self):
        content = (bench.REPO_ROOT / ".gitignore").read_text(encoding="utf-8")
        self.assertIn("benchmarks/.logs/", content)


# --------------------------------------------------------------------------- Docs consistency


class TestSkillDocConsistency(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.skill_md = (SKILL_DIR / "SKILL.md").read_text(encoding="utf-8")

    def test_frontmatter_declares_name_and_description(self):
        self.assertTrue(self.skill_md.startswith("---\n"))
        frontmatter_end = self.skill_md.index("\n---\n", 4)
        frontmatter = self.skill_md[:frontmatter_end]
        self.assertIn("name: benchmark-gql", frontmatter)
        self.assertIn("description:", frontmatter)

    def test_references_bench_script_and_queries_file_that_exist(self):
        self.assertIn("bench.py", self.skill_md)
        self.assertTrue((SKILL_DIR / "bench.py").exists())
        self.assertIn("queries.json", self.skill_md)
        self.assertTrue((SKILL_DIR / "queries.json").exists())

    def test_documents_the_self_test_and_rerender_flags(self):
        self.assertIn("--self-test", self.skill_md)
        self.assertIn("--rerender", self.skill_md)


class TestBenchmarksReadmeConsistency(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.readme_path = bench.REPO_ROOT / "benchmarks" / "README.md"
        cls.readme = cls.readme_path.read_text(encoding="utf-8")

    def test_relative_link_to_skill_doc_resolves(self):
        self.assertIn("../.claude/skills/benchmark-gql/SKILL.md", self.readme)
        target = (self.readme_path.parent / "../.claude/skills/benchmark-gql/SKILL.md").resolve()
        self.assertTrue(target.exists())

    def test_mentions_index_and_raw_output_paths(self):
        self.assertIn("index.md", self.readme)
        self.assertIn("raw/", self.readme)


class TestAgentsMdDocumentsBench(unittest.TestCase):
    def test_agents_md_mentions_make_bench(self):
        content = (bench.REPO_ROOT / "AGENTS.md").read_text(encoding="utf-8")
        self.assertIn("make bench", content)
        self.assertIn("benchmark-gql", content)


if __name__ == "__main__":
    unittest.main()