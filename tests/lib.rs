use radix_engine::ledger::*;
use radix_engine::transaction::TransactionReceipt;
use radix_engine::types::*;
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

    let autre = receipt
        .expect_commit()
        .entity_changes
        .new_resource_addresses[2];

    println!("The stx adress is {}",stx);
    println!("The autre adress is {}",autre);
    println!("The nft adress is {}",nft);

    // See Balances:
      let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
      .call_method(account_component, "balance", args!(stx))
      .call_method(account_component, "balance", args!(nft))
      .call_method(account_component, "balance", args!(autre))
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

    // See Balances:
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
    .call_method(account_component, "balance", args!(stx))
    .call_method(account_component, "balance", args!(nft))
    .call_method(account_component, "balance", args!(autre))
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
        .call_method(component, "free_nft", args!())
        .call_method(
            account_component,
            "deposit_batch",
            args!(Expression::entire_worktop()),
        )
        .build();
    let receipt = test_runner.execute_manifest_ignoring_fee(manifest, vec![public_key.into()]);
    println!("{:?}\n", receipt);
    receipt.expect_commit_success();

        // See Balances:
        let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .call_method(account_component, "balance", args!(stx))
        .call_method(account_component, "balance", args!(nft))
        .call_method(account_component, "balance", args!(autre))
        .call_method(
            account_component,
            "deposit_batch",
            args!(Expression::entire_worktop()),
        )
        .build();
    let receipt = test_runner.execute_manifest_ignoring_fee(manifest, vec![public_key.into()]);
    println!("{:?}\n", receipt);
    receipt.expect_commit_success();


    if let Some(account_component) = test_runner.inspect_component_state(component) {
        let account_comp_state = ScryptoValue::from_slice(account_component.state()).unwrap();
        println!("{:?} \n\n\n\n\n\n\n\n\n\n\n",account_comp_state);
        // let decoded_state: StructFromContract = scrypto_decode(&account_comp_state.raw).unwrap();

    }


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


        // See Balances:
        let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .call_method(account_component, "balance", args!(stx))
        .call_method(account_component, "balance", args!(nft))
        .call_method(
            account_component,
            "deposit_batch",
            args!(Expression::entire_worktop()),
        )
        .build();
    let receipt = test_runner.execute_manifest_ignoring_fee(manifest, vec![public_key.into()]);
    println!("{:?}\n", receipt);
    receipt.expect_commit_success();


if let Some(account_component) = test_runner.inspect_component_state(component) {
        let account_comp_state = ScryptoValue::from_slice(account_component.state()).unwrap();
        println!("{:?} \n\n\n\n\n\n\n\n\n\n\n",account_comp_state);
        // let decoded_state: StructFromContract = scrypto_decode(&account_comp_state.raw).unwrap();
}


           
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

        .take_from_worktop( nft, |builder, bucket_id| {
            println!("{:?} \n\n\n\n\n\n\n\n\n\n\n",bucket_id);
            builder.create_proof_from_bucket(
                bucket_id,
                | builder2, proof| {
                    builder2.call_method(component, "unstake", args!(scrypto::resource::Proof(proof), dec!("1")))
                })
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
