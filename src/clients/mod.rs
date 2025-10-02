//! Contains implementations of web push clients.
//!
//! [`request_builder`] contains the functions used to send and consume push http messages.
//! This module should be consumed by each client, by using [`http`]'s flexible api.

use async_trait::async_trait;

use crate::{WebPushError, WebPushMessage};

pub mod request_builder;

#[cfg(feature = "hyper-client")]
pub mod hyper_client;

const MAX_RESPONSE_SIZE: usize = 64 * 1024;

/// An async client for sending the notification payload.
/// Other features, such as thread safety, may vary by implementation.
#[async_trait]
pub trait WebPushClient {
    /// Sends a notification. Never times out.
    async fn send(&self, message: WebPushMessage) -> Result<(), WebPushError>;
}
