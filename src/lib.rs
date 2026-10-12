use electrum_client::ElectrumApi;
use pgrx::prelude::*;

mod electrum;

mod tests;

#[cfg(any(test, feature = "pg_test"))]
pub mod pg_test {
    #[cfg(test)]
    use electrum_client::ElectrumApi;

    pub fn setup(_options: Vec<&str>) {}

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec![]
    }

    #[test]
    pub fn try_client() {
        let client = crate::electrum::get_client();
        assert!(client.is_some());
        if let Some(client) = client {
            assert!(client.ping().is_ok());
        }
    }
}

::pgrx::pg_module_magic!(name, version);

fn match_electrum_error(e: electrum::ElectrumError) -> ! {
    use electrum::ElectrumError::*;
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
    }
}

#[pg_extern]
fn bitcoin_ping() -> bool {
    if let Some(client) = crate::electrum::get_client() {
        client.ping().is_ok()
    } else {
        false
    }
}

#[pg_extern]
fn bitcoin_get_confirmed_balance(address: String) -> i64 {
    match crate::electrum::get_balance(address.as_str()) {
        Ok((confirmed, _)) => confirmed,
        Err(e) => match_electrum_error(e),
    }
}

#[pg_extern]
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

#[pg_extern]
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

#[pg_extern]
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

#[pg_extern]
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
