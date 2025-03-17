use anyhow::Result;
use jito_sdk_rust::JitoJsonRpcSDK;
use solana_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcProgramAccountsConfig, RpcTransactionConfig},
    rpc_filter::{Memcmp, MemcmpEncodedBytes, RpcFilterType},
};
use solana_pump::{
    grpc::get_pumpfun_stream,
    monitor::{
        create_sell_transaction, find_bonding_curve, process_tx_with_meta, send_bundle, transfer_tx,
    },
};
use solana_sdk::{
    bs58,
    commitment_config::CommitmentConfig,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use solana_transaction_status::{
    EncodedTransaction, UiCompiledInstruction, UiInnerInstructions, UiInstruction,
    UiParsedInstruction, UiTransactionEncoding, UiTransactionStatusMeta,
};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use std::str::FromStr;

use yellowstone_grpc_proto::prelude::subscribe_update::UpdateOneof;

use chrono::{DateTime, Utc};
use dotenv::{dotenv, var};
use futures_util::stream::StreamExt;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    env,
    sync::Arc,
    thread::sleep,
    time::{Duration, Instant},
};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let target_key: Pubkey = env::var("TOKEN_MINT")?.parse()?;
    let bd = find_bonding_curve(&target_key);
    let keypair = Keypair::from_base58_string(&env::var("PK")?);
    let wallet = keypair.pubkey();
    let rpc = RpcClient::new(env::var("RPC_URL")?);
    let balance = get_balance(&rpc, &keypair.pubkey(), &target_key).await?;
    let jito = JitoJsonRpcSDK::new(&env::var("JITO")?, None);
    let tip = env::var("TIP")?.parse::<u64>()?;
    
    // => 有人买就自动清仓

    // 1. 发币花了多少个 SOL
    // 2. 达到阈值之后自动清仓
    // 3. SOL 本位换位U本位


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
                                    let tx1 = create_sell_transaction(
                                        &bd,
                                        price,
                                        &target_key,
                                        &keypair,
                                        balance,
                                        1500.0,
                                        blockhash,
                                    )?;
                                    if tip > 0 {
                                        let tx2 = transfer_tx(
                                            &wallet,
                                            &get_tip_account()?,
                                            &keypair,
                                            10000,
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
use solana_transaction_status::EncodedTransactionWithStatusMeta;
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
    // Helper function to get the balance of a token account

    let token_ata = get_associated_token_address_with_program_id(wallet, mint, &spl_token::id());

    match rpc
        .get_token_account_balance_with_commitment(&token_ata, CommitmentConfig::confirmed())
        .await
    {
        Ok(balance) => Ok(balance.value.amount.as_str().parse::<u64>().unwrap()),
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
