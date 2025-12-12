use anyhow::Context;
use parking_lot::RwLock;
use serde::{de::DeserializeOwned, Serialize};
use std::{
    fs::OpenOptions,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{Mutex, Notify},
    task::JoinHandle,
    time,
};

const WAL_SUFFIX: &str = ".wal";

#[derive(Clone)]
pub struct JsonDbOptions {
    /// Delay to debounce writes. Default 50ms.
    pub write_delay: Duration,
    /// Whether to enable WAL. Default true.
    pub enable_wal: bool,
}

impl Default for JsonDbOptions {
    fn default() -> Self {
        Self {
            write_delay: Duration::from_millis(50),
            enable_wal: true,
        }
    }
}

pub struct JsonDb<T>
where
    T: Serialize + DeserializeOwned + Default + Send + Sync + 'static,
{
    path: PathBuf,
    wal_path: PathBuf,
    inner: Arc<RwLock<T>>, // in-memory copy
    // last-modified to detect external change
    last_modified: Arc<parking_lot::Mutex<Option<std::time::SystemTime>>>,
    // background writer control
    write_notify: Arc<Notify>,
    writer_handle: Mutex<Option<JoinHandle<()>>>,
    // a flag that there are pending writes
    pending: Arc<parking_lot::Mutex<bool>>,
    opts: JsonDbOptions,
}

impl<T> JsonDb<T>
where
    T: Serialize + DeserializeOwned + Send + Sync + Default + 'static,
{
    /// Open database at path. If file missing, default T is created and written.
    pub async fn open<P: AsRef<Path>>(path: P) -> anyhow::Result<Self> {
        Self::open_opts(path, JsonDbOptions::default()).await
    }

    pub async fn open_opts<P: AsRef<Path>>(path: P, opts: JsonDbOptions) -> anyhow::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let wal_path = path.with_extension(format!(
            "{}{}",
            path.extension().and_then(|s| s.to_str()).unwrap_or(""),
            WAL_SUFFIX
        ));

        // load file or create default
        let initial: T = match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str(&s).context("deserializing DB file")?,
            Err(_) => T::default(),
        };

        // replay WAL if present
        if opts.enable_wal && wal_path.exists() {
            let wal = std::fs::read_to_string(&wal_path).context("reading WAL")?;
            // wal can contain one or more JSON objects to apply. We apply by deserializing into T and replacing.
            // For simplicity assume WAL contains full snapshots appended. In production you'd use op-based WAL.
            if !wal.trim().is_empty() {
                if let Ok(t) = serde_json::from_str::<T>(&wal) {
                    // take WAL as authoritative
                    let initial = t;
                    // write back to main file below
                    let _ = std::fs::write(&path, serde_json::to_string_pretty(&initial)?);
                    let _ = std::fs::remove_file(&wal_path);
                }
            }
        }

        let last_modified = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok());

        let db = Self {
            path: path.clone(),
            wal_path: wal_path.clone(),
            inner: Arc::new(RwLock::new(initial)),
            last_modified: Arc::new(parking_lot::Mutex::new(last_modified)),
            write_notify: Arc::new(Notify::new()),
            writer_handle: Mutex::new(None),
            pending: Arc::new(parking_lot::Mutex::new(false)),
            opts,
        };

        db.start_writer().await;
        Ok(db)
    }

    fn lock_file(&self) -> anyhow::Result<std::fs::File> {
        // open or create the file and lock it using fs2
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&self.path)
            .with_context(|| format!("opening DB file {}", self.path.display()))?;
        fs2::FileExt::lock_exclusive(&f).with_context(|| "locking DB file")?;
        Ok(f)
    }

    fn write_file_atomic(&self, json: &str) -> anyhow::Result<()> {
        // Optionally write WAL first
        if self.opts.enable_wal {
            std::fs::write(&self.wal_path, json).context("writing WAL")?;
        }

        // Acquire exclusive lock on main file while writing
        let f = self.lock_file()?;
        // replace file content
        // use tempfile strategy
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, json).context("writing temp file")?;
        std::fs::rename(&tmp, &self.path).context("atomic rename")?;

        // update last_modified
        let m = std::fs::metadata(&self.path).and_then(|m| m.modified());
        *self.last_modified.lock() = m.ok();

        // remove WAL
        if self.opts.enable_wal {
            let _ = std::fs::remove_file(&self.wal_path);
        }

        // unlock by dropping file
        drop(f);
        Ok(())
    }

    async fn start_writer(&self) {
        let notify = self.write_notify.clone();
        let inner = self.inner.clone();
        let pending = self.pending.clone();
        let path = self.path.clone();
        let wal_path = self.wal_path.clone();
        let opts = self.opts.clone();
        let last_modified = self.last_modified.clone();

        let handle = tokio::spawn(async move {
            loop {
                notify.notified().await;
                // debounce
                time::sleep(opts.write_delay).await;

                // was pending cleared in the meantime?
                let mut do_write = false;
                {
                    let mut p = pending.lock();
                    if *p {
                        do_write = true;
                        *p = false;
                    }
                }
                if !do_write {
                    continue;
                }

                // prepare JSON
                let snapshot = {
                    let guard = inner.read();
                    serde_json::to_string_pretty(&*guard)
                };
                let json = match snapshot {
                    Ok(j) => j,
                    Err(e) => {
                        eprintln!("serialize error when writing DB: {}", e);
                        continue;
                    }
                };

                // check external changes and reload if needed
                if let Ok(meta) = std::fs::metadata(&path) {
                    if let Ok(modified) = meta.modified() {
                        let last = *last_modified.lock();
                        if last.map(|t| t < modified).unwrap_or(true) {
                            // external change detected. reload and merge: here we replace memory then continue.
                            if let Ok(s) = std::fs::read_to_string(&path) {
                                if let Ok(external) = serde_json::from_str::<T>(&s) {
                                    let mut w = inner.write();
                                    *w = external;
                                    *last_modified.lock() = Some(modified);
                                    // after reload, we must try write again with new snapshot
                                    // compute new snapshot
                                    if let Ok(j2) = serde_json::to_string_pretty(&*w) {
                                        if opts.enable_wal {
                                            let _ = std::fs::write(&wal_path, &j2);
                                        }
                                        let tmp = path.with_extension("tmp");
                                        let _ = std::fs::write(&tmp, &j2);
                                        let _ = std::fs::rename(&tmp, &path);
                                        let _ = std::fs::remove_file(&wal_path);
                                        *last_modified.lock() = std::fs::metadata(&path)
                                            .and_then(|m| m.modified())
                                            .ok();
                                    }
                                    continue;
                                }
                            }
                        }
                    }
                }

                // normal write
                if opts.enable_wal {
                    let _ = std::fs::write(&wal_path, &json);
                }
                let tmp = path.with_extension("tmp");
                if let Err(e) = std::fs::write(&tmp, &json) {
                    eprintln!("write tmp failed: {}", e);
                    continue;
                }
                if let Err(e) = std::fs::rename(&tmp, &path) {
                    eprintln!("rename failed: {}", e);
                    continue;
                }
                let _ = std::fs::remove_file(&wal_path);
                *last_modified.lock() = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
            }
        });

        let mut h = self.writer_handle.lock().await;
        *h = Some(handle);
    }

    /// Read-only access. The closure gets an `&T` and returns `R`.
    /// The closure is executed synchronously under a read lock.
    pub fn read<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&T) -> R,
    {
        let guard = self.inner.read();
        f(&*guard)
    }

    /// Mutating access. Closure gets `&mut T`. The closure runs under write lock.
    /// Changes will be scheduled to be written asynchronously.
    pub fn update<R, F>(&self, f: F) -> anyhow::Result<R>
    where
        F: FnOnce(&mut T) -> R,
    {
        let mut guard = self.inner.write();
        let res = f(&mut *guard);
        // mark pending
        *self.pending.lock() = true;
        self.write_notify.notify_one();
        Ok(res)
    }

    /// Force immediate flush to disk. This performs a blocking sync write.
    pub async fn flush(&self) -> anyhow::Result<()> {
        // serialize snapshot
        let json = {
            let guard = self.inner.read();
            serde_json::to_string_pretty(&*guard)?
        };
        // attempt to write atomically with lock
        // use dedicated function
        // use blocking file ops in blocking task
        let this = self.clone_simple();
        tokio::task::spawn_blocking(move || this.write_file_atomic(&json)).await??;
        Ok(())
    }

    fn clone_simple(&self) -> Self {
        Self {
            path: self.path.clone(),
            wal_path: self.wal_path.clone(),
            inner: self.inner.clone(),
            last_modified: self.last_modified.clone(),
            write_notify: self.write_notify.clone(),
            writer_handle: Mutex::new(None),
            pending: self.pending.clone(),
            opts: self.opts.clone(),
        }
    }
}

impl<T> Drop for JsonDb<T>
where
    T: Serialize + DeserializeOwned + Default + Send + Sync + 'static,
{
    fn drop(&mut self) {
        // try to flush synchronously. Best-effort only.
        if let Ok(Ok(json)) = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let guard = self.inner.read();
            serde_json::to_string_pretty(&*guard)
        })) {
            let _ = self.write_file_atomic(&json);
        }
        // writer task is detached. In most runtimes tokio will shut down tasks gracefully.
    }
}
