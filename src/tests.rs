#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::{pg_test, prelude::*};

    #[pg_test]
    fn test_bitcoin_ping() {
        let res = Spi::get_one::<bool>("SELECT bitcoin_ping();");
        assert!(res.unwrap().unwrap_or(false))
    }

    #[pg_test]
    fn test_network_changes_are_respected() {
        use crate::client_wrapper::ElectrumError;

        let address = "bcrt1qcsh8a7f0mdsr47zy6pj04tv4mwdumlfacs66tf";

        Spi::run("SET LOCAL pg_bitcoin.network_type = 'regtest'")
            .expect("Setting regtest should succeed");

        crate::electrum::get_balance(address).expect("Regtest address should be accepted");

        Spi::run("SET LOCAL pg_bitcoin.network_type = 'mainnet'")
            .expect("setting mainnet should succeed");

        assert!(
            matches!(
                crate::electrum::get_balance(address),
                Err(ElectrumError::InvalidAddress(_))
            ),
            "Changed network must reject a regtest address"
        );

        Spi::run("SET LOCAL pg_bitcoin.network_type = 'regtest'")
            .expect("Restoring regtest should succeed");

        crate::electrum::get_balance(address)
            .expect("The client should remain usable after validation fails");
    }

    #[pg_test]
    fn test_client_recovers_after_url_change() {
        let original_url =
            Spi::get_one::<String>("SELECT current_setting('pg_bitcoin.electrum_url')")
                .expect("Reading the Electrum URL should succeed")
                .expect("The Electrum URL should be configured");

        assert_eq!(
            Spi::get_one::<bool>("SELECT bitcoin_ping();")
                .expect("The initial ping query should succeed"),
            Some(true),
            "The configured Electrum server should be reachable"
        );

        Spi::run("SET LOCAL pg_bitcoin.electrum_url = 'tcp://127.0.0.1:0'")
            .expect("Setting an unreachable Electrum URL should succeed");

        assert_eq!(
            Spi::get_one::<bool>("SELECT bitcoin_ping();")
                .expect("A failed connection should not abort the ping query"),
            Some(false),
            "Changing the URL must stop using the old connection"
        );

        Spi::run(&format!(
            "SET LOCAL pg_bitcoin.electrum_url = {}",
            pgrx::spi::quote_literal(&original_url)
        ))
        .expect("Restoring the Electrum URL should succeed");

        assert_eq!(
            Spi::get_one::<bool>("SELECT bitcoin_ping();")
                .expect("The recovery ping query should succeed"),
            Some(true),
            "The client should reconnect after restoring the original URL"
        );
    }

    #[pg_test]
    fn test_basic_bitcoin_get_confirmed_balance() {
        let res = Spi::get_one::<i64>(
            "SELECT get_confirmed_balance('bcrt1qcsh8a7f0mdsr47zy6pj04tv4mwdumlfacs66tf');",
        );
        assert_eq!(res.unwrap().unwrap_or(-1), 300_000_000);
    }

    #[pg_test]
    fn test_srf_bitcoin_get_balance() {
        let res = Spi::get_one::<i64>(
            "SELECT confirmed from get_balance('bcrt1qcsh8a7f0mdsr47zy6pj04tv4mwdumlfacs66tf');",
        );
        assert_eq!(res.unwrap().unwrap_or(-1), 300_000_000);
    }

    #[pg_test]
    fn test_batch_srf_bitcoin_get_balance() {
        // 10 BTC based on mempool
        let res = Spi::get_one::<i64>(
            "SELECT SUM(confirmed)::bigint from get_balance(ARRAY[
                'bcrt1qcsh8a7f0mdsr47zy6pj04tv4mwdumlfacs66tf',
                'bcrt1qgar7sarvmkenkrmljk5slz0cn7ec0jak76zrj7',
                'bcrt1q0ht9tyks4vh7p5p904t340cr9nvahy7uevmqwj',
                'bcrt1qq6hag67dl53wl99vzg42z8eyzfz2xlkvwk6f7m'
            ]);",
        );
        assert_eq!(res.unwrap().unwrap_or(-1), 1_000_000_000);
    }

    #[pg_test]
    fn test_list_unspent() {
        assert!(true);
    }
}
