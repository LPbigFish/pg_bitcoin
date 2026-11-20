use bitcoin::Network;
use pgrx::{GucContext, GucFlags, GucRegistry, GucSetting, PostgresGucEnum};
use std::ffi::CString;

#[derive(PostgresGucEnum, Debug, Clone, Copy)]
pub enum NetworkType {
    #[name = c"mainnet"]
    MAINNET,
    #[name = c"regtest"]
    REGTEST,
    #[name = c"testnet"]
    TESTNET,
    #[name = c"testnet4"]
    TESTNET4,
    #[name = c"signet"]
    SIGNET,
}

trait ToBitcoinNetwork {
    fn to_bitcoin_network(&self) -> Network;
}

impl ToBitcoinNetwork for NetworkType {
    fn to_bitcoin_network(&self) -> Network {
        match &self {
            NetworkType::MAINNET => Network::Bitcoin,
            NetworkType::REGTEST => Network::Regtest,
            NetworkType::SIGNET => Network::Signet,
            NetworkType::TESTNET => Network::Testnet,
            NetworkType::TESTNET4 => Network::Testnet4,
        }
    }
}

pub static ELECTRUM_URL: GucSetting<Option<CString>> =
    GucSetting::<Option<CString>>::new(Some(c"tcp://127.0.0.1:60401"));

pub static NETWORK: GucSetting<NetworkType> = GucSetting::<NetworkType>::new(NetworkType::REGTEST);

pub fn init() {
    GucRegistry::define_string_guc(
        c"pg_bitcoin.electrum_url",
        c"Electrum connection URL",
        c"URL of the server. Example: tcp://127.0.0.1:60401",
        &ELECTRUM_URL,
        GucContext::Suset,
        GucFlags::default(),
    );

    GucRegistry::define_enum_guc(
        c"pg_bitcoin.network_type",
        c"Network type on electrum",
        c"Specify network type of the electrum server. (mainnet, regtest, testnet, signet)",
        &NETWORK,
        GucContext::Suset,
        GucFlags::default(),
    );
}

#[derive(Clone, PartialEq, Eq)]
pub struct ExtensionConfiguration(Option<String>, Network);

impl ExtensionConfiguration {
    pub fn current() -> Self {
        Self(
            ELECTRUM_URL
                .get()
                .map_or(None, |cstring| cstring.into_string().ok()),
            NETWORK.get().to_bitcoin_network(),
        )
    }

    pub fn get_url(&self) -> Option<String> {
        self.0.clone()
    }

    pub fn get_network(&self) -> Network {
        self.1
    }
}
