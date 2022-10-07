use std::collections::HashMap;
use scrypto::core::Runtime;
use scrypto::dec;
use scrypto::math::Decimal;
use scrypto::prelude::NonFungibleId;
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
           epoch_expiration: Runtime::current_epoch() + self.support_period
       };

        self.new_proposal_id += 1;
        self.proposals.push(proposal);
    }

    pub fn support_proposal(&mut self, proposal_id: usize, voter_card: &mut VoterCard)
    {
        assert!(proposal_id < self.new_proposal_id, "This proposal does not exist!");

        let proposal: &mut Proposal = self.proposals.get_mut(proposal_id).unwrap();
        assert!(proposal.epoch_expiration > Runtime::current_epoch(), "This proposal has expired");
        assert!(proposal.status.is_suggestion_phase(), "Cannot support a proposal that is not in suggestion phase");

        let can_support = voter_card.try_vote_for(proposal_id, &proposal.status);

        if !can_support
        {
            panic!("You already supported the proposition");
        }

        proposal.supporting_votes = proposal.supporting_votes + self.voting_power(voter_card);
    }

    pub fn advance_with_proposal(&mut self, proposal_id: usize, total_tokens: Decimal)
    {
        assert!(proposal_id < self.new_proposal_id, "This proposal does not exist!");

        let proposal: &mut Proposal = self.proposals.get_mut(proposal_id).unwrap();
        assert!(proposal.epoch_expiration < Runtime::current_epoch(), "This proposal has not finished its current period");


        match proposal.status
        {
            ProposalStatus::SuggestionPhase =>
                {
                    if proposal.supporting_votes / total_tokens > self.suggestion_approval_threshold
                    {
                        proposal.status = ProposalStatus::VotingPhase;
                        proposal.epoch_expiration = Runtime::current_epoch() + self.vote_period;
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
                            self.execute_proposal(&proposal.change);
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

    pub fn delegate_for_proposal(&mut self, proposal_id: usize, delegate_to: &NonFungibleId, voter_card: &mut VoterCard)
    {
        assert!(proposal_id < self.new_proposal_id, "This proposal does not exist!");
        assert!(voter_card.can_delegate_to(delegate_to), "Cannot delegate to this id");

        let proposal: &mut Proposal = self.proposals.get_mut(proposal_id).unwrap();
        assert!(proposal.epoch_expiration > Runtime::current_epoch(), "The voting period has already ended for this proposal.");
        assert!(proposal.status.is_voting_phase(), "The proposal is not in its voting phase.");

        let can_delegate = voter_card.try_vote_for(proposal_id, &proposal.status);

        if !can_delegate
        {
            panic!("Already voted or delegated for this proposal!");
        }
        else
        {
            let nb_votes = self.voting_power(voter_card);
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
        assert!(proposal.epoch_expiration > Runtime::current_epoch(), "The voting period has already ended for this proposal.");
        assert!(proposal.status.is_voting_phase(), "The proposal is not in its voting phase.");


        let mut total_voting_power = dec!(0);
        let can_vote = voter_card.try_vote_for(proposal_id, &proposal.status);

        if can_vote
        {
            total_voting_power = self.voting_power(voter_card);
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



    fn voting_power(&self, voter_card: &VoterCard) -> Decimal
    {
        if Runtime::current_epoch() == voter_card.epoch_of_conversion
        {
            return Decimal::zero();
        }
        let time = Runtime::current_epoch() - voter_card.epoch_of_conversion;
        let tokens = voter_card.nb_of_token;
        let exp = exp(- dec!(5012) / time );
        let time_multiplicator = ( ( exp - 1 )/ (exp + 1)  + 1) / 2;
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
}
