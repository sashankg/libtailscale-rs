use std::ffi::{CStr, CString};
use std::net::TcpStream;
use std::os::fd::FromRawFd;
use std::os::raw::c_int;

use libtailscale_sys::*;

/// A handle onto a Tailscale server
#[derive(Debug)]
pub struct Tailscale {
    inner: tailscale,
}

/// A socket on the tailnet listening for connections.
#[derive(Debug)]
pub struct Listener<'a> {
    tailscale: &'a Tailscale,
    listener: tailscale_listener,
}

impl Tailscale {
    /// Create a tailscale server object
    ///
    /// No network connection is initialized until [`Tailscale::start`] is called.
    pub fn new() -> Self {
        let inner = unsafe { tailscale_new() };
        Self { inner }
    }

    /// Connect the server to the tailnet
    ///
    /// Calling this function is optional as it will be called by the first use
    /// of [`Tailscale::listen`] or [`Tailscale::dial`] on a server
    pub fn start(&mut self) -> Result<(), String> {
        let ret = unsafe { tailscale_start(self.inner) };
        if ret != 0 {
            Err(self.last_error())
        } else {
            Ok(())
        }
    }

    /// Connect the server to the tailnet and waits for it to be usable
    ///
    /// To cancel an in-progress call, use [`Tailscale::close`]
    pub fn up(&mut self) -> Result<(), String> {
        let ret = unsafe { tailscale_up(self.inner) };
        if ret != 0 {
            Err(self.last_error())
        } else {
            Ok(())
        }
    }

    /// Shut down the server
    pub fn close(&mut self) -> Result<(), ()> {
        let ret = unsafe { tailscale_close(self.inner) };
        if ret != 0 {
            Err(())
        } else {
            Ok(())
        }
    }

    /// Set the name of the directory to use for state.
    pub fn set_dir(&mut self, dir: &str) -> Result<(), String> {
        let dir = CString::new(dir).unwrap();
        let ret = unsafe { tailscale_set_dir(self.inner, dir.as_ptr()) };
        if ret != 0 {
            Err(self.last_error())
        } else {
            Ok(())
        }
    }

    /// Set the hostname to present to the control server
    pub fn set_hostname(&mut self, hostname: &str) -> Result<(), String> {
        let hostname = CString::new(hostname).unwrap();
        let ret = unsafe { tailscale_set_hostname(self.inner, hostname.as_ptr()) };
        if ret != 0 {
            Err(self.last_error())
        } else {
            Ok(())
        }
    }

    /// Set the auth key to create the node and will be preferred over the
    /// `TS_AUTHKEY` environment variable.
    pub fn set_authkey(&mut self, authkey: &str) -> Result<(), String> {
        let authkey = CString::new(authkey).unwrap();
        let ret = unsafe { tailscale_set_authkey(self.inner, authkey.as_ptr()) };
        if ret != 0 {
            Err(self.last_error())
        } else {
            Ok(())
        }
    }

    /// Set the URL of the coordination server to use.
    ///
    /// If empty or unset, the Tailscale default is used.
    pub fn set_control_url(&mut self, control_url: &str) -> Result<(), String> {
        let control_url = CString::new(control_url).unwrap();
        let ret = unsafe { tailscale_set_control_url(self.inner, control_url.as_ptr()) };
        if ret != 0 {
            Err(self.last_error())
        } else {
            Ok(())
        }
    }

    /// Specifies whether the node should be ephemeral.
    pub fn set_ephemeral(&mut self, ephemeral: bool) -> Result<(), String> {
        let ret = unsafe { tailscale_set_ephemeral(self.inner, ephemeral as _) };
        if ret != 0 {
            Err(self.last_error())
        } else {
            Ok(())
        }
    }

    /// Instruct the tailscale instance to write logs to `logfd`
    ///
    /// An `logfd` value of `-1` means discard all logging.
    pub fn set_logfd(&mut self, logfd: c_int) -> Result<(), String> {
        let ret = unsafe { tailscale_set_logfd(self.inner, logfd) };
        if ret != 0 {
            Err(self.last_error())
        } else {
            Ok(())
        }
    }

    /// Connect to the address on the tailnet.
    ///
    /// * `network` is a string of the form "tcp", "udp", etc.
    /// * `address` is a string of an IP address or domain name.
    ///
    /// It will start the server if it has not been started yet.
    pub fn dial(&self, network: &str, address: &str) -> Result<TcpStream, String> {
        let c_network = CString::new(network).unwrap();
        let c_address = CString::new(address).unwrap();
        let mut conn = 0;
        let ret = unsafe {
            tailscale_dial(
                self.inner,
                c_network.as_ptr(),
                c_address.as_ptr(),
                &mut conn,
            )
        };
        if ret != 0 {
            Err(self.last_error())
        } else {
            Ok(unsafe { TcpStream::from_raw_fd(conn) })
        }
    }

    /// Listen for a connection on the tailnet.
    ///
    /// * `network` is a string of the form "tcp", "udp", etc.
    /// * `address` is a string of an IP address or domain name.
    ///
    /// It will start the server if it has not been started yet.
    pub fn listen(&self, network: &str, address: &str) -> Result<Listener, String> {
        let c_network = CString::new(network).unwrap();
        let c_address = CString::new(address).unwrap();
        let mut listener = 0;
        let ret = unsafe {
            tailscale_listen(
                self.inner,
                c_network.as_ptr(),
                c_address.as_ptr(),
                &mut listener,
            )
        };
        if ret != 0 {
            Err(self.last_error())
        } else {
            Ok(Listener {
                tailscale: self,
                listener,
            })
        }
    }

    /// Start a LocalAPI listener on a loopback address, and returns the address
    /// and password credential string for the instance.
    pub fn loopback_api(&mut self) -> Result<(String, String), String> {
        let mut addr = [0; 1024];
        let mut cred = [0; 33];
        let ret = unsafe {
            tailscale_loopback_api(self.inner, addr.as_mut_ptr(), addr.len(), cred.as_mut_ptr())
        };
        if ret != 0 {
            Err(self.last_error())
        } else {
            let addr = unsafe { CStr::from_ptr(addr.as_ptr()) };
            let cred = unsafe { CStr::from_ptr(cred.as_ptr()) };
            Ok((
                addr.to_str().unwrap().to_owned(),
                cred.to_str().unwrap().to_owned(),
            ))
        }
    }

    fn last_error(&self) -> String {
        let mut buffer = [0; 256];
        unsafe {
            tailscale_errmsg(self.inner, buffer.as_mut_ptr(), buffer.len() as _);
        }
        let cstr = unsafe { CStr::from_ptr(buffer.as_ptr()) };
        cstr.to_string_lossy().into_owned()
    }
}

impl Drop for Tailscale {
    fn drop(&mut self) {
        let _ret = self.close();
    }
}

impl Default for Tailscale {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Listener<'a> {
    /// Accept a connection on a Tailscale [`Listener`].
    pub fn accept(&self) -> Result<TcpStream, String> {
        let mut conn = 0;
        let ret = unsafe { tailscale_accept(self.listener, &mut conn) };
        if ret != 0 {
            Err(self.tailscale.last_error())
        } else {
            Ok(unsafe { TcpStream::from_raw_fd(conn) })
        }
    }

    /// Close the listener.
    pub fn close(&mut self) -> Result<(), String> {
        let ret = unsafe { tailscale_listener_close(self.listener) };
        if ret != 0 {
            Err(self.tailscale.last_error())
        } else {
            Ok(())
        }
    }
}

impl<'a> Drop for Listener<'a> {
    fn drop(&mut self) {
        let _ret = self.close();
    }
}
