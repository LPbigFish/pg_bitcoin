use bitcoin::{Address, Network, ScriptBuf};
use electrum_client::ElectrumApi;
use std::str::FromStr;

use crate::client_wrapper::{with_client, ElectrumError};

fn address_to_script(address: &str, network: Network) -> Result<ScriptBuf, ElectrumError> {
    Address::from_str(address)
        .and_then(|add| add.require_network(network))
        .map(|add| add.script_pubkey())
        .map_err(|_| ElectrumError::InvalidAddress(address.to_string()))
}

pub fn get_balance(address: &str) -> Result<(i64, i64), ElectrumError> {
    with_client(|client, network| {
        let script_hash = address_to_script(address, network)?;
        let r = client.script_get_balance(&script_hash)?;

        Ok((r.confirmed as i64, r.unconfirmed))
    })
}

pub fn get_balances<I>(
    addresses: I,
) -> Result<impl Iterator<Item = (String, i64, i64)>, ElectrumError>
where
    I: IntoIterator<Item = String>,
{
    let (scripts, balances) = with_client(|client, network| {
        let scripts = addresses
            .into_iter()
            .map(|address| {
                let script = address_to_script(&address, network)
                    .map_err(|_| ElectrumError::InvalidAddress(address.to_string()))?;
                Ok((address, script))
            })
            .collect::<Result<Vec<_>, ElectrumError>>()?;

        let script_refs = scripts.iter().map(|s| s.1.as_script());

        let balances = client.batch_script_get_balance(script_refs)?;

        Ok((scripts, balances))
    })?;

    let result = scripts
        .into_iter()
        .zip(balances)
        .map(|((address, _), balance)| (address, balance.confirmed as i64, balance.unconfirmed));

    Ok(result)
}

pub fn get_history(
    address: &str,
) -> Result<impl Iterator<Item = (i32, String, Option<i64>)>, ElectrumError> {
    let history = with_client(|client, network| {
        let scripthash = address_to_script(address, network)?;
        let r = client.script_get_history(&scripthash)?;

        Ok(r)
    })?;

    Ok(history.into_iter().map(|record| {
        (
            record.height,
            record.tx_hash.to_string(),
            record.fee.map(|fee| fee as i64),
        )
    }))
}

pub fn get_histories<I>(
    addresses: I,
) -> Result<impl Iterator<Item = (String, i32, String, Option<i64>)>, ElectrumError>
where
    I: IntoIterator<Item = String>,
{
    let (scripts, histories) = with_client(|client, network| {
        let scripts = addresses
            .into_iter()
            .map(|address| {
                let script = address_to_script(&address, network)
                    .map_err(|_| ElectrumError::InvalidAddress(address.to_string()))?;
                Ok((address, script))
            })
            .collect::<Result<Vec<_>, ElectrumError>>()?;

        let script_refs = scripts.iter().map(|s| s.1.as_script());

        let histories = client.batch_script_get_history(script_refs)?;

        Ok((scripts, histories))
    })?;

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
) -> Result<impl Iterator<Item = (i64, String, i64, i64)>, ElectrumError> {
    with_client(|client, network| {
        let scripthash = address_to_script(address, network)?;
        let uns = client.script_list_unspent(&scripthash)?;

        Ok(uns.into_iter().map(|r| {
            (
                r.height as i64,
                r.tx_hash.to_string(),
                r.tx_pos as i64,
                r.value as i64,
            )
        }))
    })
}
