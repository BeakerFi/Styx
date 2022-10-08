use std::collections::HashMap;
use scrypto::prelude::{Decimal, NonFungibleId};

pub enum Vote
{
    For,
    Against,
    Blank
}
#[derive(sbor::TypeId, sbor::Encode, sbor::Decode, sbor::Describe, Clone)]
pub enum ProposalStatus
{
    SuggestionPhase,
    SuggestionRejected,
    VotingPhase,
    ProposalRejected,
    ProposalAccepted
}

#[derive(sbor::TypeId, sbor::Encode, sbor::Decode, sbor::Describe, Clone)]
pub enum VotingParametersChange
{
    SupportPeriod(u64),
    VotePeriod(u64),
    SuggestionApprovalThreshold(Decimal),
}

#[derive(sbor::TypeId, sbor::Encode, sbor::Decode, sbor::Describe, Clone)]
pub struct Proposal
{
    /// Id of the proposal
    pub id: usize,

    /// Description of the proposal
    pub description: String,

    /// Change to be enacted
    pub change: VotingParametersChange,

    /// Current status of the proposal
    pub status: ProposalStatus,

    /// Support
    pub supporting_votes: Decimal,

    /// Casted votes for of the proposal
    pub voted_for: Decimal,

    pub voted_against: Decimal,

    pub blank_votes: Decimal,

    /// Number of votes delegated to a ComponentAddress
    pub delegated_votes: HashMap<NonFungibleId, Decimal>,

    /// Epoch of expiration
    pub epoch_expiration: u64,

}

impl ProposalStatus
{
    pub fn is_suggestion_phase(&self) -> bool
    {
        match self
        {
            ProposalStatus::SuggestionPhase => {true},
            _ => {false}
        }
    }

    pub fn is_voting_phase(&self) -> bool
    {
        match self
        {
            ProposalStatus::VotingPhase => {true}
            _ => {false}
        }
    }

}