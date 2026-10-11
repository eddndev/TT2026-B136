//! Typed audit metadata with a bounded, versioned canonical representation.

use crate::identity::UserId;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;
use uuid::Uuid;

/// Execution boundary that recorded an audit event.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditSource {
    /// Request received through the HTTP interface.
    Http,
    /// Command invoked through the command-line interface.
    Cli,
    /// Operation performed by an internal worker or service.
    Internal,
}

/// Observed audit metadata; missing identifiers or addresses remain unavailable.
///
/// Its canonical extension occupies 12 to 60 bytes and follows this layout:
///
/// ```text
/// version     8 literal bytes: AUDCTX01
/// user        0 absent, or 1 followed by 16 UUID bytes
/// address     0 absent, 4 followed by 4 IPv4 octets, or 6 and 16 IPv6 octets
/// source      1 HTTP, 2 CLI, or 3 internal
/// request     0 absent, or 1 followed by 16 UUID bytes
/// ```
///
/// UUID and address bytes use network order. An unavailable value has its
/// own tag and cannot alias a present nil UUID or unspecified address.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuditContext {
    /// Persisted user identity, when established for the operation.
    pub user_id: Option<UserId>,
    /// Peer address, when established by the transport boundary.
    pub ip_address: Option<IpAddr>,
    /// Interface or internal process that recorded the operation.
    pub source: AuditSource,
    /// Correlation identifier assigned to the request, when one exists.
    pub request_id: Option<Uuid>,
}

impl AuditContext {
    /// Appends the context extension after the complete legacy event encoding.
    pub(super) fn append_canonical(&self, bytes: &mut Vec<u8>) {
        bytes.extend_from_slice(b"AUDCTX01");
        append_uuid(bytes, self.user_id.map(UserId::as_uuid));
        match self.ip_address {
            None => bytes.push(0),
            Some(IpAddr::V4(address)) => {
                bytes.push(4);
                bytes.extend_from_slice(&address.octets());
            }
            Some(IpAddr::V6(address)) => {
                bytes.push(6);
                bytes.extend_from_slice(&address.octets());
            }
        }
        bytes.push(match self.source {
            AuditSource::Http => 1,
            AuditSource::Cli => 2,
            AuditSource::Internal => 3,
        });
        append_uuid(bytes, self.request_id);
    }
}

fn append_uuid(bytes: &mut Vec<u8>, value: Option<Uuid>) {
    match value {
        None => bytes.push(0),
        Some(value) => {
            bytes.push(1);
            bytes.extend_from_slice(value.as_bytes());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::net::{Ipv4Addr, Ipv6Addr};

    #[test]
    fn source_json_uses_only_the_three_exact_labels() {
        for (label, source) in [
            ("http", AuditSource::Http),
            ("cli", AuditSource::Cli),
            ("internal", AuditSource::Internal),
        ] {
            assert_eq!(serde_json::to_value(source).unwrap(), json!(label));
            assert_eq!(
                serde_json::from_value::<AuditSource>(json!(label)).unwrap(),
                source
            );
        }
        for invalid in [
            json!("Http"),
            json!("worker"),
            json!(""),
            json!(1),
            json!(null),
        ] {
            assert!(serde_json::from_value::<AuditSource>(invalid).is_err());
        }
    }

    #[test]
    fn context_json_rejects_unknown_fields_and_invalid_typed_values() {
        let valid = json!({
            "user_id": "00000000-0000-0000-0000-000000000001",
            "ip_address": "192.0.2.1",
            "source": "http",
            "request_id": "00000000-0000-0000-0000-000000000002"
        });
        for (field, invalid) in [
            ("user_id", json!("not-a-uuid")),
            ("ip_address", json!("192.0.2.1:443")),
            ("source", json!("browser")),
            ("source", json!(null)),
            ("request_id", json!("not-a-uuid")),
            ("unexpected", json!(true)),
        ] {
            let mut value = valid.clone();
            value[field] = invalid;
            assert!(
                serde_json::from_value::<AuditContext>(value).is_err(),
                "{field}"
            );
        }
        let mut missing_source = valid;
        missing_source.as_object_mut().unwrap().remove("source");
        assert!(serde_json::from_value::<AuditContext>(missing_source).is_err());
    }

    #[test]
    fn unavailable_fields_have_an_exact_versioned_encoding() {
        let context = AuditContext {
            user_id: None,
            ip_address: None,
            source: AuditSource::Internal,
            request_id: None,
        };
        let mut bytes = Vec::new();
        context.append_canonical(&mut bytes);
        assert_eq!(bytes, b"AUDCTX01\x00\x00\x03\x00");
        for changed in [
            AuditContext {
                user_id: Some(UserId::from_uuid(Uuid::nil())),
                ..context.clone()
            },
            AuditContext {
                ip_address: Some(IpAddr::V4(Ipv4Addr::UNSPECIFIED)),
                ..context.clone()
            },
            AuditContext {
                ip_address: Some(IpAddr::V6(Ipv6Addr::UNSPECIFIED)),
                ..context.clone()
            },
            AuditContext {
                request_id: Some(Uuid::nil()),
                ..context
            },
        ] {
            let mut present = Vec::new();
            changed.append_canonical(&mut present);
            assert_ne!(present, bytes);
        }
    }

    #[test]
    fn complete_context_has_an_exact_bounded_encoding() {
        let context = AuditContext {
            user_id: Some(UserId::from_uuid(Uuid::from_u128(1))),
            ip_address: Some(IpAddr::V6(Ipv6Addr::LOCALHOST)),
            source: AuditSource::Http,
            request_id: Some(Uuid::from_u128(2)),
        };
        let mut bytes = Vec::new();
        context.append_canonical(&mut bytes);
        let expected = [
            b'A', b'U', b'D', b'C', b'T', b'X', b'0', b'1', 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
        ];
        assert_eq!(bytes, expected);
        assert_eq!(bytes.len(), 60);
    }
}
