use anyhow::{anyhow, Result};
use iroh::endpoint::SendStream;
use iroh::{Endpoint, NodeAddr};
use iroh::protocol::AcceptError;
use serde::{Deserialize, Serialize};

use crate::bucket_log::BucketLogProvider;
use crate::peer::Peer;

use super::ALPN;

/// Generic trait for handling bidirectional stream protocols
///
/// This trait eliminates boilerplate by providing default implementations
/// for all the serialization, stream I/O, and error handling logic.
///
/// Implementors only need to define the business logic:
/// - `handle_request`: What to send back when receiving a request (responder side)
/// - `handle_response`: What to do when receiving a response (initiator side)
pub trait BidirectionalHandler: Sized {
    /// The request message type
    type Request: Serialize + for<'de> Deserialize<'de>;

    /// The response message type
    type Response: Serialize + for<'de> Deserialize<'de>;

    /// Handle an incoming request and generate a response
    ///
    /// **Responder side:** Called when a request is received.
    ///
    /// Implement only the business logic - serialization and I/O are handled automatically.
    /// This is where you decide what to send back based on the request and peer state.
    fn handle_request<L: BucketLogProvider>(
        peer: &Peer<L>,
        req: &Self::Request,
    ) -> impl std::future::Future<Output = Self::Response> + Send;

    /// Side effects after sending a response
    ///
    /// **Responder side:** Called after the response has been sent to the peer.
    ///
    /// This is where you can trigger background operations, spawn sync tasks,
    /// log metrics, etc. The response has already been sent, so this won't
    /// block the peer from receiving it.
    ///
    /// Default implementation does nothing.
    fn after_response_sent<L: BucketLogProvider>(
        _peer: &Peer<L>,
        _req: &Self::Request,
        _resp: &Self::Response,
    ) -> impl std::future::Future<Output = Result<()>> + Send {
        async { Ok(()) }
    }

    /// Handle an incoming response and take action
    ///
    /// **Initiator side:** Called when a response is received.
    ///
    /// Implement only the business logic - deserialization is handled automatically.
    /// This is where you decide what to do based on the peer's response.
    fn handle_response<L: BucketLogProvider>(
        peer: &Peer<L>,
        resp: &Self::Response,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Process an incoming request on the responder peer
    ///
    /// This is a provided method that handles all the boilerplate:
    /// - Calls the handler function
    /// - Serializes the response
    /// - Writes to the stream
    /// - Finishes the stream
    /// - Calls after_response_sent hook for side effects
    /// - Error handling
    async fn process_peer_request<L>(
        peer: &Peer<L>,
        request: Self::Request,
        mut send: SendStream,
    ) -> Result<(), AcceptError>
    where
        L: BucketLogProvider,
    {
        // Call the handler to get the response
        let response = Self::handle_request(peer, &request).await;

        // Serialize the response
        let reply_bytes = bincode::serialize(&response).map_err(|e| {
            tracing::error!("Failed to serialize reply: {}", e);
            let err: Box<dyn std::error::Error + Send + Sync> =
                anyhow!("failed to serialize reply: {}", e).into();
            AcceptError::from(err)
        })?;

        // Write the response to the stream
        send.write_all(&reply_bytes).await.map_err(|e| {
            tracing::error!("failed to send reply: {}", e);
            AcceptError::from(std::io::Error::other(e))
        })?;

        // Finish the stream
        send.finish().map_err(|e| {
            tracing::error!("failed to finish stream: {}", e);
            AcceptError::from(std::io::Error::other(e))
        })?;

        // Call the after_response_sent hook for side effects
        // This happens after the response is sent, so it won't block the peer
        if let Err(e) = Self::after_response_sent(peer, &request, &response).await {
            tracing::error!("Error in after_response_sent hook: {}", e);
            // Don't fail the whole request if the side effect fails
        }

        Ok(())
    }

    /// Send a request to a peer and return the response
    ///
    /// This is a provided method that handles all the boilerplate:
    /// - Connects to the peer
    /// - Opens a bidirectional stream
    /// - Serializes and sends the request
    /// - Receives and deserializes the response
    /// - Returns the response for the caller to handle
    /// - Error handling
    ///
    /// If you want automatic response handling, call `handle_response` on the result.
    async fn send_to_peer<L>(
        endpoint: &Endpoint,
        peer_addr: &NodeAddr,
        request: Self::Request,
    ) -> Result<Self::Response>
    where
        L: BucketLogProvider,
    {
        // Connect to the peer
        let conn = endpoint
            .connect(peer_addr.node_id, ALPN)
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to connect to peer {}: {}",
                    peer_addr.node_id,
                    e
                );
                anyhow!("Failed to connect to peer: {}", e)
            })?;

        // Open a bidirectional stream
        let (mut send, mut recv) = conn
            .open_bi()
            .await
            .map_err(|e| {
                tracing::error!("Failed to open bidirectional stream: {}", e);
                anyhow!("Failed to open bidirectional stream: {}", e)
            })?;

        // Serialize the request
        let request_bytes = bincode::serialize(&request)
            .map_err(|e| anyhow!("Failed to serialize request: {}", e))?;

        // Send the request
        send.write_all(&request_bytes)
            .await
            .map_err(|e| anyhow!("Failed to write request: {}", e))?;

        send.finish()
            .map_err(|e| anyhow!("Failed to finish sending request: {}", e))?;

        // Read the response
        let response_bytes = recv
            .read_to_end(1024 * 1024)
            .await
            .map_err(|e| anyhow!("Failed to read response: {}", e))?;

        // Deserialize the response
        let response: Self::Response = bincode::deserialize(&response_bytes)
            .map_err(|e| anyhow!("Failed to deserialize response: {}", e))?;

        Ok(response)
    }

    /// Send a request to a peer and automatically handle the response
    ///
    /// This is a convenience method that combines `send_to_peer` and `handle_response`.
    async fn send_and_handle<L>(
        peer: &Peer<L>,
        endpoint: &Endpoint,
        peer_addr: &NodeAddr,
        request: Self::Request,
    ) -> Result<()>
    where
        L: BucketLogProvider,
    {
        let response = Self::send_to_peer::<L>(endpoint, peer_addr, request).await?;
        Self::handle_response(peer, &response).await
    }
}
