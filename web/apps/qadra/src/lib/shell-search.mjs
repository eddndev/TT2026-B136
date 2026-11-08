// Client-side matching for the top search bar. The case list is fetched once per
// search session and filtered here, so typing does not issue one audited query
// per keystroke and matching ignores case and accents.

export function foldText(value) {
  return String(value ?? '')
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .toLowerCase()
    .trim();
}

function haystack(row) {
  const ids = row.penal_identifiers;
  return [row.title, row.reference, ids?.nuc, ids?.judicial_case_number].map(foldText);
}

export function matchCases(rows, query, limit = 6) {
  const words = foldText(query).split(/\s+/).filter(Boolean);
  if (words.join('').length < 2) return [];
  const scored = [];
  for (const row of rows) {
    const fields = haystack(row);
    const joined = fields.join(' ');
    if (!words.every((word) => joined.includes(word))) continue;
    const title = fields[0];
    const score = title.startsWith(words[0]) ? 0 : title.includes(words[0]) ? 1 : 2;
    scored.push({ row, score });
  }
  scored.sort(
    (a, b) => a.score - b.score || foldText(a.row.title).localeCompare(foldText(b.row.title)),
  );
  return scored.slice(0, limit).map((item) => item.row);
}
