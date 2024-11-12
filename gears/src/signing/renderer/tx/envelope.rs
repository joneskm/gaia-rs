use crate::crypto::public::PublicKey;
use crate::signing::handler::MetadataGetter;
use crate::signing::{
    hasher::hash_get,
    renderer::value_renderer::{
        DefaultPrimitiveRenderer, PrimitiveValueRenderer, RenderError, TryPrimitiveValueRenderer,
        TryPrimitiveValueRendererWithMetadata, ValueRenderer,
    },
};
use crate::types::address::AccAddress;
use crate::types::auth::info::AuthInfo;
use crate::types::tx::body::TxBody;
use crate::types::{
    auth::tip::Tip,
    base::coins::UnsignedCoins,
    rendering::screen::{Indent, Screen},
    tx::{signer::SignerData, TxMessage},
};

use core_types::Protobuf;
use tendermint::types::chain_id::ChainId;

/// Envelope is an internal data structure used to generate the tx envelope
/// screens. Used in the same way as the Cosmos SDK Envelope type:
/// https://github.com/cosmos/cosmos-sdk/blob/main/x/tx/signing/textual/tx.go
#[derive(Debug)]
pub struct Envelope<M> {
    chain_id: ChainId,
    account_number: u64,
    sequence: u64,
    address: AccAddress,
    public_key: PublicKey,
    message: Vec<M>,
    memo: String,
    fees: Option<UnsignedCoins>,
    fee_payer: Option<AccAddress>,
    fee_granter: String, // TODO: this should be an AccAddress
    tip: Option<UnsignedCoins>,
    tipper: Option<AccAddress>,
    gas_limit: u64,
    timeout_height: u32,
    // TODO: need to add the fields below in:
    //other_signer: Vec<SignerInfo>,
    //extension_options: Vec<>,
    //non_critical_extension_options: Vec<>,
    hash_of_raw_bytes: String,
}

impl<M: TxMessage> Envelope<M> {
    pub fn new(signer_data: SignerData, body: &TxBody<M>, auth_info: &AuthInfo) -> Self {
        let body_bytes = body.encode_vec();
        let auth_info_bytes = auth_info.encode_vec();

        let (tip, tipper) = match auth_info.tip.to_owned() {
            Some(Tip { amount, tipper }) => (amount, Some(tipper)),
            None => (None, None),
        };

        Envelope {
            chain_id: signer_data.chain_id,
            account_number: signer_data.account_number,
            sequence: signer_data.sequence,
            address: signer_data.address,
            public_key: signer_data.pub_key,
            message: body.messages.to_owned().into_vec(),
            memo: body.memo.to_owned(),
            fees: auth_info.fee.amount.to_owned(),
            fee_payer: auth_info.fee.payer.to_owned(),
            fee_granter: auth_info.fee.granter.to_owned(),
            tip,
            tipper,
            gas_limit: auth_info.fee.gas_limit.into(),
            timeout_height: body.timeout_height,
            hash_of_raw_bytes: hash_get(&body_bytes, &auth_info_bytes),
        }
    }
}

// NOTE: fields with protobuf default values are not rendered to screens
impl<M: TxMessage + ValueRenderer> ValueRenderer for Envelope<M> {
    fn format<MG: MetadataGetter>(&self, get_metadata: &MG) -> Result<Vec<Screen>, RenderError> {
        let mut screens = vec![];

        screens.push(Screen {
            title: "Chain id".to_string(),
            content: DefaultPrimitiveRenderer::try_format(self.chain_id.clone().to_string())?,
            indent: None,
            expert: false,
        });
        if self.account_number != 0 {
            screens.push(Screen {
                title: "Account number".to_string(),
                content: DefaultPrimitiveRenderer::format(self.account_number),
                indent: None,
                expert: false,
            });
        }
        if self.sequence != 0 {
            screens.push(Screen {
                title: "Sequence".to_string(),
                content: DefaultPrimitiveRenderer::format(self.sequence),
                indent: None,
                expert: false,
            })
        };
        screens.push(Screen {
            title: "Address".to_string(),
            content: DefaultPrimitiveRenderer::format(self.address.to_owned()),
            indent: None,
            expert: true,
        });

        screens.append(&mut ValueRenderer::format(&self.public_key, get_metadata)?);

        let messages_count = self.message.len();

        screens.push(Screen {
            title: String::new(),
            content: DefaultPrimitiveRenderer::try_format(match messages_count {
                1 => "This transaction has 1 Message".to_string(),
                _ => format!("This transaction has {} Messages", messages_count),
            })
            .expect("hard coded Strings are not empty"),
            indent: None,
            expert: false,
        });

        for (i, ms) in self.message.iter().enumerate() {
            screens.push(Screen {
                title: format!("Message ({}/{messages_count})", i + 1),
                content: DefaultPrimitiveRenderer::try_format(ms.type_url().to_string())?,
                indent: Some(Indent::one()),
                expert: false,
            });
            screens.append(&mut ValueRenderer::format(ms, get_metadata)?);
        }
        screens.push(Screen {
            title: String::new(),
            content: DefaultPrimitiveRenderer::try_format("End of Message".to_string())
                .expect("hard coded String is not empty"),
            indent: None,
            expert: false,
        });

        if let Ok(memo) = DefaultPrimitiveRenderer::try_format(self.memo.clone()) {
            screens.push(Screen {
                title: "Memo".to_string(),
                content: memo,
                indent: None,
                expert: false,
            });
        }

        if let Some(amount) = &self.fees {
            screens.push(Screen {
                title: "Fees".to_string(),
                content: DefaultPrimitiveRenderer::try_format_with_metadata(
                    amount.to_owned(),
                    get_metadata,
                )?,
                indent: None,
                expert: false,
            });
        }

        if let Some(payer) = &self.fee_payer {
            screens.push(Screen {
                title: "Fee payer".to_string(),
                content: DefaultPrimitiveRenderer::format(payer.to_owned()),
                indent: None,
                expert: true,
            });
        }
        if let Ok(granter) = DefaultPrimitiveRenderer::try_format(self.fee_granter.to_owned()) {
            screens.push(Screen {
                title: "Fee granter".to_string(),
                content: granter,
                indent: None,
                expert: true,
            });
        }

        if let Some(amount) = &self.tip {
            screens.push(Screen {
                title: "Tip".to_string(),
                content: DefaultPrimitiveRenderer::try_format_with_metadata(
                    amount.to_owned(),
                    get_metadata,
                )?,
                indent: None,
                expert: false,
            });
        }

        if let Some(tipper) = &self.tipper {
            screens.push(Screen {
                title: "Tipper".to_string(),
                content: DefaultPrimitiveRenderer::format(tipper.to_owned()),
                indent: None,
                expert: false,
            });
        }

        if self.gas_limit != 0 {
            screens.push(Screen {
                title: "Gas limit".to_string(),
                content: DefaultPrimitiveRenderer::format(self.gas_limit),
                indent: None,
                expert: true,
            });
        }

        if self.timeout_height != 0 {
            screens.push(Screen {
                title: "Timeout height".to_string(),
                content: DefaultPrimitiveRenderer::format(self.timeout_height),
                indent: None,
                expert: true,
            });
        }

        screens.push(Screen {
            title: "Hash of raw bytes".to_string(),
            content: DefaultPrimitiveRenderer::try_format(self.hash_of_raw_bytes.clone())
                .expect("hash will not be empty"),
            indent: None,
            expert: true,
        });

        Ok(screens)
    }
}

#[cfg(test)]
mod tests {
    use crate::signing::renderer::test_functions::TestMetadataGetter;
    use crate::signing::renderer::value_renderer::ValueRenderer;
    use crate::types::address::AccAddress;
    use crate::types::auth::fee::Fee;
    use crate::types::auth::info::AuthInfo;
    use crate::types::base::coin::UnsignedCoin;
    use crate::types::base::coins::UnsignedCoins;
    use crate::types::denom::Denom;
    use crate::types::msg::send::MsgSend;
    use crate::types::rendering::screen::{Content, Indent, Screen};
    use crate::types::signing::SignerInfo;
    use crate::types::tx::body::TxBody;
    use crate::types::tx::signer::SignerData;
    use core_types::tx::mode_info::{ModeInfo, SignMode};
    use cosmwasm_std::Uint256;
    use extensions::testing::UnwrapTesting;
    use std::str::FromStr;
    use tendermint::types::chain_id::ChainId;
    use vec1::vec1;

    use super::Envelope;

    #[test]
    fn envelope_data_formatting() -> anyhow::Result<()> {
        let data = envelope_data_get()?;
        let expected_screens = expected_screens_get()?;

        let actual_screens = ValueRenderer::format(&data, &TestMetadataGetter)
            .map_err(|e| anyhow::anyhow!(e.to_string()))?;

        if expected_screens != actual_screens {
            let expected = serde_json::to_string(&expected_screens)?;
            let actual = serde_json::to_string(&actual_screens)?;
            panic!("Expected: {expected} \n !=\n Actual: {actual}")
        }

        Ok(())
    }

    fn envelope_data_get() -> anyhow::Result<Envelope<MsgSend>> {
        let signer_info = SignerInfo {
            public_key: Some(serde_json::from_str(
                r#"{
                        "@type": "/cosmos.crypto.secp256k1.PubKey",
                        "key": "Auvdf+T963bciiBe9l15DNMOijdaXCUo6zqSOvH7TXlN"
                    }"#,
            )?),

            mode_info: ModeInfo::Single(SignMode::Textual),
            sequence: 2,
        };

        let auth_info = AuthInfo {
            signer_infos: vec![signer_info],
            fee: Fee {
                amount: Some(
                    UnsignedCoins::new(vec![UnsignedCoin {
                        denom: Denom::try_from("uatom".to_owned())?,
                        amount: Uint256::from(2000u32),
                    }])
                    .unwrap_test(),
                ),
                gas_limit: 100000_u64.try_into().expect("this is a valid gas limit"),
                payer: None,
                granter: String::new(),
            },
            tip: None,
        };

        let signer_data = SignerData {
            address: AccAddress::from_bech32("cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs")?,
            chain_id: ChainId::from_str("my-chain").expect("this is a valid chain id"),
            account_number: 1,
            sequence: 2,
            pub_key: serde_json::from_str(
                r#"{
				"@type": "/cosmos.crypto.secp256k1.PubKey",
				"key": "Auvdf+T963bciiBe9l15DNMOijdaXCUo6zqSOvH7TXlN"
			}"#,
            )?,
        };

        let tx_body = TxBody::<MsgSend> {
            messages: vec1![MsgSend {
                from_address: AccAddress::from_bech32(
                    "cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs",
                )?,
                to_address: AccAddress::from_bech32(
                    "cosmos1ejrf4cur2wy6kfurg9f2jppp2h3afe5h6pkh5t",
                )?,
                amount: UnsignedCoins::new(vec![UnsignedCoin {
                    denom: Denom::try_from("uatom".to_string())?,
                    amount: Uint256::from(10000000u32),
                }])
                .unwrap_test(),
            }],
            memo: String::new(),
            timeout_height: 0,
            extension_options: Vec::new(),
            non_critical_extension_options: Vec::new(),
        };

        let data = Envelope::new(signer_data, &tx_body, &auth_info);

        Ok(data)
    }

    fn expected_screens_get() -> anyhow::Result<Vec<Screen>> {
        let screens = vec![
            Screen {
                title: "Chain id".to_string(),
                content: Content::try_new("my-chain".to_string())?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Account number".to_string(),
                content: Content::try_new(1.to_string())?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Sequence".to_string(),
                content: Content::try_new(2.to_string())?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Address".to_string(),
                content: Content::try_new("cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs")?,
                indent: None,
                expert: true,
            },
            Screen {
                title: "Public key".to_string(),
                content: Content::try_new("/cosmos.crypto.secp256k1.PubKey")?,
                indent: None,
                expert: true,
            },
            Screen {
                title: "Key".to_string(),
                content: Content::try_new( "02EB DD7F E4FD EB76 DC8A 205E F65D 790C D30E 8A37 5A5C 2528 EB3A 923A F1FB 4D79 4D" )?,
                indent: Some(Indent::one()),
                expert: true,
            },
            Screen {
                title: String::new(),
                content: Content::try_new("This transaction has 1 Message")?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Message (1/1)".to_string(),
                content: Content::try_new("/cosmos.bank.v1beta1.MsgSend")?,
                indent: Some(Indent::one()),
                expert: false,
            },
            Screen {
                title: "From address".to_string(),
                content: Content::try_new("cosmos1ulav3hsenupswqfkw2y3sup5kgtqwnvqa8eyhs")?,
                indent: Some(Indent::two()),
                expert: false,
            },
            Screen {
                title: "To address".to_string(),
                content: Content::try_new("cosmos1ejrf4cur2wy6kfurg9f2jppp2h3afe5h6pkh5t")?,
                indent: Some(Indent::two()),
                expert: false,
            },
            Screen {
                title: "Amount".to_string(),
                content: Content::try_new("10 ATOM")?,
                indent: Some(Indent::two()),
                expert: false,
            },
            Screen {
                title: String::new(),
                content: Content::try_new("End of Message")?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Fees".to_string(),
                content: Content::try_new("0.002 ATOM".to_string())?,
                indent: None,
                expert: false,
            },
            Screen {
                title: "Gas limit".to_string(),
                content: Content::try_new("100'000".to_string())?,
                indent: None,
                expert: true,
            },
            Screen {
                title: "Hash of raw bytes".to_string(),
                content: Content::try_new(
                    "785bd306ea8962cdb9600089bdd65f3dc029e1aea112dee69e19546c9adad86e",
                )?,
                indent: None,
                expert: true,
            },
        ];

        Ok(screens)
    }
}
