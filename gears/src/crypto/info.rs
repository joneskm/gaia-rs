use core_types::{
    signing::SignDoc,
    tx::mode_info::{ModeInfo, SignMode},
    Protobuf,
};
use prost::Message;
use tendermint::types::chain_id::ChainId;

use std::{
    error::Error,
    fmt::{Display, Formatter},
};

use crate::{
    application::handlers::client::{MetadataViaRPC, NodeFetcher},
    signing::{
        errors::SigningErrors, handler::SignModeHandler, renderer::value_renderer::ValueRenderer,
    },
    types::{
        auth::{fee::Fee, info::AuthInfo, tip::Tip},
        signing::SignerInfo,
        tx::{body::TxBody, signer::SignerData, Tx, TxMessage},
    },
};

use super::keys::{GearsPublicKey, ReadAccAddress, SigningKey};

/// Contains info required to sign a Tx
#[derive(Debug)]
pub struct SigningInfo<'a, K> {
    pub key: &'a K,
    pub sequence: u64,
    pub account_number: u64,
}

pub fn create_signed_transaction_direct<M: TxMessage, K: SigningKey + GearsPublicKey>(
    signing_infos: Vec<SigningInfo<'_, K>>,
    chain_id: ChainId,
    fee: Fee,
    tip: Option<Tip>,
    body: TxBody<M>,
) -> Result<Tx<M>, K::Error> {
    let auth_info = auth_info(&signing_infos, fee, tip, Mode::Direct);

    let mut sign_doc = SignDoc {
        body_bytes: body.encode_vec(),
        auth_info_bytes: auth_info.encode_vec(),
        chain_id: chain_id.into(),
        account_number: 0, // This gets overwritten
    };

    let signatures = signing_infos
        .iter()
        .map(|s| {
            sign_doc.account_number = s.account_number;

            s.key.sign(&sign_doc.encode_to_vec())
        })
        .collect::<Result<Vec<Vec<u8>>, <K as crate::crypto::keys::SigningKey>::Error>>()?;

    Ok(Tx {
        body,
        auth_info,
        signatures,
        signatures_data: Vec::new(), // TODO: WHERE TO GET THOSE?
    })
    // Ok()
}

// NOTE: we can't implement From<K::Error> for this type
#[derive(Debug)]
pub enum TextualSigningError<K: SigningKey> {
    Rendering(SigningErrors),
    Key(K::Error),
}

impl<K: SigningKey + std::fmt::Debug> Error for TextualSigningError<K> {}

impl<K: SigningKey> Display for TextualSigningError<K> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            TextualSigningError::Rendering(e) => write!(f, "{}", e),
            TextualSigningError::Key(e) => write!(f, "{}", e),
        }
    }
}

pub fn create_signed_transaction_textual<
    M: TxMessage + ValueRenderer,
    K: SigningKey + ReadAccAddress + GearsPublicKey,
    F: NodeFetcher + Clone,
>(
    signing_infos: Vec<SigningInfo<'_, K>>,
    chain_id: ChainId,
    fee: Fee,
    tip: Option<Tip>,
    node: url::Url,
    body: TxBody<M>,
    fetcher: &F,
) -> Result<Tx<M>, TextualSigningError<K>> {
    let auth_info = auth_info(&signing_infos, fee, tip, Mode::Textual);

    let sign_mode_handler = SignModeHandler;

    let signatures = signing_infos
        .into_iter()
        .map(|s| {
            let signer_data = SignerData {
                address: s.key.get_address(),
                chain_id: chain_id.clone(),
                account_number: s.account_number,
                sequence: s.sequence,
                pub_key: s.key.get_gears_public_key(),
            };

            let sign_bytes = sign_mode_handler
                .sign_bytes_get(
                    &MetadataViaRPC {
                        node: node.clone(),
                        fetcher: fetcher.clone(),
                    },
                    signer_data,
                    &body,
                    &auth_info,
                )
                .map_err(|e| TextualSigningError::Rendering(e))?;

            s.key
                .sign(&sign_bytes)
                .map_err(|e| TextualSigningError::Key(e))
        })
        .collect::<Result<Vec<Vec<u8>>, TextualSigningError<K>>>()?;

    Ok(Tx {
        body,
        auth_info,
        signatures,
        signatures_data: Vec::new(), // TODO: WHERE TO GET THOSE?
    })
}

#[derive(Clone)]
enum Mode {
    Direct,
    Textual,
}

impl From<Mode> for SignMode {
    fn from(mode: Mode) -> Self {
        match mode {
            Mode::Direct => SignMode::Direct,
            Mode::Textual => SignMode::Textual,
        }
    }
}

fn auth_info<K: GearsPublicKey>(
    signing_infos: &[SigningInfo<'_, K>],
    fee: Fee,
    tip: Option<Tip>,
    mode: Mode,
) -> AuthInfo {
    let signer_infos: Vec<SignerInfo> = signing_infos
        .iter()
        .map(|s| {
            let public_key = Some(s.key.get_gears_public_key());

            SignerInfo {
                public_key,
                mode_info: ModeInfo::Single(mode.clone().into()),
                sequence: s.sequence,
            }
        })
        .collect();

    AuthInfo {
        signer_infos,
        fee,
        tip,
    }
}
