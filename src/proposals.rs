use std::collections::HashMap;
use scrypto::dec;
use scrypto::prelude::{Decimal};

#[derive(sbor::TypeId, sbor::Encode, sbor::Decode, sbor::Describe, Clone)]
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
    pub delegated_votes: HashMap<u64, Decimal>,

    /// To whom someone has delegated
    /// Should use a union find algorithm in the future
    pub delegation_to: HashMap<u64, u64>,

    /// Epoch of expiration
    pub epoch_expiration: u64,

}

impl Proposal
{

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