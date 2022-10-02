use scrypto::prelude::*;



blueprint! {
    struct Styx {
        // Define what resources and data will be managed by Hello components
        sample_vault: Vault,
        internal_authority : Vault
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
                .initial_supply(1);

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
                .initial_supply(100);    
            
            // Instantiate a Hello component, populating its vault with our supply of 1000 HelloToken
            Self {
                sample_vault: Vault::with_bucket(my_bucket),
                internal_authority: Vault::with_bucket(internal_admin)
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
    }
}