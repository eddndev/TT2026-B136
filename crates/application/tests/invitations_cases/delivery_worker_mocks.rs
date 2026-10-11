use super::*;

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
mockall::mock! { pub Delivery {} impl InvitationDelivery for Delivery {
    fn deliver(&self,lease:&InvitationDeliveryLease,token:Zeroizing<Vec<u8>>)->Result<ResetDeliveryOutcome,ApplicationError>;
} }
mockall::mock! { pub Time {} impl Clock for Time {
    fn now(&self)->OffsetDateTime;
} }
