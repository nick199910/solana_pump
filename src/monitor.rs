use std::str::FromStr;

use jito_sdk_rust::JitoJsonRpcSDK;
use serde_json::json;
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_client::SerializableTransaction};
use solana_program::pubkey;
use solana_sdk::{
    bs58, compute_budget::ComputeBudgetInstruction, hash::Hash, pubkey::Pubkey, signature::Keypair,
    signer::Signer, system_instruction, transaction::Transaction,
};
use solana_transaction_status::{
    option_serializer::OptionSerializer, EncodedTransactionWithStatusMeta, UiCompiledInstruction,
    UiInnerInstructions, UiInstruction,
};
use spl_associated_token_account::{
    get_associated_token_address, get_associated_token_address_with_program_id,
};
use spl_token::instruction::close_account;

use anyhow::{anyhow, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::instruction::{AccountMeta, Instruction};


// program相关
pub const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");
pub const SYSTEM_RENT_PROGRAM_ID: Pubkey = pubkey!("SysvarRent111111111111111111111111111111111");
pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const ASSOC_TOKEN_ACC_PROGRAM_ID: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const EVENT_AUTHORITY: Pubkey = pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
pub const KEY_PREFIX: &'static str = "token:info:";
pub const UNIT_LIMIT: u32 = 500000;
pub const UNIT_PRICE: u64 = 20000;

// pumpfun
pub const PUMPFUN_PROGRAM_ID: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub const PUMPFUN_GLOBAL: Pubkey = pubkey!("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
pub const PUMPFUN_FEE_RECIPIENT: Pubkey = pubkey!("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM");
pub const INIT_SOL_REVERSES: u64 = 30_000_000_000;
pub const INIT_TOKEN_REVERSES: u64 = 1_073_000_191_000_000;
pub const INIT_PRICE: f32 = (INIT_SOL_REVERSES as f32 / 1e9) / (INIT_TOKEN_REVERSES as f32 / 1e6);
pub const PUMPFUN_TOTAL_SUPPLY: u64 = 1_000_000_000_000_000;

// 标量
pub const MINUTES: u64 = 60 * 1000;
pub const SECONDS: u64 = 1000;

// use crate::common::constants::{
//     ASSOC_TOKEN_ACC_PROGRAM_ID, EVENT_AUTHORITY, PUMPFUN_FEE_RECIPIENT, PUMPFUN_GLOBAL,
//     PUMPFUN_PROGRAM_ID, SYSTEM_RENT_PROGRAM_ID, TOKEN_PROGRAM_ID, UNIT_LIMIT, UNIT_PRICE,
// };

/// 鉴别符
const PUMPFUN_CREATE_EVENT: [u8; 8] = [27, 114, 169, 77, 222, 235, 99, 118];
const PUMPFUN_COMPLETE_EVENT: [u8; 8] = [95, 114, 97, 156, 212, 46, 152, 8];
const PUMPFUN_TRADE_EVENT: [u8; 8] = [189, 219, 127, 211, 78, 230, 97, 238];

#[derive(Debug, Clone)]
pub enum TargetEvent {
    PumpfunBuy(TradeEvent),
    PumpfunSell(TradeEvent),
    PumpfunCreate(CreateEvent),
    PumpfunComplete(CompleteEvent),
}

impl TryFrom<UiInstruction> for TargetEvent {
    type Error = anyhow::Error;

    fn try_from(inner_instruction: UiInstruction) -> Result<Self, Self::Error> {
        // 处理每一条指令
        match inner_instruction {
            solana_transaction_status::UiInstruction::Compiled(ui_compiled_instruction) => {
                if let Some(create) =
                    CreateEvent::try_from_compiled_instruction(&ui_compiled_instruction)
                {
                    return Ok(TargetEvent::PumpfunCreate(create));
                }
                if let Some(complete) =
                    CompleteEvent::try_from_compiled_instruction(&ui_compiled_instruction)
                {
                    return Ok(Self::PumpfunComplete(complete));
                }
                if let Some(trade) =
                    TradeEvent::try_from_compiled_instruction(&ui_compiled_instruction)
                {
                    if trade.is_buy {
                        return Ok(TargetEvent::PumpfunBuy(trade));
                    } else {
                        return Ok(TargetEvent::PumpfunSell(trade));
                    }
                }
            }
            _ => {}
        }
        return Err(anyhow!("failed to convert to target tx"));
    }
}

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct CreateEvent {
    pub name: String,
    pub symbol: String,
    pub uri: String,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub user: Pubkey,
}

impl CreateEvent {
    pub fn try_from_compiled_instruction(
        ui_compiled_instruction: &UiCompiledInstruction,
    ) -> Option<CreateEvent> {
        let data = bs58::decode(ui_compiled_instruction.data.clone())
            .into_vec()
            .unwrap();
        if data.len() > 16 && data[8..16].eq(&PUMPFUN_CREATE_EVENT) {
            match CreateEvent::try_from_slice(&data[16..]) {
                Ok(event) => return Some(event),
                Err(_) => return None,
            }
        } else {
            return None;
        }
    }
}

#[derive(Debug, BorshSerialize, Clone, BorshDeserialize, Copy)]
pub struct CompleteEvent {
    pub user: Pubkey,
    pub mint: Pubkey,
    pub bonding_curve: Pubkey,
    pub timestamp: i64,
}

impl CompleteEvent {
    pub fn try_from_compiled_instruction(
        ui_compiled_instruction: &UiCompiledInstruction,
    ) -> Option<CompleteEvent> {
        let data = bs58::decode(ui_compiled_instruction.data.clone())
            .into_vec()
            .unwrap();
        if data.len() > 16 && data[8..16].eq(&PUMPFUN_COMPLETE_EVENT) {
            match CompleteEvent::try_from_slice(&data[16..]) {
                Ok(event) => return Some(event),
                Err(_) => return None,
            }
        } else {
            return None;
        }
    }
}

#[derive(Debug, BorshSerialize, Clone, BorshDeserialize)]
pub struct BuyArgs {
    pub amount: u64,
    pub max_sol_cost: u64,
}

#[derive(Debug, BorshSerialize, Clone, BorshDeserialize, Copy)]
pub struct TradeEvent {
    pub mint: Pubkey,
    pub sol_amount: u64,
    pub token_amount: u64,
    pub is_buy: bool,
    pub user: Pubkey,
    pub timestamp: i64,
    pub virtual_sol_reserves: u64,
    pub virtual_token_reserves: u64,
    pub real_sol_reserves: u64,
    pub real_token_reserves: u64,
}

impl TradeEvent {
    pub fn try_from_compiled_instruction(
        ui_compiled_instruction: &UiCompiledInstruction,
    ) -> Option<TradeEvent> {
        let data = bs58::decode(ui_compiled_instruction.data.clone())
            .into_vec()
            .unwrap();
        if data.len() > 16 && data[8..16].eq(&PUMPFUN_TRADE_EVENT) {
            match TradeEvent::try_from_slice(&data[16..]) {
                Ok(event) => return Some(event),
                Err(_) => return None,
            }
        } else {
            return None;
        }
    }
}

pub fn process_tx_with_meta(
    target: Pubkey,
    tx_with_meta: EncodedTransactionWithStatusMeta,
) -> Option<f32> {
    let meta = tx_with_meta.meta.unwrap();
    if let OptionSerializer::Some(inner_ixs) = meta.inner_instructions {
        process_ixs(target, inner_ixs)
    } else {
        None
    }
}

fn process_ixs(target: Pubkey, inner_ixs: Vec<UiInnerInstructions>) -> Option<f32> {
    for inner in inner_ixs {
        for ix in inner.instructions {
            if let Ok(target_event) = TargetEvent::try_from(ix) {
                match target_event {
                    TargetEvent::PumpfunBuy(buy_event) => {
                        // println!("buy_event {:?} \n ", buy_event);

                        let price = cal_pumpfun_price(
                            buy_event.virtual_sol_reserves,
                            buy_event.virtual_token_reserves,
                        );
                        if buy_event.mint.eq(&target) {
                        println!("buy_event {:?} \n ", buy_event);

                            return Some(price);
                        } else {
                            return None;
                        }
                    }
                    TargetEvent::PumpfunSell(sell_event) => {
                        return None;
                    }
                    _ => {
                        return None;
                    }
                }
            }
        }
    }
    None
}

pub fn cal_pumpfun_price(virtual_sol_reserves: u64, virtual_token_reserves: u64) -> f32 {
    (virtual_sol_reserves as f32 / 10f32.powi(9)) / (virtual_token_reserves as f32 / 10f32.powi(6))
}

/// Represents a bonding curve for token pricing and liquidity management
#[derive(Debug, Clone, BorshSerialize, BorshDeserialize)]
pub struct BondingCurveAccount {
    /// Unique identifier for the bonding curve
    pub discriminator: u64,
    /// Virtual token reserves used for price calculations
    pub virtual_token_reserves: u64,
    /// Virtual SOL reserves used for price calculations
    pub virtual_sol_reserves: u64,
    /// Actual token reserves available for trading
    pub real_token_reserves: u64,
    /// Actual SOL reserves available for trading
    pub real_sol_reserves: u64,
    /// Total supply of tokens
    pub token_total_supply: u64,
    /// Whether the bonding curve is complete/finalized
    pub complete: bool,
}


/// Gets the Program Derived Address (PDA) for a token's bonding curve account
///
/// # Arguments 
/// 
/// * `mint` - Public key of the token mint
/// 
/// # Returns 
///
/// Returns Some(PDA) if derivation succeeds, or None if it fails
pub const BONDING_CURVE_SEED: &[u8] = b"bonding-curve";    
pub fn get_bonding_curve_pda(mint: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 2] = &[BONDING_CURVE_SEED, mint.as_ref()]; 
    let program_id: &Pubkey = &PUMPFUN_PROGRAM_ID;  
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);   
    pda.map(|pubkey| pubkey.0)   
}

pub async fn get_pumpfun_reserve(rpc: &RpcClient, target: Pubkey) -> Option<BondingCurveAccount> {
    let pda = get_bonding_curve_pda(&target)?;
               
    let account_data = rpc.get_account(&pda).await.expect("Failed to get account");

        match BondingCurveAccount::try_from_slice(&account_data.data) {
        Ok(bonding_curve_account) => {
            Some(bonding_curve_account)
        } 
        Err(_e) => {  
            None
        }
    }
}


#[tokio::test]
async fn test_get_pumpfun_reserve() {
    let rpc = "https://solana-rpc.publicnode.com";
    let rpc_client = RpcClient::new(rpc.to_string());
    let target = Pubkey::from_str("aYQoMtHaLqpXgDM5TD39ii6Fb8u4AoXKF4EhXBhpump").unwrap();
    get_pumpfun_reserve(&rpc_client, target).await;
}


pub fn buy_amount_out_ix(
    mint: &Pubkey,
    bonding_curve: &Pubkey,
    associated_bonding_curve: &Pubkey,
    wallet: &Pubkey,
    associated_token_account: &Pubkey,
    amount_out: u64,
    max_amount_in_sol: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(PUMPFUN_GLOBAL, false),
        AccountMeta::new(PUMPFUN_FEE_RECIPIENT, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*bonding_curve, false),
        AccountMeta::new(*associated_bonding_curve, false),
        AccountMeta::new(*associated_token_account, false),
        AccountMeta::new(*wallet, true),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(SYSTEM_RENT_PROGRAM_ID, false),
        AccountMeta::new_readonly(EVENT_AUTHORITY, false),
        AccountMeta::new_readonly(PUMPFUN_PROGRAM_ID, false),
    ];

    let mut data: Vec<u8> = Vec::new();
    data.extend_from_slice(&[0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea]);
    data.extend_from_slice(&amount_out.to_le_bytes());
    data.extend_from_slice(&max_amount_in_sol.to_le_bytes());

    Instruction {
        program_id: PUMPFUN_PROGRAM_ID,
        accounts,
        data,
    }
}

pub fn sell_amount_in_ix(
    mint: &Pubkey,
    bonding_curve: &Pubkey,
    associated_bonding_curve: &Pubkey,
    wallet: &Pubkey,
    associated_token_account: &Pubkey,
    amount_in: u64,
    min_amount_out_sol: u64,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(PUMPFUN_GLOBAL, false),
        AccountMeta::new(PUMPFUN_FEE_RECIPIENT, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*bonding_curve, false),
        AccountMeta::new(*associated_bonding_curve, false),
        AccountMeta::new(*associated_token_account, false),
        AccountMeta::new(*wallet, true),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new_readonly(ASSOC_TOKEN_ACC_PROGRAM_ID, false),
        AccountMeta::new_readonly(TOKEN_PROGRAM_ID, false),
        AccountMeta::new_readonly(EVENT_AUTHORITY, false),
        AccountMeta::new_readonly(PUMPFUN_PROGRAM_ID, false),
    ];

    let mut data: Vec<u8> = Vec::new();
    // 33e685a4017f83ad
    data.extend_from_slice(&[0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad]);
    data.extend_from_slice(&amount_in.to_le_bytes());
    data.extend_from_slice(&min_amount_out_sol.to_le_bytes());

    Instruction {
        program_id: PUMPFUN_PROGRAM_ID,
        accounts,
        data,
    }
}

pub fn create_sell_transaction(
    bonding_curve: &Pubkey,
    price: f32,
    mint: &Pubkey,
    keypair: &Keypair,
    amount_in_token: u64,
    slippage_bps: f64,
    recent_block_hash: Hash,
) -> Result<Transaction> {
    let owner = keypair.pubkey();

    let amount_out_sol = (amount_in_token as f32 * price * 1000.0) as u64;
    let min_amount_out_sol = (amount_out_sol as f64 * (10000.0 - slippage_bps) / 10000.0) as u64;

    let mut ixs: Vec<Instruction> = Vec::new();

    let modify_compute_units = ComputeBudgetInstruction::set_compute_unit_limit(UNIT_LIMIT);
    let add_priority_fee = ComputeBudgetInstruction::set_compute_unit_price(UNIT_PRICE);
    ixs.insert(0, modify_compute_units);
    ixs.insert(1, add_priority_fee);

    let token_ata = get_associated_token_address_with_program_id(&owner, mint, &spl_token::id());

    ixs.push(sell_amount_in_ix(
        mint,
        &bonding_curve,
        &get_associated_token_address(bonding_curve, mint),
        &owner,
        &token_ata,
        amount_in_token,
        min_amount_out_sol,
    ));

    ixs.push(close_account(
        &spl_token::id(),
        &token_ata,
        &owner,
        &owner,
        &[],
    )?);

    let tx = Transaction::new_signed_with_payer(
        &ixs.to_vec(),
        Some(&owner),
        &[&keypair],
        recent_block_hash,
    );

    Ok(tx)
}

pub fn find_bonding_curve(mint: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(
        &["bonding-curve".as_bytes(), mint.as_ref()],
        &PUMPFUN_PROGRAM_ID,
    )
    .0
}

pub fn transfer_tx(
    from: &Pubkey,
    to: &Pubkey,
    keypair: &Keypair,
    lamports: u64,
    blockhash: Hash,
) -> Transaction {
    let ix = system_instruction::transfer(from, to, lamports);
    Transaction::new_signed_with_payer(&[ix], Some(from), &[&keypair], blockhash)
}

pub async fn send_bundle(
    jito: &JitoJsonRpcSDK,
    bundle: Vec<impl SerializableTransaction>,
) -> Result<Option<String>> {
    let mut params = vec![];
    for tx in bundle {
        params.push(bs58::encode(bincode::serialize(&tx)?).into_string());
    }
    let bundle = json!(params);
    let result = match jito.send_bundle(Some(bundle), None).await {
        Ok(resp) => match resp.get("result") {
            Some(bundle_id) => Some(bundle_id.as_str().unwrap().to_string()),
            None => None,
        },
        Err(_) => None,
    };
    Ok(result)
}

//  // 构造购买交易
//  let tx1 = create_buy_transaction(
//     &bonding_curve,
//     action.price,
//     &action.mint,
//     &self.keypair,
//     config.buy_amount,
//     config.slippage_bps,
//     blockhash,
// );
// let tx2 = transfer_tx(
//     &self.keypair.pubkey(),
//     &get_tip_account()?,
//     &self.keypair,
//     config.tip,
//     blockhash,
// );
// let bundle_id = send_bundle(&self.jito, vec![tx1, tx2]).await?;
