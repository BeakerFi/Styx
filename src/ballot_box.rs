use std::collections::HashMap;
use scrypto::dec;
use scrypto::math::Decimal;
use scrypto::prelude::ResourceAddress;
use crate::proposals::{Proposal, ProposalStatus, Vote, Change};
use crate::voter_card::VoterCard;

// Suggestion: before a proposal, duration: 1 week ~ 168 epoch ( 1 epoch ~ 1h)
// EPOCH cooldown : 3 months ~ 2016 epochs
// UK: 100k signatures = parliament vote; nb_citizens: 65kk : 0.15% for a proposal to reach a vote

/// A BallotBox is simply a list of proposals and some voting parameters that can be changed by voting
/// In the future, the voting_power function that computes the voting power associated to a bunch of
/// tokens, will also be a parameter that can be changed. Unfortunately, Scrypto doesn not enable us
/// to use closures in blueprints yet.
#[derive(sbor::TypeId, sbor::Encode, sbor::Decode, sbor::Describe, Clone)]
pub struct BallotBox
{
    /// Id of the next proposal that will be made
    new_proposal_id: usize,

    /// List of all made proposals
    proposals: Vec<Proposal>,

    /// Period of time for the support phase
    support_period: u64,

    /// Period of the time for the voting phase
    vote_period: u64,

    /// Threshold for a suggestion to be turned into a proper vote
    suggestion_approval_threshold: Decimal,

    /// Minimum of votes that should be casted for a vote to be considered legitimate
    minimum_votes_threshold: Decimal
}

impl BallotBox
{

    /// Instantiates a new BallotBox with our choice of parameters
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

    /// Creates a new proposal from the given parameters
    pub fn make_proposal(&mut self, description: String, suggested_changes: Vec<Change>, current_epoch: u64)
    {
       let proposal = Proposal
       {
           id: self.new_proposal_id,
           description,
           changes: suggested_changes,
           status: ProposalStatus::SuggestionPhase,
           supporting_votes: Decimal::zero(),
           voted_for: Decimal::zero(),
           voted_against: Decimal::zero(),
           blank_votes: Decimal::zero(),
           delegated_votes: HashMap::new(),
           delegation_to: HashMap::new(),
           epoch_expiration: current_epoch + self.support_period
       };

        self.new_proposal_id += 1;
        self.proposals.push(proposal);
    }

    /// Enables a voter to support a proposal with its voter card
    pub fn support_proposal(&mut self, proposal_id: usize, voter_card: &mut VoterCard, current_epoch: u64)
    {
        assert!(proposal_id < self.new_proposal_id, "This proposal does not exist!");

        let proposal: &mut Proposal = self.proposals.get_mut(proposal_id).unwrap();
        assert!(proposal.epoch_expiration > current_epoch, "This proposal has expired");
        assert!(proposal.status.is_suggestion_phase(), "Cannot support a proposal that is not in suggestion phase");

        let can_support = voter_card.try_vote_for(proposal_id, &proposal.status);

        if !can_support
        {
            panic!("You already supported the proposition");
        }
        let voting_power = voter_card.voting_power(current_epoch);
        proposal.supporting_votes = proposal.supporting_votes + voting_power;
    }

    /// Makes a proposal advance to its next phase if possible
    pub fn advance_with_proposal(&mut self, proposal_id: usize, total_tokens: Decimal, current_epoch: u64)
        -> Option<Vec<Change>>
    {
        assert!(proposal_id < self.new_proposal_id, "This proposal does not exist!");

        let proposal: &mut Proposal = self.proposals.get_mut(proposal_id).unwrap();
        assert!(proposal.epoch_expiration <= current_epoch, "This proposal has not finished its current period");


        match proposal.status
        {
            ProposalStatus::SuggestionPhase =>
                {
                    if proposal.supporting_votes / total_tokens > self.suggestion_approval_threshold
                    {
                        proposal.status = ProposalStatus::VotingPhase;
                        proposal.epoch_expiration = current_epoch + self.vote_period;
                    }
                    else
                    {
                        proposal.status = ProposalStatus::SuggestionRejected;
                    }
                    None
                }

            ProposalStatus::VotingPhase =>
                {

                    if proposal.voted_for + proposal.voted_against > self.minimum_votes_threshold
                    {
                        if proposal.voted_for >= proposal.voted_against
                        {
                            proposal.status = ProposalStatus::ProposalAccepted;
                            let changes = proposal.changes.clone();
                            self.execute_proposal(&changes)
                        }
                        else
                        {
                            proposal.status = ProposalStatus::ProposalRejected;
                            None
                        }
                    }
                    else
                    {
                        proposal.status = ProposalStatus::ProposalRejected;
                        None
                    }
                }
            _ => { panic!("Proposal cannot advance forward! It has already been accepted or rejected.") }
        }
    }

    /// Enables a voter to delegate its token to another voter for the given proposal
    pub fn delegate_for_proposal(&mut self, proposal_id: usize, delegate_to: u64, voter_card: &mut VoterCard, current_epoch: u64)
    {
        assert!(proposal_id < self.new_proposal_id, "This proposal does not exist!");
        assert_ne!(delegate_to, voter_card.voter_id, "Delegating to yourself does not make sense");
        assert!(voter_card.can_delegate_to(delegate_to), "Cannot delegate to this person id");

        let proposal: &mut Proposal = self.proposals.get_mut(proposal_id).unwrap();
        assert!(proposal.epoch_expiration > current_epoch, "The voting period has already ended for this proposal.");
        assert!(proposal.status.is_voting_phase(), "The proposal is not in its voting phase.");

        let can_delegate = voter_card.try_vote_for(proposal_id, &proposal.status);

        if !can_delegate
        {
            panic!("Already voted or delegated for this proposal!");
        }
        else
        {
            let nb_votes = voter_card.voting_power(current_epoch);
            proposal.add_delegation(voter_card.voter_id, delegate_to, nb_votes);
        }




    }

    /// Enables a voter to vote for a specific proposal using its own tokens and the tokens of people
    /// who delegated to them.
    pub fn vote_for_proposal(&mut self, proposal_id: usize, voter_card: &mut VoterCard, vote: Vote, current_epoch: u64)
    {
        assert!(proposal_id < self.new_proposal_id, "This proposal does not exist!");

        let proposal: &mut Proposal = self.proposals.get_mut(proposal_id).unwrap();
        assert!(proposal.epoch_expiration > current_epoch, "The voting period has already ended for this proposal.");
        assert!(proposal.status.is_voting_phase(), "The proposal is not in its voting phase.");


        let mut total_voting_power = dec!(0);
        let can_vote = voter_card.try_vote_for(proposal_id, &proposal.status);

        if can_vote
        {
            total_voting_power = voter_card.voting_power(current_epoch);
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

    /// Executes a proposal if it was accepted
    fn execute_proposal(&mut self, changes_to_do: &Vec<Change>) -> Option<Vec<Change>>
    {
        let mut changes_to_return = vec![];
        for change in changes_to_do
        {
            match change
            {
                Change::ChangeSupportPeriod(new_period) =>
                    {
                        self.support_period = *new_period;
                    }
                Change::ChangeVotePeriod(new_period) =>
                    {
                        self.vote_period = *new_period;
                    }
                Change::ChangeSuggestionApprovalThreshold(threshold) =>
                    {
                        self.suggestion_approval_threshold = *threshold;
                    }
                Change::AllowSpending(address, amount, to) =>
                    {
                        changes_to_return.push(Change::AllowSpending(address.clone(), amount.clone(), *to));
                    }
            }
        }

        if changes_to_return.is_empty()
        {
            None
        }
        else
        {
            Some(changes_to_return)
        }

    }

}

//     / \
//    / | \  TO MAKE THE TESTS WORK, MAKE CHANGES IN THE FUNCTION current_epoch OF voter_card.rs AND
//   /  •  \ ballot_box.rs
#[cfg(test)]
mod tests
{
    use std::thread::current;
    use radix_engine::ledger::TypedInMemorySubstateStore;
    use scrypto::core::Runtime;
    use scrypto::dec;
    use scrypto::math::Decimal;
    use scrypto_unit::TestRunner;
    use crate::ballot_box::BallotBox;
    use crate::proposals::{ProposalStatus, Vote, Change};
    use crate::voter_card::VoterCard;

    #[test]
    fn test_new_proposal()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description.clone(),
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch(),
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
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );

        let mut voting_card = VoterCard::new(0);
        voting_card.add_tokens(dec!(1000), test_runner.get_current_epoch());
        ballot_box.support_proposal(0, &mut voting_card, test_runner.get_current_epoch());

        let proposal = ballot_box.proposals.get(0).unwrap();

        assert_eq!(proposal.supporting_votes, voting_card.voting_power(test_runner.get_current_epoch()));
    }

    #[test]
    #[should_panic]
    fn test_support_proposal_fail()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch(),

        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();

        proposal.status = ProposalStatus::VotingPhase;

        let mut voting_card = VoterCard::new(0);
        voting_card.add_tokens(dec!(1234), test_runner.get_current_epoch());

        ballot_box.support_proposal(0, &mut voting_card,test_runner.get_current_epoch());
    }

    #[test]
    fn test_advance_with_proposal_support()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.supporting_votes = dec!(100);

        let current = test_runner.get_current_epoch();
        let new_epoch = current + ballot_box.support_period +1;
        test_runner.set_current_epoch(new_epoch);

        ballot_box.advance_with_proposal(0, dec!(100), test_runner.get_current_epoch());

        let updated_proposal = ballot_box.proposals.get_mut(0).unwrap();
        assert!(updated_proposal.status.is_voting_phase());
        assert_eq!(updated_proposal.epoch_expiration, new_epoch + ballot_box.vote_period);
    }

    #[test]
    #[should_panic]
    fn test_advance_with_proposal_support_time_fail()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let proposal = ballot_box.proposals.get(0).unwrap();
        ballot_box.advance_with_proposal(0, dec!(100), test_runner.get_current_epoch());
    }

    #[test]
    fn test_advance_with_proposal_support_amount_fail()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.supporting_votes = dec!(100);

        let current = test_runner.get_current_epoch();
        let new_epoch = current + ballot_box.support_period +1;
        test_runner.set_current_epoch(new_epoch);

        ballot_box.advance_with_proposal(0, dec!(100000), test_runner.get_current_epoch());

        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        assert!(updated_proposal.status.is_suggestion_rejected());
    }

    #[test]
    fn test_advance_with_proposal_vote()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;
        proposal.voted_for = dec!(1);

        let current = test_runner.get_current_epoch();
        let new_epoch = current + ballot_box.vote_period +1;
        test_runner.set_current_epoch(new_epoch);

        ballot_box.advance_with_proposal(0, dec!(10), test_runner.get_current_epoch());

        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        assert!(updated_proposal.status.is_proposal_accepted());
    }

    #[test]
    fn test_advance_with_proposal_vote_against()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;
        proposal.voted_against = dec!(1);

        let current = test_runner.get_current_epoch();
        let new_epoch = current + ballot_box.vote_period +1;
        test_runner.set_current_epoch(new_epoch);

        ballot_box.advance_with_proposal(0, dec!(10), test_runner.get_current_epoch());

        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        assert!(updated_proposal.status.is_proposal_rejected());
    }

    #[test]
    fn test_delegate_for_proposal()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;

        let mut voting_card = VoterCard::new(0);
        voting_card.add_tokens(dec!(1000), test_runner.get_current_epoch());
        voting_card.add_delegatee(1, test_runner.get_current_epoch());

        ballot_box.delegate_for_proposal(0, 1, &mut voting_card, test_runner.get_current_epoch());
        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        assert_eq!(*updated_proposal.delegated_votes.get(&1).unwrap(), Decimal::zero());
    }

    #[test]
    #[should_panic]
    fn test_delegate_for_proposal_fail_cannot_delegate_to_id()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;
        let mut voting_card = VoterCard::new(0);
        voting_card.add_tokens(dec!(1234), test_runner.get_current_epoch());

        ballot_box.delegate_for_proposal(0, 1, &mut voting_card, test_runner.get_current_epoch());
    }

    #[test]
    #[should_panic]
    fn test_delegate_for_proposal_fail_not_voting_phase()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut voting_card = VoterCard::new(0);
        voting_card.add_tokens(dec!(1000), test_runner.get_current_epoch());
        voting_card.add_delegatee(1, test_runner.get_current_epoch());

        ballot_box.delegate_for_proposal(0, 1, &mut voting_card, test_runner.get_current_epoch());
    }

    #[test]
    #[should_panic]
    fn test_delegate_for_proposal_fail_expired()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;
        proposal.epoch_expiration = 0;

        let mut voting_card = VoterCard::new(0);
        voting_card.add_tokens(dec!(1234), test_runner.get_current_epoch());
        voting_card.add_delegatee(1, test_runner.get_current_epoch());

        ballot_box.delegate_for_proposal(0,1, &mut voting_card, test_runner.get_current_epoch());
    }

    #[test]
    #[should_panic]
    fn test_delegate_for_proposal_fail_already_delegated()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;

        let mut voting_card = VoterCard::new(0);
        voting_card.add_tokens(dec!(1234), test_runner.get_current_epoch());
        voting_card.add_delegatee(1, test_runner.get_current_epoch());
        voting_card.add_delegatee(2, test_runner.get_current_epoch());

        ballot_box.delegate_for_proposal(0,1, &mut voting_card, test_runner.get_current_epoch());
        ballot_box.delegate_for_proposal(0,2, &mut voting_card, test_runner.get_current_epoch());


    }

    #[test]
    #[should_panic]
    fn test_delegate_for_proposal_fail_already_voted()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;

        let mut voting_card = VoterCard::new(0);
        voting_card.add_tokens(dec!(1000), test_runner.get_current_epoch());
        voting_card.add_delegatee(1, test_runner.get_current_epoch());
        voting_card.try_vote_for(0, &ProposalStatus::VotingPhase);

        ballot_box.delegate_for_proposal(0,1, &mut voting_card, test_runner.get_current_epoch());
    }

    #[test]
    fn test_vote_for_proposal()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();


        let mut voting_card = VoterCard::new(0);
        voting_card.add_tokens(dec!(1000), test_runner.get_current_epoch());

        let current = test_runner.get_current_epoch();
        test_runner.set_current_epoch(current + 2016);

        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;

        ballot_box.vote_for_proposal(0, &mut voting_card, Vote::For, test_runner.get_current_epoch());

        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        assert!(updated_proposal.voted_for > dec!("1.74"));
    }

    #[test]
    fn test_vote_for_proposal_with_delegated_votes_and_own_vote()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");


        let mut voting_card_1 = VoterCard::new(0);
        voting_card_1.add_delegatee(1, test_runner.get_current_epoch());
        voting_card_1.add_tokens(dec!(1), test_runner.get_current_epoch());

        let mut voting_card_2 = VoterCard::new(1);
        voting_card_2.add_tokens(dec!(1), test_runner.get_current_epoch());

        let current = test_runner.get_current_epoch();
        test_runner.set_current_epoch(current + 2016);

        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;

        ballot_box.delegate_for_proposal(0, 1, &mut voting_card_1, test_runner.get_current_epoch());
        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        ballot_box.vote_for_proposal(0, &mut voting_card_2, Vote::For, test_runner.get_current_epoch());

        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        assert!(updated_proposal.voted_for > dec!("1.5"));
    }

    #[test]
    fn test_vote_for_proposal_with_only_delegated_votes()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");


        let mut voting_card_1 = VoterCard::new(0);
        voting_card_1.add_delegatee(1, test_runner.get_current_epoch());
        voting_card_1.add_tokens(dec!(1), test_runner.get_current_epoch());

        let mut voting_card_2 = VoterCard::new(1);
        voting_card_2.add_tokens(dec!(1), test_runner.get_current_epoch());

        let current = test_runner.get_current_epoch();
        test_runner.set_current_epoch(current + 2016);
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );
        let mut proposal = ballot_box.proposals.get_mut(0).unwrap();
        proposal.status = ProposalStatus::VotingPhase;

        ballot_box.delegate_for_proposal(0, 1, &mut voting_card_1, test_runner.get_current_epoch());

        ballot_box.vote_for_proposal(0, &mut voting_card_2, Vote::For, test_runner.get_current_epoch());

        let updated_proposal = ballot_box.proposals.get(0).unwrap();
        assert!(updated_proposal.voted_for > dec!("0.75"));
    }

    #[test]
    #[should_panic]
    fn test_vote_for_proposal_fail_not_in_voting_phase()
    {
        let mut store = TypedInMemorySubstateStore::with_bootstrap();
        let mut test_runner = TestRunner::new(true, &mut store);

        let mut ballot_box = BallotBox::new();
        let description = String::from("Test proposal");
        ballot_box.make_proposal(
            description,
            vec![Change::ChangeVotePeriod(0)],
            test_runner.get_current_epoch()
        );

        let mut voting_card_1 = VoterCard::new(0);
        voting_card_1.add_tokens(dec!(1), Runtime::current_epoch());

        ballot_box.vote_for_proposal(0, &mut voting_card_1, Vote::Blank, test_runner.get_current_epoch());
    }


    #[test]
    fn execute_proposal_test()
    {
        let mut ballot_box = BallotBox::new();

        ballot_box.execute_proposal(&vec![Change::ChangeVotePeriod(0)]);
        assert_eq!(ballot_box.vote_period, 0);

        ballot_box.execute_proposal(&vec![Change::ChangeSuggestionApprovalThreshold(dec!(0))]);
        assert_eq!(ballot_box.suggestion_approval_threshold, dec!(0));

        ballot_box.execute_proposal(&vec![Change::ChangeSupportPeriod(0)]);
        assert_eq!(ballot_box.support_period, 0);
    }

    #[test]
    fn execute_multiple_proposal_test()
    {
        let mut ballot_box = BallotBox::new();
        let proposals = vec![Change::ChangeVotePeriod(0), Change::ChangeSuggestionApprovalThreshold(dec!(0)),Change::ChangeSupportPeriod(0)];
        ballot_box.execute_proposal(&proposals);
        assert_eq!(ballot_box.vote_period, 0);
        assert_eq!(ballot_box.suggestion_approval_threshold, dec!(0));
        assert_eq!(ballot_box.support_period, 0);
    }

}