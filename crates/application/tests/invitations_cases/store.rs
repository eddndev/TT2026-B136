use super::*;

pub fn configure(harness: &mut Harness) {
    let state = harness.state.clone();
    harness.store.expect_inspect().returning(move |digest| {
        let mut state = state.lock().unwrap();
        state.calls.push("inspect");
        assert_eq!(digest, token_digest(&state));
        Ok(state.candidate.clone())
    });
    let state = harness.state.clone();
    harness
        .store
        .expect_save_pending()
        .returning(move |command| {
            let mut state = state.lock().unwrap();
            state.calls.push("save_pending");
            assert_eq!(command.id, id());
            assert_eq!(command.revision, 1);
            assert_eq!(command.token_digest, token_digest(&state));
            assert_eq!(command.at, now());
            let candidate = state.candidate.as_mut().unwrap();
            if let Some(existing) = &candidate.pending {
                return Ok(existing.clone());
            }
            candidate.pending = Some(command.pending.clone());
            state.pending_writes += 1;
            Ok(command.pending)
        });
    let state = harness.state.clone();
    harness
        .store
        .expect_completed()
        .returning(move |digest, operation_id| {
            let mut state = state.lock().unwrap();
            state.calls.push("completed");
            assert_eq!(digest, token_digest(&state));
            Ok(state
                .completion
                .as_ref()
                .filter(|value| value.receipt.operation_id == operation_id)
                .cloned())
        });
    let state = harness.state.clone();
    harness.store.expect_finish().returning(move |command| {
        let mut state = state.lock().unwrap();
        state.calls.push("finish");
        assert_eq!(command.token_digest, token_digest(&state));
        if state.finish_error {
            state.finish_commands.push(command);
            return Err(ApplicationError::InvalidCredentials);
        }
        let receipt = InvitationReceipt {
            id: command.id,
            revision: command.revision,
            operation_id: command.operation_id,
            user_id: command.user.id,
            email: command.user.email.clone(),
            role: command.user.role,
            accepted_at: now(),
        };
        state.user = Some(command.user.clone());
        state.completion = Some(InvitationCompletion {
            receipt: receipt.clone(),
            request_digest: command.request_digest,
        });
        state.candidate = None;
        state.finish_commands.push(command);
        if state.lose_finish_response {
            return Err(ApplicationError::Port("commit response lost".into()));
        }
        Ok(receipt)
    });
}
