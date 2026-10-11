use super::*;

mockall::mock! { pub Identity {} impl IdentityWorkflow for Identity {
    fn bootstrap_owner(&self, email:&str, password:&str)->Result<EnrollmentResult,ApplicationError>;
    fn create_user(&self, token:&str, email:&str, password:&str, role:Role)->Result<EnrollmentResult,ApplicationError>;
    fn start_login(&self, email:&str, password:&str)->Result<LoginChallenge,ApplicationError>;
    fn complete_totp(&self, token:&str, code:&str)->Result<SessionResult,ApplicationError>;
    fn complete_recovery(&self, token:&str, code:&str)->Result<SessionResult,ApplicationError>;
    fn authenticate(&self, token:&str)->Result<Principal,ApplicationError>;
    fn session_status(&self, token:&str)->Result<SessionStatus,ApplicationError>;
    fn record_activity(&self, token:&str)->Result<SessionStatus,ApplicationError>;
    fn authorize(&self, token:&str, permission:Permission)->Result<Principal,ApplicationError>;
    fn logout(&self, token:&str)->Result<(),ApplicationError>;
} }
mockall::mock! { pub Users {} impl UserRepository for Users {
    fn has_users(&self)->Result<bool,ApplicationError>;
    fn insert_initial_owner(&self,user:UserRecord,at:OffsetDateTime)->Result<bool,ApplicationError>;
    fn insert(&self,user:UserRecord,actor:UserId,at:OffsetDateTime)->Result<(),ApplicationError>;
    fn find_by_email(&self,email:&str)->Result<Option<UserRecord>,ApplicationError>;
    fn find_by_id(&self,id:UserId)->Result<Option<UserRecord>,ApplicationError>;
    fn replace_recovery_codes(&self,id:UserId,expected:u64,codes:RecoveryCodeSet,at:OffsetDateTime)->Result<(),ApplicationError>;
} }
mockall::mock! { pub Store {} impl InvitationStore for Store {
    fn issue(&self,actor:InvitationActor,command:IssueInvitation)->Result<InvitationSummary,ApplicationError>;
    fn resend(&self,actor:InvitationActor,id:InvitationId,command:ResendInvitation)->Result<InvitationSummary,ApplicationError>;
    fn list(&self,actor:InvitationActor,query:InvitationQuery)->Result<InvitationPage,ApplicationError>;
    fn get(&self,actor:InvitationActor,id:InvitationId)->Result<InvitationSummary,ApplicationError>;
    fn inspect(&self,digest:Sha256Digest)->Result<Option<InvitationCandidate>,ApplicationError>;
    fn save_pending(&self,command:PrepareInvitation)->Result<PendingInvitation,ApplicationError>;
    fn finish(&self,command:FinishInvitation)->Result<InvitationReceipt,ApplicationError>;
    fn completed(&self,digest:Sha256Digest,operation:Uuid)->Result<Option<InvitationCompletion>,ApplicationError>;
    fn claim_delivery(&self,claim:InvitationDeliveryClaim)->Result<Option<InvitationDeliveryLease>,ApplicationError>;
    fn finish_delivery(&self,result:InvitationDeliveryFinish)->Result<bool,ApplicationError>;
} }
mockall::mock! { pub Protector {} impl InvitationProtector for Protector {
    fn protect(&self,context:InvitationProtectionContext,plaintext:&[u8])->Result<Vec<u8>,ApplicationError>;
    fn expose(&self,context:InvitationProtectionContext,protected:&[u8])->Result<Zeroizing<Vec<u8>>,ApplicationError>;
} }
mockall::mock! { pub Secrets {} impl SecretProtector for Secrets {
    fn protect(&self,id:UserId,secret:&[u8])->Result<Vec<u8>,ApplicationError>;
    fn expose(&self,id:UserId,protected:&[u8])->Result<Zeroizing<Vec<u8>>,ApplicationError>;
} }
mockall::mock! { pub Passwords {} impl PasswordHasher for Passwords {
    fn hash(&self,value:&str)->Result<String,DomainError>;
    fn verify(&self,value:&str,stored:&str)->Result<PasswordVerification,DomainError>;
} }
mockall::mock! { pub Totp {} impl TotpProvider for Totp {
    fn enroll(&self,email:&str)->Result<TotpEnrollment,DomainError>;
    fn verify(&self,secret:&[u8],code:&str,at:u64)->Result<TotpVerification,DomainError>;
    fn current_code(&self,secret:&[u8],at:u64)->Result<String,DomainError>;
} }
mockall::mock! { pub Recovery {} impl RecoveryCodeGenerator for Recovery {
    fn generate(&self,count:usize)->Result<Vec<Zeroizing<String>>,DomainError>;
} }
mockall::mock! { pub Tokens {} impl ResetTokenSource for Tokens {
    fn generate(&self)->Result<Zeroizing<[u8;32]>,ApplicationError>;
} }
mockall::mock! { pub Limiter {} impl InvitationLimiter for Limiter {
    fn admit(&self,digest:Sha256Digest)->Result<bool,ApplicationError>;
} }
mockall::mock! { pub Digests {} impl DocumentHasher for Digests {
    fn hash_bytes(&self,bytes:&[u8])->Sha256Digest;
    fn hash_stream(&self,reader:&mut dyn Read)->Result<Sha256Digest,DomainError>;
} }
mockall::mock! { pub Time {} impl Clock for Time {
    fn now(&self)->OffsetDateTime;
} }
