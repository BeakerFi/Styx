use scrypto::prelude::*;
use crate::ballot_box::BallotBox;
use crate::proposals::{Vote, Change};
use crate::voter_card::VoterCard;

blueprint! {
    struct Styx {
        // Define what resources and data will be managed by Hello components
        emission_vault: Vault,
        internal_authority : Vault,
        locker : Vault,
        styx_address: ResourceAddress,
        voter_card_address: ResourceAddress,
        emitted_tokens: Decimal,
        new_voter_card_id: u64,
        ballot_box: BallotBox,
        assets_under_management: HashMap<ResourceAddress, Vault>,
        claimable_tokens: HashMap<u64, HashMap<ResourceAddress, Decimal>>
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
                ballot_box: BallotBox::new(),
                new_voter_card_id: 0,
                emitted_tokens: Decimal::zero(),
                assets_under_management: HashMap::new(),
                claimable_tokens: HashMap::new()
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
                    VoterCard::new(self.new_voter_card_id)
                )
            });
            self.new_voter_card_id+=1;

            voter_card_bucket
        }

        pub fn mint_voter_card_with_bucket(&mut self, deposit : Bucket) -> Bucket {
            assert_eq!(deposit.resource_address(), self.styx_address);

            info!("You are going to lock : {}", deposit.amount());
            let mut voter_card = VoterCard::new(self.new_voter_card_id);
            if !deposit.amount().is_zero()
            {
                voter_card.add_tokens(deposit.amount(), Runtime::current_epoch());
            }

            let voter_card_bucket = self.internal_authority.authorize(|| {
                borrow_resource_manager!(self.voter_card_address).mint_non_fungible(
                    &NonFungibleId::from_u64(self.new_voter_card_id),
                    voter_card
                )
            });
            self.locker.put(deposit);
            self.new_voter_card_id+=1;

            voter_card_bucket
        }


        pub fn lock(&mut self, voter_card_proof : Proof, deposit : Bucket)
        {
            assert_eq!(deposit.resource_address(), self.styx_address);

            let validated_proof = self.check_proof(voter_card_proof);

            let amount = deposit.amount();

            // avoir accès à validated
            let mut voter_card : VoterCard = self.get_voter_card_data_from_proof(&validated_proof);


            voter_card.add_tokens(amount, Runtime::current_epoch());

            self.change_data(&validated_proof, voter_card);

        }

        pub fn unlock(&mut self, proof : Proof, amount: Decimal) -> Bucket
        {

            let validated_proof = self.check_proof(proof);

            // avoir accès à validated
            let mut voter_card : VoterCard = self.get_voter_card_data_from_proof(&validated_proof);
            assert!(voter_card.total_number_of_token >= amount);

            voter_card.retrieve_tokens(amount);

            self.change_data(&validated_proof, voter_card);
            self.locker.take(amount)
        }

        pub fn unlock_all(&mut self, proof : Proof) -> Bucket
        {
            let validated_proof = self.check_proof(proof);

            let mut voter_card : VoterCard = self.get_voter_card_data_from_proof(&validated_proof);

            let total_number_of_token = voter_card.retrieve_all_tokens();

            // Je pense mettre vec![] vide à la place (je ne peux pas encore), ou burn en fait
            // Ou alors pouvoir autoriser n'importe qui a burn sa carte, ou n'importe qui tant que total_number_of_token ==0
            // Ou faire fct burn_card qui unlock_all puis burn

            self.change_data(&validated_proof, voter_card);
            self.locker.take(total_number_of_token)
        }

        pub fn make_proposal(&mut self, description: String, suggested_change: Change)
        {
            self.ballot_box.make_proposal(description, suggested_change);
        }

        pub fn support_proposal(&mut self, proposal_id: usize, voter_card_proof: Proof)
        {
            let validated_id = self.check_proof(voter_card_proof);
            let mut voter_card = self.get_voter_card_data_from_proof(&validated_id);

            self.ballot_box.support_proposal(proposal_id, &mut voter_card);
            self.change_data(&validated_id, voter_card);
        }

        pub fn advance_with_proposal(&mut self, proposal_id: usize)
        {
            match self.ballot_box.advance_with_proposal(proposal_id, self.emitted_tokens)
            {
                None => {}
                Some((address, amount, to)) =>
                {
                    match self.claimable_tokens.get_mut(&to)
                    {
                        None =>
                            {
                                let mut new_hashmap: HashMap<ResourceAddress, Decimal> = HashMap::new();
                                new_hashmap.insert(address, amount);
                                self.claimable_tokens.insert(to, new_hashmap);
                            },
                        Some(hashmap) =>
                            {
                                match hashmap.get_mut(&address)
                                {
                                    None => { hashmap.insert(address, amount); }
                                    Some(tokens) =>
                                        {
                                            *tokens = *tokens + amount;
                                        }
                                }
                            }
                    }
                }
            }
        }

        pub fn delegate_for_proposal(&mut self, proposal_id: usize, delegate_to: u64, voter_card_proof: Proof)
        {
            let validated_id = self.check_proof(voter_card_proof);
            let mut voter_card = self.get_voter_card_data_from_proof(&validated_id);

            self.ballot_box.delegate_for_proposal(proposal_id, delegate_to, &mut voter_card);
            self.change_data(&validated_id, voter_card);
        }

        pub fn vote_for_proposal(&mut self, proposal_id: usize, voter_card_proof: Proof, vote: Vote)
        {
            let validated_id = self.check_proof(voter_card_proof);
            let mut voter_card = self.get_voter_card_data_from_proof(&validated_id);

            self.ballot_box.vote_for_proposal(proposal_id, &mut voter_card, vote);
            self.change_data(&validated_id, voter_card);
        }

        pub fn gift_asset(&mut self, mut asset: Bucket)
        {
            match self.assets_under_management.get_mut(&asset.resource_address())
            {
                None =>
                    {
                        let mut  vault= Vault::new(asset.resource_address());
                        vault.put(asset.take(asset.amount()));
                        self.assets_under_management.insert(asset.resource_address(), vault);
                    }
                Some(vault) =>
                    {
                        vault.put(asset.take(asset.amount()));
                    }
            }
        }

        pub fn amount_owned(&self, asset_address: ResourceAddress) -> Decimal
        {
            match self.assets_under_management.get(&asset_address)
            {
                None => Decimal::zero(),
                Some(vault) => vault.amount()
            }
        }

        pub fn claim_tokens(&mut self, voter_card_proof: Proof) -> Vec<Bucket>
        {
            let validated_proof = self.check_proof(voter_card_proof);
            let voter_card = self.get_voter_card_data_from_proof(&validated_proof);

            let mut buckets: Vec<Bucket> = vec![];

            match self.claimable_tokens.get_mut(&voter_card.voter_id)
            {
                None => {}
                Some(hashmap) =>
                    {

                        let mut resource_to_remove = vec![];

                        for (resource, amount) in hashmap.iter_mut()
                        {
                            match self.assets_under_management.get_mut(&resource)
                            {
                                None => {}
                                Some(vault) =>
                                    {
                                        let mut new_bucket = Bucket::new(*resource);
                                        let owned = vault.amount();
                                        let amount_to_take = owned.max(*amount);

                                        new_bucket.put(vault.take(amount_to_take));

                                        if amount_to_take == owned
                                        {
                                            self.assets_under_management.remove(&resource);
                                        }

                                        *amount = *amount - amount_to_take;
                                        buckets.push(new_bucket);
                                        if amount.is_zero()
                                        {
                                            resource_to_remove.push(*resource);
                                        }
                                    }
                            }
                        }

                        for resource in resource_to_remove.into_iter()
                        {
                            hashmap.remove(&resource);
                        }

                    }
            }

            buckets
        }


        fn change_data(&self, valid_proof: &ValidatedProof, new_voter_card: VoterCard)
        {
            let resource_manager : &mut ResourceManager = borrow_resource_manager!(self.voter_card_address);
            let id = valid_proof.non_fungible::<VoterCard>().id();
            self.internal_authority
                .authorize(|| resource_manager.update_non_fungible_data(&id, new_voter_card));
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

        fn get_voter_card_data(&self, voter_card_bucket : Bucket ) -> VoterCard {

            let resource_manager: &ResourceManager =
                borrow_resource_manager!(self.voter_card_address);
            let id = voter_card_bucket.non_fungible::<VoterCard>().id();
            resource_manager.get_non_fungible_data::<VoterCard>(&id)

        }

    }
}