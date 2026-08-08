//! aws-lc-rs backed `Cryptographer` implementation for the `ece` crate.

use std::any::Any;

use aws_lc_rs::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_128_GCM},
    agreement::{self, UnparsedPublicKey},
    hkdf::{Salt, HKDF_SHA256},
    rand,
};
use ece::{
    crypto::{Cryptographer, EcKeyComponents, LocalKeyPair, RemotePublicKey},
    Error,
};

/// A remote public key stored in its raw uncompressed point representation.
struct AwsLcRsRemotePublicKey {
    raw: Vec<u8>,
}

impl RemotePublicKey for AwsLcRsRemotePublicKey {
    fn as_raw(&self) -> ece::Result<Vec<u8>> {
        Ok(self.raw.clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// A local P-256 key pair backed by aws-lc-rs.
struct AwsLcRsLocalKeyPair {
    private_key: agreement::PrivateKey,
}

impl AwsLcRsLocalKeyPair {
    fn generate() -> ece::Result<Self> {
        let private_key = agreement::PrivateKey::generate(&agreement::ECDH_P256).map_err(|_| Error::CryptoError)?;
        Ok(Self { private_key })
    }

    fn from_components(components: &EcKeyComponents) -> ece::Result<Self> {
        let private_key = agreement::PrivateKey::from_private_key(&agreement::ECDH_P256, components.private_key())
            .map_err(|_| Error::CryptoError)?;
        Ok(Self { private_key })
    }
}

impl LocalKeyPair for AwsLcRsLocalKeyPair {
    fn pub_as_raw(&self) -> ece::Result<Vec<u8>> {
        let public_key = self.private_key.compute_public_key().map_err(|_| Error::CryptoError)?;
        Ok(public_key.as_ref().to_vec())
    }

    fn raw_components(&self) -> ece::Result<EcKeyComponents> {
        use aws_lc_rs::encoding::{AsBigEndian, EcPrivateKeyBin};

        let public_key = self.private_key.compute_public_key().map_err(|_| Error::CryptoError)?;
        let private_key: EcPrivateKeyBin<'static> = self.private_key.as_be_bytes().map_err(|_| Error::CryptoError)?;
        Ok(EcKeyComponents::new(
            private_key.as_ref().to_vec(),
            public_key.as_ref().to_vec(),
        ))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The aws-lc-rs backed `Cryptographer`.
pub struct AwsLcRsCryptographer;

impl Cryptographer for AwsLcRsCryptographer {
    fn generate_ephemeral_keypair(&self) -> ece::Result<Box<dyn LocalKeyPair>> {
        Ok(Box::new(AwsLcRsLocalKeyPair::generate()?))
    }

    fn import_key_pair(&self, components: &EcKeyComponents) -> ece::Result<Box<dyn LocalKeyPair>> {
        Ok(Box::new(AwsLcRsLocalKeyPair::from_components(components)?))
    }

    fn import_public_key(&self, raw: &[u8]) -> ece::Result<Box<dyn RemotePublicKey>> {
        Ok(Box::new(AwsLcRsRemotePublicKey { raw: raw.to_vec() }))
    }

    fn compute_ecdh_secret(&self, remote: &dyn RemotePublicKey, local: &dyn LocalKeyPair) -> ece::Result<Vec<u8>> {
        let local = local
            .as_any()
            .downcast_ref::<AwsLcRsLocalKeyPair>()
            .ok_or(Error::CryptoError)?;
        let remote = remote
            .as_any()
            .downcast_ref::<AwsLcRsRemotePublicKey>()
            .ok_or(Error::CryptoError)?;

        let peer_public_key = UnparsedPublicKey::new(&agreement::ECDH_P256, &remote.raw);
        agreement::agree(&local.private_key, peer_public_key, Error::CryptoError, |secret| {
            Ok(secret.to_vec())
        })
    }

    fn hkdf_sha256(&self, salt: &[u8], secret: &[u8], info: &[u8], len: usize) -> ece::Result<Vec<u8>> {
        let prk = Salt::new(HKDF_SHA256, salt).extract(secret);
        let mut okm = vec![0u8; len];
        prk.expand(&[info], OkmLength(len))
            .map_err(|_| Error::CryptoError)?
            .fill(&mut okm)
            .map_err(|_| Error::CryptoError)?;
        Ok(okm)
    }

    fn aes_gcm_128_encrypt(&self, key: &[u8], iv: &[u8], data: &[u8]) -> ece::Result<Vec<u8>> {
        let unbound_key = UnboundKey::new(&AES_128_GCM, key).map_err(|_| Error::CryptoError)?;
        let sealing_key = LessSafeKey::new(unbound_key);
        let nonce = Nonce::try_assume_unique_for_key(iv).map_err(|_| Error::CryptoError)?;

        let mut in_out = data.to_vec();
        sealing_key
            .seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out)
            .map_err(|_| Error::CryptoError)?;
        Ok(in_out)
    }

    fn aes_gcm_128_decrypt(&self, key: &[u8], iv: &[u8], ciphertext_and_tag: &[u8]) -> ece::Result<Vec<u8>> {
        let unbound_key = UnboundKey::new(&AES_128_GCM, key).map_err(|_| Error::CryptoError)?;
        let opening_key = LessSafeKey::new(unbound_key);
        let nonce = Nonce::try_assume_unique_for_key(iv).map_err(|_| Error::CryptoError)?;

        let mut in_out = ciphertext_and_tag.to_vec();
        let plaintext = opening_key
            .open_in_place(nonce, Aad::empty(), &mut in_out)
            .map_err(|_| Error::CryptoError)?;
        Ok(plaintext.to_vec())
    }

    fn random_bytes(&self, dest: &mut [u8]) -> ece::Result<()> {
        rand::fill(dest).map_err(|_| Error::CryptoError)
    }
}

/// A `KeyType` for `hkdf::Prk::expand` that reports an arbitrary output length.
struct OkmLength(usize);

impl aws_lc_rs::hkdf::KeyType for OkmLength {
    fn len(&self) -> usize {
        self.0
    }
}
