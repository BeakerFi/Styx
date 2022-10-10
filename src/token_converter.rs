use scrypto::prelude::*;
use crate::voter_card::VoterCard;

blueprint! {
    struct Styx {
        // Define what resources and data will be managed by Hello components
        emission_vault: Vault,
        internal_authority : Vault,
        locker : Vault,
        styx_address: ResourceAddress,
        voter_card_address: ResourceAddress,
        new_voter_card_id: u64
    }

    impl Styx {
        // Implement the functions and methods which will manage those resources and data

        // This is a function, and can be called directly on the blueprint once deployed
        pub fn instantiate() -> ComponentAddress {

            // Next we will create a badge we'll hang on to for minting & transfer authority
            let internal_admin: Bucket = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .metadata("name", "RegulatedToken internal authority badge")
                .burnable(rule!(allow_all), LOCKED)
                .initial_supply(dec!("1"));

            let access_rule: AccessRule = rule!(require(internal_admin.resource_address()));

            let my_bucket: Bucket = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_MAXIMUM)
                .metadata("name", "StyxToken")
                .metadata("symbol", "STX")
                .updateable_metadata(
                    access_rule.clone(),
                    MUTABLE(access_rule.clone())
                )
                .mintable(
                    access_rule.clone(),
                    MUTABLE(access_rule.clone())
                )
                .initial_supply(dec!("100"));

            let styx_address: ResourceAddress = my_bucket.resource_address();

            let address = ResourceBuilder::new_non_fungible()
                .metadata(
                    "name",
                    "Promise tokenx for BasicFlashLoan - must be returned to be burned!",
                )
                .mintable(rule!(require(internal_admin.resource_address())), LOCKED)
                .burnable(rule!(require(internal_admin.resource_address())), LOCKED) 
                .restrict_withdraw(rule!(require(internal_admin.resource_address())), MUTABLE(rule!(require(internal_admin.resource_address()))))
                .updateable_non_fungible_data(rule!(require(internal_admin.resource_address())), LOCKED)
                .no_initial_supply();

            // Instantiate a Hello component, populating its vault with our supply of 1000 HelloToken
            Self {
                emission_vault: Vault::with_bucket(my_bucket),
                internal_authority: Vault::with_bucket(internal_admin),
                voter_card_address : address,
                locker : Vault::new(styx_address),
                styx_address,
                new_voter_card_id: 0
            }
            .instantiate()
            .globalize()
        }


        // This is a method, because it needs a reference to self.  Methods can only be called on components
        pub fn free_token(&mut self) -> Bucket {
            info!("My balance is: {} HelloToken. Now giving away a token!", self.emission_vault.amount());
            // If the semi-colon is omitted on the last line, the last value seen is automatically returned
            // In this case, a bucket containing 1 HelloToken is returned
            self.emission_vault.take(1)
        }

        pub fn mint_voter_card(&mut self) -> Bucket {

            let voter_card_bucket = self.internal_authority.authorize(|| {
                borrow_resource_manager!(self.voter_card_address).mint_non_fungible(
                    &NonFungibleId::from_u64(self.new_voter_card_id),
                    VoterCard::new(self.new_voter_card_id, None)
                )
            });
            self.new_voter_card_id+=1;

            voter_card_bucket
        }

        pub fn mint_voter_card_with_bucket(&mut self, deposit : Bucket) -> Bucket {
            assert_eq!(deposit.resource_address(), self.styx_address);

            info!("You are going to lock : {}", deposit.amount());
            let voter_card_bucket = self.internal_authority.authorize(|| {
                borrow_resource_manager!(self.voter_card_address).mint_non_fungible(
                    &NonFungibleId::from_u64(self.new_voter_card_id),
                    VoterCard::new(self.new_voter_card_id, Some(deposit.amount()))
                )
            });
            self.locker.put(deposit);
            self.new_voter_card_id+=1;

            voter_card_bucket
        }



        pub fn lock(&mut self, voter_card_proof : Proof, deposit : Bucket) {
            assert_eq!(deposit.resource_address(), self.styx_address);
            let amount = deposit.amount();

            let resource_manager : &mut ResourceManager = borrow_resource_manager!(self.voter_card_address);

            let validated_proof = self.check_proof(voter_card_proof);

            let id = validated_proof.non_fungible::<VoterCard>().id();

            // avoir accès à validated
            let mut voter_card : VoterCard = self.get_voter_card_data_from_proof(&validated_proof);


            voter_card.add_amount(amount);

            self.internal_authority
                .authorize(|| resource_manager.update_non_fungible_data(&id, voter_card));

        }

        pub fn unlock(&mut self, proof : Proof, amount: Decimal) -> Bucket {

            let resource_manager : &mut ResourceManager = borrow_resource_manager!(self.voter_card_address);

            let validated_proof = self.check_proof(proof);

            let id = validated_proof.non_fungible::<VoterCard>().id();

            // avoir accès à validated
            let mut voter_card : VoterCard = self.get_voter_card_data_from_proof(&validated_proof);

            assert!(voter_card.total_number_of_token >= amount);

            voter_card.remove_amount(amount);

            //self.internal_authority.authorize(|| voter_card.burn());
            self.internal_authority
                .authorize(|| resource_manager.update_non_fungible_data(&id, voter_card));
            self.locker.take(amount)
        }

        pub fn unlock_all(&mut self, proof : Proof) -> Bucket {
            let resource_manager : &mut ResourceManager = borrow_resource_manager!(self.voter_card_address);

            let validated_proof = self.check_proof(proof);

            let id = validated_proof.non_fungible::<VoterCard>().id();

            // avoir accès à validated
            let mut voter_card : VoterCard = self.get_voter_card_data_from_proof(&validated_proof);

            let total_number_of_token = voter_card.remove_all();

            // Je pense mettre vec![] vide à la place (je ne peux pas encore), ou burn en fait
            // Ou alors pouvoir autoriser n'importe qui a burn sa carte, ou n'importe qui tant que total_number_of_token ==0
            // Ou faire fct burn_card qui unlock_all puis burn

            //self.internal_authority.authorize(|| voter_card.burn());
            self.internal_authority
                .authorize(|| resource_manager.update_non_fungible_data(&id, voter_card));
            self.locker.take(total_number_of_token)
        }

        /// Checks that a given [`Proof`] corresponds to a position and returns the associated
        /// [`ValidatedProof`]
        fn check_proof(&self, position_nft: Proof) -> ValidatedProof
        {

            let valid_proof: ValidatedProof =  position_nft.validate_proof
            (
                    ProofValidationMode::ValidateContainsAmount
                        (
                            self.voter_card_address,
                            dec!(1)
                        )
            ).expect("Invalid proof provided");

            valid_proof
        }

        fn get_voter_card_data_from_proof(&self, validated_proof: &ValidatedProof) -> VoterCard
        {
            let resource_manager: &ResourceManager =
                borrow_resource_manager!(self.voter_card_address);
            let id = validated_proof.non_fungible::<VoterCard>().id();
            resource_manager.get_non_fungible_data::<VoterCard>(&id)
        }

    }
}