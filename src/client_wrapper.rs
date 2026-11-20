use std::{cell::RefCell, time::Duration};

use bitcoin::Network;
use electrum_client::{Client, ConfigBuilder};

use crate::config::ExtensionConfiguration;

#[derive(thiserror::Error, Debug)]
pub enum ElectrumError {
    #[error("Invalid Bitcoin address {0}")]
    InvalidAddress(String),
    #[error("Electrum client unavailable")]
    ClientUnavailable,
    #[error("Client is in use")]
    ClientInUse,
    #[error("Electrum client error: {0}")]
    ClientError(#[from] electrum_client::Error),
    #[error("Extension configuration option unset: {0}")]
    BadExtensionConfiguration(String),
}

thread_local! {
    static CONNECTION: RefCell<Option<ElectrumConnection>> = const { RefCell::new(None) };
}

pub struct ElectrumConnection {
    config: ExtensionConfiguration,
    client: Client,
}

impl ElectrumConnection {
    fn connect() -> Result<Self, ElectrumError> {
        let client_config = ConfigBuilder::new()
            .timeout(Some(Duration::from_secs(5)))
            .retry(1)
            .build();

        let extension_config = ExtensionConfiguration::current();

        if let Some(url) = extension_config.get_url() {
            let client = Client::from_config(&url, client_config)
                .map_err(|e| ElectrumError::ClientError(e))?;

            return Ok(Self {
                config: extension_config,
                client: client,
            });
        }

        Err(ElectrumError::BadExtensionConfiguration(String::from(
            "ELECTRUM_URL",
        )))
    }
}

pub fn with_client<T>(
    op: impl FnOnce(&Client, Network) -> Result<T, ElectrumError>,
) -> Result<T, ElectrumError> {
    let current_config = ExtensionConfiguration::current();

    CONNECTION.with(|slot| {
        let mut slot = slot
            .try_borrow_mut()
            .map_err(|_| ElectrumError::ClientInUse)?;

        let is_config_outdated = slot
            .as_ref()
            .is_none_or(|conn| conn.config != current_config);

        if is_config_outdated {
            *slot = None;
            *slot = Some(ElectrumConnection::connect()?);
        }

        let conn = slot.as_ref().ok_or(ElectrumError::ClientUnavailable)?;

        op(&conn.client, conn.config.get_network())
    })
}
