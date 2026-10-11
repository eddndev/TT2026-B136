use super::*;

pub fn configure(harness: &mut Harness) {
    let state = harness.state.clone();
    harness
        .clock
        .expect_now()
        .returning(move || state.lock().unwrap().current_time);
    let state = harness.state.clone();
    harness.digests.expect_hash_bytes().returning(move |bytes| {
        let mut state = state.lock().unwrap();
        state.calls.push("digest");
        let index = match state.digest_inputs.iter().position(|value| value == bytes) {
            Some(index) => index,
            None => {
                state.digest_inputs.push(bytes.to_vec());
                state.digest_inputs.len() - 1
            }
        };
        Sha256Digest::from_array([(index + 1) as u8; 32])
    });
    let state = harness.state.clone();
    harness.tokens.expect_generate().returning(move || {
        let mut state = state.lock().unwrap();
        state.calls.push("token");
        state.tokens += 1;
        Ok(Zeroizing::new(state.token_value))
    });
    let state = harness.state.clone();
    harness.limiter.expect_admit().returning(move |_| {
        let mut state = state.lock().unwrap();
        state.calls.push("limit");
        Ok(!state.limited)
    });
    let state = harness.state.clone();
    harness
        .protector
        .expect_protect()
        .returning(move |context, plaintext| {
            let mut state = state.lock().unwrap();
            state.calls.push("protect_invitation");
            assert!(!context.id.as_uuid().is_nil());
            assert!((1..=i64::MAX as u64).contains(&context.revision));
            let opaque = vec![0xa5, (state.sealed.len() + 1) as u8];
            state.sealed.push(SealedValue {
                context,
                plaintext: plaintext.to_vec(),
                opaque: opaque.clone(),
            });
            if state.change_owner_on_protect {
                state.principal = None;
            }
            Ok(opaque)
        });
    let state = harness.state.clone();
    harness
        .protector
        .expect_expose()
        .returning(move |context, protected| {
            let mut state = state.lock().unwrap();
            state.calls.push("expose_invitation");
            if state.corrupt_ciphertext {
                return Err(ApplicationError::Port(
                    "invalid invitation ciphertext".into(),
                ));
            }
            let value = state
                .sealed
                .iter()
                .find(|value| value.context == context && value.opaque == protected)
                .ok_or_else(|| ApplicationError::Port("invitation context mismatch".into()))?;
            Ok(Zeroizing::new(value.plaintext.clone()))
        });
    let state = harness.state.clone();
    harness.passwords.expect_hash().returning(move |value| {
        let mut state = state.lock().unwrap();
        state.calls.push("hash_password");
        state.hashes.push(value.into());
        Ok(password_hash(value))
    });
    let state = harness.state.clone();
    harness
        .passwords
        .expect_verify()
        .returning(move |value, stored| {
            state.lock().unwrap().calls.push("verify_password");
            Ok(if stored == password_hash(value) {
                PasswordVerification::Match
            } else {
                PasswordVerification::Mismatch
            })
        });
    let state = harness.state.clone();
    harness.totp.expect_enroll().returning(move |email| {
        assert_eq!(email, EMAIL);
        let mut state = state.lock().unwrap();
        state.calls.push("enroll_totp");
        state.enrollments += 1;
        Ok(TotpEnrollment {
            secret: Zeroizing::new(vec![7; 20]),
            secret_base32: Zeroizing::new("PUBLIC-ONLY-TO-RECIPIENT".into()),
            otpauth_uri: Zeroizing::new(format!("otpauth://totp/Qadra:{email}?secret=TEST")),
        })
    });
    let state = harness.state.clone();
    harness
        .totp
        .expect_verify()
        .returning(move |secret, code, at| {
            let mut state = state.lock().unwrap();
            state.calls.push("verify_totp");
            assert_eq!(secret, &[7; 20]);
            assert_eq!(at, state.current_time.unix_timestamp() as u64);
            Ok(if code == CODE {
                TotpVerification::Accepted
            } else {
                TotpVerification::Rejected
            })
        });
    let state = harness.state.clone();
    harness.recovery.expect_generate().returning(move |count| {
        assert_eq!(count, RECOVERY_CODE_COUNT);
        let mut state = state.lock().unwrap();
        state.calls.push("recovery");
        state.recoveries += 1;
        Ok((0..count)
            .map(|index| Zeroizing::new(format!("RECOVERY-{}-{index}", state.recoveries)))
            .collect())
    });
    let state = harness.state.clone();
    harness
        .secrets
        .expect_protect()
        .returning(move |id, secret| {
            assert!(!id.as_uuid().is_nil());
            assert_eq!(secret, &[7; 20]);
            state.lock().unwrap().calls.push("protect_user_totp");
            Ok(vec![0xb6, 7])
        });
    let state = harness.state.clone();
    harness
        .secrets
        .expect_expose()
        .returning(move |id, protected| {
            assert!(!id.as_uuid().is_nil());
            assert_eq!(protected, &[0xb6, 7]);
            state.lock().unwrap().calls.push("expose_user_totp");
            Ok(Zeroizing::new(vec![7; 20]))
        });
}
