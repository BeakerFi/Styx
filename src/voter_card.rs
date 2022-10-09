//! We define here what is a voter card, what enables users to vote and participate in the DAO.

use scrypto::prelude::{Decimal};
use scrypto::{NonFungibleData};
use scrypto::core::Runtime;
use crate::proposals::ProposalStatus;

/// A voter card, records the different tokens locked and the epoch when they were.
/// It also records the votes that the voters casted and the delegatees the voter has.
/// Adding a delegatee resets the locking epoch of the tokens.
#[derive(NonFungibleData)]
pub struct VoterCard {

    /// Id of the voter
    pub voter_id: u64,

    /// Number of tokens of the voter
    pub nb_of_token: Decimal,

    /// Epoch when the tokens were locked
    pub lock_epoch: u64,

    /// Votes casted by the voter
    pub votes : Vec<(usize, ProposalStatus)>,

    /// Possible delagtees of the voter
    pub delegatees: Vec<u64>
}

impl VoterCard
{
    /// Instantiates a new voter card from an id and an amount of tokens
    pub fn new(voter_id: u64, with_tokens: Option<Decimal>) -> VoterCard
    {
        let initial_tokens = match with_tokens
        {
            None => { Decimal::zero() }
            Some(tokens) => { tokens }
        };

        VoterCard
        {
            voter_id: voter_id,
            nb_of_token: initial_tokens,
            lock_epoch: Self::current_epoch() ,
            votes: vec![],
            delegatees: vec![]
        }
    }

    /// Returns a boolean stating whether the given voter can delegate its tokens to another given voter
    pub fn can_delegate_to(&self, other_voter: u64) -> bool
    {
        for nfid in self.delegatees.iter()
        {
            if *nfid == other_voter
            {
                return true;
            }
        }
        false
    }

    /// Adds a delegatee to the possible delegatees of the voter and resets the lock epoch
    pub fn add_delegatee(&mut self, other_voter: u64)
    {
        if !self.can_delegate_to(other_voter)
        {
            self.delegatees.push(other_voter);
            self.lock_epoch = Self::current_epoch();
        }
    }

    /// Returns a boolean stating if the given voter can vote for a given proposal.
    /// If they can vote for the proposal, the list of votes is updated.
    pub fn try_vote_for(&mut self, proposal_id: usize, current_status: &ProposalStatus) -> bool
    {
        if !current_status.is_voting_phase() && !current_status.is_suggestion_phase()
        {
            false
        }
        else
        {

            for (id,status) in self.votes.iter()
            {
                if *id == proposal_id
                {
                    // If the proposal id was found, then the voter can only vote if the status is Voting Phase
                    // And the previous status was suggestion phase
                    return match current_status
                    {
                        ProposalStatus::VotingPhase =>
                            {
                                if status.is_suggestion_phase()
                                {
                                    true
                                } else {
                                    false
                                }
                            }
                        _ => { false }
                    }
                }
            }

            // If nothing was found, add the vote to the votes and return true
            self.votes.push((proposal_id, current_status.clone()));
            true
        }

    }

    /// This function is used as a trick to be able to unit test the module.
    /// The function `Runtime::current_epoch` returns a `Not yet implemented error`.
    #[inline]
    fn current_epoch() -> u64
    {
        // For tests change next line to 0
        Runtime::current_epoch()
    }
}

//     / \
//    / | \  TO MAKE THE TESTS WORK, MAKE CHANGES IN THE FUNCTION current_epoch
//   /  •  \
#[cfg(test)]
mod tests
{
    use crate::proposals::ProposalStatus;
    use crate::voter_card::VoterCard;

    #[test]
    fn test_delegate()
    {
        let mut voter_card = VoterCard::new(0, None);
        voter_card.add_delegatee(1);

        assert!(voter_card.can_delegate_to(1));
    }

    #[test]
    fn test_vote_for_suggestion_phase()
    {
        let mut voter_card = VoterCard::new(0, None);
        let vote = voter_card.try_vote_for(0, &ProposalStatus::SuggestionPhase);

        assert!(vote);
        assert_eq!(voter_card.votes.get(0).unwrap().0, 0);
    }

    #[test]
    fn test_vote_for_voting_phase()
    {
        let mut voter_card = VoterCard::new(0, None);
        let vote = voter_card.try_vote_for(0, &ProposalStatus::VotingPhase);

        assert!(vote);
        assert_eq!(voter_card.votes.get(0).unwrap().0, 0)
    }

    #[test]
    fn test_already_vote_suggestion_phase()
    {
        let mut voter_card = VoterCard::new(0, None);
        voter_card.try_vote_for(0, &ProposalStatus::SuggestionPhase);
        let vote = voter_card.try_vote_for(0, &ProposalStatus::SuggestionPhase);

        assert!(!vote);
    }

    #[test]
    fn test_already_vote_suggestion_phase_2()
    {
        let mut voter_card = VoterCard::new(0, None);
        voter_card.try_vote_for(0, &ProposalStatus::VotingPhase);
        let vote = voter_card.try_vote_for(0, &ProposalStatus::SuggestionPhase);

        assert!(!vote);
    }

    #[test]
    fn test_already_vote_voting_phase()
    {
        let mut voter_card = VoterCard::new(0, None);
        voter_card.try_vote_for(0, &ProposalStatus::VotingPhase);
        let vote = voter_card.try_vote_for(0, &ProposalStatus::VotingPhase);

        assert!(!vote);
    }

    #[test]
    fn test_multiple_votes()
    {
        let mut voter_card = VoterCard::new(0, None);
        for i in 0..10
        {
            let vote = voter_card.try_vote_for(i, &ProposalStatus::VotingPhase);
            assert!(vote);
            assert_eq!(voter_card.votes.get(i).unwrap().0, i);
        }
    }
}