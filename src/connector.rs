use std::{cell::RefCell, future::Future, rc::Rc};

use ntex::codec::{AsyncRead, AsyncWrite};
use ntex::connect::{self, Address, Connect, Connector};
use ntex::framed::{ReadTask, State, WriteTask};
use ntex::{service::Service, time::Seconds, util::ByteString, util::PoolId, util::PoolRef};

#[cfg(feature = "openssl")]
use ntex::connect::openssl::{OpensslConnector, SslConnector};

#[cfg(feature = "rustls")]
use ntex::connect::rustls::{ClientConfig, RustlsConnector};

use super::errors::ConnectError;
use super::{cmd, Client, SimpleClient};

/// Redis connector
pub struct RedisConnector<A, T> {
    address: A,
    connector: T,
    passwords: Vec<ByteString>,
    pool: PoolRef,
}

impl<A> RedisConnector<A, ()>
where
    A: Address + Clone,
{
    #[allow(clippy::new_ret_no_self)]
    /// Create new redis connector
    pub fn new(address: A) -> RedisConnector<A, Connector<A>> {
        RedisConnector {
            address,
            passwords: Vec::new(),
            connector: Connector::default(),
            pool: PoolId::P7.pool_ref(),
        }
    }
}

impl<A, T> RedisConnector<A, T>
where
    A: Address + Clone,
    T: Service<Request = Connect<A>, Error = connect::ConnectError>,
    T::Response: AsyncRead + AsyncWrite + Unpin + 'static,
{
    /// Add redis auth password
    pub fn password<U>(mut self, password: U) -> Self
    where
        U: AsRef<str>,
    {
        self.passwords
            .push(ByteString::from(password.as_ref().to_string()));
        self
    }

    /// Set memory pool.
    ///
    /// Use specified memory pool for memory allocations. By default P7
    /// memory pool is used.
    pub fn memory_pool(mut self, id: PoolId) -> Self {
        self.pool = id.pool_ref();
        self
    }

    #[doc(hidden)]
    #[deprecated(since = "0.2.4", note = "Use memory pool config")]
    #[inline]
    /// Set read/write buffer params
    ///
    /// By default read buffer is 16kb, write buffer is 16kb
    pub fn buffer_params(
        self,
        _max_read_buf_size: u16,
        _max_write_buf_size: u16,
        _min_buf_size: u16,
    ) -> Self {
        self
    }

    /// Use custom connector
    pub fn connector<U>(self, connector: U) -> RedisConnector<A, U>
    where
        U: Service<Request = Connect<A>, Error = connect::ConnectError>,
        U::Response: AsyncRead + AsyncWrite + Unpin + 'static,
    {
        RedisConnector {
            connector,
            address: self.address,
            passwords: self.passwords,
            pool: self.pool,
        }
    }

    #[cfg(feature = "openssl")]
    /// Use openssl connector.
    pub fn openssl(self, connector: SslConnector) -> RedisConnector<A, OpensslConnector<A>> {
        RedisConnector {
            address: self.address,
            passwords: self.passwords,
            connector: OpensslConnector::new(connector),
            pool: self.pool,
        }
    }

    #[cfg(feature = "rustls")]
    /// Use rustls connector.
    pub fn rustls(
        self,
        config: std::sync::Arc<ClientConfig>,
    ) -> RedisConnector<A, RustlsConnector<A>> {
        RedisConnector {
            address: self.address,
            passwords: self.passwords,
            connector: RustlsConnector::new(config),
            pool: self.pool,
        }
    }

    /// Connect to redis server and create shared client
    pub fn connect(&self) -> impl Future<Output = Result<Client, ConnectError>> {
        let pool = self.pool;
        let passwords = self.passwords.clone();
        let fut = self.connector.call(Connect::new(self.address.clone()));

        async move {
            let io = fut.await?;

            let state = State::with_memory_pool(pool);
            state.set_disconnect_timeout(Seconds::ZERO);
            let io = Rc::new(RefCell::new(io));
            ntex::rt::spawn(ReadTask::new(io.clone(), state.clone()));
            ntex::rt::spawn(WriteTask::new(io, state.clone()));

            let client = Client::new(state);

            if passwords.is_empty() {
                Ok(client)
            } else {
                for password in passwords {
                    if client.exec(cmd::Auth(password)).await? {
                        return Ok(client);
                    }
                }
                Err(ConnectError::Unauthorized)
            }
        }
    }

    /// Connect to redis server and create simple client
    pub fn connect_simple(&self) -> impl Future<Output = Result<SimpleClient, ConnectError>> {
        let pool = self.pool;
        let passwords = self.passwords.clone();
        let fut = self.connector.call(Connect::new(self.address.clone()));

        async move {
            let io = fut.await?;

            let state = State::with_memory_pool(pool);
            state.set_disconnect_timeout(Seconds::ZERO);
            let io = Rc::new(RefCell::new(io));
            ntex::rt::spawn(ReadTask::new(io.clone(), state.clone()));
            ntex::rt::spawn(WriteTask::new(io, state.clone()));

            let mut client = SimpleClient::new(state);

            if passwords.is_empty() {
                Ok(client)
            } else {
                for password in passwords {
                    if client.exec(cmd::Auth(password)).await? {
                        return Ok(client);
                    }
                }
                Err(ConnectError::Unauthorized)
            }
        }
    }
}
