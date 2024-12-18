use std::sync::Arc;

use alloy::{
    primitives::B256,
    providers::Provider,
    pubsub::PubSubFrontend,
    rpc::types::{
        eth::{Block, Filter, Log},
        Header,
    },
};
use async_trait::async_trait;
use futures::StreamExt;

use crate::types::{Collector, CollectorStream};

pub struct LogsInBlockCollector {
    provider: Arc<dyn Provider<PubSubFrontend>>,
    filter: Filter,
}

impl LogsInBlockCollector {
    pub fn new(provider: Arc<dyn Provider<PubSubFrontend>>, filter: Filter) -> Self {
        Self { provider, filter }
    }

    async fn block_to_logs(&self, block_hash: B256) -> Option<Vec<Log>> {
        let logs = self
            .provider
            .get_logs(&self.filter.clone().at_block_hash(block_hash))
            .await;

        match logs {
            Ok(logs) => Some(logs),
            Err(e) => {
                tracing::error!(?block_hash, "fail to get logs: {e:#}");
                None
            }
        }
    }
}

#[async_trait]
impl Collector<(Header, Vec<Log>)> for LogsInBlockCollector {
    async fn get_event_stream(&self) -> eyre::Result<CollectorStream<'_, (Header, Vec<Log>)>> {
        let mut stream = self.provider.subscribe_blocks().await?.into_stream();

        let stream = async_stream::stream! {
            while let Some(header) = stream.next().await {
                let block_hash = header.hash;

                let logs = match self.block_to_logs(block_hash).await {
                    Some(logs) => logs,
                    None => continue,
                };

                yield (header, logs);
            }
        };

        Ok(Box::pin(stream))
    }
}
