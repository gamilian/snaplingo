use std::fs::{File, OpenOptions};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const NOTIFICATION_TIMEOUT: Duration = Duration::from_secs(5);
const MESSAGE_MAGIC: [u8; 4] = *b"SNAP";
type Activation = Arc<dyn Fn() + Send + Sync>;

#[derive(serde::Serialize, serde::Deserialize)]
struct Endpoint {
    port: u16,
    token: [u8; 32],
}

#[derive(Default)]
struct Notifications {
    pending: usize,
    activation: Option<Activation>,
}

/// Owns the process lock and authenticated local activation listener independently of the UI shell.
pub struct SingleInstance {
    _lock: File,
    address: SocketAddrV4,
    notifications: Arc<Mutex<Notifications>>,
    stopped: Arc<AtomicBool>,
    worker: Mutex<Option<JoinHandle<()>>>,
}

impl SingleInstance {
    /// Returns `None` only after the running primary has acknowledged activation.
    pub fn acquire(path: &Path) -> io::Result<Option<Self>> {
        let mut options = OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
        }
        let mut file = options.open(path)?;
        let deadline = Instant::now() + NOTIFICATION_TIMEOUT;
        loop {
            if platform::try_lock(&file)? {
                return Self::primary(file).map(Some);
            }
            // The lock may precede endpoint publication, or the previous owner may be exiting.
            // Retry the lock as well as notification, so a crashed primary cannot block takeover.
            if let Ok(endpoint) = read_endpoint(&mut file) {
                if notify(&endpoint).is_ok() {
                    return Ok(None);
                }
            }
            if Instant::now() >= deadline {
                return Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "The running application did not acknowledge activation",
                ));
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    fn primary(mut file: File) -> io::Result<Self> {
        let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0))?;
        let mut token = [0; 32];
        getrandom::fill(&mut token).map_err(io::Error::other)?;
        let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, listener.local_addr()?.port());
        let endpoint = Endpoint {
            port: address.port(),
            token,
        };
        file.set_len(0)?;
        file.seek(SeekFrom::Start(0))?;
        serde_json::to_writer(&mut file, &endpoint)?;
        file.flush()?;

        let notifications = Arc::new(Mutex::new(Notifications::default()));
        let stopped = Arc::new(AtomicBool::new(false));
        let requests = notifications.clone();
        let stop = stopped.clone();
        let worker = std::thread::Builder::new()
            .name("snaplingo-instance".into())
            .spawn(move || {
                for connection in listener.incoming() {
                    if stop.load(Ordering::Acquire) {
                        break;
                    }
                    let Ok(mut stream) = connection else { break };
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
                    let mut message = [0; 36];
                    if stream.read_exact(&mut message).is_err()
                        || message[..4] != MESSAGE_MAGIC
                        || message[4..] != token
                    {
                        continue;
                    }
                    let activation = {
                        let mut notifications = requests.lock().unwrap();
                        match &notifications.activation {
                            Some(activation) => Some(activation.clone()),
                            None => {
                                notifications.pending += 1;
                                None
                            }
                        }
                    };
                    let _ = stream.write_all(&[1]);
                    if let Some(activation) = activation {
                        activation();
                    }
                }
            })?;
        Ok(Self {
            _lock: file,
            address,
            notifications,
            stopped,
            worker: Mutex::new(Some(worker)),
        })
    }

    /// Install after application state is ready; earlier activations are retained.
    pub fn on_activation(&self, activation: impl Fn() + Send + Sync + 'static) {
        let activation: Activation = Arc::new(activation);
        let pending = {
            let mut notifications = self.notifications.lock().unwrap();
            notifications.activation = Some(activation.clone());
            std::mem::take(&mut notifications.pending)
        };
        for _ in 0..pending {
            activation();
        }
    }

    pub fn shutdown(&self) {
        if self.stopped.swap(true, Ordering::AcqRel) {
            return;
        }
        self.notifications.lock().unwrap().activation = None;
        // Wake the blocking accept without an idle polling loop.
        let _ = TcpStream::connect_timeout(&self.address.into(), Duration::from_secs(1));
        if let Some(worker) = self.worker.lock().unwrap().take() {
            let _ = worker.join();
        }
    }
}

impl Drop for SingleInstance {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn read_endpoint(file: &mut File) -> io::Result<Endpoint> {
    file.seek(SeekFrom::Start(0))?;
    serde_json::from_reader(file.take(4096)).map_err(io::Error::other)
}

fn notify(endpoint: &Endpoint) -> io::Result<()> {
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, endpoint.port);
    let timeout = Duration::from_millis(250);
    let mut stream = TcpStream::connect_timeout(&address.into(), timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    stream.write_all(&MESSAGE_MAGIC)?;
    stream.write_all(&endpoint.token)?;
    let mut acknowledgement = [0];
    stream.read_exact(&mut acknowledgement)?;
    if acknowledgement != [1] {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Invalid activation acknowledgement",
        ));
    }
    Ok(())
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::os::fd::AsRawFd;

    pub fn try_lock(file: &File) -> io::Result<bool> {
        // The descriptor stays open in SingleInstance; flock releases it on normal exit or crash.
        if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
            return Ok(true);
        }
        let error = io::Error::last_os_error();
        if error.kind() == io::ErrorKind::WouldBlock {
            Ok(false)
        } else {
            Err(error)
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use std::os::windows::io::AsRawHandle;
    use windows::Win32::Foundation::{ERROR_LOCK_VIOLATION, HANDLE};
    use windows::Win32::Storage::FileSystem::{
        LockFileEx, LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY,
    };
    use windows::Win32::System::IO::OVERLAPPED;

    pub fn try_lock(file: &File) -> io::Result<bool> {
        let mut overlapped = OVERLAPPED::default();
        // Lock beyond the endpoint data, allowing secondary processes to read it.
        unsafe { overlapped.Anonymous.Anonymous.Offset = 4096 };
        match unsafe {
            LockFileEx(
                HANDLE(file.as_raw_handle() as isize),
                LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY,
                0,
                1,
                0,
                &mut overlapped,
            )
        } {
            Ok(()) => Ok(true),
            Err(error) if error.code() == ERROR_LOCK_VIOLATION.to_hresult() => Ok(false),
            Err(error) => Err(io::Error::other(error)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn exclusive_owner_retains_early_activation_and_releases_for_takeover() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("instance.lock");
        let primary = SingleInstance::acquire(&path).unwrap().unwrap();
        assert!(SingleInstance::acquire(&path).unwrap().is_none());
        let calls = Arc::new(AtomicUsize::new(0));
        let received = calls.clone();
        primary.on_activation(move || {
            received.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(SingleInstance::acquire(&path).unwrap().is_none());
        let deadline = Instant::now() + Duration::from_secs(1);
        while calls.load(Ordering::SeqCst) != 2 {
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        drop(primary);
        let replacement = SingleInstance::acquire(&path).unwrap().unwrap();
        drop(replacement);
    }

    #[test]
    fn invalid_activation_token_cannot_trigger_the_callback() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("instance.lock");
        let primary = SingleInstance::acquire(&path).unwrap().unwrap();
        let calls = Arc::new(AtomicUsize::new(0));
        let received = calls.clone();
        primary.on_activation(move || {
            received.fetch_add(1, Ordering::SeqCst);
        });
        let mut file = File::open(path).unwrap();
        let mut endpoint = read_endpoint(&mut file).unwrap();
        endpoint.token[0] ^= 1;
        assert!(notify(&endpoint).is_err());
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(file.metadata().unwrap().permissions().mode() & 0o777, 0o600);
        }
    }
}
