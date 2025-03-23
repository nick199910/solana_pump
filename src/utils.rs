
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{account::Account, pubkey::Pubkey};
use anyhow::Result;
use pyth_sdk_solana::{state::SolanaPriceAccount, Price}; 
 
pub fn check_sol_change(
    price: f32,
    balance: u64, 
    gas_cost: u64, 
    jito_cost: u64
) -> Result<f32> {
    // 获取交易前的SOL余额
    let sol_lamports_cost = (balance as f32 * price * 1000.0) as f32 - gas_cost as f32 - jito_cost as f32;
    Ok(sol_lamports_cost)
}

pub async fn get_sol_price(rpc: &RpcClient) -> Result<f64> {
    let price_key: Pubkey = "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG".parse().unwrap();
    let mut price_account: Account = rpc.get_account(&price_key).await?;
    match SolanaPriceAccount::account_to_feed(&price_key, &mut price_account) {
        Ok(feed) => {
            let current_price: Price = feed.get_price_unchecked();
            Ok(current_price.price as f64 * 10.0f64.powi(current_price.expo))
        },
        Err(e) => {
            Err(e.into())
        }
    }
}



#[tokio::test]
async fn test_get_sol_price() {
    use dotenv::dotenv;
    use std::env;
    dotenv().ok();
    let rpc = RpcClient::new(env::var("RPC_URL").unwrap());
    let price = get_sol_price(&rpc).await.unwrap();
}