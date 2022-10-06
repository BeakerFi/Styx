use scrypto::prelude::Decimal;
use scrypto::NonFungibleData;

#[derive(NonFungibleData)]
pub struct Receipt {
    pub nb_of_token: Decimal,
    pub epoch_of_conversion : Decimal,
    pub proposed_votes : Vec<u32>,
    pub participation_votes : Vec<u32>
}