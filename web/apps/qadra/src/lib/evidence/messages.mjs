// Plain Spanish messages for every finding code of the evidence validator.
// Each code of crates/evidence-verify has its own entry; a code missing from
// the table, as from a newer engine, falls back to the message of its family
// (the text before the first dot). tests/evidence-messages.test.mjs checks
// that the table covers every code in the engine sources. Rationale:
// docs/adr/0085-in-browser-evidence-validation.md.
import {
  individualJsonMessages,
  jsonMessages,
  packageFallbacks,
  profileMessages,
  zipMessages,
} from './messages-package.mjs';
import {
  contextMessages,
  signatureMessages,
  signingFallbacks,
  statementMessages,
} from './messages-signing.mjs';
import { encodingMessages, tokenFallbacks, tokenMessages } from './messages-token.mjs';
import { manifestFallbacks, manifestMessages, receiptMessages } from './messages-manifest.mjs';
import {
  certificateFallbacks,
  certificateMessages,
  crlMessages,
  currentMessages,
  trustMessages,
} from './messages-certificates.mjs';

export { NOT_QADRA_EXPORT } from './messages-package.mjs';

/** Shown with every result: the strict profiles can disagree with the server. */
export const HISTORICAL_NOTE =
  'Para paquetes hist\u00f3ricos este resultado puede diferir del que mostr\u00f3 el servidor.';

export const findingMessages = Object.freeze({
  ...zipMessages,
  ...profileMessages,
  ...individualJsonMessages,
  ...jsonMessages,
  ...signatureMessages,
  ...statementMessages,
  ...contextMessages,
  ...tokenMessages,
  ...encodingMessages,
  ...certificateMessages,
  ...crlMessages,
  ...trustMessages,
  ...currentMessages,
  ...manifestMessages,
  ...receiptMessages,
});

export const FAMILY_FALLBACKS = Object.freeze({
  ...packageFallbacks,
  ...signingFallbacks,
  ...tokenFallbacks,
  ...certificateFallbacks,
  ...manifestFallbacks,
});

const UNKNOWN =
  'El validador encontr\u00f3 un detalle que esta versi\u00f3n de Qadra no sabe describir.';

/** The message of `code`, or of its family when the code is not listed. */
export function findingMessage(code) {
  const text = String(code ?? '');
  if (Object.hasOwn(findingMessages, text)) return findingMessages[text];
  const family = text.split('.')[0];
  return Object.hasOwn(FAMILY_FALLBACKS, family) ? FAMILY_FALLBACKS[family] : UNKNOWN;
}
