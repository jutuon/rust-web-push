use jsonwebtoken::EncodingKey;
use p256::{ecdsa::SigningKey, pkcs8::EncodePrivateKey, SecretKey};

use crate::error::WebPushError;

/// The P256 curve key pair used for VAPID ECDHSA.
#[derive(Clone)]
pub struct VapidKey {
    private_key: SecretKey,
}

impl VapidKey {
    pub fn new(private_key: SecretKey) -> VapidKey {
        VapidKey { private_key }
    }

    /// Gets the uncompressed public key bytes derived from this private key.
    pub fn public_key(&self) -> Vec<u8> {
        SigningKey::from(&self.private_key)
            .verifying_key()
            .to_sec1_bytes()
            .to_vec()
    }

    /// Builds a jsonwebtoken `EncodingKey` from the private key.
    pub(crate) fn encoding_key(&self) -> Result<EncodingKey, WebPushError> {
        let der = self
            .private_key
            .to_pkcs8_der()
            .map_err(|_| WebPushError::InvalidCryptoKeys)?;
        Ok(EncodingKey::from_ec_der(der.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;

    use crate::vapid::key::VapidKey;

    #[test]
    /// Tests that VapidKey derives the correct public key.
    fn test_public_key_derivation() {
        let f = File::open("resources/vapid_test_key.pem").unwrap();
        let key = crate::VapidSignatureBuilder::read_pem(f).unwrap();
        let key = VapidKey::new(key);

        assert_eq!(
            vec![
                4, 202, 53, 30, 162, 133, 234, 201, 12, 101, 140, 164, 174, 215, 189, 118, 234, 152, 192, 16, 244, 242,
                96, 208, 41, 59, 167, 70, 66, 93, 15, 123, 19, 39, 209, 62, 203, 35, 122, 176, 153, 79, 89, 58, 74, 54,
                26, 126, 203, 98, 158, 75, 170, 0, 52, 113, 126, 171, 124, 55, 237, 176, 165, 111, 181
            ],
            key.public_key()
        );
    }

    #[test]
    /// Tests that VapidKey clones properly.
    fn test_key_clones() {
        let f = File::open("resources/vapid_test_key.pem").unwrap();
        let key = crate::VapidSignatureBuilder::read_pem(f).unwrap();
        let key = VapidKey::new(key);

        let key2 = key.clone();

        assert_eq!(key.private_key, key2.private_key)
    }
}
