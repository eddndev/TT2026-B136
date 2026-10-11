use super::*;
use application::audit_context::{snapshot, AuditScope};
use domain::audit::{AuditContext, AuditSource};
use std::net::Ipv4Addr;
use uuid::Uuid;

struct CapturedAudit(Arc<Mutex<Vec<ChainedEvent>>>);

impl AuditLog for CapturedAudit {
    fn append(
        &mut self,
        actor: &str,
        action: &str,
        resource: &str,
        timestamp: OffsetDateTime,
    ) -> Result<ChainedEvent, DomainError> {
        let mut records = self.0.lock().unwrap();
        let mut event = AuditEvent::new(records.len() as u64, timestamp, actor, action, resource);
        event.context =
            Some(snapshot().map_err(|_| {
                DomainError::AuditStorageFailure("audit context unavailable".into())
            })?);
        let entry = ChainedEvent {
            event,
            chain: Sha256Digest::from_array([0; 32]),
        };
        records.push(entry.clone());
        Ok(entry)
    }

    fn load_all(&self) -> Result<Vec<ChainedEvent>, DomainError> {
        Ok(self.0.lock().unwrap().clone())
    }
}

fn http_context() -> AuditContext {
    AuditContext {
        user_id: None,
        ip_address: Some(Ipv4Addr::new(192, 0, 2, 1).into()),
        source: AuditSource::Http,
        request_id: Some(Uuid::from_u128(1)),
    }
}

fn captured_service() -> (IdentityService, Arc<Mutex<Vec<ChainedEvent>>>) {
    let records = Arc::new(Mutex::new(Vec::new()));
    let mut ports = ports(
        Arc::new(MemoryUsers::default()),
        Arc::new(MemorySessions::default()),
    );
    ports.audit_log = Box::new(CapturedAudit(records.clone()));
    (IdentityService::new(ports), records)
}

fn assert_second_factor_binds_identity(recovery: bool) {
    let (service, records) = captured_service();
    let owner = service
        .bootstrap_owner("owner@example.test", "correct horse battery")
        .unwrap();
    let challenge = service
        .start_login("owner@example.test", "correct horse battery")
        .unwrap();
    let scope = AuditScope::new(http_context());
    let session = scope
        .run(|| {
            if recovery {
                service.complete_recovery(&challenge.challenge_token, &owner.recovery_codes[0])
            } else {
                service.complete_totp(&challenge.challenge_token, "123456")
            }
        })
        .unwrap();
    let expected = AuditContext {
        user_id: Some(owner.principal.id),
        ..http_context()
    };
    assert_eq!(session.principal.id, owner.principal.id);
    assert_eq!(scope.snapshot().unwrap(), expected);
    let records = records.lock().unwrap();
    let accepted = records.last().unwrap();
    assert_eq!(
        accepted.event.action,
        if recovery {
            "identity.recovery_accepted"
        } else {
            "identity.totp_accepted"
        }
    );
    assert_eq!(accepted.event.context.as_ref(), Some(&expected));
    assert_eq!(accepted.event.resource, owner.principal.id.to_string());
}

#[test]
fn accepted_totp_binds_http_identity_before_the_business_audit() {
    assert_second_factor_binds_identity(false);
}

#[test]
fn accepted_recovery_binds_http_identity_before_the_business_audit() {
    assert_second_factor_binds_identity(true);
}

#[test]
fn invalid_factors_and_unknown_challenges_leave_http_identity_unavailable() {
    for recovery in [false, true] {
        for unknown in [false, true] {
            let (service, records) = captured_service();
            service
                .bootstrap_owner("owner@example.test", "correct horse battery")
                .unwrap();
            let challenge = service
                .start_login("owner@example.test", "correct horse battery")
                .unwrap();
            let initial_count = records.lock().unwrap().len();
            let scope = AuditScope::new(http_context());
            let token = if unknown {
                "unknown"
            } else {
                &challenge.challenge_token
            };
            let result = scope.run(|| {
                if recovery {
                    service.complete_recovery(token, "invalid-recovery-code")
                } else {
                    service.complete_totp(token, "000000")
                }
            });
            assert!(matches!(result, Err(ApplicationError::MfaRejected)));
            assert_eq!(scope.snapshot().unwrap(), http_context());
            assert_eq!(records.lock().unwrap().len(), initial_count);
        }
    }
}
