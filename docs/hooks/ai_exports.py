from __future__ import annotations

import json
import re
from collections import OrderedDict
from pathlib import Path
from typing import Any, Iterable


def on_config(config: Any, **_: Any) -> Any:
    docs_dir = Path(str(config["docs_dir"]))
    site_name = str(config.get("site_name") or "Documentation")
    site_description = str(config.get("site_description") or "").strip()
    site_url = _normalize_site_url(str(config.get("site_url") or ""))
    repo_url = str(config.get("repo_url") or "").rstrip("/")

    extra = dict(config.get("extra") or {})
    docs_branch = str(extra.get("docs_branch") or "main")
    raw_docs_base_url = _derive_raw_docs_base_url(repo_url, docs_branch)
    extra["raw_docs_base_url"] = raw_docs_base_url
    config["extra"] = extra

    pages = _collect_pages(
        nav_items=config.get("nav") or [],
        docs_dir=docs_dir,
        site_description=site_description,
        site_url=site_url,
        raw_docs_base_url=raw_docs_base_url,
    )

    _write_if_changed(
        docs_dir / "llms.txt",
        _render_llms_txt(
            site_name=site_name,
            site_description=site_description,
            site_url=site_url,
            repo_url=repo_url,
            pages=pages,
        ),
    )
    _write_if_changed(
        docs_dir / "llms-full.txt",
        _render_llms_full_txt(
            site_name=site_name,
            site_description=site_description,
            site_url=site_url,
            repo_url=repo_url,
            pages=pages,
        ),
    )

    manifest = {
        "site_name": site_name,
        "site_description": site_description,
        "site_url": site_url,
        "repo_url": repo_url,
        "llms_txt_url": f"{site_url}llms.txt",
        "llms_full_url": f"{site_url}llms-full.txt",
        "pages": {page["route"]: page for page in pages},
    }
    _write_if_changed(
        docs_dir / "assets" / "ai" / "page-manifest.json",
        json.dumps(manifest, ensure_ascii=True, indent=2) + "\n",
    )
    return config


def _collect_pages(
    *,
    nav_items: list[Any],
    docs_dir: Path,
    site_description: str,
    site_url: str,
    raw_docs_base_url: str,
) -> list[dict[str, str]]:
    pages: list[dict[str, str]] = []

    for section, nav_title, src_path in _iter_nav(nav_items):
        if not src_path.endswith(".md"):
            continue

        doc_path = docs_dir / src_path
        if not doc_path.exists():
            continue

        markdown = doc_path.read_text(encoding="utf-8").strip() + "\n"
        title = _extract_title(markdown, nav_title)
        summary_fallback = site_description if src_path == "index.md" else nav_title
        summary = _extract_summary(markdown, summary_fallback)
        route = _route_from_src_path(src_path)
        canonical_url = site_url if not route else f"{site_url}{route}/"

        pages.append(
            {
                "section": section,
                "title": title,
                "nav_title": nav_title,
                "route": route,
                "source_path": src_path,
                "canonical_url": canonical_url,
                "markdown_url": f"{raw_docs_base_url}{src_path}",
                "summary": summary,
                "markdown": markdown,
            }
        )

    return pages


def _iter_nav(items: Iterable[Any], section: str | None = None) -> Iterable[tuple[str, str, str]]:
    current_section = section or "Overview"

    for item in items:
        if isinstance(item, str):
            yield current_section, _title_from_path(item), item
            continue

        if not isinstance(item, dict):
            continue

        for label, value in item.items():
            if isinstance(value, str):
                yield current_section if section else "Overview", str(label), value
                continue

            if isinstance(value, list):
                yield from _iter_nav(value, section=str(label))


def _title_from_path(path: str) -> str:
    stem = Path(path).stem
    if stem == "index":
        return "Home"
    return stem.replace("-", " ").replace("_", " ").title()


def _extract_title(markdown: str, fallback: str) -> str:
    for line in markdown.splitlines():
        stripped = line.strip()
        if stripped.startswith("# "):
            return stripped[2:].strip()
    return fallback


def _extract_summary(markdown: str, fallback: str) -> str:
    content = _strip_front_matter(markdown)
    in_code_block = False
    paragraph_lines: list[str] = []

    for raw_line in content.splitlines():
        line = raw_line.strip()

        if line.startswith("```"):
            in_code_block = not in_code_block
            continue

        if in_code_block:
            continue

        if not line:
            if paragraph_lines:
                break
            continue

        if (
            line.startswith("#")
            or line.startswith(">")
            or line.startswith("|")
            or line.startswith("- ")
            or line.startswith("* ")
            or line.startswith("<")
            or re.match(r"^\d+\.\s", line)
        ):
            if paragraph_lines:
                break
            continue

        paragraph_lines.append(line)

    if not paragraph_lines:
        return fallback

    summary = " ".join(paragraph_lines)
    summary = re.sub(r"`([^`]+)`", r"\1", summary)
    summary = re.sub(r"\[([^\]]+)\]\([^)]+\)", r"\1", summary)
    summary = re.sub(r"\s+", " ", summary)
    return summary.strip() or fallback


def _strip_front_matter(markdown: str) -> str:
    if not markdown.startswith("---\n"):
        return markdown
    _, _, remainder = markdown.partition("\n---\n")
    return remainder or markdown


def _route_from_src_path(src_path: str) -> str:
    if src_path == "index.md":
        return ""
    return src_path[:-3]


def _normalize_site_url(site_url: str) -> str:
    return site_url.rstrip("/") + "/"


def _derive_raw_docs_base_url(repo_url: str, docs_branch: str) -> str:
    if repo_url.startswith("https://github.com/"):
        repo_path = repo_url.removeprefix("https://github.com/").rstrip("/")
        return f"https://raw.githubusercontent.com/{repo_path}/{docs_branch}/docs/"
    return repo_url.rstrip("/") + f"/raw/{docs_branch}/docs/"


def _render_llms_txt(
    *,
    site_name: str,
    site_description: str,
    site_url: str,
    repo_url: str,
    pages: list[dict[str, str]],
) -> str:
    lines = [
        f"# {site_name} Docs For AI",
        "",
        f"> {site_description}",
        "",
        f"Canonical site: {site_url}",
        f"Repository: {repo_url}",
        f"Full export: {site_url}llms-full.txt",
        "",
        "Prefer the raw Markdown URLs for ingestion when possible. Each documentation page also exposes AI actions in the rendered site UI.",
        "",
        "## Documentation Index",
        "",
    ]

    grouped: OrderedDict[str, list[dict[str, str]]] = OrderedDict()
    for page in pages:
        grouped.setdefault(page["section"], []).append(page)

    for section, section_pages in grouped.items():
        lines.append(f"### {section}")
        lines.append("")
        for page in section_pages:
            lines.append(f"- [{page['title']}]({page['canonical_url']}): {page['summary']}")
            lines.append(f"  Markdown: {page['markdown_url']}")
        lines.append("")

    return "\n".join(lines).rstrip() + "\n"


def _render_llms_full_txt(
    *,
    site_name: str,
    site_description: str,
    site_url: str,
    repo_url: str,
    pages: list[dict[str, str]],
) -> str:
    lines = [
        f"# {site_name} Full Documentation Export",
        "",
        f"> {site_description}",
        "",
        f"Canonical site: {site_url}",
        f"Repository: {repo_url}",
        f"Index: {site_url}llms.txt",
        "",
        "This file concatenates the current documentation source Markdown so AI systems can fetch the docs corpus in one request.",
        "",
        "## Page Index",
        "",
    ]

    for page in pages:
        lines.append(f"- [{page['title']}]({page['canonical_url']}): {page['summary']}")

    for page in pages:
        lines.extend(
            [
                "",
                "---",
                "",
                f"## {page['title']}",
                "",
                f"Canonical URL: {page['canonical_url']}",
                f"Markdown URL: {page['markdown_url']}",
                f"Source path: docs/{page['source_path']}",
                "",
                page["markdown"].rstrip(),
                "",
            ]
        )

    return "\n".join(lines).rstrip() + "\n"


def _write_if_changed(path: Path, content: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    if path.exists() and path.read_text(encoding="utf-8") == content:
        return
    path.write_text(content, encoding="utf-8")
