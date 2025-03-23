use anyhow::Result;
use jito_sdk_rust::JitoJsonRpcSDK;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_pump::{
    grpc::get_pumpfun_stream,
    monitor::{ 
        create_sell_transaction, find_bonding_curve, process_tx_with_meta, send_bundle, transfer_tx,
    }, utils::{check_sol_change, get_sol_price},
};
use solana_sdk::{
    commitment_config::CommitmentConfig, pubkey::Pubkey, signature::Keypair, signer::Signer
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use std::str::FromStr;

use yellowstone_grpc_proto::prelude::subscribe_update::UpdateOneof;

use dotenv::dotenv;
use futures_util::stream::StreamExt;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let target_key: Pubkey = env::var("TOKEN_MINT")?.parse()?;
    println!("target_key: {}", target_key); 
    let bd = find_bonding_curve(&target_key);
    let keypair = Keypair::from_base58_string(&env::var("PK")?);

    let wallet = keypair.pubkey();
    // println!("wallet: {}", wallet);
    let rpc = RpcClient::new(env::var("RPC_URL")?);
    let balance = get_balance(&rpc, &wallet, &target_key).await?;

    let launch_cost = env::var("LAUNCH_COST")?.parse::<u64>()?;
    println!("launch_cost: {}", launch_cost);
    let min_profit = env::var("MIN_PROFIT")?.parse::<u64>()?;

    let jito = JitoJsonRpcSDK::new(&env::var("JITO")?, None);
    let tip = env::var("TIP")?.parse::<u64>()?;

    let min_profit = launch_cost + min_profit + tip;

    if balance == 0 {
        println!("wallet balance = 0, return");
        return Ok(());
    }

    
    // => 有人买就自动清仓

    // 1. 发币花了多少个 SOL
    // 2. 达到阈值之后自动清仓
    // 3. SOL 本位换位U本位
    // 
    println!(" ========================================= Pump 狙击手 ========================================= ");
    let wallet_balance = rpc.get_balance(&wallet).await? as f64;
    println!("阻击手 原始资金: {} SOL", wallet_balance  / 1000000000.0);
    let sol_price_usd = get_sol_price(&rpc).await?;
    println!("当前 SOL 价格: {}", sol_price_usd);
    let wallet_balance_usd = wallet_balance * sol_price_usd / 1000000000.0;
    println!("阻击手 原始资金: {} USDT", wallet_balance_usd);
    
    let mut stream = get_pumpfun_stream().await?;
    while let Some(data) = stream.next().await {
        match data {
            Ok(update) => {
                if let Some(update_oneof) = update.update_oneof {
                    match update_oneof {
                        UpdateOneof::Transaction(sub_tx) => {
                            if let Some(tx_info) = sub_tx.transaction {
                                let tx_with_meta = convert_to_encoded_tx(tx_info)?;
                                if let Some(price) = process_tx_with_meta(target_key, tx_with_meta)
                                { 
                                    let blockhash = rpc.get_latest_blockhash().await?;
                                    // 在创建卖的交易的时候
                                    let tx1 = create_sell_transaction(
                                        &bd,
                                        price,
                                        &target_key,
                                        &keypair,
                                        balance,
                                        1500.0,
                                        blockhash,
                                    )?; 
                                    // 

                                    // 检查卖出交易是否是亏本交易
                                    // 亏本或者亏本小于10000
                                    let sol_change = check_sol_change(price, balance, launch_cost, tip)?;
                                    println!("狙击手利润为 {}", sol_change);
                                    if sol_change < 0.0 || sol_change < min_profit as f32 {
                                        println!("狙击手利润为 {}，亏本或者亏本小于最小利润 {}，不交易", sol_change, min_profit);
                                        continue;
                                    }
                                    if tip > 0 {
                                        let tx2 = transfer_tx(
                                            &wallet,
                                            &get_tip_account()?,
                                            &keypair,
                                            tip,
                                            blockhash, 
                                        );
                                        let bundle_id = send_bundle(&jito, vec![tx1, tx2]).await?;
                                        println!("bundle id {:?}", bundle_id);

                                    } else {
                                        let sig = rpc.send_and_confirm_transaction(&tx1).await?;
                                        // todo! 去模拟交易
                                        println!("sig {:?}", sig);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => {}
        }
    }

    Ok(())
}
use anyhow::anyhow;
use solana_transaction_status::{EncodedTransactionWithStatusMeta, UiTransactionEncoding};
use yellowstone_grpc_proto::convert_from;
use yellowstone_grpc_proto::prelude::SubscribeUpdateTransactionInfo;

pub fn convert_to_encoded_tx(
    tx_info: SubscribeUpdateTransactionInfo,
) -> Result<EncodedTransactionWithStatusMeta> {
    convert_from::create_tx_with_meta(tx_info)
        .unwrap()
        .encode(UiTransactionEncoding::Base64, Some(u8::MAX), true)
        .map_err(|e| anyhow!("{}", e))
}

pub async fn get_balance(rpc: &RpcClient, wallet: &Pubkey, mint: &Pubkey) -> Result<u64> {
    let token_ata = get_associated_token_address_with_program_id(wallet, mint, &spl_token::id());
    
    // 首先检查账户是否存在
    match rpc.get_account_with_commitment(&token_ata, CommitmentConfig::confirmed()).await {
        Ok(response) => {
            if let Some(_) = response.value {
                // 账户存在，获取余额
                match rpc.get_token_account_balance_with_commitment(&token_ata, CommitmentConfig::confirmed()).await {
                    Ok(balance) => Ok(balance.value.amount.as_str().parse::<u64>().unwrap()),
                    Err(e) => Err(e.into()),
                }
            } else {
                // 账户不存在，返回0余额
                Ok(0)
            }
        },
        Err(e) => Err(e.into()),
    }
}

use rand::{rng, seq::IteratorRandom};
pub fn get_tip_account() -> Result<Pubkey> {
    let accounts = [
        "ADaUMid9yfUytqMBgopwjb2DTLSokTSzL1zt6iGPaS49",
        "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
        "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
        "ADuUkR4vqLUMWXxW9gh6D6L8pMSawimctcNZ5pGwDcEt",
        "HFqU5x63VTqvQss8hp11i4wVV8bD44PvwucfZ2bU7gRe",
        "DfXygSm4jCyNCybVYYK6DwvWqjKee8pbDmJGcLWNDXjh",
        "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnizKZ6jT",
        "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
    ];
    let mut rng = rng();
    match accounts.iter().choose(&mut rng) {
        Some(acc) => Ok(Pubkey::from_str(acc).inspect_err(|err| {})?),
        None => Err(anyhow!("jito: no tip accounts available")),
    }
}
