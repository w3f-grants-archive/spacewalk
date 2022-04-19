use frame_support::error::LookupError;
use sp_core::ed25519;
use sp_runtime::traits::{IdentifyAccount, StaticLookup};
use sp_runtime::{AccountId32, MultiSigner};
use stellar::IntoAccountId;
use substrate_stellar_sdk as stellar;

pub struct AddressConversion;

impl StaticLookup for AddressConversion {
    type Source = AccountId32;
    type Target = stellar::PublicKey;

    fn lookup(key: Self::Source) -> Result<Self::Target, LookupError> {
        let key = match key.into_account_id() {
            Ok(k @ stellar::PublicKey::PublicKeyTypeEd25519(_)) => k,
            Err(_) => {
                return Err(LookupError);
            }
        };
        
        Ok(key)
    }

    fn unlookup(stellar_addr: stellar::PublicKey) -> Self::Source {
        MultiSigner::Ed25519(ed25519::Public::from_raw(*stellar_addr.as_binary())).into_account()
    }
}