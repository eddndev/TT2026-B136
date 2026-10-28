"""Tests for the fictitious demonstration case PDF generator."""
import re
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import demo_case_documents as documents  # noqa: E402
import demo_case_pdf as pdf  # noqa: E402

SOURCE = """---
id: D99
archivo: D99-prueba.pdf
titulo: Documento de prueba
encabezado: Juzgado de Control Demo
referencia: Carpeta judicial 999/2026
lugar_fecha: Ciudad de M\u00e9xico, a {{fecha}}
---
# ACTA DE PRUEBA

Primer p\u00e1rrafo con acentos y e\u00f1es, \u00bfpreguntas? y \u00abcomillas\u00bb para comprobar la
codificaci\u00f3n WinAnsi del texto justificado en varias l\u00edneas de la p\u00e1gina.

## Resolutivos

- Primer punto.

> Mtro. Ernesto Vidal Quiroga
> Juez de Control Demo
"""


def check_xref(data):
    """Every xref entry must point at the start of its object."""
    start = int(re.search(rb"startxref\n(\d+)\n%%EOF\n$", data).group(1))
    table = data[start:].split(b"trailer")[0].split(b"\n")
    count = int(table[1].split()[1])
    offsets = [int(line.split()[0]) for line in table[3:3 + count - 1]]
    for number, offset in enumerate(offsets, start=1):
        if not data[offset:].startswith(f"{number} 0 obj\n".encode()):
            raise AssertionError(f"object {number} is not at offset {offset}")
    return count - 1


class GeneratorTest(unittest.TestCase):
    def build(self, values=None):
        meta, blocks = documents.parse(SOURCE, "test", values or {"fecha": "12 de enero de 2026"})
        return meta, pdf.render(documents.layout_document(meta, blocks), meta["titulo"])

    def test_output_is_deterministic_and_well_formed(self):
        _, first = self.build()
        _, second = self.build()
        self.assertEqual(first, second)
        self.assertTrue(first.startswith(b"%PDF-1.4\n"))
        self.assertGreaterEqual(check_xref(first), 7)
        self.assertNotIn(b"CreationDate", first)

    def test_marker_values_change_the_document(self):
        _, first = self.build({"fecha": "12 de enero de 2026"})
        _, second = self.build({"fecha": "13 de enero de 2026"})
        self.assertNotEqual(first, second)

    def test_missing_marker_value_is_rejected(self):
        with self.assertRaises(documents.SourceError):
            documents.parse(SOURCE, "test", {})

    def test_metadata_is_required_and_file_name_is_ascii(self):
        with self.assertRaises(documents.SourceError):
            documents.parse("# sin metadatos\n", "test")
        bad = SOURCE.replace("D99-prueba.pdf", "D99 prueba \u00f1.pdf")
        with self.assertRaises(documents.SourceError):
            documents.parse(bad, "test", {"fecha": "x"})

    def test_parse_returns_blocks_in_order(self):
        meta, blocks = documents.parse(SOURCE, "test", {"fecha": "x"})
        self.assertEqual(meta["encabezado"], ["Juzgado de Control Demo"])
        self.assertEqual([kind for kind, _ in blocks], ["title", "p", "h", "li", "sign", "sign"])

    def test_unsupported_character_is_rejected(self):
        with self.assertRaises(ValueError):
            pdf.encode("emoji \u2603")
        with self.assertRaises(ValueError):
            pdf.encode("tilde \u00e3")

    def test_wrapped_lines_fit_the_column(self):
        text = "palabra " * 200
        for line in pdf.wrap(text, "Times-Roman", 11, pdf.TEXT_WIDTH):
            self.assertLessEqual(pdf.text_width(line, "Times-Roman", 11), pdf.TEXT_WIDTH)

    def test_long_documents_continue_on_new_pages(self):
        layout = pdf.Layout("pie")
        for _ in range(80):
            layout.paragraph("texto " * 40)
        self.assertGreater(len(layout.pages), 1)
        check_xref(pdf.render(layout, "largo"))

    def test_build_directory_skips_live_sources_on_request(self):
        with tempfile.TemporaryDirectory() as work:
            source_dir, output = Path(work) / "src", Path(work) / "out"
            source_dir.mkdir()
            (source_dir / "D99.md").write_text(SOURCE, encoding="utf-8")
            static = SOURCE.replace("{{fecha}}", "12 de enero de 2026").replace("D99", "D98")
            (source_dir / "D98.md").write_text(static, encoding="utf-8")
            manifest = documents.build_directory(source_dir, output, skip_live=True)
            self.assertEqual([entry["id"] for entry in manifest], ["D98"])
            self.assertTrue((output / "D98-prueba.pdf").is_file())


if __name__ == "__main__":
    unittest.main()
