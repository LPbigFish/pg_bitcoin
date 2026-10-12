#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
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
