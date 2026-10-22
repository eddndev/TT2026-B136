"""Deterministic PDF writer for the fictitious demonstration case documents.

The writer lays out letter-size pages with the standard Times fonts in
WinAnsiEncoding, draws a diagonal "fictitious document" watermark under the
text and numbers the pages in the footer. It uses only the standard library,
embeds no fonts and records no creation time, so the same input always yields
the same bytes on any host.
"""
import hashlib
import math
import zlib

from demo_case_fonts import CODES, WIDTHS

PAGE_WIDTH = 612
PAGE_HEIGHT = 792
MARGIN = 72
TEXT_WIDTH = PAGE_WIDTH - 2 * MARGIN
FONT_KEYS = {"Times-Roman": "F1", "Times-Bold": "F2", "Times-Italic": "F3"}
WATERMARK = "DOCUMENTO FICTICIO \u2014 SIN VALOR LEGAL"
_INDEX = {code: position for position, code in enumerate(CODES)}


def encode(text):
    """Return the WinAnsi bytes of text or raise ValueError on any other character."""
    try:
        raw = text.encode("cp1252")
    except UnicodeEncodeError as error:
        raise ValueError(f"unsupported character {text[error.start]!r} in {text!r}") from error
    for byte in raw:
        if byte not in _INDEX:
            raise ValueError(f"unsupported character {bytes([byte]).decode('cp1252')!r} in {text!r}")
    return raw


def text_width(text, font, size):
    widths = WIDTHS[font]
    return sum(widths[_INDEX[byte]] for byte in encode(text)) * size / 1000.0


def wrap(text, font, size, width):
    """Split text into lines no wider than width, breaking only at spaces."""
    lines, current = [], ""
    for word in text.split():
        candidate = word if not current else current + " " + word
        if not current or text_width(candidate, font, size) <= width:
            current = candidate
        else:
            lines.append(current)
            current = word
    if current:
        lines.append(current)
    for line in lines:
        if text_width(line, font, size) > width:
            raise ValueError(f"word wider than the text column: {line!r}")
    return lines


def pdf_string(text):
    raw = encode(text)
    escaped = raw.replace(b"\\", b"\\\\").replace(b"(", b"\\(").replace(b")", b"\\)")
    return b"(" + escaped + b")"


def _number(value):
    text = f"{value:.3f}".rstrip("0").rstrip(".")
    return "0" if text in ("", "-0") else text


class Layout:
    """Flow text blocks over pages, keeping a vertical cursor per page."""

    def __init__(self, footer):
        self.footer = footer
        self.pages = []
        self._new_page()

    def _new_page(self):
        self.pages.append([])
        self.y = PAGE_HEIGHT - MARGIN

    def _ops(self):
        return self.pages[-1]

    def ensure(self, height):
        if self.y - height < MARGIN + 18:
            self._new_page()

    def space(self, height):
        self.y -= height

    def line(self, text, font="Times-Roman", size=11, align="left", leading=None, indent=0, word_spacing=0.0):
        leading = leading or size * 1.32
        self.ensure(leading)
        self.y -= leading
        width = text_width(text, font, size)
        if align == "center":
            x = MARGIN + (TEXT_WIDTH - width) / 2
        elif align == "right":
            x = MARGIN + TEXT_WIDTH - width
        else:
            x = MARGIN + indent
        spacing = f" {_number(word_spacing)} Tw" if word_spacing else " 0 Tw"
        self._ops().append(
            f"BT /{FONT_KEYS[font]} {_number(size)} Tf{spacing} {_number(x)} {_number(self.y)} Td ".encode()
            + pdf_string(text) + b" Tj ET")

    def paragraph(self, text, font="Times-Roman", size=11, indent=0, justify=True, first_indent=0):
        width = TEXT_WIDTH - indent
        lines = wrap(text, font, size, width - first_indent) if first_indent else wrap(text, font, size, width)
        if first_indent and len(lines) > 1:
            rest = " ".join(lines[1:])
            lines = lines[:1] + wrap(rest, font, size, width)
        for number, text_line in enumerate(lines):
            offset = indent + (first_indent if number == 0 else 0)
            spacing = 0.0
            spaces = text_line.count(" ")
            if justify and spaces and number < len(lines) - 1:
                available = width - (first_indent if number == 0 else 0)
                spacing = (available - text_width(text_line, font, size)) / spaces
            self.line(text_line, font, size, indent=offset, word_spacing=spacing)

    def bullet(self, text, size=11):
        lines = wrap(text, "Times-Roman", size, TEXT_WIDTH - 18)
        for number, text_line in enumerate(lines):
            if number == 0:
                self.ensure(size * 1.32)
                marker_y = self.y - size * 1.32
                self._ops().append(
                    f"BT /F1 {_number(size)} Tf 0 Tw {_number(MARGIN + 4)} {_number(marker_y)} Td ".encode()
                    + pdf_string("\u2013") + b" Tj ET")
            self.line(text_line, size=size, indent=18)

    def rule(self, width=0.6):
        self.ensure(8)
        self.y -= 6
        self._ops().append(
            f"{_number(width)} w {MARGIN} {_number(self.y)} m {MARGIN + TEXT_WIDTH} {_number(self.y)} l S".encode())
        self.y -= 4


def _watermark():
    angle = math.radians(36)
    cos, sin = math.cos(angle), math.sin(angle)
    unit = text_width(WATERMARK, "Times-Bold", 1)
    size = min(40.0, (PAGE_WIDTH - 2 * MARGIN) / (unit * cos), (PAGE_HEIGHT - 2 * MARGIN) / (unit * sin))
    width = unit * size
    x = PAGE_WIDTH / 2 - (width / 2) * cos
    y = PAGE_HEIGHT / 2 - (width / 2) * sin
    matrix = " ".join(_number(value) for value in (cos, sin, -sin, cos, x, y))
    return (f"q 0.87 g BT /F2 {_number(size)} Tf 0 Tw {matrix} Tm ".encode()
            + pdf_string(WATERMARK) + b" Tj ET Q")


def _footer(text, number, total):
    label = f"P\u00e1gina {number} de {total}"
    right = MARGIN + TEXT_WIDTH - text_width(label, "Times-Italic", 8.5)
    return (f"0.4 g BT /F3 8.5 Tf 0 Tw {MARGIN} 42 Td ".encode() + pdf_string(text) + b" Tj ET "
            + f"BT /F3 8.5 Tf {_number(right)} 42 Td ".encode() + pdf_string(label) + b" Tj ET 0 g")


def _utf16(text):
    return b"<FEFF" + text.encode("utf-16-be").hex().upper().encode() + b">"


def render(layout, title):
    """Serialize the laid-out pages into PDF bytes."""
    total = len(layout.pages)
    objects = [None, None]
    font_numbers = {}
    for font in FONT_KEYS:
        objects.append(f"<< /Type /Font /Subtype /Type1 /BaseFont /{font} /Encoding /WinAnsiEncoding >>".encode())
        font_numbers[font] = len(objects)
    objects.append(b"<< /Title " + _utf16(title) + b" /Producer (Qadra demo-case generator) >>")
    info_number = len(objects)
    resources = " ".join(f"/{key} {font_numbers[font]} 0 R" for font, key in FONT_KEYS.items())
    page_numbers = []
    for number, ops in enumerate(layout.pages, start=1):
        stream = b"\n".join([_watermark(), b"0 g"] + ops + [_footer(layout.footer, number, total)])
        data = zlib.compress(stream, 9)
        objects.append(b"<< /Length " + str(len(data)).encode() + b" /Filter /FlateDecode >>\nstream\n"
                       + data + b"\nendstream")
        content_number = len(objects)
        objects.append(f"<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH} {PAGE_HEIGHT}] "
                       f"/Resources << /Font << {resources} >> >> /Contents {content_number} 0 R >>".encode())
        page_numbers.append(len(objects))
    kids = " ".join(f"{number} 0 R" for number in page_numbers)
    objects[0] = b"<< /Type /Catalog /Pages 2 0 R >>"
    objects[1] = f"<< /Type /Pages /Kids [{kids}] /Count {total} >>".encode()
    output = bytearray(b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n")
    offsets = []
    for number, body in enumerate(objects, start=1):
        offsets.append(len(output))
        output += f"{number} 0 obj\n".encode() + body + b"\nendobj\n"
    digest = hashlib.md5(bytes(output)).hexdigest().upper().encode()
    xref = len(output)
    output += f"xref\n0 {len(objects) + 1}\n0000000000 65535 f \n".encode()
    for offset in offsets:
        output += f"{offset:010d} 00000 n \n".encode()
    output += (f"trailer\n<< /Size {len(objects) + 1} /Root 1 0 R /Info {info_number} 0 R ".encode()
               + b"/ID [<" + digest + b"> <" + digest + b">] >>\n"
               + f"startxref\n{xref}\n%%EOF\n".encode())
    return bytes(output)
