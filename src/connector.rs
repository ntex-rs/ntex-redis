use ntex_bytes::ByteString;
use ntex_error::Error;
use ntex_io::IoBoxed;
use ntex_net::connect::{self, Address, Connect, Connector};
use ntex_service::{IntoService, Service, cfg::SharedCfg, pipeline::PipelineState};

use super::{Client, SimpleClient, cmd, errors::ConnectError};

type ConnectorPipeline<A> =
    PipelineState<SharedCfg, Connect<A>, IoBoxed, Error<connect::ConnectError>>;

/// Redis connector
pub struct RedisConnector<A> {
    address: A,
    connector: ConnectorPipeline<A>,
    config: SharedCfg,
    passwords: Vec<ByteString>,
}

impl<A> RedisConnector<A>
where
    A: Address + Clone,
{
    /// Create new redis connector
    pub fn new(address: A) -> RedisConnector<A> {
        RedisConnector {
            address,
            connector: PipelineState::new(Connector::default().map(IoBoxed::from)),
            config: SharedCfg::default(),
            passwords: Vec::new(),
        }
    }

    /// Add redis auth password
    pub fn password<U>(mut self, password: U) -> Self
    where
        U: AsRef<str>,
    {
        self.passwords
            .push(ByteString::from(password.as_ref().to_string()));
        self
    }

    /// Set configuration for the connector and connection io.
    ///
    /// By default [`SharedCfg::default()`] is used.
    pub fn config<U: Into<SharedCfg>>(mut self, config: U) -> Self {
        self.config = config.into();
        self
    }

    /// Use custom connector
    pub fn connector<U, F>(mut self, connector: F) -> Self
    where
        F: IntoService<U, SharedCfg, Connect<A>>,
        U: Service<SharedCfg, Connect<A>, Error = Error<connect::ConnectError>> + 'static,
        IoBoxed: From<U::Res>,
    {
        self.connector = PipelineState::new(connector.into_service().map(IoBoxed::from));
        self
    }

    async fn _connect(&self) -> Result<IoBoxed, ConnectError> {
        let io = self
            .connector
            .call(Connect::new(self.address.clone()), &self.config)
            .await?;

        if self.passwords.is_empty() {
            Ok(io)
        } else {
            let client = SimpleClient::new(io);

            for password in &self.passwords {
                if client.exec(cmd::Auth(password)).await? {
                    return Ok(client.into_inner());
                }
            }
            Err(ConnectError::Unauthorized)
        }
    }

    /// Connect to redis server and create shared client
    pub async fn connect(&self) -> Result<Client, ConnectError> {
        self._connect().await.map(Client::new)
    }

    /// Connect to redis server and create simple client
    pub async fn connect_simple(&self) -> Result<SimpleClient, ConnectError> {
        self._connect().await.map(SimpleClient::new)
    }
}
