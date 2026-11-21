"""Stage official Saitama Open Data feeds for the Worker build."""
import csv
import hashlib
import io
import json
import os
import shutil
import tempfile
from datetime import datetime
from http.client import HTTPException
from pathlib import Path
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, urlopen
from zipfile import BadZipFile, ZipFile
from zoneinfo import ZoneInfo

PORTAL = "https://opendata.pref.saitama.lg.jp/resource_download/"
CATALOG = "https://odm.bodik.jp/api/3/action/package_show"
ARCHIVES = Path("data/gtfs-archives/saitama")
CACHE = Path("data/gtfs-cache")
REQUIRED = {"routes.txt", "stops.txt", "trips.txt", "stop_times.txt"}
USER_AGENT = "TrainLCD/StationAPI (GTFS data importer)"


def get(url):
    with urlopen(Request(url, headers={"User-Agent": USER_AGENT}), timeout=60) as response:
        return response.read()


def current_archive(url, resource):
    """On a publisher 403, check its harvested revision before using the original ZIP."""
    manifest = json.loads((ARCHIVES / f"{resource}.json").read_text(encoding="utf-8"))
    query = urlencode({"id": manifest["catalog"]})
    response = json.loads(get(f"{CATALOG}?{query}"))
    dataset = response["result"]
    dates = [e["value"] for e in dataset["extras"] if e["key"] == "updatedat"]
    urls = {r["url"] for r in dataset["resources"]}
    if (not response["success"] or dataset["state"] != "active"
            or dataset["license_id"] != "pdl" or dates != [manifest["updated"]]
            or url != manifest["source"] or url not in urls):
        raise ValueError(f"Publisher metadata changed for {url}. Recheck the official ZIP.")
    data = (ARCHIVES / f"{resource}.zip").read_bytes()
    if hashlib.sha256(data).hexdigest() != manifest["sha256"]:
        raise ValueError(f"Official ZIP checksum mismatch for {url}")
    with ZipFile(io.BytesIO(data)) as archive, archive.open("feed_info.txt") as info:
        rows = list(csv.DictReader(io.TextIOWrapper(info, encoding="utf-8-sig")))
    if len(rows) != 1:
        raise ValueError(f"Expected one publisher record for {url}")
    start, end = (datetime.strptime(rows[0][key], "%Y%m%d").date()
                  for key in ("feed_start_date", "feed_end_date"))
    if not start <= datetime.now(ZoneInfo("Asia/Tokyo")).date() <= end:
        raise ValueError(f"Official edition outside {start} to {end} for {url}")
    print(f"Publisher returned HTTP 403. Using verified {manifest['updated']} edition through {end}.")
    return data


def stage(registration, url, resource):
    try:
        data = get(url)
    except HTTPError as error:
        if error.code != 403 or not (ARCHIVES / f"{resource}.json").is_file():
            raise
        data = current_archive(url, resource)
    digest = hashlib.sha256(data).hexdigest()
    print(f"{registration.stem} {url} SHA-256 {digest}", flush=True)
    destination = CACHE / registration.stem
    CACHE.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=CACHE) as stage_dir:
        extracted = Path(stage_dir) / "feed"
        extracted.mkdir()
        with ZipFile(io.BytesIO(data)) as archive:
            members = [m for m in archive.infolist()
                       if m.filename.endswith(".txt") and Path(m.filename).name == m.filename]
            names = {m.filename for m in members}
            if len(names) != len(members) or not REQUIRED <= names:
                raise ValueError(f"Invalid GTFS archive for {url}")
            for member in members:
                with archive.open(member) as source, (extracted / member.filename).open("xb") as out:
                    shutil.copyfileobj(source, out)
        previous = Path(stage_dir) / "previous"
        if destination.exists():
            destination.rename(previous)
        try:
            extracted.rename(destination)
        except OSError:
            if previous.exists():
                previous.rename(destination)
            raise


if __name__ == "__main__":
    for registration in sorted(Path("data/gtfs-feeds").glob("*.json")):
        url = json.loads(registration.read_text(encoding="utf-8"))["url"]
        resource = url.removeprefix(PORTAL)
        if url.startswith(PORTAL) and resource.isascii() and resource.isdigit():
            try:
                stage(registration, url, resource)
            except (OSError, ValueError, KeyError, BadZipFile, HTTPException) as error:
                # The Rust importer trusts an existing cache directory. Discard the old
                # edition after a failed refresh so it will retry the publisher URL.
                destination = CACHE / registration.stem
                if destination.is_symlink():
                    destination.unlink()
                elif destination.exists():
                    shutil.rmtree(destination)
                if os.environ.get("STRICT") == "true":
                    raise
                # No partial feed was staged. The normal importer can still try the live URL.
                print(f"::warning::{registration.stem}: {type(error).__name__}: GTFS preparation failed")
