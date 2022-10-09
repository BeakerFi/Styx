//! We define here what is a proposal and how it can change the DAO

use std::collections::HashMap;
use scrypto::dec;
use scrypto::prelude::{Decimal};

/// A voter can not only vote For or Against a Proposal but also Blank.
/// Blank votes are not taken into account when counting votes but we could add a reward for voting
/// and Blank votes would count to get the reward.
#[derive(sbor::TypeId, sbor::Encode, sbor::Decode, sbor::Describe, Clone)]
pub enum Vote
{
    For,
    Against,
    Blank
}


/// Status of an ongoing Proposal
#[derive(sbor::TypeId, sbor::Encode, sbor::Decode, sbor::Describe, Clone)]
pub enum ProposalStatus
{
    SuggestionPhase,
    SuggestionRejected,
    VotingPhase,
    ProposalRejected,
    ProposalAccepted
}

/// Proposed change to parameters of votes. If a proposal is accepted and changes are made to the
/// voting system, these changes are taken into accounts for new proposals.
#[derive(sbor::TypeId, sbor::Encode, sbor::Decode, sbor::Describe, Clone)]
pub enum VotingParametersChange
{
    SupportPeriod(u64),
    VotePeriod(u64),
    SuggestionApprovalThreshold(Decimal),
}

/// Proposal that can be made to the DAO.
/// A Proposal goes through different phases. Everyone can submit a Proposal and it stays in suggestion
/// phase until a certain amount of tokens support the Proposal. After that, the Proposal goes into
/// the voting phase and it the proposed changes are enacted if the majority votes for it.
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

    /// Numbers of votes supporting the proposal
    pub supporting_votes: Decimal,

    /// Casted votes for of the proposal
    pub voted_for: Decimal,

    /// Casted votes against the proposal
    pub voted_against: Decimal,

    /// Casted blank votes
    pub blank_votes: Decimal,

    /// Number of votes delegated to a ComponentAddress
    pub delegated_votes: HashMap<u64, Decimal>,

    /// To whom someone has delegated (should implement a Union-Find algorithm for better complexity)
    pub delegation_to: HashMap<u64, u64>,

    /// Epoch of expiration
    pub epoch_expiration: u64,

}

impl Proposal
{

    /// Adds a delegation link between two voters and makes sure that there is no delegation loop by
    /// doing so.
    /// It also transfers the delagetd votes of the person delegating to the delegatee
    pub fn add_delegation(&mut self, from: u64, to: u64, amount: Decimal)
    {

        assert!(self.delegation_to.get(&from).is_none(), "You already delegated to someone else");

        let mut end_of_line_delegator = to;
        // Look for the last delegator
        let mut new_link = self.get_delegatee(end_of_line_delegator);

        while new_link != end_of_line_delegator
        {
            end_of_line_delegator = new_link;
            new_link = self.get_delegatee(end_of_line_delegator);
        }

        if from == end_of_line_delegator
        {
            // Delegation loop
            panic!("Cannot delegate to voter {} because its votes are already delegated to you", to);
        }
        else
        {
            let mut number_of_votes = amount;

            match self.delegated_votes.get_mut(&from)
            {
                None => {}
                Some(votes) =>
                    {
                        number_of_votes = number_of_votes + *votes;
                        *votes = dec!(0);
                    }
            }

            self.delegation_to.insert(from, end_of_line_delegator);

            match self.delegated_votes.get_mut(&end_of_line_delegator)
            {
                None => { self.delegated_votes.insert(end_of_line_delegator, number_of_votes); }
                Some(votes) => { *votes = *votes + number_of_votes; }
            }
        }
    }

    /// Returns the delegatee's id of a voter for the Proposal. If the voter did not delegate
    /// its tokens to anyone, then it returns the id of the voter.
    pub fn get_delegatee(&self, of: u64) -> u64
    {
        match self.delegation_to.get(&of)
        {
            None => of,
            Some(del) => *del
        }
    }

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

    pub fn is_suggestion_rejected(&self) -> bool
    {
        match self
        {
            ProposalStatus::SuggestionRejected => {true}
            _ => {false}
        }
    }

    pub fn is_proposal_rejected(&self) -> bool
    {
        match self
        {
            ProposalStatus::ProposalRejected => {true}
            _ => {false}
        }
    }

    pub fn is_proposal_accepted(&self) -> bool
    {
        match self
        {
            ProposalStatus::ProposalAccepted => {true}
            _ => {false}
        }
    }

}

#[cfg(test)]
mod tests
{
    use scrypto::dec;
    use crate::proposals::{Proposal, ProposalStatus, VotingParametersChange};

    #[test]
    fn test_add_delegation()
    {
        let mut prop = Proposal
        {
            id: 0,
            description: "".to_string(),
            change: VotingParametersChange::VotePeriod(0),
            status: ProposalStatus::SuggestionPhase,
            supporting_votes: Default::default(),
            voted_for: Default::default(),
            voted_against: Default::default(),
            blank_votes: Default::default(),
            delegated_votes: Default::default(),
            delegation_to: Default::default(),
            epoch_expiration: 0
        };

        prop.add_delegation(0,1, dec!(1000));

        assert_eq!(*prop.delegation_to.get(&0).unwrap(), 1);
        assert_eq!(*prop.delegated_votes.get(&1).unwrap(), dec!(1000));
    }

    #[test]
    fn test_add_delegation_chain()
    {
        let mut prop = Proposal
        {
            id: 0,
            description: "".to_string(),
            change: VotingParametersChange::VotePeriod(0),
            status: ProposalStatus::SuggestionPhase,
            supporting_votes: Default::default(),
            voted_for: Default::default(),
            voted_against: Default::default(),
            blank_votes: Default::default(),
            delegated_votes: Default::default(),
            delegation_to: Default::default(),
            epoch_expiration: 0
        };

        prop.add_delegation(0, 1, dec!(1000));
        prop.add_delegation(1,2, dec!(300));

        assert_eq!(*prop.delegation_to.get(&1).unwrap(), 2);
        assert_eq!(*prop.delegated_votes.get(&1).unwrap(), dec!(0));
        assert_eq!(*prop.delegated_votes.get(&2).unwrap(), dec!(1300));
    }

    #[test]
    fn test_add_delegation_chain_end()
    {
        let mut prop = Proposal
        {
            id: 0,
            description: "".to_string(),
            change: VotingParametersChange::VotePeriod(0),
            status: ProposalStatus::SuggestionPhase,
            supporting_votes: Default::default(),
            voted_for: Default::default(),
            voted_against: Default::default(),
            blank_votes: Default::default(),
            delegated_votes: Default::default(),
            delegation_to: Default::default(),
            epoch_expiration: 0
        };

        prop.add_delegation(0, 1, dec!(1000));
        prop.add_delegation(2,0, dec!(300));

        assert_eq!(*prop.delegation_to.get(&2).unwrap(), 1);
        assert_eq!(*prop.delegated_votes.get(&1).unwrap(), dec!(1300));
    }

    #[test]
    #[should_panic]
    fn test_add_delegation_fail_loop()
    {
        let mut prop = Proposal
        {
            id: 0,
            description: "".to_string(),
            change: VotingParametersChange::VotePeriod(0),
            status: ProposalStatus::SuggestionPhase,
            supporting_votes: Default::default(),
            voted_for: Default::default(),
            voted_against: Default::default(),
            blank_votes: Default::default(),
            delegated_votes: Default::default(),
            delegation_to: Default::default(),
            epoch_expiration: 0
        };

        prop.add_delegation(0, 1, dec!(1000));
        prop.add_delegation(1,2, dec!(1));
        prop.add_delegation(2,0, dec!(300));
    }
}