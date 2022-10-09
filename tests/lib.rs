use radix_engine::ledger::*;
use radix_engine::transaction::TransactionReceipt;
use scrypto::core::NetworkDefinition;
use scrypto::prelude::*;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;

#[test]
fn test_hello() {
    // Setup the environment
    let mut store = TypedInMemorySubstateStore::with_bootstrap();
    let mut test_runner = TestRunner::new(true, &mut store);

    // Create an account
    let (public_key, _private_key, account_component) = test_runner.new_account();

    // Publish package
    let package_address = test_runner.compile_and_publish(this_package!());

    // Test the `instantiate_hello` function.
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .call_function(package_address, "Styx", "instantiate", args!())
        .build();
    let receipt = test_runner.execute_manifest_ignoring_fee(manifest, vec![public_key.into()]);
    println!("{:?}\n", receipt);
    receipt.expect_commit_success();
    let component = receipt
        .expect_commit()
        .entity_changes
        .new_component_addresses[0];

    let stx = receipt
        .expect_commit()
        .entity_changes
        .new_resource_addresses[1];


    let nft = receipt
        .expect_commit()
        .entity_changes
        .new_resource_addresses[0];

    println!("The stx adress is {}",stx);

    

    

    // Test the `free_token` method.
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .call_method(component, "free_token", args!())
        .call_method(
            account_component,
            "deposit_batch",
            args!(Expression::entire_worktop()),
        )
        .build();
    let receipt = test_runner.execute_manifest_ignoring_fee(manifest, vec![public_key.into()]);
    println!("{:?}\n", receipt);
    receipt.expect_commit_success();

    
    // Test the `free_token` method.
        let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .call_method(component, "free_token", args!())
        .call_method(
            account_component,
            "deposit_batch",
            args!(Expression::entire_worktop()),
        )
        .build();
    let receipt = test_runner.execute_manifest_ignoring_fee(manifest, vec![public_key.into()]);
    println!("{:?}\n", receipt);
    receipt.expect_commit_success();

        //let mut nft_adress;
        // Test the `stake` method.
        let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .call_method(account_component, "withdraw_by_amount", args!(dec!("1"),stx))
        .take_from_worktop_by_amount(dec!("1"), stx, |builder, bucket_id| {
            builder.call_method(
                component,
                "stake",
                args!(
                    scrypto::resource::Bucket(bucket_id)
                ),
            )
        })
        .call_method(
            account_component,
            "deposit_batch",
            args!(Expression::entire_worktop()),
        )
        .build();
    let receipt : TransactionReceipt = test_runner.execute_manifest_ignoring_fee(manifest, vec![public_key.into()]);
    println!("{:?}\n", receipt);

    /*

    let outcome = &receipt
    .expect_commit()
    .outcome;

    match outcome {
        TransactionOutcome::Succes(output) => {
            println!("output{:?}",output)
        }
        _ => {  println!("output error") }
        
    }
    */
        
    receipt.expect_commit_success();

    // Test the `stake then unstake` method.
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
    .call_method(account_component, "withdraw_by_amount", args!(dec!("1"),stx))
    .take_from_worktop_by_amount(dec!("1"), stx, |builder, bucket_id| {
        builder.call_method(
            component,
            "stake",
            args!(
                scrypto::resource::Bucket(bucket_id)
            ),
        )
    })
    
    .call_method(
        account_component,
        "deposit_batch",
        args!(Expression::entire_worktop()),
    )
    .build();
let receipt : TransactionReceipt = test_runner.execute_manifest_ignoring_fee(manifest, vec![public_key.into()]);
println!("{:?}\n", receipt);


    

    
}
