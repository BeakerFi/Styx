use scrypto::prelude::*;

#[derive(NonFungibleData)]
pub struct Receipt {
    pub nb_of_token: Decimal,
    pub epoch_of_conversion : Decimal,
    pub proposed_votes : Vec<u32>,
    pub participation_votes : Vec<u32>
}

blueprint! {
    struct Styx {
        // Define what resources and data will be managed by Hello components
        emission_vault: Vault,
        internal_authority : Vault,
        stake : Vault,
        styx_adress : ResourceAddress,
        receipt_address: ResourceAddress
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

            let styx_adress : ResourceAddress = my_bucket.resource_address();

            let address = ResourceBuilder::new_non_fungible()
                .metadata(
                    "name",
                    "Promise tokenx for BasicFlashLoan - must be returned to be burned!",
                )
                .mintable(rule!(require(internal_admin.resource_address())), LOCKED) // 1
                .burnable(rule!(require(internal_admin.resource_address())), LOCKED) // 1
                .restrict_deposit(rule!(require(internal_admin.resource_address())), MUTABLE(rule!(require(internal_admin.resource_address())))) // 1
                .no_initial_supply();
                
            // Instantiate a Hello component, populating its vault with our supply of 1000 HelloToken
            Self {
                emission_vault: Vault::with_bucket(my_bucket),
                internal_authority: Vault::with_bucket(internal_admin),
                receipt_address : address,
                stake : Vault::new(styx_adress),
                styx_adress : styx_adress
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

        pub fn stake(&mut self, deposit : Bucket) -> (Bucket,Proof) {
            assert!(deposit.resource_address() == self.stake.resource_address());

            info!("You are going to stake : {}", deposit.amount());
            let receipt = self.internal_authority.authorize(|| {
                borrow_resource_manager!(self.receipt_address).mint_non_fungible(
                    &NonFungibleId::random(),
                    Receipt {
                        nb_of_token : deposit.amount(),
                        epoch_of_conversion : dec!("10"),
                        proposed_votes : Vec::<u32>::new(),
                        participation_votes : Vec::<u32>::new()
                    }
                )
            });
            self.stake.put(deposit);

            let deposit_proof = self.internal_authority.create_proof();
            (receipt,deposit_proof)
        }

        pub fn unstake(&mut self, proof : Proof, amount: Decimal) -> Bucket {
            
            let resource_manager : &mut ResourceManager = borrow_resource_manager!(self.receipt_address);

            let validated_proof = self.check_proof(proof);

            let id = validated_proof.non_fungible::<Receipt>().id();

            // avoir accès à validated 
            let receipt : Receipt = self.get_receipt_data(&validated_proof);

            assert!(receipt.nb_of_token >= amount);

            let new_receipt = Receipt{
                nb_of_token : receipt.nb_of_token - amount,
                epoch_of_conversion : receipt.epoch_of_conversion,
                proposed_votes : receipt.proposed_votes,
                participation_votes : receipt.participation_votes

            };

            self.internal_authority.authorize(|| resource_manager.update_non_fungible_data(&id, new_receipt));

            self.stake.take(amount)
        }



        

        /// Checks that a given [`Proof`] corresponds to a position and returns the associated
        /// [`ValidatedProof`]
        fn check_proof(&self, receipt_proof: Proof) -> ValidatedProof
        {

            let valid_proof: ValidatedProof =  receipt_proof.validate_proof
            (
                    ProofValidationMode::ValidateContainsAmount
                        (
                            self.receipt_address,
                            dec!("1")
                        )
            ).expect("Invalid proof provided");

            valid_proof
        }

        fn get_receipt_data(&self, validated_proof: &ValidatedProof) -> Receipt
        {
            let resource_manager: &ResourceManager =
                borrow_resource_manager!(self.receipt_address);
            let id = validated_proof.non_fungible::<Receipt>().id();
            resource_manager.get_non_fungible_data::<Receipt>(&id)
        }


    }
}