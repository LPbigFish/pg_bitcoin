use electrum_client::ElectrumApi;
use pgrx::prelude::*;

use crate::client_wrapper::{with_client, ElectrumError};

mod electrum;

mod client_wrapper;

mod config;

mod tests;

::pgrx::pg_module_magic!(name, version);

#[pg_guard]
pub extern "C-unwind" fn _PG_init() {
    config::init();
}

fn match_electrum_error(e: client_wrapper::ElectrumError) -> ! {
    use client_wrapper::ElectrumError::*;
    match e {
        InvalidAddress(e_addr) => {
            pgrx::error!("Invalid address: {}", e_addr)
        }
        ClientUnavailable => {
            pgrx::error!("Electrum client unavailable, please check if the Electrum server is running and accessible, then restart the PostgreSQL server.")
        }
        ClientError(err) => {
            pgrx::error!("Electrum client error: {:?}", err)
        }
        ClientInUse => {
            pgrx::error!("Electrum client is already in use")
        }
        BadExtensionConfiguration(option) => {
            pgrx::error!("Extension configuration option unset: {}", option)
        }
    }
}

#[pg_extern]
fn bitcoin_ping() -> bool {
    with_client(|c, _| c.ping().map_err(ElectrumError::ClientError)).is_ok()
}

#[pg_extern(name = "get_confirmed_balance")]
fn bitcoin_get_confirmed_balance(address: String) -> i64 {
    match crate::electrum::get_balance(address.as_str()) {
        Ok((confirmed, _)) => confirmed,
        Err(e) => match_electrum_error(e),
    }
}

#[pg_extern(name = "get_balance")]
fn bitcoin_get_balance(
    address: String,
) -> TableIterator<
    'static,
    (
        name!(address, String),
        name!(confirmed, i64),
        name!(unconfirmed, i64),
    ),
> {
    match crate::electrum::get_balance(address.as_str()) {
        Ok((c, u)) => TableIterator::new(vec![(address, c, u)]),
        Err(e) => match_electrum_error(e),
    }
}

#[pg_extern(name = "get_balance")]
fn bitcoin_get_balances(
    addresses: Vec<String>,
) -> TableIterator<
    'static,
    (
        name!(address, String),
        name!(confirmed, i64),
        name!(unconfirmed, i64),
    ),
> {
    match crate::electrum::get_balances(addresses) {
        Ok(balances) => TableIterator::new(balances),
        Err(e) => match_electrum_error(e),
    }
}

#[pg_extern(name = "get_history")]
fn bitcoin_get_history(
    address: String,
) -> TableIterator<
    'static,
    (
        name!(height, i32),
        name!(tx_hash, String),
        name!(fee, Option<i64>),
    ),
> {
    match crate::electrum::get_history(address.as_str()) {
        Ok(history) => TableIterator::new(history),
        Err(e) => match_electrum_error(e),
    }
}

#[pg_extern(name = "get_history")]
fn bitcoin_get_histories(
    addresses: Vec<String>,
) -> TableIterator<
    'static,
    (
        name!(address, String),
        name!(height, i32),
        name!(tx_hash, String),
        name!(fee, Option<i64>),
    ),
> {
    match crate::electrum::get_histories(addresses) {
        Ok(history) => TableIterator::new(history),
        Err(e) => match_electrum_error(e),
    }
}

#[pg_extern(name = "list_unspent")]
fn bitcoin_listunspent(
    address: String,
) -> TableIterator<
    'static,
    (
        name!(height, i64),
        name!(tx_hash, String),
        name!(tx_pos, i64),
        name!(value, i64),
    ),
> {
    match crate::electrum::list_unspent(&address) {
        Ok(spent) => TableIterator::new(spent),
        Err(e) => match_electrum_error(e),
    }
}

#[cfg(any(test, feature = "pg_test"))]
pub mod pg_test {
    #[cfg(test)]

    pub fn setup(_options: Vec<&str>) {}

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }
}
