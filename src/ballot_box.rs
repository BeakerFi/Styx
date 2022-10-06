use std::collections::HashMap;
use scrypto::math::Decimal;
use crate::votes::Proposal;


pub struct BallotBox
{
    last_proposal_id: u32,
    last_vote_id: u32,
    proposals: HashMap<u32, Proposal>,
    votes: HashMap<u32, Proposal>,
}


pub new()