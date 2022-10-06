//! Blueprint implementing the PocketSwap Basic Pool model.
use crate::fee::Fee;
use crate::position::*;
use crate::utils::*;
use scrypto::prelude::*;
use scrypto_maths::sqrt;

blueprint! {
    pub struct PocketSwapBasicPool {
        /// Vault containing X tokens (liquidity + fees)
        x_vault: Vault,

        /// Vault containing Y tokens (liquidity + fees)
        y_vault: Vault,

        /// Variable tracking the amount of X used as liquidity
        x: Decimal,

        /// Variable tracking the amount of Y used as liquidity
        y: Decimal,

        /// Pool fees
        pool_fee: Decimal,

        /// Accrued fees in token X per liquidity unit
        x_fees_per_liq: Decimal,

        /// Accrued fees in token Y per liquidity unit
        y_fees_per_liq: Decimal,

        /// NFT minter
        position_minter: Vault,

        /// NFT address
        position_resource: ResourceAddress,

        /// Pool owner badge used to claim the protocol fees
        admin_badge: ResourceAddress,

        /// Protocol fees
        protocol_fee: Decimal,

        /// Unclaimed protocol fees in token X
        unclaimed_x: Decimal,

        /// Unclaimed protocol fees in token Y
        unclaimed_y: Decimal,
    }

    impl PocketSwapBasicPool {
        /// Creates an empty pool for two given tokens
        ///
        /// Returns the [`ComponentAddress`] of the pool and a [`Bucket`] containing an admin badge
        /// that can be used to claim protocol fees.
        pub fn new(
            x_token: ResourceAddress,
            y_token: ResourceAddress,
            pool_fee: Fee,
        ) -> (ComponentAddress, Bucket) {
            // Create the position minter
            let minter = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .initial_supply(dec!("1"));

            // Create the NFR address for the positions of this pool
            let position_resource = ResourceBuilder::new_non_fungible()
                .metadata("name", pair_name(x_token, y_token))
                .mintable(rule!(require(minter.resource_address())), LOCKED)
                .burnable(rule!(require(minter.resource_address())), LOCKED)
                .updateable_non_fungible_data(rule!(require(minter.resource_address())), LOCKED)
                .no_initial_supply();

            // Create the admin badge
            let admin_badge: Bucket = ResourceBuilder::new_fungible()
                .divisibility(DIVISIBILITY_NONE)
                .metadata("name", "admin badge")
                .burnable(rule!(allow_all), LOCKED)
                .initial_supply(1);

            // Create the access rules
            let rules: AccessRules = AccessRules::new()
                .method(
                    "claim_protocol_fees",
                    rule!(require(admin_badge.resource_address())),
                )
                .default(rule!(allow_all));

            // Initialize the component
            let mut component = Self {
                x_vault: Vault::new(x_token),
                y_vault: Vault::new(y_token),
                x: dec!(0),
                y: dec!(0),
                pool_fee: pool_fee.dec(),
                x_fees_per_liq: dec!(0),
                y_fees_per_liq: dec!(0),
                position_minter: Vault::with_bucket(minter),
                position_resource: position_resource,
                admin_badge: admin_badge.resource_address(),
                protocol_fee: dec!("0.15"),
                unclaimed_x: dec!(0),
                unclaimed_y: dec!(0),
            }
            .instantiate();
            component.add_access_check(rules);
            let component = component.globalize();

            (component, admin_badge)
        }

        /// Adds liquidity to the pool
        ///
        /// The function takes a [`Proof`] corresponding to a pool [`Position`].
        /// If the user is providing liquidity for the first time, they should create a position first.
        ///
        /// Returns the unclaimed fees corresponding to the position and the excess tokens.
        pub fn add_liquidity(
            &mut self,
            mut x_tokens: Bucket,
            mut y_tokens: Bucket,
            position_nft: Proof,
        ) -> (Bucket, Bucket)
        {
            // First make all necessary checks
            assert!(
                x_tokens.resource_address() == self.x_vault.resource_address()
                    && y_tokens.resource_address() == self.y_vault.resource_address(),
                "The given tokens don't match the tokens traded by the pool"
            );

            assert!(
                x_tokens.amount() != dec!(0) && y_tokens.amount() != dec!(0),
                "Please supply a positive number of both tokens X and Y"
            );

            let validated_proof = self.check_proof(position_nft);

            // Collect the uncollected fees in both tokens
            // We don't need to check the given proof because it will be done in the function call
            let (mut x_bucket, mut y_bucket) = self.compute_fees(&validated_proof);

            // These variables track the right amount of token x and y that will be added as
            // liquidity
            let right_x;
            let right_y;

            if self.x == dec!(0) && self.y == dec!(0) {
                // If the pool is empty accept both tokens as the new rate
                right_x = x_tokens.amount();
                right_y = y_tokens.amount();

                // Put the tokens in the vaults
                self.x_vault.put(x_tokens);
                self.x_vault.put(y_tokens);
            } else {
                // Otherwise, make sure that the given tokens follow the pool rate and send back the
                // excess amount of tokens

                let pool_rate: Decimal = self.x / self.y;
                let provided_rate: Decimal = x_tokens.amount() / y_tokens.amount();

                if provided_rate > pool_rate {
                    // In this case, there is too much x

                    // Compute the right amount of tokens x that should be taken from the bucket
                    right_x = y_tokens.amount() * pool_rate;
                    right_y = y_tokens.amount();

                    // Add tokens to the vaults
                    self.x_vault.put(x_tokens.take(right_x));
                    self.y_vault.put(y_tokens);

                    // Put the remaining tokens in the x_bucket
                    x_bucket.put(x_tokens);
                } else {
                    // In this case, there is either too much y or the right amount

                    // Compute the right amount of tokens y that should be taken from the bucket
                    right_x = x_tokens.amount();
                    right_y = x_tokens.amount() / pool_rate;

                    // Add tokens to the vault
                    self.x_vault.put(x_tokens);
                    self.y_vault.put(y_tokens.take(right_y));

                    // Put the remaining tokens in the y_bucket
                    y_bucket.put(y_tokens);
                }
            }

            // Update the amount of X and Y used as liquidity
            self.x += right_x;
            self.y += right_y;

            // Update the given position
            let old_position_data = self.get_position_data(&validated_proof);
            self.update_position
            (
                &validated_proof,
                old_position_data.liquidity + sqrt(right_x * right_y),
                self.x_fees_per_liq,
                self.y_fees_per_liq,
            );

            validated_proof.drop();

            // Return excess tokens
            (x_bucket, y_bucket)
        }

        /// Removes liquidity from a position
        ///
        /// Returns the unclaimed fees corresponding to the position and the tokens corresponding
        /// to the given pool [`Position`].
        pub fn remove_liquidity(
            &mut self,
            liquidity: Decimal,
            position_nft: Proof,
        ) -> (Bucket, Bucket)
        {
            // Check the proof
            let validated_proof = self.check_proof(position_nft);

            // Compute the fees
            let (mut x_bucket, mut y_bucket) = self.compute_fees(&validated_proof);

            // Get the position data
            let old_position_data = self.get_position_data(&validated_proof);

            // Check that the position has the right amount of liquidity
            assert!(
                old_position_data.liquidity >= liquidity,
                "You can't remove more liquidity than what you own"
            );

            let sqrt_price = sqrt(self.x / self.y);
            // Compute the amount of token X and Y that should be withdrawn
            let qty_x = liquidity * sqrt_price;
            let qty_y = liquidity / sqrt_price;

            // Withdraw the money from the vaults
            x_bucket.put(self.x_vault.take(qty_x));
            y_bucket.put(self.y_vault.take(qty_y));

            // Update the variables of the pool
            self.x -= qty_x;
            self.y -= qty_y;

            // Update the nft position
            self.update_position(
                &validated_proof,
                old_position_data.liquidity - liquidity,
                old_position_data.last_x_fees_per_liq,
                old_position_data.last_y_fees_per_liq,
            );

            validated_proof.drop();

            (x_bucket, y_bucket)
        }

        /// Swaps the given amount of one of the pool tokens for some of the other token of the pool
        ///
        /// Returns the other tokens quantity
        pub fn swap(&mut self, input_tokens: Bucket) -> Bucket {
            let token_address = input_tokens.resource_address();
            // Make sure that the user is trying to trade the right coins
            assert!(
                token_address == self.x_vault.resource_address()
                    || token_address == self.y_vault.resource_address(),
                "This pool cannot trade this coin!"
            );

            // Compute pool fees and protocol fees
            let fees: Decimal = input_tokens.amount() * self.pool_fee;
            let pool_fee: Decimal = fees * (dec!(1) - self.protocol_fee);
            let prot_fee: Decimal = fees * self.protocol_fee;

            // Compute real amount of input_tokens that will be traded
            let traded_tokens: Decimal = (dec!(1) - fees) * input_tokens.amount();

            let mut output_bucket: Bucket;

            match input_tokens.resource_address() == self.x_vault.resource_address() {
                true => {
                    // In this case the input is X tokens

                    // Compute the amount of tokens Y to output
                    let output_amount: Decimal = traded_tokens * self.y / (self.x + traded_tokens);

                    // Put the input tokens in the X vault
                    self.x_vault.put(input_tokens);

                    // Put the appropriate amount of Y tokens in the output bucket
                    output_bucket = Bucket::new(self.y_vault.resource_address());
                    output_bucket.put(self.y_vault.take(output_amount));

                    // Update the variables of the pool
                    self.x += traded_tokens;
                    self.y -= output_amount;
                    let pool_liq = sqrt(self.x * self.y);
                    self.x_fees_per_liq += pool_fee / pool_liq;
                    self.unclaimed_x += prot_fee;
                }

                false => {
                    // In this case the input is Y tokens

                    // Compute the amount of tokens X to output
                    let output_amount: Decimal = traded_tokens * self.x / (self.y + traded_tokens);

                    // Put the input tokens in the Y vault
                    self.y_vault.put(input_tokens);

                    // Put the appropriate amount of X tokens in the output bucket
                    output_bucket = Bucket::new(self.x_vault.resource_address());
                    output_bucket.put(self.x_vault.take(output_amount));

                    // Update the variables of the pool
                    self.x -= output_amount;
                    self.y += traded_tokens;
                    let pool_liq = sqrt(self.x * self.y);
                    self.y_fees_per_liq += pool_fee / pool_liq;
                    self.unclaimed_y += prot_fee;
                }
            }

            output_bucket
        }

        /// Collects fees for a given [`Position`]
        ///
        /// Returns a couple of bucket (`X`,`Y`) with the right amount of fees
        pub fn collect_fees(&mut self, position_nft: Proof) -> (Bucket, Bucket)
        {
            let validated_proof = self.check_proof(position_nft);
            let buckets = self.compute_fees(&validated_proof);
            validated_proof.drop();

            buckets
        }

        /// Collects the fees associated to the protocol
        pub fn claim_protocol_fees(&mut self) -> (Bucket, Bucket) {
            // Put unclaimed protocol fees in buckets
            let mut x_bucket = Bucket::new(self.x_vault.resource_address());
            x_bucket.put(self.x_vault.take(self.unclaimed_x));

            let mut y_bucket = Bucket::new(self.y_vault.resource_address());
            y_bucket.put(self.y_vault.take(self.unclaimed_y));

            // Update pool
            self.unclaimed_x = dec!(0);
            self.unclaimed_y = dec!(0);

            (x_bucket, y_bucket)
        }

        /// Creates an empty position
        pub fn create_position(&self) -> Bucket {
            let position_data = Position {
                liquidity: dec!(0),
                last_x_fees_per_liq: dec!(0),
                last_y_fees_per_liq: dec!(0),
            };

            let position_nft = self.position_minter.authorize(|| {
                let resource_manager = borrow_resource_manager!(self.position_resource);
                resource_manager.mint_non_fungible(
                    // CHANGE THIS AT SOME POINT
                    &NonFungibleId::random(),
                    position_data,
                )
            });

            position_nft
        }

        /// Private function to compute and output the fees for a given [`Position`]
        fn compute_fees(&mut self, validated_proof: &ValidatedProof) -> (Bucket, Bucket) {

            // Get the position data
            let position_data = self.get_position_data(validated_proof);

            // Compute fees to give
            let x_fees: Decimal =
                (self.x_fees_per_liq - position_data.last_x_fees_per_liq) * position_data.liquidity;
            let y_fees: Decimal =
                (self.y_fees_per_liq - position_data.last_y_fees_per_liq) * position_data.liquidity;

            // Take the fees from the vault
            let mut x_bucket: Bucket = Bucket::new(self.x_vault.resource_address());
            let mut y_bucket: Bucket = Bucket::new(self.y_vault.resource_address());
            x_bucket.put(self.x_vault.take(x_fees));
            y_bucket.put(self.x_vault.take(y_fees));

            // Update position
            self.update_position(
                &validated_proof,
                position_data.liquidity,
                self.x_fees_per_liq,
                self.y_fees_per_liq,
            );

            // Send the fees
            (x_bucket, y_bucket)
        }

        /// Returns the [`Position`] associated to a [`Proof`]
        fn get_position_data(&self, validated_proof: &ValidatedProof) -> Position
        {
            let resource_manager: &ResourceManager =
                borrow_resource_manager!(self.position_resource);
            let id = validated_proof.non_fungible::<Position>().id();
            resource_manager.get_non_fungible_data::<Position>(&id)
        }

        /// Updates a position
        fn update_position(
            &self,
            validated_proof: &ValidatedProof,
            liquidity: Decimal,
            last_x_fees_per_liq: Decimal,
            last_y_fees_per_liq: Decimal,
        ) {
            let resource_manager: &mut ResourceManager =
                borrow_resource_manager!(self.position_resource);
            let id = validated_proof.non_fungible::<Position>().id();

            let new_position_data = Position {
                liquidity,
                last_x_fees_per_liq,
                last_y_fees_per_liq,
            };

            self.position_minter
                .authorize(|| resource_manager.update_non_fungible_data(&id, new_position_data));
        }


        /// Checks that a given [`Proof`] corresponds to a position and returns the associated
        /// [`ValidatedProof`]
        fn check_proof(&self, position_nft: Proof) -> ValidatedProof
        {

            let valid_proof: ValidatedProof =  position_nft.validate_proof
            (
                    ProofValidationMode::ValidateContainsAmount
                        (
                            self.position_resource,
                            dec!(1)
                        )
            ).expect("Invalid proof provided");

            valid_proof
        }

        pub fn x_in_vault(&self) -> Decimal {
            self.x_vault.amount()
        }

        pub fn y_in_vault(&self) -> Decimal {
            self.y_vault.amount()
        }

        pub fn x(&self) -> Decimal {
            self.x
        }
        pub fn y(&self) -> Decimal {
            self.y
        }

        pub fn pool_fee(&self) -> Decimal {
            self.pool_fee
        }
        pub fn x_fees_per_liq(&self) -> Decimal {
            self.x_fees_per_liq
        }
        pub fn y_fees_per_liq(&self) -> Decimal {
            self.y_fees_per_liq
        }
        pub fn protocol_fee(&self) -> Decimal {
            self.protocol_fee
        }
        pub fn unclaimed_x(&self) -> Decimal {
            self.unclaimed_x
        }
        pub fn unclaimed_y(&self) -> Decimal {
            self.unclaimed_y
        }
    }
}