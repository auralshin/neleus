#!/usr/bin/env python3
"""Validate distribution metadata and published package metadata."""

from __future__ import annotations

import argparse
import json
import tarfile
import time
import urllib.error
import urllib.request
import zipfile
from email.parser import Parser
from pathlib import Path
from typing import Iterable


def _parse_metadata(text: str):
    return Parser().parsestr(text)


def _extract_wheel_metadata(path: Path) -> tuple[str, list[str]]:
    with zipfile.ZipFile(path) as wheel:
        metadata_name = next(
            info.filename for info in wheel.infolist() if info.filename.endswith(".dist-info/METADATA")
        )
        text = wheel.read(metadata_name).decode("utf-8")
        names = [info.filename for info in wheel.infolist()]
    return text, names


def _extract_sdist_metadata(path: Path) -> tuple[str, list[str]]:
    with tarfile.open(path, "r:gz") as archive:
        member = next(item for item in archive.getmembers() if item.name.endswith("/PKG-INFO"))
        handle = archive.extractfile(member)
        assert handle is not None
        text = handle.read().decode("utf-8")
        names = archive.getnames()
    return text, names


def _parse_project_urls(headers: Iterable[str]) -> dict[str, str]:
    urls: dict[str, str] = {}
    for header in headers:
        if "," not in header:
            continue
        label, url = header.split(",", 1)
        urls[label.strip()] = url.strip()
    return urls


def _assert_metadata(
    metadata_text: str,
    *,
    expect_name: str,
    expect_docs_url: str,
    expect_repo_url: str,
    expect_marker: str,
) -> None:
    metadata = _parse_metadata(metadata_text)
    description = metadata.get_payload()
    content_type = metadata.get("Description-Content-Type", "")
    urls = _parse_project_urls(metadata.get_all("Project-URL", []))

    if metadata.get("Name") != expect_name:
        raise SystemExit(f"Expected package name {expect_name!r}, got {metadata.get('Name')!r}")
    if not metadata.get("Summary"):
        raise SystemExit("Package summary is missing")
    if not description or expect_marker not in description:
        raise SystemExit(f"Package description is missing expected marker {expect_marker!r}")
    if "text/markdown" not in content_type:
        raise SystemExit(f"Expected markdown description content type, got {content_type!r}")
    if urls.get("Documentation") != expect_docs_url:
        raise SystemExit(f"Documentation URL mismatch: {urls.get('Documentation')!r}")
    if urls.get("Repository") != expect_repo_url:
        raise SystemExit(f"Repository URL mismatch: {urls.get('Repository')!r}")


def check_artifacts(
    paths: list[Path], *, expect_name: str, expect_docs_url: str, expect_repo_url: str, expect_marker: str
) -> None:
    for path in paths:
        if path.suffix == ".whl":
            metadata_text, _ = _extract_wheel_metadata(path)
        elif path.name.endswith(".tar.gz"):
            metadata_text, _ = _extract_sdist_metadata(path)
        else:
            raise SystemExit(f"Unsupported distribution artifact: {path}")
        _assert_metadata(
            metadata_text,
            expect_name=expect_name,
            expect_docs_url=expect_docs_url,
            expect_repo_url=expect_repo_url,
            expect_marker=expect_marker,
        )
        print(f"{path}: metadata OK")


def check_wheel_contents(paths: list[Path]) -> None:
    for path in paths:
        _, names = _extract_wheel_metadata(path)
        offenders = [
            name
            for name in names
            if "__pycache__" in name.split("/")
            or name.endswith((".pyc", ".pyo"))
            or ".dist-info/sboms/" in name
        ]
        if offenders:
            joined = "\n".join(f"  - {name}" for name in offenders)
            raise SystemExit(f"{path} contains unwanted entries:\n{joined}")
        print(f"{path}: wheel contents OK")


def check_index_metadata(
    *,
    base_url: str,
    package_name: str,
    package_version: str,
    expect_docs_url: str,
    expect_repo_url: str,
    expect_marker: str,
    retries: int,
    delay: int,
) -> None:
    url = f"{base_url.rstrip('/')}/pypi/{package_name}/{package_version}/json"
    last_error: str | None = None

    for attempt in range(1, retries + 1):
        try:
            with urllib.request.urlopen(url) as response:
                payload = json.load(response)
            info = payload["info"]
            description = info.get("description", "")
            content_type = info.get("description_content_type", "")
            project_urls = info.get("project_urls") or {}

            if not description or expect_marker not in description:
                raise ValueError("published description is missing expected content")
            if "text/markdown" not in content_type:
                raise ValueError(f"unexpected description_content_type={content_type!r}")
            if project_urls.get("Documentation") != expect_docs_url:
                raise ValueError(f"unexpected Documentation URL: {project_urls.get('Documentation')!r}")
            if project_urls.get("Repository") != expect_repo_url:
                raise ValueError(f"unexpected Repository URL: {project_urls.get('Repository')!r}")

            print(f"{url}: published metadata OK")
            return
        except (urllib.error.URLError, urllib.error.HTTPError, ValueError, KeyError) as exc:
            last_error = str(exc)
            if attempt == retries:
                break
            print(f"{url}: metadata not ready yet (attempt {attempt}/{retries}): {exc}")
            time.sleep(delay)

    raise SystemExit(f"Failed to validate published metadata at {url}: {last_error}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    subparsers = parser.add_subparsers(dest="command", required=True)

    artifact = subparsers.add_parser("artifact-metadata")
    artifact.add_argument("paths", nargs="+", type=Path)
    artifact.add_argument("--expect-name", required=True)
    artifact.add_argument("--expect-docs-url", required=True)
    artifact.add_argument("--expect-repo-url", required=True)
    artifact.add_argument("--expect-marker", default="# Neleus")

    wheel = subparsers.add_parser("wheel-contents")
    wheel.add_argument("paths", nargs="+", type=Path)

    index = subparsers.add_parser("index-metadata")
    index.add_argument("--base-url", required=True)
    index.add_argument("--package-name", required=True)
    index.add_argument("--package-version", required=True)
    index.add_argument("--expect-docs-url", required=True)
    index.add_argument("--expect-repo-url", required=True)
    index.add_argument("--expect-marker", default="# Neleus")
    index.add_argument("--retries", type=int, default=20)
    index.add_argument("--delay", type=int, default=15)

    args = parser.parse_args()

    if args.command == "artifact-metadata":
        check_artifacts(
            args.paths,
            expect_name=args.expect_name,
            expect_docs_url=args.expect_docs_url,
            expect_repo_url=args.expect_repo_url,
            expect_marker=args.expect_marker,
        )
    elif args.command == "wheel-contents":
        check_wheel_contents(args.paths)
    else:
        check_index_metadata(
            base_url=args.base_url,
            package_name=args.package_name,
            package_version=args.package_version,
            expect_docs_url=args.expect_docs_url,
            expect_repo_url=args.expect_repo_url,
            expect_marker=args.expect_marker,
            retries=args.retries,
            delay=args.delay,
        )


if __name__ == "__main__":
    main()
