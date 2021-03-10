use futures::Future;
use ntex::channel::mpsc;
use ntex::codec::{AsyncRead, AsyncWrite, Framed};
use ntex::connect::{self, Address, Connect, Connector};
use ntex::service::Service;
use ntex::util::ByteString;

#[cfg(feature = "openssl")]
use ntex::connect::openssl::{OpensslConnector, SslConnector};

#[cfg(feature = "rustls")]
use ntex::connect::rustls::{ClientConfig, RustlsConnector};
#[cfg(feature = "rustls")]
use std::sync::Arc;

use super::codec::Codec;
use super::errors::ConnectError;
use super::transport::Transport;
use super::{cmd, Client, SimpleClient};

/// Redis connector
pub struct RedisConnector<A, T> {
    address: A,
    connector: T,
    passwords: Vec<ByteString>,
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
        }
    }

    #[cfg(feature = "openssl")]
    /// Use openssl connector.
    pub fn openssl(self, connector: SslConnector) -> RedisConnector<A, OpensslConnector<A>> {
        RedisConnector {
            address: self.address,
            passwords: self.passwords,
            connector: OpensslConnector::new(connector),
        }
    }

    #[cfg(feature = "rustls")]
    /// Use rustls connector.
    pub fn rustls(self, config: Arc<ClientConfig>) -> RedisConnector<A, RustlsConnector<A>> {
        RedisConnector {
            address: self.address,
            passwords: self.passwords,
            connector: RustlsConnector::new(config),
        }
    }

    /// Connect to redis server
    pub fn connect(&self) -> impl Future<Output = Result<Client, ConnectError>> {
        let fut = self.connector.call(Connect::new(self.address.clone()));
        let passwords = self.passwords.clone();

        async move {
            let io = fut.await?;

            let (tx, rx) = mpsc::channel();
            ntex::rt::spawn(Transport::new(rx, Framed::new(io, Codec)));

            let client = Client::new(tx);

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
    pub fn connect_simple(
        &self,
    ) -> impl Future<Output = Result<SimpleClient<T::Response>, ConnectError>> {
        let fut = self.connector.call(Connect::new(self.address.clone()));
        let passwords = self.passwords.clone();

        async move {
            let io = fut.await?;
            let mut client = SimpleClient::new(Framed::new(io, Codec));

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
