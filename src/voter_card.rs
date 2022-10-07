use std::collections::HashMap;
use scrypto::prelude::{Decimal, NonFungibleId};
use scrypto::{NonFungibleData};
use crate::proposals::ProposalStatus;

#[derive(NonFungibleData)]
pub struct VoterCard {
    pub voter_id: NonFungibleId,
    pub nb_of_token: Decimal,
    pub epoch_of_conversion : u64,
    pub votes : Vec<(usize, ProposalStatus)>,
}

impl VoterCard
{


    pub fn can_delegate_to(&self, other_voter: &NonFungibleId) -> bool
    {
        true
    }

    pub fn add_delegatee(&self, other_voter: &NonFungibleId)
    {
        ()
    }

    pub fn try_vote_for(&mut self, proposal_id: usize, current_status: &ProposalStatus) -> bool
    {
        if !current_status.is_voting_phase() && !current_status.is_suggestion_phase()
        {
            false
        }
        else
        {

            for (id,status) in self.votes
            {
                if id == proposal_id
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