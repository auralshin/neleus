#!/usr/bin/env python3
"""Remove non-runtime files from built wheels and regenerate RECORD."""

from __future__ import annotations

import argparse
import base64
import csv
import hashlib
import os
import tempfile
import zipfile
from pathlib import Path


def _should_skip(name: str) -> bool:
    parts = name.split("/")
    return (
        "__pycache__" in parts
        or name.endswith((".pyc", ".pyo"))
        or ".dist-info/sboms/" in name
    )


def _record_row(name: str, data: bytes) -> tuple[str, str, str]:
    digest = hashlib.sha256(data).digest()
    encoded = base64.urlsafe_b64encode(digest).decode("ascii").rstrip("=")
    return (name, f"sha256={encoded}", str(len(data)))


def sanitize_wheel(path: Path) -> None:
    removed: list[str] = []
    temp_path: Path | None = None
    try:
        with zipfile.ZipFile(path) as source:
            record_name = next(
                info.filename for info in source.infolist() if info.filename.endswith(".dist-info/RECORD")
            )
            with tempfile.NamedTemporaryFile(
                prefix=path.stem + "-", suffix=".whl", delete=False, dir=path.parent
            ) as handle:
                temp_path = Path(handle.name)

            with zipfile.ZipFile(temp_path, "w", compression=zipfile.ZIP_DEFLATED) as target:
                record_rows: list[tuple[str, str, str]] = []
                for info in source.infolist():
                    name = info.filename
                    if info.is_dir():
                        continue
                    if name == record_name:
                        continue
                    if _should_skip(name):
                        removed.append(name)
                        continue

                    data = source.read(name)
                    clone = zipfile.ZipInfo(name, date_time=info.date_time)
                    clone.compress_type = zipfile.ZIP_DEFLATED
                    clone.comment = info.comment
                    clone.extra = info.extra
                    clone.create_system = info.create_system
                    clone.create_version = info.create_version
                    clone.extract_version = info.extract_version
                    clone.flag_bits = info.flag_bits
                    clone.volume = info.volume
                    clone.internal_attr = info.internal_attr
                    clone.external_attr = info.external_attr
                    target.writestr(clone, data)
                    record_rows.append(_record_row(name, data))

                record_rows.append((record_name, "", ""))
                target.writestr(record_name, _serialize_record(record_rows))

        assert temp_path is not None
        os.replace(temp_path, path)
    finally:
        if temp_path is not None and temp_path.exists():
            temp_path.unlink()

    if removed:
        print(f"{path}: removed {len(removed)} unwanted entries")
        for name in removed:
            print(f"  - {name}")
    else:
        print(f"{path}: no unwanted entries found")


def _serialize_record(rows: list[tuple[str, str, str]]) -> str:
    lines: list[str] = []
    for row in rows:
        buffer = []
        with tempfile.TemporaryFile("w+") as temp:
            writer = csv.writer(temp, lineterminator="")
            writer.writerow(row)
            temp.seek(0)
            buffer.append(temp.read())
        lines.append(buffer[0])
    return "\n".join(lines) + "\n"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("wheels", nargs="+", type=Path)
    args = parser.parse_args()

    for wheel in args.wheels:
        sanitize_wheel(wheel)


if __name__ == "__main__":
    main()
