//! One Rustphin per session (W-110a, D-031).
//!
//! The running Rustphin owns `org.rustphin.Rustphin` on the session bus. A
//! later launch hands its request to it and exits, without starting the UI
//! toolkit, so a window opens at once whatever the font cache holds. If the
//! running one does not answer in time, the launch starts its own process.
//!
//! Folders travel as bytes: file names need not be UTF-8.

use std::{
    ffi::OsString,
    os::unix::ffi::{OsStrExt as _, OsStringExt as _},
    path::PathBuf,
    sync::mpsc,
    thread,
    time::Duration,
};

use zbus::{fdo::RequestNameFlags, fdo::RequestNameReply, interface};

/// The bus name the running Rustphin owns.
pub const NAME: &str = "org.rustphin.Rustphin";
const PATH: &str = "/org/rustphin/Rustphin";

/// How long a launch waits for the running Rustphin before starting its own.
pub const ANSWER_TIME: Duration = Duration::from_secs(2);

/// What a launch asks the running Rustphin for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// Open a window, or these folders where the settings say. `activation`
    /// is the launch's Wayland activation token, so the window may take the
    /// focus; empty when there was none.
    Open {
        /// Folders named on the command line; empty for a plain launch.
        folders: Vec<PathBuf>,
        /// `XDG_ACTIVATION_TOKEN` of the launch, if any.
        activation: Option<String>,
    },
    /// End the process: every window closes and Rustphin exits.
    Quit,
}

struct Service {
    requests: flume::Sender<Request>,
}

#[interface(name = "org.rustphin.Rustphin")]
impl Service {
    /// Opens a window, or the folders given (as path bytes).
    fn open(&self, folders: Vec<Vec<u8>>, activation_token: String) {
        let folders = folders
            .into_iter()
            .filter(|bytes| !bytes.is_empty())
            .map(|bytes| PathBuf::from(OsString::from_vec(bytes)))
            .collect();
        let activation = Some(activation_token).filter(|token| !token.is_empty());
        let _ = self.requests.send(Request::Open {
            folders,
            activation,
        });
    }

    /// Ends the running Rustphin.
    fn quit(&self) {
        let _ = self.requests.send(Request::Quit);
    }
}

/// Why this process is not the running Rustphin.
#[derive(Debug, thiserror::Error)]
pub enum ClaimError {
    /// Another process owns the name: hand the request to it.
    #[error("another Rustphin is running")]
    Taken,
    /// The session bus could not be used.
    #[error("the session bus is not available: {0}")]
    Bus(#[from] zbus::Error),
}

/// This process as the running Rustphin, and the requests it receives.
/// Dropping it gives the name up.
pub struct Instance {
    requests: flume::Receiver<Request>,
    _connection: zbus::blocking::Connection,
}

impl std::fmt::Debug for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Instance").field("name", &NAME).finish()
    }
}

impl Instance {
    /// Claims the name on the session bus, unless another process has it.
    pub fn claim() -> Result<Self, ClaimError> {
        Self::claim_as(NAME)
    }

    /// Claims `name` (the Qt interface uses its own until it replaces the
    /// GPUI one, W-114c3), unless another process has it.
    pub fn claim_as(name: &'static str) -> Result<Self, ClaimError> {
        let (requests, incoming) = flume::unbounded();
        let connection = zbus::blocking::connection::Builder::session()?
            .serve_at(PATH, Service { requests })?
            .build()?;
        Self::claim_on(connection, name, incoming)
    }

    fn claim_on(
        connection: zbus::blocking::Connection,
        name: &'static str,
        requests: flume::Receiver<Request>,
    ) -> Result<Self, ClaimError> {
        let reply =
            match connection.request_name_with_flags(name, RequestNameFlags::DoNotQueue.into()) {
                Ok(reply) => reply,
                // zbus reports an owned name with `DoNotQueue` as this error.
                Err(zbus::Error::NameTaken) => return Err(ClaimError::Taken),
                Err(error) => return Err(error.into()),
            };
        match reply {
            RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner => Ok(Self {
                requests,
                _connection: connection,
            }),
            RequestNameReply::Exists | RequestNameReply::InQueue => Err(ClaimError::Taken),
        }
    }

    /// The requests, for a task to wait on.
    pub fn receiver(&self) -> flume::Receiver<Request> {
        self.requests.clone()
    }
}

/// What became of a request handed to the running Rustphin.
#[derive(Debug, PartialEq, Eq)]
pub enum HandOver {
    /// The running Rustphin took it.
    Taken,
    /// No Rustphin is running.
    NoneRunning,
    /// It did not answer in time, or the bus failed; the reason is logged
    /// by the caller.
    NoAnswer(String),
}

/// Hands `request` to the running Rustphin on the session bus, waiting at
/// most `wait` for its answer.
pub fn hand_over(request: &Request, wait: Duration) -> HandOver {
    hand_over_to(NAME, request, wait)
}

/// [`hand_over`] to the Rustphin owning `name`.
pub fn hand_over_to(name: &'static str, request: &Request, wait: Duration) -> HandOver {
    hand_over_with(
        || zbus::blocking::Connection::session().map_err(|error| error.to_string()),
        name,
        request,
        wait,
        true,
    )
}

/// [`hand_over`] over the connection `connect` makes.
fn hand_over_with(
    connect: impl FnOnce() -> Result<zbus::blocking::Connection, String> + Send + 'static,
    name: &'static str,
    request: &Request,
    wait: Duration,
    check_owner: bool,
) -> HandOver {
    let request = request.clone();
    let (answer, answered) = mpsc::channel();
    // The call runs on its own thread so a running Rustphin that hangs
    // cannot hold this launch beyond `wait`.
    let spawned = thread::Builder::new()
        .name("hand-over".into())
        .spawn(move || {
            let result =
                connect().and_then(|connection| send(&connection, name, &request, check_owner));
            let _ = answer.send(result);
        });
    if let Err(error) = spawned {
        return HandOver::NoAnswer(error.to_string());
    }
    match answered.recv_timeout(wait) {
        Ok(Ok(true)) => HandOver::Taken,
        Ok(Ok(false)) => HandOver::NoneRunning,
        Ok(Err(reason)) => HandOver::NoAnswer(reason),
        Err(_) => HandOver::NoAnswer(format!("no answer within {} ms", wait.as_millis())),
    }
}

/// Sends `request` over `connection` to `name`'s owner; `false` when nobody
/// owns it (checked first when `check_owner`, as on the session bus).
fn send(
    connection: &zbus::blocking::Connection,
    name: &'static str,
    request: &Request,
    check_owner: bool,
) -> Result<bool, String> {
    if check_owner {
        let bus = zbus::blocking::fdo::DBusProxy::new(connection).map_err(|e| e.to_string())?;
        let owner = zbus::names::BusName::try_from(name).map_err(|e| e.to_string())?;
        if !bus.name_has_owner(owner).map_err(|e| e.to_string())? {
            return Ok(false);
        }
    }
    let proxy = zbus::blocking::Proxy::new(connection, name, PATH, NAME)
        .map_err(|error| error.to_string())?;
    match request {
        Request::Open {
            folders,
            activation,
        } => {
            let folders: Vec<Vec<u8>> = folders
                .iter()
                .map(|folder| folder.as_os_str().as_bytes().to_vec())
                .collect();
            let token = activation.clone().unwrap_or_default();
            proxy
                .call::<_, _, ()>("Open", &(folders, token))
                .map_err(|error| error.to_string())?;
        }
        Request::Quit => {
            proxy
                .call::<_, _, ()>("Quit", &())
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(true)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::os::unix::net::UnixStream;

    use zbus::blocking::connection::Builder;

    use super::*;

    /// A service and a client joined by a private connection.
    fn pair() -> (
        zbus::blocking::Connection,
        zbus::blocking::Connection,
        flume::Receiver<Request>,
    ) {
        let (requests, incoming) = flume::unbounded();
        let (server_stream, client_stream) = UnixStream::pair().unwrap();
        let guid = zbus::Guid::generate();
        let server = thread::spawn(move || {
            Builder::async_io_unix_stream(server_stream)
                .server(guid)
                .unwrap()
                .p2p()
                .serve_at(PATH, Service { requests })
                .unwrap()
                .build()
                .unwrap()
        });
        let client = Builder::async_io_unix_stream(client_stream)
            .p2p()
            .build()
            .unwrap();
        (server.join().unwrap(), client, incoming)
    }

    #[test]
    fn folders_reach_the_running_rustphin_as_bytes_with_the_token() {
        let (_server, client, incoming) = pair();
        let odd = PathBuf::from(OsString::from_vec(b"/tmp/a\xffb".to_vec()));
        let request = Request::Open {
            folders: vec![PathBuf::from("/home/someone/Documents"), odd],
            activation: Some("token-1".into()),
        };
        assert_eq!(send(&client, NAME, &request, false), Ok(true));
        assert_eq!(
            incoming.recv_timeout(Duration::from_secs(2)).unwrap(),
            request
        );
    }

    #[test]
    fn a_plain_launch_and_a_quit_arrive_as_asked() {
        let (_server, client, incoming) = pair();
        let open = Request::Open {
            folders: Vec::new(),
            activation: None,
        };
        send(&client, NAME, &open, false).unwrap();
        send(&client, NAME, &Request::Quit, false).unwrap();
        assert_eq!(incoming.recv_timeout(Duration::from_secs(2)).unwrap(), open);
        assert_eq!(
            incoming.recv_timeout(Duration::from_secs(2)).unwrap(),
            Request::Quit
        );
    }

    #[test]
    fn a_hand_over_reaches_the_running_rustphin() {
        let (server, client, incoming) = pair();
        let answer = hand_over_with(move || Ok(client), NAME, &Request::Quit, ANSWER_TIME, false);
        assert_eq!(answer, HandOver::Taken);
        assert_eq!(
            incoming.recv_timeout(Duration::from_secs(2)).unwrap(),
            Request::Quit
        );
        drop(server);
    }

    #[test]
    fn a_rustphin_that_never_answers_is_given_up_on_in_time() {
        // A connection that never completes stands in for a hung Rustphin.
        let (_keep, never) = mpsc::channel::<()>();
        let started = std::time::Instant::now();
        let answer = hand_over_with(
            move || {
                let _ = never.recv();
                Err("unreachable".into())
            },
            NAME,
            &Request::Quit,
            Duration::from_millis(200),
            false,
        );
        assert!(matches!(answer, HandOver::NoAnswer(_)));
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_bus_that_fails_says_why() {
        let answer = hand_over_with(
            || Err("no session bus".into()),
            NAME,
            &Request::Quit,
            ANSWER_TIME,
            false,
        );
        assert_eq!(answer, HandOver::NoAnswer("no session bus".into()));
    }
}
