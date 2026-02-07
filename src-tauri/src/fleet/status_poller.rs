use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Probes a remote host's SSH port via TCP connect with a 5-second timeout.
/// Returns `true` if the connection succeeds, `false` otherwise.
pub async fn check_node_status(host: &str, port: u16) -> bool {
    let addr = format!("{}:{}", host, port);
    let connect_future = TcpStream::connect(&addr);
    match timeout(Duration::from_secs(5), connect_future).await {
        Ok(Ok(_stream)) => true,
        _ => false,
    }
}

/// Checks connectivity for a batch of nodes concurrently.
///
/// Takes a vec of `(node_id, host, port)` tuples and returns a vec of
/// `(node_id, online)` results. Each check runs as a separate Tokio task
/// so all probes happen in parallel.
pub async fn poll_node_statuses(nodes: Vec<(String, String, u16)>) -> Vec<(String, bool)> {
    let mut handles = Vec::with_capacity(nodes.len());

    for (node_id, host, port) in nodes {
        let handle = tokio::spawn(async move {
            let online = check_node_status(&host, port).await;
            (node_id, online)
        });
        handles.push(handle);
    }

    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        match handle.await {
            Ok(result) => results.push(result),
            Err(_join_err) => {
                // Task panicked — should not happen, but handle gracefully.
            }
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Checking localhost:22 should return a bool regardless of whether
    /// an SSH server is actually running. We just verify the function
    /// completes without panicking.
    #[tokio::test]
    async fn check_localhost_ssh_returns_bool() {
        let result = check_node_status("127.0.0.1", 22).await;
        // result is either true (sshd running) or false (not running);
        // both are valid — we only assert the type.
        let _: bool = result;
    }

    /// A non-routable IP (TEST-NET per RFC 5737) should time out or
    /// refuse quickly, returning false.
    #[tokio::test]
    async fn check_unreachable_host_returns_false() {
        let result = check_node_status("192.0.2.1", 22).await;
        assert!(!result, "non-routable address must return false");
    }

    /// Polling an empty list should return an empty results vec immediately.
    #[tokio::test]
    async fn poll_empty_input_returns_empty() {
        let results = poll_node_statuses(Vec::new()).await;
        assert!(results.is_empty());
    }

    /// Polling multiple nodes returns one result per input entry, and
    /// each result carries the correct node_id back.
    #[tokio::test]
    async fn poll_preserves_node_ids() {
        let nodes = vec![
            ("node-a".to_string(), "192.0.2.1".to_string(), 22),
            ("node-b".to_string(), "192.0.2.2".to_string(), 22),
        ];
        let results = poll_node_statuses(nodes).await;
        assert_eq!(results.len(), 2);

        let ids: Vec<&str> = results.iter().map(|(id, _)| id.as_str()).collect();
        assert!(ids.contains(&"node-a"));
        assert!(ids.contains(&"node-b"));
    }
}
