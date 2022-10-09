use std::collections::HashMap;
use scrypto::prelude::{Decimal, NonFungibleId};
use scrypto::{NonFungibleData};
use scrypto::core::Runtime;
use crate::proposals::ProposalStatus;

#[derive(NonFungibleData)]
pub struct VoterCard {
    pub voter_id: NonFungibleId,
    pub nb_of_token: Decimal,
    pub epoch_of_conversion : u64,
    pub votes : Vec<(usize, ProposalStatus)>,
    pub delegatees: Vec<NonFungibleId>
}

impl VoterCard
{
    pub fn new(voter_id: u64, with_tokens: Option<Decimal>) -> VoterCard
    {
        let initial_tokens = match with_tokens
        {
            None => { Decimal::zero() }
            Some(tokens) => { tokens }
        };
        let new_id = NonFungibleId::from_u64(voter_id);
        VoterCard
        {
            voter_id: new_id.clone(),
            nb_of_token: initial_tokens,
            epoch_of_conversion: Runtime::current_epoch() ,
            votes: vec![],
            delegatees: vec![new_id]
        }
    }

    pub fn can_delegate_to(&self, other_voter: &NonFungibleId) -> bool
    {
        for nfid in self.delegatees.iter()
        {
            if nfid == other_voter
            {
                return true;
            }
        }
        false
    }

    pub fn add_delegatee(&mut self, other_voter: NonFungibleId)
    {
        if !self.can_delegate_to(&other_voter)
        {
            self.delegatees.push(other_voter);
            self.epoch_of_conversion = Runtime::current_epoch();
        }
    }

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
}

#[cfg(test)]
mod tests
{
    use scrypto::dec;
    use scrypto::prelude::NonFungibleId;
    use crate::proposals::ProposalStatus;
    use crate::voter_card::VoterCard;


    #[test]
    fn test_correct_initialization()
    {
        let voter_card = VoterCard::new(0, Some(dec!(45)));
        assert_eq!(voter_card.nb_of_token, dec!(45));
        assert!(voter_card.can_delegate_to(&voter_card.voter_id));
    }

    #[test]
    fn test_delegate()
    {
        let mut voter_card = VoterCard::new(0, None);
        let new_id = NonFungibleId::from_u64(1);
        voter_card.add_delegatee(new_id.clone());

        assert!(voter_card.can_delegate_to(&new_id));
    }

    #[test]
    fn test_vote_for_suggestion_phase()
    {
        let mut voter_card = VoterCard::new(0, None);
        let vote = voter_card.try_vote_for(0, &ProposalStatus::SuggestionPhase);

        assert!(vote);
        assert_eq!(voter_card.votes.get(0).unwrap().0, 0);
    }
}