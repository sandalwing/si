use std::{
    net::{SocketAddr, ToSocketAddrs},
    path::{Path, PathBuf},
    time::Duration,
};

use derive_builder::Builder;
use si_settings::{CanonicalFile, CanonicalFileError};
use thiserror::Error;

#[remain::sorted]
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("config builder")]
    Builder(#[from] ConfigBuilderError),
    #[error(transparent)]
    CanonicalFile(#[from] CanonicalFileError),
    #[error("no socket addrs where resolved")]
    NoSocketAddrResolved,
    #[error("failed to resolve socket addrs")]
    SocketAddrResolve(#[source] std::io::Error),
}

type Result<T> = std::result::Result<T, ConfigError>;

#[derive(Debug, Builder)]
pub struct Config {
    #[builder(default)]
    watch: Option<Duration>,

    #[builder(default = "false")]
    enable_ping: bool,

    #[builder(default = "true")]
    enable_resolver: bool,

    #[builder(default = "true")]
    enable_workflow_resolve: bool,

    #[builder(default = "true")]
    enable_command_run: bool,

    #[builder(default = "true")]
    enable_reconciliation: bool,

    #[builder(default = "true")]
    enable_validation: bool,

    #[builder(default = "IncomingStream::default()")]
    incoming_stream: IncomingStream,

    #[builder(try_setter, setter(into))]
    lang_server_path: CanonicalFile,

    #[builder(setter(into), default)]
    limit_requests: Option<u32>,
}

impl Config {
    /// Constructs a builder for creating a Config
    #[must_use]
    pub fn builder() -> ConfigBuilder {
        ConfigBuilder::default()
    }

    /// Gets a reference to the config's watch.
    #[must_use]
    pub fn watch(&self) -> Option<Duration> {
        self.watch
    }

    /// Gets a reference to the config's enable ping.
    #[must_use]
    pub fn enable_ping(&self) -> bool {
        self.enable_ping
    }

    /// Gets a reference to the config's enable resolver.
    #[must_use]
    pub fn enable_resolver(&self) -> bool {
        self.enable_resolver
    }

    /// Gets a reference to the config's enable workflow resolve.
    #[must_use]
    pub fn enable_workflow_resolve(&self) -> bool {
        self.enable_workflow_resolve
    }

    /// Gets a reference to the config's enable command run.
    #[must_use]
    pub fn enable_command_run(&self) -> bool {
        self.enable_command_run
    }

    /// Gets a reference to the config's enable reconciliation.
    #[must_use]
    pub fn enable_reconciliation(&self) -> bool {
        self.enable_reconciliation
    }

    /// Gets a reference to the config's enable validation
    #[must_use]
    pub fn enable_validation(&self) -> bool {
        self.enable_validation
    }

    /// Gets a reference to the config's incoming stream.
    #[must_use]
    pub fn incoming_stream(&self) -> &IncomingStream {
        &self.incoming_stream
    }

    /// Gets a reference to the config's lang server path.
    #[must_use]
    pub fn lang_server_path(&self) -> &Path {
        self.lang_server_path.as_path()
    }

    /// Gets a reference to the config's limit requests.
    #[must_use]
    pub fn limit_requests(&self) -> Option<u32> {
        self.limit_requests
    }
}

impl ConfigBuilder {
    pub fn http_socket(&mut self, socket_addrs: impl ToSocketAddrs) -> Result<&mut Self> {
        Ok(self.incoming_stream(IncomingStream::http_socket(socket_addrs)?))
    }

    pub fn unix_domain_socket(&mut self, path: impl Into<PathBuf>) -> &mut Self {
        self.incoming_stream(IncomingStream::unix_domain_socket(path))
    }
}

#[remain::sorted]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IncomingStream {
    HTTPSocket(SocketAddr),
    UnixDomainSocket(PathBuf),
}

impl Default for IncomingStream {
    fn default() -> Self {
        Self::HTTPSocket(SocketAddr::from(([0, 0, 0, 0], 5157)))
    }
}

impl IncomingStream {
    pub fn http_socket(socket_addrs: impl ToSocketAddrs) -> Result<Self> {
        let socket_addr = socket_addrs
            .to_socket_addrs()
            .map_err(ConfigError::SocketAddrResolve)?
            .next()
            .ok_or(ConfigError::NoSocketAddrResolved)?;
        Ok(Self::HTTPSocket(socket_addr))
    }

    pub fn unix_domain_socket(path: impl Into<PathBuf>) -> Self {
        let pathbuf = path.into();
        Self::UnixDomainSocket(pathbuf)
    }
}
