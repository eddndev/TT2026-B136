import { caseId, id, document, validReport, setup, login, openDocument } from './helpers.mjs';

export async function versionSetup(page, { conflict = false } = {}) {
  const calls = await setup(page, 'owner', [{ ...document, sealed: true }]);
  const records = [{ ...document, sealed: true }];
  let conflictPending = conflict;
  await page.route(`**/cases/${caseId}/documents?**`, (route) =>
    route.fulfill({ json: { documents: [records.at(-1)], has_more: false } }),
  );
  await page.route(`**/cases/${caseId}/documents/**`, async (route) => {
    const request = route.request();
    const url = new URL(request.url());
    const suffix = url.pathname.split(`/documents/${id}`)[1];
    calls.push({
      path: url.pathname,
      search: url.search,
      method: request.method(),
      body: request.postData(),
    });
    if (suffix === '') return route.fulfill({ json: records.at(-1) });
    if (suffix === '/versions') {
      if (request.method() === 'POST') {
        if (conflictPending) {
          conflictPending = false;
          records.push({ ...document, version: 2, name: 'other-editor.txt' });
          return route.fulfill({
            status: 409,
            json: { error: { code: 'document_version_conflict' } },
          });
        }
        const record = {
          ...document,
          version: records.length + 1,
          name: request.headers()['x-document-name'],
        };
        records.push(record);
        return route.fulfill({ status: 201, json: record });
      }
      return route.fulfill({
        json: {
          versions: [...records].reverse(),
          has_more: false,
          next_before_version: null,
          first_available_version: 1,
        },
      });
    }
    const version = Number(suffix?.split('/')[2]);
    const record = records.find((item) => item.version === version);
    if (!record) return route.fulfill({ status: 404 });
    if (suffix.endsWith('/verify')) return route.fulfill({ json: { ...validReport, id, version } });
    if (suffix.endsWith('/seal')) {
      record.sealed = true;
      return route.fulfill({ json: record });
    }
    if (suffix.endsWith('/evidence'))
      return route.fulfill({
        contentType: 'application/zip',
        body: `archive version ${version}`,
        headers: { 'X-Document-Id': id, 'X-Document-Version': String(version) },
      });
    return route.fulfill({ json: record });
  });
  await login(page);
  await openDocument(page);
  return { calls, records };
}

export async function append(page, filename = 'version-two.txt') {
  await page.getByRole('button', { name: 'Agregar versi\u00f3n', exact: true }).click();
  await page
    .getByLabel('Archivo de la nueva versi\u00f3n', { exact: true })
    .setInputFiles({ name: filename, mimeType: 'text/plain', buffer: Buffer.from('new contents') });
  await page.getByRole('button', { name: 'Guardar nueva versi\u00f3n', exact: true }).click();
}
