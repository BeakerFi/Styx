//! We define here what is a voter card, what enables users to vote and participate in the DAO.

use scrypto::prelude::{Decimal};
use scrypto::{dec, NonFungibleData};
use crate::decimal_maths::{cbrt, exp, ln};
use crate::proposals::ProposalStatus;

/// A voter card, records the different tokens locked and the epoch when they were.
/// It also records the votes that the voters casted and the delegatees the voter has.
/// Adding a delegatee resets the locking epoch of the tokens.
#[derive(NonFungibleData)]
pub struct VoterCard {

    /// Id of the voter
    pub voter_id: u64,

    /// Total number of tokens held by the voter
    pub total_number_of_token : Decimal,

    /// Pairs of tokens with their lock period
    pub locked_tokens: Vec<(Decimal,u64)>,

    /// Votes casted by the voter
    pub votes : Vec<(usize, ProposalStatus)>,

    /// Possible delegatees of the voter
    pub delegatees: Vec<u64>
}

impl VoterCard
{
    /// Instantiates a new voter card from an id and an amount of tokens
    pub fn new(voter_id: u64) -> VoterCard
    {
        VoterCard
        {
            voter_id: voter_id,
            total_number_of_token : dec!(0),
            locked_tokens: vec![],
            votes: vec![],
            delegatees: vec![]
        }
    }

    pub fn add_tokens(&mut self, amount: Decimal, lock_period: u64)
    {
        self.total_number_of_token += amount;
        self.locked_tokens.push((amount, lock_period));
    }

    /// Returns a boolean stating whether the given voter can delegate its tokens to another given voter
    pub fn can_delegate_to(&self, other_voter: u64) -> bool
    {
        if self.voter_id == other_voter
        {
            return true;
        }
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
    pub fn add_delegatee(&mut self, other_voter: u64, current_epoch: u64)
    {
        if !self.can_delegate_to(other_voter)
        {
            self.delegatees.push(other_voter);
            self.merge(current_epoch);
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

    pub fn retrieve_tokens(&mut self, amount: Decimal)
    {
        assert!(amount > self.total_number_of_token, "Cannot retrieve more tokens than owned");

        if amount == self.total_number_of_token
        {
            self.locked_tokens = vec![];
            self.total_number_of_token = dec!(0);
        }
        else
        {
            let mut amount_loop = amount;
            while amount_loop > dec!("0")
            {
                let (tokens,time) = self.locked_tokens.pop().unwrap();
                if tokens > amount
                {
                    self.locked_tokens.push( (tokens- amount_loop,time));
                    self.total_number_of_token = self.total_number_of_token + amount_loop;
                }

                self.total_number_of_token = self.total_number_of_token - tokens;
                amount_loop = amount_loop - tokens;
            }
        }
    }

    pub fn retrieve_all_tokens(&mut self) -> Decimal
    {
        let total_number_of_token = self.total_number_of_token;
        self.total_number_of_token = dec!("0");
        self.locked_tokens = vec![];
        total_number_of_token
    }

    /// Computes the voting power associated to a voter card
    /// The function is power(t,x) = (tanh(-2016/t) + 1)*cbrt(ln(x)),
    /// where `t = current_epoch - lock_epoch` and `x` is the total amount of tokens.
    /// For more details on this choice, please read the whitepaper.
    pub fn voting_power(&self, current_epoch: u64) -> Decimal
    {
        let mut total = Decimal::zero();
        for (tokens,time_tmp) in &self.locked_tokens
        {
            // In our tests, time can get negative so we transform in Decimal before subtracting
            let time = current_epoch - *time_tmp;
            total = total + Self::sub_voting_function(time, *tokens);
        }

        total
    }

    fn sub_voting_function(time: u64, tokens: Decimal) -> Decimal
    {

        if time==0
        {
            return Decimal::zero();
        }


        let exp = exp(- dec!(2016) / time );
        let time_multiplicator =  ( exp - 1 )/ (exp + 1)  + 1;
        if time_multiplicator == Decimal::zero()
        {
            Decimal::zero()
        }
        else
        {
            let corrected_tokens = time_multiplicator*tokens + 1; // Add 1 to make sure that it is > 0
            let total = cbrt(ln(corrected_tokens));
            total.max(Decimal::zero())
        }
    }

    fn merge(&mut self, current_epoch: u64)
    {
        if !self.locked_tokens.is_empty()
        {
            self.locked_tokens = vec![(self.total_number_of_token, current_epoch)];
        }
    }
}

#[cfg(test)]
mod tests
{
    use radix_engine::ledger::TypedInMemorySubstateStore;
    use scrypto::dec;
    use scrypto_unit::TestRunner;
    use transaction::builder::ManifestBuilder;
    use crate::proposals::ProposalStatus;
    use crate::voter_card::VoterCard;


    #[test]
    fn test_correct_initialization()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut voter_card = VoterCard::new(0);
        voter_card.add_tokens(dec!(45), test_runner.get_current_epoch());
        assert_eq!(voter_card.locked_tokens, vec![(dec!("45"), test_runner.get_current_epoch())]);
        assert!(voter_card.can_delegate_to(voter_card.voter_id));
    }

    #[test]
    fn test_delegate()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);
        let mut voter_card = VoterCard::new(0);
        voter_card.add_delegatee(1, test_runner.get_current_epoch());

        assert!(voter_card.can_delegate_to(1));
    }

    #[test]
    fn test_vote_for_suggestion_phase()
    {
        let mut voter_card = VoterCard::new(0);
        let vote = voter_card.try_vote_for(0, &ProposalStatus::SuggestionPhase);

        assert!(vote);
        assert_eq!(voter_card.votes.get(0).unwrap().0, 0);
    }

    #[test]
    fn test_vote_for_voting_phase()
    {
        let mut voter_card = VoterCard::new(0);
        let vote = voter_card.try_vote_for(0, &ProposalStatus::VotingPhase);

        assert!(vote);
        assert_eq!(voter_card.votes.get(0).unwrap().0, 0)
    }

    #[test]
    fn test_already_vote_suggestion_phase()
    {
        let mut voter_card = VoterCard::new(0);
        voter_card.try_vote_for(0, &ProposalStatus::SuggestionPhase);
        let vote = voter_card.try_vote_for(0, &ProposalStatus::SuggestionPhase);

        assert!(!vote);
    }

    #[test]
    fn test_already_vote_suggestion_phase_2()
    {
        let mut voter_card = VoterCard::new(0);
        voter_card.try_vote_for(0, &ProposalStatus::VotingPhase);
        let vote = voter_card.try_vote_for(0, &ProposalStatus::SuggestionPhase);

        assert!(!vote);
    }

    #[test]
    fn test_already_vote_voting_phase()
    {
        let mut voter_card = VoterCard::new(0);
        voter_card.try_vote_for(0, &ProposalStatus::VotingPhase);
        let vote = voter_card.try_vote_for(0, &ProposalStatus::VotingPhase);

        assert!(!vote);
    }

    #[test]
    fn test_multiple_votes()
    {
        let mut voter_card = VoterCard::new(0);
        for i in 0..10
        {
            let vote = voter_card.try_vote_for(i, &ProposalStatus::VotingPhase);
            assert!(vote);
            assert_eq!(voter_card.votes.get(i).unwrap().0, i);
        }
    }
}
