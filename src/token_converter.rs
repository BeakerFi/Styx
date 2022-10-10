use scrypto::prelude::*;
use crate::ballot_box::BallotBox;
use crate::proposals::{Vote, Change};
use crate::voter_card::VoterCard;

blueprint! {
    struct Styx {

        // The emission vault is the vault in where all token will first be minted until there owner withdraw them
        styx_vault: Vault,

        // The internal_authority is used to mint and burn tokens but also to 
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
        

        // Instantiate given the initial_supply return the component adress of the DAO, the external admin badge that allow to mint new tokens and        
        pub fn instantiate(initial_supply: Decimal) -> (ComponentAddress, Bucket) {


            // If the DAO is not instancied with an admin badge, a default one is created to the instantiation and then returned to the instantiator
            let default_admin_badge = ResourceBuilder::new_fungible()
            .divisibility(DIVISIBILITY_NONE)
            .metadata("name", "External Admin Badge")
            .burnable(rule!(allow_all), LOCKED)
            .initial_supply(dec!(1));
 
            Self::instantiate_custom(default_admin_badge, initial_supply)
        }


        // A contract can instantiate a DAO with it's own internal admin badge which give the power to mint new styx
        pub fn instantiate_custom(admin_badge : Bucket, initial_supply: Decimal) -> (ComponentAddress, Bucket) {

            
            let internal_admin: Bucket = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .metadata("name", "Internal Admin Badge")
                .burnable(rule!(allow_all), LOCKED)
                .initial_supply(dec!(1));

            let access_rule: AccessRule = rule!(require(internal_admin.resource_address()));

            let styx_bucket: Bucket = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_MAXIMUM)
                .metadata("name", "StyxToken")
                .metadata("symbol", "STX")
                .updateable_metadata(
                    access_rule.clone(),
                    MUTABLE(access_rule.clone())
                )
                // Both the internal or external admin can mint StyxToken
                .mintable(
                    rule!( require(internal_admin.resource_address()) || require(admin_badge.resource_address()) ),
                    MUTABLE(access_rule.clone())
                )
                // Both can withdraw from the styx vault
                .restrict_withdraw(
                    rule!( require(internal_admin.resource_address()) || require(admin_badge.resource_address()) ),
                    LOCKED
                )
                .initial_supply(initial_supply);

            let styx_address: ResourceAddress = styx_bucket.resource_address();

            let voter_card_address = ResourceBuilder::new_non_fungible()
                .metadata("name","VoterCard")
                .mintable(access_rule.clone(), LOCKED)
                .burnable(access_rule.clone(), LOCKED)
                .restrict_withdraw(access_rule.clone(), MUTABLE(access_rule.clone()))
                .updateable_non_fungible_data(access_rule.clone(), LOCKED)
                .no_initial_supply();


            let dao = Self {
                styx_vault: Vault::with_bucket(styx_bucket),
                internal_authority: Vault::with_bucket(internal_admin),
                voter_card_address : voter_card_address,
                locker : Vault::new(styx_address),
                styx_address,
                ballot_box: BallotBox::new(),
                new_voter_card_id: 0,
                emitted_tokens: initial_supply,
                assets_under_management: HashMap::new(),
                claimable_tokens: HashMap::new()
            }
            .instantiate();

            return (dao.globalize(),admin_badge)
        }


        // Using for test only 
        pub fn free_token(&mut self) -> Bucket {
            info!("My balance is: {} HelloToken. Now giving away a token!", self.styx_vault.amount());
            self.styx_vault.take(1)
        }


        // Mint a new voter card 
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

        pub fn withdraw(&mut self, amount: Decimal) -> Bucket
        {
            assert!(amount < self.styx_vault.amount());
            self.styx_vault.take(amount)
        }

        pub fn emit(&mut self, amount: Decimal)
        {
            let bucket = self.styx_vault.authorize(|| {
                borrow_resource_manager!(self.styx_address).mint(amount)
            });
            self.emitted_tokens += amount;
            self.styx_vault.put(bucket);
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

            self.change_data(&validated_proof, voter_card);
            self.locker.take(total_number_of_token)
        }

        pub fn make_proposal(&mut self, description: String, suggested_changes: Vec<Change>)
        {
            self.ballot_box.make_proposal(description, suggested_changes, Runtime::current_epoch());
        }

        pub fn support_proposal(&mut self, proposal_id: usize, voter_card_proof: Proof)
        {
            let validated_id = self.check_proof(voter_card_proof);
            let mut voter_card = self.get_voter_card_data_from_proof(&validated_id);

            self.ballot_box.support_proposal(proposal_id, &mut voter_card, Runtime::current_epoch());
            self.change_data(&validated_id, voter_card);
        }

        pub fn advance_with_proposal(&mut self, proposal_id: usize)
        {
            match self.ballot_box.advance_with_proposal(proposal_id, self.emitted_tokens, Runtime::current_epoch())
            {
                None => {}
                Some(changes) =>
                {
                    for change in changes
                    {
                        match change
                        {
                            Change::AllowSpending(address, amount, to) =>
                                {
                                    self.allow_spending(address, amount, to);
                                }

                            Change::AllowMinting(amount) =>
                                {
                                    self.emit(amount);
                                }
                            _ => { panic!("critical error in code. This should not happen.") }
                        }
                    }
                }
            }
        }

        pub fn delegate_for_proposal(&mut self, proposal_id: usize, delegate_to: u64, voter_card_proof: Proof)
        {
            let validated_id = self.check_proof(voter_card_proof);
            let mut voter_card = self.get_voter_card_data_from_proof(&validated_id);

            self.ballot_box.delegate_for_proposal(proposal_id, delegate_to, &mut voter_card, Runtime::current_epoch());
            self.change_data(&validated_id, voter_card);
        }

        pub fn vote_for_proposal(&mut self, proposal_id: usize, voter_card_proof: Proof, vote: Vote)
        {
            let validated_id = self.check_proof(voter_card_proof);
            let mut voter_card = self.get_voter_card_data_from_proof(&validated_id);

            self.ballot_box.vote_for_proposal(proposal_id, &mut voter_card, vote, Runtime::current_epoch());
            self.change_data(&validated_id, voter_card);
        }

        pub fn gift_asset(&mut self, mut asset: Bucket)
        {
            if asset.resource_address() == self.styx_address
            {
                self.styx_vault.put(asset.take(asset.amount()))
            }
            else
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
                            let mut opt_bucket = None;
                            let mut opt_resource_to_remove = None;

                            let vault_to_take_from : Option<&mut Vault>;

                            if *resource == self.styx_address
                            {
                                vault_to_take_from = Some(&mut self.styx_vault);
                            }
                            else {
                                vault_to_take_from = self.assets_under_management.get_mut(resource);
                            }

                            match vault_to_take_from
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
                                            // If the resource is the DAO token then this line does not do anything
                                            self.assets_under_management.remove(&resource);
                                        }

                                        *amount = *amount - amount_to_take;
                                        opt_bucket = Some(new_bucket);
                                        if amount.is_zero()
                                        {
                                            opt_resource_to_remove = Some(*resource);
                                        }
                                    }
                            }

                            match opt_bucket
                            {
                                None => {},
                                Some(bucket) => { buckets.push(bucket); }
                            }

                            match opt_resource_to_remove
                            {
                                None => {},
                                Some(resource_rem) => {resource_to_remove.push(resource_rem);}
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


        fn allow_spending(&mut self, address: ResourceAddress, amount: Decimal, to: u64)
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
    }
}