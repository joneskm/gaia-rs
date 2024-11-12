use ledger_cosmos::CosmosValidatorApp;

use crate::types::address::AccAddress;

use super::{
    keys::{GearsPublicKey, ReadAccAddress, SigningKey},
    public::PublicKey,
    secp256k1::Secp256k1PubKey,
};

pub type LedgerError = ledger_cosmos::Error;

/// Proxy structure between ledger device and rust code
pub struct LedgerProxyKey {
    app: CosmosValidatorApp,
    address: AccAddress,
    public_key: Secp256k1PubKey,
}

impl std::fmt::Debug for LedgerProxyKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LedgerProxyKey")
            .field("address", &self.address)
            .field("public_key", &self.public_key)
            .finish()
    }
}

impl LedgerProxyKey {
    pub fn new() -> Result<Self, LedgerError> {
        let app = CosmosValidatorApp::connect()?;
        let pub_key_raw = app.public_key_secp256k1()?;
        let public_key = Secp256k1PubKey::try_from(pub_key_raw.to_vec())
            .map_err(|_| ledger_cosmos::Error::InvalidPK)?;
        let address = public_key.get_address();
        Ok(Self {
            app,
            address,
            public_key,
        })
    }
}

impl ReadAccAddress for LedgerProxyKey {
    fn get_address(&self) -> AccAddress {
        self.address.clone()
    }
}

// TODO: find better name without gears part
impl GearsPublicKey for LedgerProxyKey {
    fn get_gears_public_key(&self) -> PublicKey {
        PublicKey::Secp256k1(self.public_key.clone())
    }
}

impl SigningKey for LedgerProxyKey {
    type Error = LedgerError;

    fn sign(&self, message: &[u8]) -> Result<Vec<u8>, Self::Error> {
        let der_sig = self.app.sign_v2(message)?;

        // convert signature from DER to BER
        let signature = secp256k1::ecdsa::Signature::from_der(&der_sig)
            .map_err(|_| ledger_cosmos::Error::InvalidPK)?;
        Ok(signature.serialize_compact().to_vec())
    }
}
