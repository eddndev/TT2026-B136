import { test, expect } from '@playwright/test';
import { setup, login, navigate, caseId, id } from './helpers.mjs';
import { versionSetup } from './version-helpers.mjs';
import {
  admissionRejections,
  privateParserDetail,
  rejectionPayload,
} from '../fixtures/document-admission-errors.mjs';

const source = {
  name: 'seleccionado.pdf',
  mimeType: 'application/pdf',
  buffer: Buffer.from('retained-file-bytes'),
};
const retainedMetadata = {
  document_type: 'Escrito',
  classification: 'Reservado',
  tags: ['accion, prueba'],
};
for (const rejection of admissionRejections)
  for (const append of [false, true])
    test(`${append ? 'append' : 'classified upload'} explains ${rejection.code} and preserves its rejected draft`, async ({
      page,
    }) => {
      let records;
      if (append) ({ records } = await versionSetup(page));
      else {
        await setup(page);
        await login(page);
        await navigate(page, 'Documentos');
      }
      const writes = [];
      const suffix = append ? `/${id}/versions?expected_version=*` : '/with-metadata';
      await page.route(`**/cases/${caseId}/documents${suffix}`, async (route) => {
        const request = route.request();
        const entry = { url: request.url(), name: request.headers()['x-document-name'] };
        if (append) entry.bytes = request.postDataBuffer();
        else {
          const form = await new Response(request.postDataBuffer(), {
            headers: { 'Content-Type': request.headers()['content-type'] },
          }).formData();
          entry.bytes = Buffer.from(await form.get('file').arrayBuffer());
          entry.metadata = JSON.parse(await form.get('metadata').text());
        }
        writes.push(entry);
        await route.fulfill({ status: rejection.status, json: rejectionPayload(rejection.code) });
      });
      const dialogName = append ? 'Agregar versi\u00f3n' : 'Subir documento';
      await page.getByRole('button', { name: dialogName, exact: true }).click();
      const modal = page.getByRole('dialog', { name: dialogName, exact: true });
      const file = modal.getByLabel(append ? 'Archivo de la nueva versi\u00f3n' : 'Archivo', {
        exact: true,
      });
      const name = modal.getByLabel(
        append ? 'Nombre de la nueva versi\u00f3n' : 'Nombre del documento',
        { exact: true },
      );
      await file.setInputFiles(source);
      await name.fill('nombre-revisado.pdf');
      if (!append) {
        await modal
          .getByLabel('Tipo de documento (opcional)', { exact: true })
          .fill(retainedMetadata.document_type);
        await modal
          .getByLabel('Clasificaci\u00f3n (opcional)', { exact: true })
          .fill(retainedMetadata.classification);
        await modal.getByLabel('Nueva etiqueta', { exact: true }).fill(retainedMetadata.tags[0]);
      }
      const submit = modal.getByRole('button', {
        name: append ? 'Guardar nueva versi\u00f3n' : 'Cargar documento',
        exact: true,
      });
      await submit.click();
      await expect(modal.getByRole('alert')).toBeVisible();
      await expect(submit).toBeEnabled();
      await expect(modal).toBeVisible();
      await expect(name).toHaveValue('nombre-revisado.pdf');
      const selected = await file.evaluate(async (input) => ({
        name: input.files[0].name,
        bytes: [...new Uint8Array(await input.files[0].arrayBuffer())],
      }));
      expect(selected).toEqual({ name: source.name, bytes: [...source.buffer] });
      expect(writes).toHaveLength(1);
      expect(writes[0].name).toBe('nombre-revisado.pdf');
      expect(writes[0].bytes).toEqual(source.buffer);
      if (append) {
        expect(new URL(writes[0].url).searchParams.get('expected_version')).toBe('1');
        expect(records).toHaveLength(1);
        await expect(modal.getByText('Versi\u00f3n de partida: 1', { exact: true })).toBeVisible();
        await expect(
          modal.getByRole('button', { name: 'Consultar versi\u00f3n actual', exact: true }),
        ).toHaveCount(0);
      } else {
        expect(writes[0].metadata).toEqual(retainedMetadata);
        await expect(modal.getByLabel('Tipo de documento (opcional)', { exact: true })).toHaveValue(
          retainedMetadata.document_type,
        );
        await expect(
          modal.getByLabel('Clasificaci\u00f3n (opcional)', { exact: true }),
        ).toHaveValue(retainedMetadata.classification);
        await expect(
          modal.getByRole('button', { name: 'Quitar etiqueta: accion, prueba', exact: true }),
        ).toBeVisible();
      }
      await expect(
        page.getByRole('heading', { name: 'nombre-revisado.pdf', exact: true }),
      ).toHaveCount(0);
      await expect(modal.getByRole('alert')).not.toContainText(privateParserDetail);
      for (const explanation of rejection.explanation)
        await expect(modal.getByRole('alert')).toContainText(explanation);
      expect(writes).toHaveLength(1);
    });
