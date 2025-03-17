// use std::env;

// use once_cell::sync::Lazy;
// use solana_program::pubkey;
// use solana_sdk::pubkey::Pubkey;

// pub static GRPC: Lazy<String> = Lazy::new(|| env::var("GRPC_URL").unwrap());
// pub static RPC: Lazy<String> = Lazy::new(|| env::var("RPC_URL").unwrap());
// pub static WS: Lazy<String> = Lazy::new(|| env::var("WS_URL").unwrap());
// pub static JITO: Lazy<String> = Lazy::new(|| env::var("JITO_BLOCK_ENGINE_URL").unwrap());
// pub static UNIT_PRICE: Lazy<u64> = Lazy::new(|| {
//     env::var("UNIT_PRICE")
//         .unwrap()
//         .parse::<u64>()
//         .unwrap_or(20000)
// });

// pub static UNIT_LIMIT: Lazy<u32> = Lazy::new(|| {
//     env::var("UNIT_LIMIT")
//         .unwrap()
//         .parse::<u32>()
//         .unwrap_or(2000000)
// });
// pub static PRIVATE_KEY: Lazy<String> = Lazy::new(|| env::var("PRIVATE_KEY").unwrap());
// pub static REDIS_URL: Lazy<String> = Lazy::new(|| env::var("REDIS_URL").unwrap());
// pub static PRICE_INCREASE_BUY_LOWER: Lazy<f32> = Lazy::new(|| {
//     env::var("PRICE_INCREASE_BUY_LOWER")
//         .unwrap()
//         .parse::<f32>()
//         .unwrap_or(0.1)
// });
// pub static PRICE_INCREASE_BUY_UPPER: Lazy<f32> = Lazy::new(|| {
//     env::var("PRICE_INCREASE_BUY_UPPER")
//         .unwrap()
//         .parse::<f32>()
//         .unwrap_or(0.2)
// });

// // program相关
// pub const SYSTEM_PROGRAM_ID: Pubkey = pubkey!("11111111111111111111111111111111");
// pub const SYSTEM_RENT_PROGRAM_ID: Pubkey = pubkey!("SysvarRent111111111111111111111111111111111");
// pub const TOKEN_PROGRAM_ID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
// pub const ASSOC_TOKEN_ACC_PROGRAM_ID: Pubkey =
//     pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
// pub const EVENT_AUTHORITY: Pubkey = pubkey!("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1");
// pub const KEY_PREFIX: &'static str = "token:info:";

// // pumpfun
// pub const PUMPFUN_PROGRAM_ID: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
// pub const PUMPFUN_GLOBAL: Pubkey = pubkey!("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf");
// pub const PUMPFUN_FEE_RECIPIENT: Pubkey = pubkey!("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM");
// pub const INIT_SOL_REVERSES: u64 = 30_000_000_000;
// pub const INIT_TOKEN_REVERSES: u64 = 1_073_000_191_000_000;
// pub const INIT_PRICE: f32 = (INIT_SOL_REVERSES as f32 / 1e9) / (INIT_TOKEN_REVERSES as f32 / 1e6);
// pub const PUMPFUN_TOTAL_SUPPLY: u64 = 1_000_000_000_000_000;

// // 标量
// pub const MINUTES: u64 = 60 * 1000;
// pub const SECONDS: u64 = 1000;
