use scrypto::prelude::{Decimal};
use scrypto::{NonFungibleData};
use scrypto::core::Runtime;
use crate::proposals::ProposalStatus;
use scrypto::dec;


#[derive(NonFungibleData)]
pub struct VoterCard {
    pub voter_id: u64,
    pub total_number_of_token : Decimal,
    pub locked_tokens: Vec<Decimal>,
    pub lock_epoch: Vec<u64>,
    pub votes : Vec<(usize, ProposalStatus)>,
    pub delegatees: Vec<u64>
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

        VoterCard
        {
            voter_id: voter_id, //new_id.clone(),
            total_number_of_token : initial_tokens,
            locked_tokens: vec![initial_tokens],
            lock_epoch: vec![Self::current_epoch()] ,
            votes: vec![],
            delegatees: vec![voter_id]
        }
    }

    pub fn add_amount(&mut self, amount : Decimal) {
        self.total_number_of_token += amount;
        self.locked_tokens.push(amount);
        self.lock_epoch.push(Self::current_epoch())
    }


    pub fn remove_amount(&mut self, amount : Decimal){

        let mut amount = amount; // Pour faire une copie mutable de amount, j'ai peur que autrement ça change amount ailleur

        if self.total_number_of_token == amount {
            self.remove_all();
        } 
        else {

            while amount > dec!("0") {
                let tokens = self.locked_tokens.pop().unwrap();
                if tokens > amount {
                    self.locked_tokens.push(tokens - amount)
                    // putain je deviens matrixé : ça fait gagner 2 opérations (soustraction + comparaison si je break là). En fait on peut faire encore plus moche et mettre un While True si je fais ça, ça vaut le coup ?
                } else {
                    self.lock_epoch.pop();
                }
                amount -= tokens;
            }

        }

    }

    pub fn remove_all(&mut self) -> Decimal {
        let total_number_of_token = self.total_number_of_token;
        self.total_number_of_token = dec!("0");
        self.locked_tokens = vec![];
        self.lock_epoch = vec![];
        total_number_of_token
    }



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

    pub fn add_delegatee(&mut self, other_voter: u64)
    {
        if !self.can_delegate_to(other_voter)
        {
            self.delegatees.push(other_voter);
            // self.lock_epoch = Self::current_epoch(); 
            self.init_fusion();
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

    fn init_fusion(&mut self){
        self.locked_tokens = vec![self.total_number_of_token];
        self.lock_epoch = vec![Self::current_epoch()];
    }

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
    use scrypto::dec;
    use scrypto::prelude::NonFungibleId;
    use crate::proposals::ProposalStatus;
    use crate::voter_card::VoterCard;


    #[test]
    fn test_correct_initialization()
    {
        let voter_card = VoterCard::new(0, Some(dec!("45")));
        assert_eq!(voter_card.locked_tokens, vec![dec!("45")]);
        assert!(voter_card.can_delegate_to(voter_card.voter_id));
    }

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