# API de identidades representadas y participantes tipificados

Contrato de las rutas autenticadas de participantes. Se complementa con
[el contrato HTTP general](http-api.md), [la decision de identidades y roles](adr/0025-case-subjects-and-typed-participants.md)
y [el perfil de credenciales internas](adr/0026-internal-participant-declarations.md).

Las identidades representadas pertenecen a un expediente y no son cuentas de
Qadra. Una ficha manual conserva PART1; completar su perfil agrega una revision
PART2 vinculada a una revision exacta SUBJ1. No se reescribe la historia.

Los UUID se transportan como cadenas; las revisiones son enteros JSON positivos,
con cero reservado a la expectativa de ausencia. Los digests son SHA-256 en
64 caracteres hexadecimales minusculos. Los bytes usan base64 estandar con
padding. El numero de CRL es una cadena decimal para conservar todos sus digitos
u64 al consumir JSON en JavaScript.

Los objetos rechazan campos desconocidos, duplicados e incompatibles con su
variante. Antes de invocar un caso de uso se lee el cuerpo completo, hasta
256 KiB para rutas tipificadas u 8 KiB para escritura manual, y se exige fin de
entrada despues del objeto JSON. Todos los resultados usan Cache-Control:
no-store. Las claves privadas nunca se reciben por estas rutas.

## Rutas bajo /api/v1/cases/{case_id}

- Existing GET/POST /participants and PUT/GET /participants/{id}; manual POST/PUT retain current inputs and response fields. List adds kind=<stable kind> and profile=all|manual|typed (default all), applied after selecting current union heads and before paging.
- Existing PUT /participants/{id}/directory-status and GET /participants/{id}/history become mixed manual/typed projections.
- GET /participants/{id}/revisions/{revision}: exact mixed detail.
- GET /participants/{id}/revisions/{revision}/credential: public evidence associated with that exact revision. A status-only revision follows its preserved credential_origin; the response reference identifies the original attested revision. No evidence means a dedicated not-found, no fabricated empty proof.
- POST /participants/proposals/review: ParticipantProposalRequest -> ParticipantProposalReview, 200.
- POST /participants/proposals/prepare: ParticipantPreparationRequest -> ParticipantSigningDraft, 200.
- POST /participants/proposals/commit: ParticipantSubmission -> ParticipantDetail, 201 for new root, 200 for existing root.
- GET /subjects: current compact subjects (limit default50, 1..100; after_id, name, kind).
- GET /subjects/{id}, /subjects/{id}/revisions/{revision}, /subjects/{id}/history: current/exact/history.
- POST /subjects/{id}/review: expected_revision + values -> SubjectReplacementReview, 200.
- PUT /subjects/{id}: expected_revision + values + review -> SubjectSnapshot, 200.

All management/review/preparation uses Owner global or assigned Litigator; reads also allow assigned Paralegal; Client denied. No new private permission. Actor/resource revalidation belongs to application/store. CaseClosed remains 409 and detail reads survive closure. No identifier/certificate query strings.

## Valores compartidos

Evidence locator: {document_id, version, digest, locator}.
Subject reference: {id, revision, values_digest}.
Known/unknown declared value: {state:"known", value:T} or {state:"unknown", reason:string}.
Name: {state:"known", value:string} or {state:"unidentified", label:string, reason:string}.
Professional license: {number, issuer}; preserve leading zeros.

Subject values:
- {kind:"natural_person", name:Name, curp:Declared<string>, identity_support:Locator}
- {kind:"institutional_body", name:string, institutional_identifier:Declared<string>, identity_support:Locator}

Role: {organization:string|null, legal_status:string|null, profile:Profile, role_support:Locator}.
Typed values: {subject:SubjectRef, directory_status:"active"|"archived", role:Role}.

Profile variants use kind and only their fields:
- defendant: custody Declared<"at_liberty"|"detained">
- victim: contact and protection, each as defined below
- defense_counsel: license License, mode "private"|"public"
- prosecutor: office_identifier Declared<string>, unit Declared<string>, license Declared<License>
- victim_counsel: institution string, license Declared<License>
- control_judge: court string
- trial_court: judicial_district string, composition "single"|"collegiate"
- expert: specialty Declared<string>, license Declared<License>
- police: agency Declared<string>, unit Declared<string>
- precautionary_supervisor: authority Declared<string>, unit Declared<string>
- other: label string, description Declared<string>

Contact: {state:"unknown",reason} | {state:"not_recorded",reason} | {state:"documented",support:Locator}.
Protection: {state:"unknown",reason} | {state:"none_declared",reason} | {state:"documented",support:Locator}.

## Revision de identidad, preparacion y registro

Review request:
{
  subject: {operation:"create",values:SubjectValues}
         | {operation:"keep",reference:SubjectRef}
         | {operation:"replace",current:SubjectRef,values:SubjectValues},
  participant: {operation:"create"}
             | {operation:"existing",id,expected_revision},
  role: Role,
  certificate_base64: string|null
}

Returned proposal:
{
  subject: {operation:"keep",reference:SubjectRef}
         | {operation:"append",id,expected_revision:0|n,values:SubjectValues},
  participant_id,
  expected_participant_revision: 0|n,
  values: TypedValues
}

Proposal response: {case_id, proposal, directory_stamp, candidates:[Candidate]}.
Candidate: {reference:{kind:"subject"|"manual_participant",id,revision}, display_name, kind:"natural_person"|"institutional_body"|null, signals:["name"|"declared_identifier"|"certificate"|"documentary_evidence"]}.
No sensitive identifier or matched value is returned by candidate search.

Identity review submission: {directory_stamp, selection_reason, different:[{candidate:CandidateRef, reason, support:Locator}]}.
Complete candidate set is revalidated. Unknown identity and a reported empty set do not establish absence of real-world duplicates. Candidate limit is capacity, not an invitation to change identifiers to evade matching.

Preparation request: {proposal:Proposal, review:IdentityReview, certificate_base64:string|null}.
Preparation response: {case_id, proposal:Proposal, review:IdentityReview, declaration:null|Declaration, submission_digest:string|null, submission_revision:number}. The digest is computed before submit for unsigned operations; signed preparations return null until a signature exists. The revision is always the proposed target revision.
Declaration: {bytes_base64, digest, participant_values_digest, certificate:{der_base64,fingerprint,summary:{subject,issuer,serial_hex,not_before_unix,not_after_unix}}, deployment_id, trust_revision, root_fingerprint, policy:"internal_demo_v1"}.
This is a prepared statement, not an accepted signature. Certificate is selected before preparation; the app computes its DER fingerprint. The receipt may include the whole returned response, while the signed file is only bytes_base64 decoded without reserialization.

Commit request: {prepared:PreparationRequest, signature_base64:string|null}.
No client-provided CredentialCheck, success result, accepted time or actor. Service reconstructs all digests and verifies signature with current published trust.

## Proyecciones

Participant overview: {case_id,id,revision,display_name,procedural_role,organization,directory_status,canonical_format:"part1"|"part2",kind:ParticipantKind|null,subject:SubjectRef|null}.

El listado filtra la última revisión de cada ficha, después de combinar ambas
familias. `profile=all|manual|typed` selecciona la familia actual; `kind` filtra
el tipo estable de un participante tipificado. El filtro histórico
`procedural_role` se limita a fichas manuales. Ningún filtro recupera una revisión
manual anterior de una ficha ya tipificada. La búsqueda del nombre usa la
revisión exacta de identidad vinculada al participante.
No CURP/license/contact/protection/certificate, legal-status free text, actor email or credential data in the index. Page shape remains {participants,has_more,next_after_id}.

Manual detail retains existing scalar fields plus canonical_format:"part1", profile:null, subject:null, credential_origin:null, submission_digest:null, submission_revision:null.
Typed detail retains common scalar fields (display_name from exact bound subject; procedural_role is stable kind key), plus canonical_format:"part2", profile:Profile, role_support:Locator, subject:SubjectSnapshot, credential_origin:CredentialRef|null, submission_digest, submission_revision. values_digest hashes PART2. Common changed_at/changed_by reflect the revision; no current subject values injected.
CredentialRef: {participant_id, participant_revision, statement_digest}.
History shape remains {revisions:[Detail],has_more,next_before_revision}.
Status-only preserves origin references and labels their original revision explicitly.

SubjectSnapshot: {case_id,id,revision,values:SubjectValues,values_digest,changed_at,changed_by:{id,email}}.
Subject overview: {case_id,id,revision,kind,display_name}.
Subject page: {subjects,has_more,next_after_id}; history: {revisions,has_more,next_before_revision}.
Subject replacement review: {case_id,id,expected_revision,values,directory_stamp,candidates}.

Credential evidence JSON: {case_id,reference:CredentialRef,subject:SubjectRef,declaration_base64,statement_digest,certificate_der_base64,certificate_fingerprint,certificate_summary,signature_base64,policy:"internal_demo_v1",checked_at_unix,valid_from_unix,valid_until_unix,trust:{deployment_id,revision,root_der_base64,crl_der_base64,root_fingerprint,crl_digest,crl_number,crl_this_update_unix,crl_next_update_unix,valid_from_unix,valid_until_unix,published_at,published_by},accepted_at,accepted_by:{id,email}}.
Evidence contains public material and the captured trust/check, not private keys. This read is explicitly authorized/audited, distinct from the compact index. Credential download is JSON and exact binary statement/signature extracted from it; it is not the sealed-document ZIP contract.

## Errores

Se conservan los codigos generales de autenticacion, autorizacion y expediente cerrado. Se distinguen los codigos subject_not_found, subject_revision_conflict/exhausted, participant_profile_required, participant_role_conflict, participant_subject_change_forbidden, participant_identity_review_required/conflict, participant_candidate_limit, participant_support_changed, participant_credential_required/unexpected, credential_trust_unavailable/changed/revision_conflict/revision_exhausted. Los errores de credencial usan participant_credential_ y los sufijos limit_exceeded, malformed_certificate, unsupported_certificate, untrusted_issuer, not_yet_valid, expired, malformed_crl, unsupported_crl, untrusted_crl, crl_not_yet_valid, crl_expired, revoked e invalid_signature. La evidencia ausente usa participant_credential_not_found. No parser error text, DN, serial, identifier or body echoed. Los conflictos de estado reciben 409; los datos o credenciales rechazados reciben 422; los recursos ausentes, 404. No se debe reintentar automaticamente cualquier 409.


El indice no expone CURP, licencia, contacto, proteccion ni datos del certificado.
Los soportes de identidad, rol, contacto, proteccion y decisiones Different
comparten un maximo de dos versiones documentales distintas, 16 MiB por version
y 32 MiB por operacion. Sus digests y localizadores forman parte de los valores
capturados. No se realizan consultas globales de identificadores o certificados.

La preparacion sin firma devuelve submission_digest; una respuesta perdida se
concilia consultando la revision exacta y su origen. Una preparacion que requiere
firma devuelve null hasta recibirla: el origen de la credencial permite consultar
su evidencia exacta. Para la edicion separada de identidad, la consulta de una
revision posterior permite revisar los valores, sin atribuir el guardado al
propio envio por coincidencia de datos. Ningun flujo reintenta automaticamente.
