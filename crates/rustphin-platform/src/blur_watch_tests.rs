//! Real-wire capability transitions and cancellation, without a desktop.

// Fixture construction and assertions deliberately panic on setup failure.
#![allow(clippy::unwrap_used)]

use std::{
    os::unix::net::UnixListener,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    time::Duration,
};

use super::*;
use crate::blur::protocol_tests::Server;

const DEADLINE: Duration = Duration::from_secs(3);

fn accept(listener: &UnixListener) -> UnixStream {
    let mut fds = [PollFd::new(listener, PollFlags::IN)];
    poll(
        &mut fds,
        Some(&rustix::event::Timespec::try_from(DEADLINE).unwrap()),
    )
    .unwrap();
    assert!(
        fds[0].revents().contains(PollFlags::IN),
        "monitor did not connect"
    );
    listener.accept().unwrap().0
}

fn watch(connection: Connection) -> BlurWatch {
    let (stop, cancelled) = UnixStream::pair().unwrap();
    let (sender, changes) = flume::unbounded();
    let worker = thread::spawn(move || {
        if let Err(error) = monitor_connection(connection, &cancelled, sender.clone()) {
            let _ = sender.send(Err(error));
        }
    });
    BlurWatch {
        changes,
        stop,
        worker: Some(worker),
    }
}

fn fixture(flags: Option<u32>) -> (Server, BlurWatch, Arc<AtomicUsize>) {
    let (client, server) = UnixStream::pair().unwrap();
    let requests = Arc::new(AtomicUsize::new(0));
    let server = Server::start(server, flags, requests.clone());
    (
        server,
        watch(Connection::from_socket(client).unwrap()),
        requests,
    )
}

fn next(watch: &BlurWatch) -> Result<bool, BlurError> {
    watch.changes().recv_timeout(DEADLINE).unwrap()
}

#[test]
fn live_blur_loss_and_recovery_are_reported_without_requesting_effects() {
    let (server, watch, requests) = fixture(Some(1));
    assert_eq!(next(&watch), Ok(true));
    server.capabilities(1);
    server.capabilities(0);
    assert_eq!(next(&watch), Ok(false));
    server.capabilities(1);
    assert_eq!(next(&watch), Ok(true));
    assert_eq!(requests.load(Ordering::SeqCst), 0);
}

#[test]
fn only_the_known_blur_bit_enables_glass_when_capabilities_change() {
    let (server, watch, _) = fixture(Some(2));
    assert_eq!(next(&watch), Ok(false));
    server.capabilities(3);
    assert_eq!(next(&watch), Ok(true));
    server.capabilities(2);
    assert_eq!(next(&watch), Ok(false));
}

#[test]
fn missing_removed_and_reintroduced_managers_update_availability() {
    let (server, watch, _) = fixture(None);
    assert_eq!(next(&watch), Ok(false));
    server.add(1);
    assert_eq!(next(&watch), Ok(true));
    server.remove();
    assert_eq!(next(&watch), Ok(false));
    server.add(1);
    assert_eq!(next(&watch), Ok(true));
}

#[test]
fn compositor_disconnect_reports_an_error_then_closes_the_subscription() {
    let (server, watch, _) = fixture(Some(1));
    assert_eq!(next(&watch), Ok(true));
    drop(server);
    assert!(matches!(next(&watch), Err(BlurError::Connection(_))));
    assert_eq!(
        watch.changes().recv_timeout(DEADLINE),
        Err(flume::RecvTimeoutError::Disconnected)
    );
}

#[test]
fn cancellation_does_not_wait_for_a_silent_compositor() {
    let (client, _silent_server) = UnixStream::pair().unwrap();
    let watch = watch(Connection::from_socket(client).unwrap());
    let (done, completed) = flume::bounded(1);
    let worker = thread::spawn(move || {
        drop(watch);
        done.send(()).unwrap();
    });
    completed.recv_timeout(DEADLINE).unwrap();
    worker.join().unwrap();
}

#[test]
fn an_unnamed_toolkit_connection_fails_closed() {
    let (client, _server) = UnixStream::pair().unwrap();
    let result = Endpoint::from_socket(&client);
    assert!(
        matches!(result, Err(BlurError::Unmonitorable)),
        "unexpected result: {:?}",
        result.err()
    );
}

#[test]
fn the_monitor_reconnects_to_the_original_named_peer() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("display");
    let listener = UnixListener::bind(&path).unwrap();
    let original = UnixStream::connect(&path).unwrap();
    let (_original_server, _) = listener.accept().unwrap();
    let watch = BlurWatch::spawn(Endpoint::from_socket(&original).unwrap()).unwrap();
    let socket = accept(&listener);
    let _server = Server::start(socket, Some(1), Arc::new(AtomicUsize::new(0)));
    assert_eq!(next(&watch), Ok(true));
}

#[test]
fn an_abstract_display_socket_is_monitored_without_environment_lookup() {
    use std::os::linux::net::SocketAddrExt as _;
    use std::os::unix::net::SocketAddr;
    let directory = tempfile::tempdir().unwrap();
    let address =
        SocketAddr::from_abstract_name(directory.path().as_os_str().as_encoded_bytes()).unwrap();
    let listener = UnixListener::bind_addr(&address).unwrap();
    let original = UnixStream::connect_addr(&address).unwrap();
    let (_original_server, _) = listener.accept().unwrap();
    let watch = BlurWatch::spawn(Endpoint::from_socket(&original).unwrap()).unwrap();
    let socket = accept(&listener);
    let _server = Server::start(socket, Some(0), Arc::new(AtomicUsize::new(0)));
    assert_eq!(next(&watch), Ok(false));
}

#[test]
fn a_replaced_peer_does_not_supply_capabilities_for_the_original_display() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("display");
    let listener = UnixListener::bind(&path).unwrap();
    let original = UnixStream::connect(&path).unwrap();
    let (_original_server, _) = listener.accept().unwrap();
    let mut endpoint = Endpoint::from_socket(&original).unwrap();
    // Different credentials stand in for a display path now served by another peer.
    endpoint.credentials.uid =
        rustix::process::Uid::from_raw(endpoint.credentials.uid.as_raw() ^ 1);
    assert_ne!(socket_peercred(&original).unwrap(), endpoint.credentials);
    let watch = BlurWatch::spawn(endpoint).unwrap();
    assert_eq!(next(&watch), Err(BlurError::Unmonitorable));
}
