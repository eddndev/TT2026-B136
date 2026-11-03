//! Inbox notices of request events: who receives each kind and the plain
//! text they read. The text names the contract and the instant, never a
//! token or a signature, and links to the contracts space when a public
//! address is configured. Recipients per kind: the section "Storage" of
//! `docs/adr/0086-bilateral-contract-signing.md`.

use domain::identity::UserId;

use super::{model::EventKind, port::ContractNotice};
use domain::document_signing::contract::ContractRequestFields;

/// The sender address and optional public address of every notice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoticeSettings {
    pub from: String,
    pub public_url: Option<String>,
}

/// The distinct recipients of `kind`, in a fixed order.
pub fn recipients(kind: EventKind, fields: &ContractRequestFields) -> Vec<UserId> {
    let (client, firm, requester) = (fields.client, fields.firm_signer, fields.requester);
    let all = match kind {
        EventKind::Requested => vec![client],
        EventKind::Rejected => vec![requester, firm],
        EventKind::Declined => vec![client, requester],
        EventKind::Cancelled => vec![client, firm],
        EventKind::ClientSigned | EventKind::Completed | EventKind::Expired => {
            vec![client, firm, requester]
        }
        EventKind::Created | EventKind::Abandoned => Vec::new(),
    };
    let mut distinct = Vec::new();
    for recipient in all {
        if !distinct.contains(&recipient) {
            distinct.push(recipient);
        }
    }
    distinct
}

fn summary(kind: EventKind) -> (&'static str, &'static str) {
    match kind {
        EventKind::Requested => (
            "Solicitud de firma de contrato",
            "Tienes una solicitud de firma de contrato pendiente.",
        ),
        EventKind::ClientSigned => (
            "El Cliente firmo el contrato",
            "El Cliente firmo; falta la firma del despacho antes del vencimiento.",
        ),
        EventKind::Completed => (
            "Contrato firmado por ambas partes",
            "El contrato quedo firmado por ambas partes; su paquete final ya esta disponible.",
        ),
        EventKind::Rejected => (
            "El Cliente rechazo la solicitud",
            "El Cliente rechazo la solicitud de firma.",
        ),
        EventKind::Declined => (
            "El despacho declino la firma",
            "El firmante del despacho declino la solicitud despues de la firma del Cliente.",
        ),
        EventKind::Cancelled => (
            "Solicitud de firma cancelada",
            "La solicitud de firma fue cancelada.",
        ),
        EventKind::Expired | EventKind::Created | EventKind::Abandoned => (
            "Solicitud de firma vencida",
            "La solicitud de firma vencio sin completarse.",
        ),
    }
}

/// One notice per recipient of `kind` for the request `fields`, titled
/// `title`.
pub fn notices(
    settings: &NoticeSettings,
    kind: EventKind,
    fields: &ContractRequestFields,
    title: &str,
) -> Vec<ContractNotice> {
    let (subject, line) = summary(kind);
    let mut text = format!(
        "{line}\n\nContrato: {title}\nSolicitud: {}\nVence (Unix): {}\n",
        fields.request, fields.expires_at_seconds
    );
    if let Some(url) = &settings.public_url {
        text.push_str(&format!("\n{}#contracts\n", url.trim_end_matches('/')));
    }
    recipients(kind, fields)
        .into_iter()
        .map(|recipient| ContractNotice {
            recipient,
            from: settings.from.clone(),
            subject: format!("Qadra: {subject}"),
            text: text.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::document_signing::Uuid;
    use domain::{
        cases::CaseId,
        crypto::{DocumentId, DocumentVersion, DocumentVersionRef, Sha256Digest},
    };

    fn fields(requester: u128) -> ContractRequestFields {
        let user = |n| UserId::from_uuid(Uuid::from_u128(n));
        ContractRequestFields {
            deployment: Uuid::from_u128(1),
            request: Uuid::from_u128(2),
            case: CaseId::from_uuid(Uuid::from_u128(3)),
            document: DocumentVersionRef {
                id: DocumentId::from_uuid(Uuid::from_u128(4)),
                version: DocumentVersion::new(1).unwrap(),
            },
            content_digest: Sha256Digest::from_array([1; 32]),
            client: user(10),
            firm_signer: user(20),
            requester: user(requester),
            client_leaf: Sha256Digest::from_array([2; 32]),
            firm_leaf: Sha256Digest::from_array([3; 32]),
            created_at_seconds: 0,
            expires_at_seconds: 86_400,
        }
    }

    fn ids(values: &[UserId]) -> Vec<u128> {
        values.iter().map(|id| id.as_uuid().as_u128()).collect()
    }

    #[test]
    fn each_kind_reaches_exactly_its_recipients_once() {
        let f = fields(30);
        for (kind, expected) in [
            (EventKind::Requested, vec![10]),
            (EventKind::ClientSigned, vec![10, 20, 30]),
            (EventKind::Completed, vec![10, 20, 30]),
            (EventKind::Expired, vec![10, 20, 30]),
            (EventKind::Rejected, vec![30, 20]),
            (EventKind::Declined, vec![10, 30]),
            (EventKind::Cancelled, vec![10, 20]),
            (EventKind::Created, vec![]),
        ] {
            assert_eq!(ids(&recipients(kind, &f)), expected, "{kind:?}");
        }
        assert_eq!(
            ids(&recipients(EventKind::Completed, &fields(20))),
            vec![10, 20]
        );
    }

    #[test]
    fn notices_link_the_contracts_space_without_any_token() {
        let settings = NoticeSettings {
            from: "qadra@example.test".into(),
            public_url: Some("https://qadra.example.test/".into()),
        };
        let all = notices(&settings, EventKind::Requested, &fields(30), "Servicios");
        assert_eq!(all.len(), 1);
        assert!(all[0].text.contains("https://qadra.example.test#contracts"));
        assert!(all[0].text.is_ascii() && all[0].subject.is_ascii());
        assert!(all[0].subject.len() <= 200 && all[0].text.len() <= 4096);
    }
}
