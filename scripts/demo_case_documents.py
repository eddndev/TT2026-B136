#!/usr/bin/env python3
"""Build the fictitious demonstration case PDFs from their Markdown sources.

Each source under docs/usability/expediente-demostracion/documentos/ starts
with a metadata block between '---' lines (repeatable 'encabezado' lines and
single 'id', 'archivo', 'titulo', 'referencia' and 'lugar_fecha' values),
followed by a small Markdown subset: '# ' title, '## ' heading, '- ' bullet,
'> ' signature line and blank-line separated paragraphs. '{{name}}' markers
are replaced with values given on the command line (dates of documents that
are created relative to the load time); a marker without a value is an error.
"""
import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

from demo_case_pdf import Layout, render

FOOTER = "Expediente ficticio de demostraci\u00f3n \u2014 Qadra TT2026-B136"
MARKER = re.compile(r"\{\{([a-z0-9_]+)\}\}")
REPEATABLE = {"encabezado"}
REQUIRED = ("id", "archivo", "titulo")


class SourceError(ValueError):
    """A document source does not follow the expected format."""


def substitute(text, values, origin):
    def replace(match):
        name = match.group(1)
        if name not in values:
            raise SourceError(f"{origin}: no value for marker {{{{{name}}}}}")
        return values[name]
    return MARKER.sub(replace, text)


def parse(text, origin="document", values=None):
    """Return (metadata, blocks) for one Markdown source."""
    text = substitute(text, values or {}, origin)
    lines = text.split("\n")
    if not lines or lines[0].strip() != "---":
        raise SourceError(f"{origin}: missing metadata block")
    try:
        end = lines.index("---", 1)
    except ValueError as error:
        raise SourceError(f"{origin}: unterminated metadata block") from error
    meta = {"encabezado": []}
    for raw in lines[1:end]:
        if not raw.strip():
            continue
        key, separator, value = raw.partition(":")
        key, value = key.strip(), value.strip()
        if not separator or not key or not value:
            raise SourceError(f"{origin}: invalid metadata line {raw!r}")
        if key in REPEATABLE:
            meta[key].append(value)
        elif key in meta:
            raise SourceError(f"{origin}: repeated metadata key {key!r}")
        else:
            meta[key] = value
    for key in REQUIRED:
        if key not in meta:
            raise SourceError(f"{origin}: missing metadata key {key!r}")
    if not re.fullmatch(r"[A-Za-z0-9._-]{1,128}", meta["archivo"]) or not meta["archivo"].endswith(".pdf"):
        raise SourceError(f"{origin}: file name must be an ASCII .pdf name of at most 128 bytes")
    blocks, paragraph = [], []

    def flush():
        if paragraph:
            blocks.append(("p", " ".join(part.strip() for part in paragraph)))
            paragraph.clear()

    for raw in lines[end + 1:]:
        line = raw.rstrip()
        if not line.strip():
            flush()
        elif line.startswith("# "):
            flush()
            blocks.append(("title", line[2:].strip()))
        elif line.startswith("## "):
            flush()
            blocks.append(("h", line[3:].strip()))
        elif line.startswith("- "):
            flush()
            blocks.append(("li", line[2:].strip()))
        elif line.startswith(">"):
            flush()
            blocks.append(("sign", line[1:].strip()))
        else:
            paragraph.append(line)
    flush()
    return meta, blocks


def layout_document(meta, blocks):
    layout = Layout(FOOTER)
    for number, header in enumerate(meta["encabezado"]):
        layout.line(header, "Times-Bold", 12 if number == 0 else 11, align="center")
    if meta["encabezado"]:
        layout.rule()
    if "referencia" in meta:
        layout.line(meta["referencia"], "Times-Italic", 10, align="right")
    if "lugar_fecha" in meta:
        layout.line(meta["lugar_fecha"], "Times-Roman", 11, align="right")
    layout.space(10)
    signing = False
    for kind, text in blocks:
        if kind == "title":
            layout.space(4)
            if len(text) > 70:
                layout.paragraph(text, "Times-Bold", 13, justify=False)
            else:
                layout.line(text, "Times-Bold", 13, align="center")
            layout.space(8)
        elif kind == "h":
            layout.space(6)
            layout.ensure(40)
            layout.line(text, "Times-Bold", 11.5)
            layout.space(2)
        elif kind == "li":
            layout.bullet(text)
        elif kind == "sign":
            if not signing:
                layout.ensure(80)
                layout.space(36)
                layout.line("_" * 34, align="center")
                signing = True
            if text:
                layout.line(text, "Times-Roman", 11, align="center")
            else:
                signing = False
        else:
            signing = False
            layout.paragraph(text, first_indent=24)
            layout.space(4)
    return layout


def build(source, values=None):
    """Return (metadata, pdf_bytes) for one source file."""
    meta, blocks = parse(source.read_text(encoding="utf-8"), str(source), values)
    return meta, render(layout_document(meta, blocks), meta["titulo"])


def source_id(text):
    match = re.search(r"^id:\s*(\S+)\s*$", text, re.MULTILINE)
    return match.group(1) if match else None


def build_directory(source_dir, output_dir, values=None, only=None, skip_live=False):
    """Write the selected PDFs into output_dir and return the manifest entries.

    Sources with markers are live documents; skip_live leaves them out when no
    load-time values exist yet.
    """
    output_dir.mkdir(parents=True, exist_ok=True)
    manifest = []
    for source in sorted(source_dir.glob("*.md")):
        text = source.read_text(encoding="utf-8")
        if only and source_id(text) not in only:
            continue
        if skip_live and MARKER.search(text):
            continue
        meta, data = build(source, values)
        target = output_dir / meta["archivo"]
        target.write_bytes(data)
        manifest.append({"id": meta["id"], "file": meta["archivo"], "title": meta["titulo"],
                         "sha256": hashlib.sha256(data).hexdigest(), "bytes": len(data)})
    return manifest


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--source", type=Path, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--values", type=Path, help="JSON object with marker values")
    parser.add_argument("--only", nargs="*", help="document ids to build")
    parser.add_argument("--skip-live", action="store_true", help="omit sources with load-time markers")
    args = parser.parse_args(argv)
    values = json.loads(args.values.read_text(encoding="utf-8")) if args.values else {}
    try:
        manifest = build_directory(args.source, args.out, values, set(args.only or []), args.skip_live)
    except (SourceError, ValueError) as error:
        print(f"demo_case_documents: {error}", file=sys.stderr)
        return 1
    json.dump(manifest, sys.stdout, indent=1, ensure_ascii=True)
    print()
    return 0


if __name__ == "__main__":
    sys.exit(main())
