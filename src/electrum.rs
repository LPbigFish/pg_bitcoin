use std::sync::OnceLock;

use bitcoin::{Address, ScriptBuf};
use electrum_client::{Client, ElectrumApi};
use std::str::FromStr;

#[derive(thiserror::Error, Debug)]
pub enum ElectrumError {
    #[error("Invalid Bitcoin address {0}")]
    InvalidAddress(String),
    #[error("Electrum client unavailable")]
    ClientUnavailable,
    #[error("Electrum client error: {0}")]
    ClientError(#[from] electrum_client::Error),
}

static CLIENT: OnceLock<Option<Client>> = OnceLock::new();

pub fn get_client() -> &'static Option<Client> {
    CLIENT.get_or_init(|| Client::new("tcp://127.0.0.1:60401").ok())
}

fn address_to_script(address: &str) -> Result<ScriptBuf, ElectrumError> {
    Address::from_str(address)
        .map(|add| add.assume_checked())
        .map(|add| add.script_pubkey())
        .map_err(|_| ElectrumError::InvalidAddress(address.to_string()))
}

pub fn get_balance(address: &str) -> Result<(i64, i64), ElectrumError> {
    let client = get_client()
        .as_ref()
        .ok_or(ElectrumError::ClientUnavailable)?;
    let script_hash = address_to_script(address)?;
    let result = client.script_get_balance(&script_hash);
    let tuple = result
        .map(|res| (res.confirmed as i64, res.unconfirmed))
        .map_err(ElectrumError::ClientError)?;
    Ok(tuple)
}

pub fn get_balances<I>(
    addresses: I,
) -> Result<impl Iterator<Item = (String, i64, i64)>, ElectrumError>
where
    I: IntoIterator<Item = String>,
{
    let client = get_client()
        .as_ref()
        .ok_or(ElectrumError::ClientUnavailable)?;

    let scripts = addresses
        .into_iter()
        .map(|address| {
            let script = address_to_script(&address)
                .map_err(|_| ElectrumError::InvalidAddress(address.to_string()))?;
            Ok((address, script))
        })
        .collect::<Result<Vec<_>, ElectrumError>>()?;

    let script_refs = scripts.iter().map(|s| s.1.as_script());

    let balances = client
        .batch_script_get_balance(script_refs)
        .map_err(ElectrumError::ClientError)?;

    let result = scripts
        .into_iter()
        .zip(balances)
        .map(|((address, _), balance)| (address, balance.confirmed as i64, balance.unconfirmed));

    Ok(result)
}

pub fn get_history(
    address: &str,
) -> Result<impl Iterator<Item = (i32, String, Option<i64>)>, ElectrumError> {
    let client = get_client()
        .as_ref()
        .ok_or(ElectrumError::ClientUnavailable)?;

    let scripthash = address_to_script(address)?;

    client
        .script_get_history(&scripthash)
        .map(|history| {
            history.into_iter().map(|record| {
                (
                    record.height,
                    record.tx_hash.to_string(),
                    record.fee.map(|fee| fee as i64),
                )
            })
        })
        .map_err(ElectrumError::ClientError)
}

pub fn get_histories<I>(
    addresses: I,
) -> Result<impl Iterator<Item = (String, i32, String, Option<i64>)>, ElectrumError>
where
    I: IntoIterator<Item = String>,
{
    let client = get_client()
        .as_ref()
        .ok_or(ElectrumError::ClientUnavailable)?;

    let scripts = addresses
        .into_iter()
        .map(|address| {
            let script = address_to_script(&address)
                .map_err(|_| ElectrumError::InvalidAddress(address.to_string()))?;
            Ok((address, script))
        })
        .collect::<Result<Vec<_>, ElectrumError>>()?;

    let script_refs = scripts.iter().map(|s| s.1.as_script());

    let histories = client
        .batch_script_get_history(script_refs)
        .map_err(ElectrumError::ClientError)?;

    let result = scripts
        .into_iter()
        .zip(histories)
        .flat_map(|((address, _), history)| {
            history.into_iter().map(move |record| {
                (
                    address.clone(),
                    record.height,
                    record.tx_hash.to_string(),
                    record.fee.map(|fee| fee as i64),
                )
            })
        });

    Ok(result)
}

pub fn list_unspent(
    address: &str,
) -> Result<impl Iterator<Item = (String, u32, i64)>, ElectrumError> {
    let client = get_client()
        .as_ref()
        .ok_or(ElectrumError::ClientUnavailable)?;

    let scripthash = address_to_script(address)?;

    client
        .script_list_unspent(&scripthash)
        .map(|unspent_vec| {
            unspent_vec
                .into_iter()
                .map(|r| (r.tx_hash.to_string(), r.tx_pos as u32, r.value as i64))
        })
        .map_err(ElectrumError::ClientError)
}
