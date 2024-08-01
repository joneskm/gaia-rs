use std::{env, error::Error};
fn main() -> Result<(), Box<dyn Error>> {
    let account_address_prefix = env::var("BECH_32_MAIN_PREFIX").map_err(|_| "BECH_32_MAIN_PREFIX environment variable must be set. This is best done in a .cargo/config.toml file in the root of your project")?;
    println!(
        "cargo:rustc-env=BECH_32_MAIN_PREFIX={}",
        account_address_prefix
    );

    //println!("cargo:rerun-if-env-changed=ACCOUNT_ADDRESS_PREFIX"); //not working https://github.com/rust-lang/cargo/issues/10358
    Ok(())
}
