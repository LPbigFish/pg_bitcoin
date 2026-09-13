use electrum_client::ElectrumApi;
use pgrx::prelude::*;

mod electrum;

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
        name!(value, Option<i64>),
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
        name!(value, Option<i64>),
    ),
> {
    match crate::electrum::get_histories(addresses) {
        Ok(history) => TableIterator::new(history),
        Err(e) => match_electrum_error(e),
    }
}

#[cfg(any(test, feature = "pg_test"))]
#[pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_bitcoin_ping() {
        let res = Spi::get_one::<bool>("SELECT bitcoin_ping();");
        assert!(res.unwrap().unwrap_or(false))
    }

    #[pg_test]
    fn test_basic_bitcoin_get_confirmed_balance() {
        let res = Spi::get_one::<i64>(
            "SELECT bitcoin_get_confirmed_balance('bcrt1qcsh8a7f0mdsr47zy6pj04tv4mwdumlfacs66tf');",
        );
        assert_eq!(res.unwrap().unwrap_or(-1), 300_000_000);
    }

    #[pg_test]
    fn test_srf_bitcoin_get_balance() {
        let res = Spi::get_one::<i64>(
            "SELECT confirmed from bitcoin_get_balance('bcrt1qcsh8a7f0mdsr47zy6pj04tv4mwdumlfacs66tf');",
        );
        assert_eq!(res.unwrap().unwrap_or(-1), 300_000_000);
    }

    #[pg_test]
    fn test_batch_srf_bitcoin_get_balance() {
        // 10 BTC based on mempool
        let res = Spi::get_one::<i64>(
            "SELECT SUM(confirmed)::bigint from bitcoin_get_balances(ARRAY[
                'bcrt1qcsh8a7f0mdsr47zy6pj04tv4mwdumlfacs66tf',
                'bcrt1qgar7sarvmkenkrmljk5slz0cn7ec0jak76zrj7',
                'bcrt1q0ht9tyks4vh7p5p904t340cr9nvahy7uevmqwj',
                'bcrt1qq6hag67dl53wl99vzg42z8eyzfz2xlkvwk6f7m'
            ]);",
        );
        assert_eq!(res.unwrap().unwrap_or(-1), 1_000_000_000);
    }
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(test)]
pub mod pg_test {
    use electrum_client::ElectrumApi;

    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        // return any postgresql.conf settings that are required for your tests
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
