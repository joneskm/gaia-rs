use core_types::address::AccAddress;
use ledger_cosmos::CosmosValidatorApp;

use super::{
    keys::{GearsPublicKey, ReadAccAddress, SigningKey},
    public::PublicKey,
    secp256k1::Secp256k1PubKey,
};

pub type LedgerError = ledger_cosmos::Error;

pub struct LedgerProxyKey {
    app: CosmosValidatorApp,
    address: AccAddress,
    public_key: Secp256k1PubKey,
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
