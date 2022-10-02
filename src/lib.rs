use scrypto::prelude::*;



blueprint! {
    struct Styx {
        // Define what resources and data will be managed by Hello components
        sample_vault: Vault,
        internal_authority : Vault,
        stake : Vault,
        //styx_adress : RessourceAddress,
        transiant_ressource_adress: ResourceAddress
    }

    impl Styx {
        // Implement the functions and methods which will manage those resources and data
        
        // This is a function, and can be called directly on the blueprint once deployed
        pub fn instantiate_stx() -> ComponentAddress {

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
                    "Promise token for BasicFlashLoan - must be returned to be burned!",
                )
                .mintable(rule!(require(internal_admin.resource_address())), LOCKED) // 1
                .burnable(rule!(require(internal_admin.resource_address())), LOCKED) // 1
                .restrict_deposit(rule!(deny_all), LOCKED) // 1
                .no_initial_supply();
                
            // Instantiate a Hello component, populating its vault with our supply of 1000 HelloToken
            Self {
                sample_vault: Vault::with_bucket(my_bucket),
                internal_authority: Vault::with_bucket(internal_admin),
                transiant_ressource_adress : address,
                stake : Vault::new(styx_adress),
                //styx_adress,
            }
            .instantiate()
            .globalize()
        }


        // This is a method, because it needs a reference to self.  Methods can only be called on components
        pub fn free_token(&mut self) -> Bucket {
            info!("My balance is: {} HelloToken. Now giving away a token!", self.sample_vault.amount());
            // If the semi-colon is omitted on the last line, the last value seen is automatically returned
            // In this case, a bucket containing 1 HelloToken is returned
            self.sample_vault.take(1)
        }

        pub fn stake_styx(&mut self, deposit : Bucket) -> Bucket {
            info!("You are going to stake : {}", deposit.amount());
            deposit
        }


    }
}