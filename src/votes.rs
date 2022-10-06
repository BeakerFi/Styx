use scrypto::prelude::Decimal;

pub struct Proposal
{
    description: String,
    number_agreeing: Decimal,
    end_of_proposal: Decimal,
    id: u32

}

pub struct Vote
{
    description: String,
    number_of_votes_for: Decimal,
    number_of_votes_against: Decimal,
    end_of_vote: Decimal,
    id: u32
}