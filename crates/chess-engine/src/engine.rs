//! DD-EN-001: the contract `EngineGateway` uses (architecture 5.7, IFP-EN-01).
//!
//! Native async-fn-in-trait: the trait is not dyn-compatible (finding FIND-E4). Callers use
//! generics or an enum.

use std::future::Future;

use crate::error::EngineError;
use crate::types::{Health, SearchRequest, SearchResult};

pub trait Engine: Send + Sync {
    /// Returns at `bestmove`. Cancel-safe: dropping the future stops the search.
    fn search(
        &self,
        req: SearchRequest,
    ) -> impl Future<Output = Result<SearchResult, EngineError>> + Send;

    /// Queues UCI `stop` ahead of any other command. No result. No search: nothing happens.
    fn stop(&self);

    /// Answered from the driver's state, without a round trip.
    fn health(&self) -> Health;
}
