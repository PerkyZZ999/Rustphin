//! KWin's window menu for a window that draws its own title bar.
//!
//! Qt has no public call that asks the compositor for its window menu, so a
//! right-click on Rustphin's title bar asks KWin's global shortcut service
//! to run the shortcut "Window Operations Menu", as if its keys were
//! pressed. KWin then opens the menu for the active window, at the window's
//! top-left corner (the maintainer's choice over Qt Wayland's private API).

use zbus::blocking::{Connection, Proxy};

/// The bus name of KDE's global shortcut service.
pub const SERVICE: &str = "org.kde.kglobalaccel";
/// KWin's component in that service.
const PATH: &str = "/component/kwin";
const INTERFACE: &str = "org.kde.kglobalaccel.Component";
/// The name KWin gives the shortcut that opens its window menu.
pub const SHORTCUT: &str = "Window Operations Menu";

/// Why the window menu could not be asked for.
#[derive(Debug, thiserror::Error)]
pub enum WindowMenuError {
    /// The session bus or the shortcut service could not be reached, or the
    /// service refused the call.
    #[error("KWin's window menu is not available: {0}")]
    Bus(#[from] zbus::Error),
}

/// Asks KWin, over the session bus, for the active window's menu.
pub fn open() -> Result<(), WindowMenuError> {
    open_over(&Connection::session()?, SERVICE)
}

/// [`open`] over `connection`, asking the owner of `service`.
pub fn open_over(connection: &Connection, service: &str) -> Result<(), WindowMenuError> {
    let proxy = Proxy::new(connection, service.to_owned(), PATH, INTERFACE)?;
    proxy.call::<_, _, ()>("invokeShortcut", &(SHORTCUT,))?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::{os::unix::net::UnixStream, thread, time::Duration};

    use zbus::{blocking::connection::Builder, interface};

    use super::*;

    /// KWin's shortcut component, recording the shortcuts it is asked to run.
    struct Component {
        invoked: flume::Sender<String>,
    }

    #[interface(name = "org.kde.kglobalaccel.Component")]
    impl Component {
        #[zbus(name = "invokeShortcut")]
        fn invoke_shortcut(&self, name: String) {
            let _ = self.invoked.send(name);
        }
    }

    /// A client joined to a stand-in for the service at `path`.
    fn pair(path: &str) -> (Connection, Connection, flume::Receiver<String>) {
        let (invoked, incoming) = flume::unbounded();
        let (server_stream, client_stream) = UnixStream::pair().unwrap();
        let guid = zbus::Guid::generate();
        let path = path.to_owned();
        let server = thread::spawn(move || {
            Builder::async_io_unix_stream(server_stream)
                .server(guid)
                .unwrap()
                .p2p()
                .serve_at(path, Component { invoked })
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
    fn a_right_click_asks_kwin_for_its_window_operations_menu() {
        let (_server, client, incoming) = pair(PATH);
        open_over(&client, SERVICE).unwrap();
        assert_eq!(
            incoming.recv_timeout(Duration::from_secs(2)).unwrap(),
            "Window Operations Menu"
        );
    }

    #[test]
    fn a_service_without_kwin_s_component_is_an_error_not_a_hang() {
        let (_server, client, incoming) = pair("/component/plasmashell");
        assert!(matches!(
            open_over(&client, SERVICE),
            Err(WindowMenuError::Bus(_))
        ));
        assert!(incoming.try_recv().is_err());
    }
}
