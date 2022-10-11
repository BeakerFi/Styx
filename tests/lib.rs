//! NOTE these tests use a global resource (the resim exectuable's
//! simulator) and therefore MUST be run single threaded, like this
//! from the command line:
//!
//! cargo test -- --test-threads=1
//!
//! Also note that if you run the tests with increased output
//! verbosity enabled you may see panics or stacktraces during a
//! successful run. This is expected behaviour as we use
//! std::panic::catch_unwind to test calls under conditions that
//! should make them panic. One way to see a lot of this sort of
//! output would be to run the tests like this (in a Unix-like shell):
//!
//! RUST_BACKTRACE=1 cargo test -- --nocapture --test-threads=1

use std::process::Command;
use std::collections::HashSet;
use std::collections::HashMap;
use regex::Regex;
use lazy_static::lazy_static;


use radix_engine::ledger::*;
use radix_engine::transaction::TransactionReceipt;
use scrypto::core::NetworkDefinition;
use scrypto::prelude::*;
use scrypto_unit::*;
use transaction::builder::ManifestBuilder;

const RADIX_TOKEN: &str = "030000000000000000000000000000000000000000000000000004";


#[derive(Debug)]
struct Account {
    address: String,
    _pubkey: String,
    _privkey: String,
}


#[derive(Debug)]
struct DAO_component {
    address: String,
    external_admin_address: String,
    internal_admin_adress : String,
    styx_adress: String,
    voter_card_address: String,
}



/// Runs a command line program, panicking if it fails and returning
/// its stdout if it succeeds
fn run_command(command: &mut Command) -> String {
    let output = command
        .output()
        .expect("Failed to run command line");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        println!("stdout:\n{}", stdout);
        panic!("{}", stderr);
    }
    stdout
}


/// Calls "resim reset"
fn reset_sim() {
    run_command(Command::new("resim")
        .arg("reset"));
}



/// Calls "resim new-account"
///
/// Returns a tuple containing first the new account's address, then
/// its public key, and then last its private key.
fn create_account() -> Account {
    let output = run_command(Command::new("resim")
                             .arg("new-account"));

    lazy_static! {
        static ref RE_ADDRESS: Regex = Regex::new(r"Account component address: (\w*)").unwrap();
        static ref RE_PUBKEY:  Regex = Regex::new(r"Public key: (\w*)").unwrap();
        static ref RE_PRIVKEY: Regex = Regex::new(r"Private key: (\w*)").unwrap();
    }

    let address = &RE_ADDRESS.captures(&output).expect("Failed to parse new-account address")[1];
    let pubkey = &RE_PUBKEY.captures(&output).expect("Failed to parse new-account pubkey")[1];
    let privkey = &RE_PRIVKEY.captures(&output).expect("Failed to parse new-account privkey")[1];

    Account {
        address: address.to_string(),
        _pubkey: pubkey.to_string(),
        _privkey: privkey.to_string()
    }
}

// Create a token and return it's address
fn create_admin_badge() -> String {
    let output = run_command(Command::new("resim")
                            .arg("new-token-fixed")
                            .arg("--name")
                            .arg("admin_bagde")
                            .arg("1")
                        );

    String::from(output.split("\n").collect::<Vec<&str>>()[13].split(" ").collect::<Vec<&str>>()[2])

}


/// Publishes the package by calling "resim publish ."
///
/// Returns the new blueprint's address
fn publish_package(path: Option<&str>) -> String {
    let path = path.unwrap_or(".");
    let output = run_command(Command::new("resim")
                             .arg("publish")
                             .arg(path));
    lazy_static! {
        static ref RE_ADDRESS: Regex = Regex::new(r"New Package: (\w*)").unwrap();
    }

    RE_ADDRESS.captures(&output).expect("Failed to parse new blueprint address")[1].to_string()
}




/// Creates a new Dao catalog via
/// rtm/instantiate.rtm
///
/// Returns the dao created.
fn instantiate(account_addr: &str, package_addr: &str)
                                   -> DAO_component
{
    let output = run_command(Command::new("resim")
                             .arg("run")
                             .arg("src/rtm/instantiate.rtm")
                             .env("account", account_addr)
                             .env("package", &package_addr)
                             .env("initial_supply", "100"));




    println!("{}",output);

    let result = output.split("\n").collect::<Vec<&str>>();

    let i = 4 ; // for translation due to more info

    let dao_adress = result[13+i];
    let external_admin_adress = result[14+i];
    let internal_admin_adress = result[15+i];
    let styx_adress = result[16+i];
    let voter_card_adress = result[17+i];

    let dao_adress = dao_adress.split(" ").collect::<Vec<&str>>()[2];
    let external_admin_adress = external_admin_adress.split(" ").collect::<Vec<&str>>()[2];
    let internal_admin_adress = internal_admin_adress.split(" ").collect::<Vec<&str>>()[2];
    let styx_adress = styx_adress.split(" ").collect::<Vec<&str>>()[2];
    let voter_card_adress = voter_card_adress.split(" ").collect::<Vec<&str>>()[2];




    let dao = DAO_component {
        address: String::from(dao_adress),
        external_admin_address: String::from(external_admin_adress),
        internal_admin_adress : String::from(internal_admin_adress),
        styx_adress: String::from(styx_adress),
        voter_card_address: String::from(voter_card_adress),
    };
    dao
}


/// Creates a new Dao catalog via
/// rtm/instantiate_custom.rtm
///
/// Returns the dao created.
fn instantiate_custom(account_addr: &str, package_addr: &str, admin_badge_addr: &str)
                                   -> DAO_component
{
    let output = run_command(Command::new("resim")
                             .arg("run")
                             .arg("src/rtm/instantiate.rtm")
                             .env("account", account_addr)
                             .env("package", &package_addr)
                             .env("admin_badge", admin_badge_addr)
                             .env("initial_supply", "100"));




    println!("{}",output);

    let result = output.split("\n").collect::<Vec<&str>>();

    let i = 4 ; // for translation due to more info

    let dao_adress = result[13+i];
    let external_admin_adress = result[14+i];
    let internal_admin_adress = result[15+i];
    let styx_adress = result[16+i];
    let voter_card_adress = result[17+i];

    let dao_adress = dao_adress.split(" ").collect::<Vec<&str>>()[2];
    let external_admin_adress = external_admin_adress.split(" ").collect::<Vec<&str>>()[2];
    let internal_admin_adress = internal_admin_adress.split(" ").collect::<Vec<&str>>()[2];
    let styx_adress = styx_adress.split(" ").collect::<Vec<&str>>()[2];
    let voter_card_adress = voter_card_adress.split(" ").collect::<Vec<&str>>()[2];




    let dao = DAO_component {
        address: String::from(dao_adress),
        external_admin_address: String::from(external_admin_adress),
        internal_admin_adress : String::from(internal_admin_adress),
        styx_adress: String::from(styx_adress),
        voter_card_address: String::from(voter_card_adress),
    };
    dao
}


fn mint_voter_card_with_bucket(account_addr: &str,dao_address : &str , styx_address : &str, bucket_amount : &str) {
    let output = run_command(Command::new("resim")
                             .arg("run")
                             .arg("src/rtm/mint_voter_card_with_bucket.rtm")
                             .env("account", account_addr)
                             .env("dao", &dao_address)
                             .env("styx", styx_address)
                             .env("amount", bucket_amount));
}


#[test]
fn test_publish() {
    reset_sim();
    let user = create_account();
    let package_addr = publish_package(Some("."));
    println!("User Package : {:?}", package_addr);
}



#[test]
fn test_instantiate() {
    reset_sim();
    let user = create_account();
    let package_addr = publish_package(Some("."));
    let dao = instantiate(&user.address, &package_addr);
    println!("dao component : {:#?}", dao);
}

#[test]
fn test_instantiate_custom() {
    reset_sim();
    let user = create_account();
    let package_addr = publish_package(Some("."));
    let admin_badge_addr = create_admin_badge();
    let dao = instantiate_custom(&user.address, &package_addr, &admin_badge_addr );
    println!("dao component : {:#?}", dao);
}


#[test]
fn test_hello() {
    // ├─ CallMethod { component_address: system_sim1qsqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqs9fh54n, method_name: "lock_fee", args: Struct(Decimal("1000")) }


    // Setup the environment
    let mut store = TypedInMemorySubstateStore::with_bootstrap();
    let mut test_runner = TestRunner::new(true, &mut store);

    // Create an account
    let (public_key, _private_key, account_component) = test_runner.new_account();

    // Publish package
    let package_address = test_runner.compile_and_publish(this_package!());

    // Test the `instantiate_hello` function.
    let manifest = ManifestBuilder::new(&NetworkDefinition::simulator())
        .call_function(package_address, "Styx", "instantiate", args!(dec!("100")))
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
