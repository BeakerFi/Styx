use std::collections::HashMap;
use scrypto::core::Runtime;
use scrypto::dec;
use scrypto::math::Decimal;
use crate::decimal_maths::{exp, ln, cbrt};
use crate::proposals::{Proposal, ProposalStatus, Vote, VotingParametersChange};
use crate::voter_card::VoterCard;

// Suggestion: before a proposal, duration: 1 week ~ 168 epoch ( 1 epoch ~ 1h)
// EPOCH cooldown : 3 months ~ 2016 epochs
// UK: 100k signatures = parliament vote; nb_citizens: 65kk : 0.15% for a proposal to reach a vote

pub struct BallotBox
{
    new_proposal_id: usize,
    proposals: Vec<Proposal>,
    support_period: u64,
    vote_period: u64,
    suggestion_approval_threshold: Decimal,
    minimum_votes_threshold: Decimal
}

impl BallotBox
{

    pub fn new() -> BallotBox
    {

        BallotBox
        {
            new_proposal_id: 0,
            proposals: vec![],
            support_period: 168,
            vote_period: 168,
            suggestion_approval_threshold: dec!("0.0015"),
            minimum_votes_threshold: Decimal::zero(),
        }

    }

    pub fn make_proposal(&mut self, description: String, suggested_change: VotingParametersChange)
    {
       let proposal = Proposal
       {
           id: self.new_proposal_id,
           description,
           change: suggested_change,
           status: ProposalStatus::SuggestionPhase,
           supporting_votes: Decimal::zero(),
           voted_for: Decimal::zero(),
           voted_against: Decimal::zero(),
           blank_votes: Decimal::zero(),
           delegated_votes: HashMap::new(),
           epoch_expiration: Self::current_epoch() + self.support_period
       };

        self.new_proposal_id += 1;
        self.proposals.push(proposal);
    }

    pub fn support_proposal(&mut self, proposal_id: usize, voter_card: &mut VoterCard)
    {
        assert!(proposal_id < self.new_proposal_id, "This proposal does not exist!");

        let proposal: &mut Proposal = self.proposals.get_mut(proposal_id).unwrap();
        assert!(proposal.epoch_expiration > Self::current_epoch(), "This proposal has expired");
        assert!(proposal.status.is_suggestion_phase(), "Cannot support a proposal that is not in suggestion phase");

        let can_support = voter_card.try_vote_for(proposal_id, &proposal.status);

        if !can_support
        {
            panic!("You already supported the proposition");
        }
        let voting_power = Self::voting_power(voter_card);
        proposal.supporting_votes = proposal.supporting_votes + voting_power;
    }

    pub fn advance_with_proposal(&mut self, proposal_id: usize, total_tokens: Decimal)
    {
        assert!(proposal_id < self.new_proposal_id, "This proposal does not exist!");

        let proposal: &mut Proposal = self.proposals.get_mut(proposal_id).unwrap();
        assert!(proposal.epoch_expiration <= Self::current_epoch(), "This proposal has not finished its current period");


        match proposal.status
        {
            ProposalStatus::SuggestionPhase =>
                {
                    if proposal.supporting_votes / total_tokens > self.suggestion_approval_threshold
                    {
                        proposal.status = ProposalStatus::VotingPhase;
                        proposal.epoch_expiration = Self::current_epoch() + self.vote_period;
                    }
                    else
                    {
                        proposal.status = ProposalStatus::SuggestionRejected;
                    }
                }

            ProposalStatus::VotingPhase =>
                {

                    if proposal.voted_for + proposal.voted_against > self.minimum_votes_threshold
                    {
                        if proposal.voted_for >= proposal.voted_against
                        {
                            proposal.status = ProposalStatus::ProposalAccepted;
                            let changes = proposal.change.clone();
                            self.execute_proposal(&changes);
                        }
                        else
                        {
                            proposal.status = ProposalStatus::ProposalRejected;
                        }
                    }
                    else
                    {
                        proposal.status = ProposalStatus::ProposalRejected;
                    }
                }
            _ => { panic!("Proposal cannot advance forward! It has already been accepted or rejected.") }
        }
    }

    pub fn delegate_for_proposal(&mut self, proposal_id: usize, delegate_to: u64, voter_card: &mut VoterCard)
    {
        assert!(proposal_id < self.new_proposal_id, "This proposal does not exist!");
        assert!(voter_card.can_delegate_to(delegate_to), "Cannot delegate to this person id");

        let proposal: &mut Proposal = self.proposals.get_mut(proposal_id).unwrap();
        assert!(proposal.epoch_expiration > Self::current_epoch(), "The voting period has already ended for this proposal.");
        assert!(proposal.status.is_voting_phase(), "The proposal is not in its voting phase.");

        let can_delegate = voter_card.try_vote_for(proposal_id, &proposal.status);

        if !can_delegate
        {
            panic!("Already voted or delegated for this proposal!");
        }
        else
        {
            let nb_votes = Self::voting_power(voter_card);
            match proposal.delegated_votes.get_mut(&delegate_to)
            {
                None => { proposal.delegated_votes.insert(delegate_to.clone(), nb_votes ); }
                Some(votes) => { *votes = *votes + nb_votes; }
            }
        }




    }

    pub fn vote_for_proposal(&mut self, proposal_id: usize, voter_card: &mut VoterCard, vote: Vote)
    {
        assert!(proposal_id < self.new_proposal_id, "This proposal does not exist!");

        let proposal: &mut Proposal = self.proposals.get_mut(proposal_id).unwrap();
        assert!(proposal.epoch_expiration > Self::current_epoch(), "The voting period has already ended for this proposal.");
        assert!(proposal.status.is_voting_phase(), "The proposal is not in its voting phase.");


        let mut total_voting_power = dec!(0);
        let can_vote = voter_card.try_vote_for(proposal_id, &proposal.status);

        if can_vote
        {
            total_voting_power = Self::voting_power(voter_card);
        }

        match proposal.delegated_votes.get_mut(&voter_card.voter_id)
        {
            None => {}
            Some(deleg_votes) =>
                {
                    total_voting_power = total_voting_power + *deleg_votes;
                    *deleg_votes = dec!(0);
                }
        }

        match vote
        {
            Vote::For => { proposal.voted_for = proposal.voted_for + total_voting_power; }
            Vote::Against => { proposal.voted_against = proposal.voted_against + total_voting_power; }
            Vote::Blank => { proposal.blank_votes = proposal.blank_votes; }
        }
    }

    fn voting_power(voter_card: &VoterCard) -> Decimal
    {
        if Self::current_epoch() == voter_card.lock_epoch
        {
            return Decimal::zero();
        }
        // In our tests, time can get negative so we transform in Decimal before subtracting
        let time = Decimal::from(Self::current_epoch()) - Decimal::from(voter_card.lock_epoch);
        let tokens = voter_card.nb_of_token;
        let exp = exp(- dec!(2016) / time );
        let time_multiplicator =  ( exp - 1 )/ (exp + 1)  + 1;
        if time_multiplicator == Decimal::zero()
        {
                Decimal::zero()
        }
        else
        {
            let total = cbrt(ln(time_multiplicator*tokens));
            total.max(Decimal::zero())
        }

    }

    fn execute_proposal(&mut self, change_to_do: &VotingParametersChange)
    {
        match change_to_do
        {
            VotingParametersChange::SupportPeriod(new_period) => { self.support_period = *new_period }
            VotingParametersChange::VotePeriod(new_period) => { self.vote_period = *new_period }
            VotingParametersChange::SuggestionApprovalThreshold(threshold) => { self.suggestion_approval_threshold = *threshold }
        }
    }

    fn current_epoch() -> u64
    {
        // For tests change to 0
        Runtime::current_epoch()
    }
}

//     / \
//    / | \  TO MAKE THE TESTS WORK, MAKE CHANGES IN THE FUNCTION current_epoch OF voter_card.rs AND
//   /  •  \ ballot_box.rs
#[cfg(test)]
mod tests
{
    use scrypto::dec;
    use scrypto::math::Decimal;
    use crate::ballot_box::BallotBox;
    use crate::proposals::{ProposalStatus, Vote, VotingParametersChange};
    use crate::voter_card::VoterCard;

    #[test]
    fn test_new_proposal()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description.clone(),
            VotingParametersChange::VotePeriod(0)
        );

        let proposal = ballot_box.proposals.get(0).unwrap();

        assert_eq!(proposal.description, description);
        assert_eq!(proposal.id, 0);
        assert_eq!(ballot_box.new_proposal_id, 1);
        assert!(proposal.status.is_suggestion_phase());
        assert_eq!(proposal.epoch_expiration, 168);
    }

    #[test]
    fn test_support_proposal()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );

        let mut voting_card = VoterCard::new(0, Some(dec!(1234)));
        ballot_box.support_proposal(0, &mut voting_card);

        let proposal = ballot_box.proposals.get(0).unwrap();

        assert_eq!(proposal.supporting_votes, BallotBox::voting_power(&voting_card));
    }

    #[test]
    #[should_panic]
    fn test_support_proposal_fail()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();

        proposal.status = ProposalStatus::VotingPhase;

        let mut voting_card = VoterCard::new(0, Some(dec!(1234)));

        ballot_box.support_proposal(0, &mut voting_card);
    }

    #[test]
    fn test_advance_with_proposal_support()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();

        proposal.epoch_expiration = 0;
        proposal.supporting_votes = dec!(100);

        ballot_box.advance_with_proposal(0, dec!(100));

        let updated_proposal = ballot_box.proposals.get_mut(0).unwrap();
        assert!(updated_proposal.status.is_voting_phase());
        assert_eq!(updated_proposal.epoch_expiration, 168);
    }

    #[test]
    #[should_panic]
    fn test_advance_with_proposal_support_time_fail()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let proposal = ballot_box.proposals.get(0).unwrap();
        ballot_box.advance_with_proposal(0, dec!(100));
    }

    #[test]
    fn test_advance_with_proposal_support_amount_fail()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();

        proposal.epoch_expiration = 0;
        proposal.supporting_votes = dec!(100);

        ballot_box.advance_with_proposal(0, dec!(100000));

        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        assert!(updated_proposal.status.is_suggestion_rejected());
    }

    #[test]
    fn test_advance_with_proposal_vote()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();

        proposal.epoch_expiration = 0;
        proposal.status = ProposalStatus::VotingPhase;
        proposal.voted_for = dec!(1);

        ballot_box.advance_with_proposal(0, dec!(10));

        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        assert!(updated_proposal.status.is_proposal_accepted());
    }

    #[test]
    fn test_advance_with_proposal_vote_against()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();

        proposal.epoch_expiration = 0;
        proposal.status = ProposalStatus::VotingPhase;
        proposal.voted_against = dec!(1);

        ballot_box.advance_with_proposal(0, dec!(10));

        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        assert!(updated_proposal.status.is_proposal_rejected());
    }

    #[test]
    fn test_delegate_for_proposal()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;
        let mut voting_card = VoterCard::new(0, Some(dec!(1234)));
        voting_card.add_delegatee(1);

        ballot_box.delegate_for_proposal(0, 1, &mut voting_card);
        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        assert_eq!(*updated_proposal.delegated_votes.get(&1).unwrap(), Decimal::zero());
    }

    #[test]
    #[should_panic]
    fn test_delegate_for_proposal_fail_cannot_delegate_to_id()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;
        let mut voting_card = VoterCard::new(0, Some(dec!(1234)));

        ballot_box.delegate_for_proposal(0, 1, &mut voting_card);
    }

    #[test]
    #[should_panic]
    fn test_delegate_for_proposal_fail_not_voting_phase()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut voting_card = VoterCard::new(0, Some(dec!(1234)));
        voting_card.add_delegatee(1);

        ballot_box.delegate_for_proposal(0, 1, &mut voting_card);
    }

    #[test]
    #[should_panic]
    fn test_delegate_for_proposal_fail_expired()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;
        proposal.epoch_expiration = 0;

        let mut voting_card = VoterCard::new(0, Some(dec!(1234)));
        voting_card.add_delegatee(1);

        ballot_box.delegate_for_proposal(0,1, &mut voting_card);
    }

    #[test]
    #[should_panic]
    fn test_delegate_for_proposal_fail_already_delegated()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;

        let mut voting_card = VoterCard::new(0, Some(dec!(1234)));
        voting_card.add_delegatee(1);
        voting_card.add_delegatee(2);

        ballot_box.delegate_for_proposal(0,1, &mut voting_card);
        ballot_box.delegate_for_proposal(0,2, &mut voting_card);


    }

    #[test]
    #[should_panic]
    fn test_delegate_for_proposal_fail_already_voted()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;

        let mut voting_card = VoterCard::new(0, Some(dec!(1234)));
        voting_card.add_delegatee(1);
        voting_card.try_vote_for(0, &ProposalStatus::VotingPhase);

        ballot_box.delegate_for_proposal(0,1, &mut voting_card);
    }

    #[test]
    fn test_vote_for_proposal()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;

        let mut voting_card = VoterCard::new(0, Some(dec!(1234)));
        voting_card.lock_epoch = 2016;

        ballot_box.vote_for_proposal(0, &mut voting_card, Vote::For);

        let updated_proposal = ballot_box.proposals.get(0).unwrap();

        assert!(updated_proposal.voted_for > dec!(0));
    }

    #[test]
    fn test_vote_for_proposal_with_delegated_votes_and_own_vote()
    {
        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            VotingParametersChange::VotePeriod(0)
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;

        let mut voting_card_1 = VoterCard::new(0, Some(dec!(1)));
        voting_card_1.add_delegatee(1);
        voting_card_1.lock_epoch = 2016;

        let mut voting_card_2 = VoterCard::new(1, Some(dec!(1)));
        voting_card_2.lock_epoch = 2016;

        ballot_box.delegate_for_proposal(0, 1, &mut voting_card_1);

        ballot_box.vote_for_proposal(0, &mut voting_card_2, Vote::For);

        let updated_proposal = ballot_box.proposals.get(0).unwrap();

        assert!(updated_proposal.voted_for > dec!(1));
    }



}