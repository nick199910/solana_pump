use anyhow::{anyhow, Ok, Result};
use futures_util::Stream;
use solana_program::pubkey;
use solana_sdk::pubkey::Pubkey;
use std::env;
use std::{collections::HashMap, time::Duration};
use yellowstone_grpc_client::{ClientTlsConfig, GeyserGrpcBuilder, GeyserGrpcClient, Interceptor};
use yellowstone_grpc_proto::{
    geyser::{
        geyser_client::GeyserClient, CommitmentLevel, SubscribeRequest,
        SubscribeRequestFilterAccounts, SubscribeRequestFilterBlocks,
        SubscribeRequestFilterBlocksMeta, SubscribeRequestFilterTransactions, SubscribeUpdate,
    },
    tonic::Status,
};
/// grpc需要的交易过滤map
type TransactionsFilterMap = HashMap<String, SubscribeRequestFilterTransactions>;
pub const PUMPFUN_PROGRAM_ID: Pubkey = pubkey!("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
pub struct GrpcClient {
    endpoint: String,
}

impl GrpcClient {
    pub fn new(endpoint: String) -> Self {
        Self { endpoint }
    }

    pub async fn get_client(&self) -> Result<GeyserGrpcClient<impl Interceptor>> {
        GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .connect()
            .await
            .map_err(|e| anyhow!("{:?}", e.to_string()))
    }

    /// 订阅指定地址的账户信息更新
    pub async fn subscribe_transaction(
        &self,
        account_include: Vec<String>,  // 包含在内的地址相关交易都会收到
        account_exclude: Vec<String>,  // 不包含这些地址的相关交易都会收到
        account_required: Vec<String>, // 必须要包含的地址
        commitment: CommitmentLevel,
    ) -> Result<impl Stream<Item = Result<SubscribeUpdate, Status>>> {
        // client
        let mut client = GeyserGrpcClient::build_from_shared(self.endpoint.clone())?
            .tls_config(ClientTlsConfig::new().with_native_roots())?
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .connect()
            .await?;

        // 过滤规则
        let mut transactions: TransactionsFilterMap = HashMap::new();
        transactions.insert(
            "client".to_string(),
            SubscribeRequestFilterTransactions {
                vote: None,
                failed: None,
                signature: None,
                account_include,
                account_exclude,
                account_required,
            },
        );

        // request
        let subscribe_request = SubscribeRequest {
            transactions,
            commitment: Some(commitment.into()),
            ..Default::default()
        };

        // 返回流
        let (_, stream) = client
            .subscribe_with_request(Some(subscribe_request))
            .await?;

        Ok(stream)
    }
}

pub async fn get_pumpfun_stream() -> Result<impl Stream<Item = Result<SubscribeUpdate, Status>>> {
    let mut client = GeyserGrpcClient::build_from_shared(env::var("GRPC_URL")?)?
        .tls_config(ClientTlsConfig::new().with_native_roots())?
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(60))
        .connect()
        .await
        .map_err(|e| anyhow!("{:?}", e.to_string()))?;

    // 过滤规则

    let mut transactions: TransactionsFilterMap = HashMap::new();
    transactions.insert(
        "client".to_string(),
        SubscribeRequestFilterTransactions {
            vote: None,
            failed: None,
            signature: None,
            account_include: vec![],
            account_exclude: vec![],
            account_required: vec![PUMPFUN_PROGRAM_ID.to_string()],
        },
    );

    // request
    let subscribe_request = SubscribeRequest {
        transactions,
        commitment: Some(CommitmentLevel::Processed.into()),
        ..Default::default()
    };

    // 返回流
    let (_, stream) = client
        .subscribe_with_request(Some(subscribe_request))
        .await?;
    Ok(stream)
}
