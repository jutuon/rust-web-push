use std::time::{SystemTime, UNIX_EPOCH};

use http::uri::Uri;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::Serialize;

use crate::{error::WebPushError, vapid::VapidKey};

/// A struct representing a VAPID signature. Should be generated using the
/// [VapidSignatureBuilder](struct.VapidSignatureBuilder.html).
#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct VapidSignature {
    /// The signed JWT, base64-encoded
    pub auth_t: String,
    /// The public key bytes
    pub auth_k: Vec<u8>,
}

/// JWT claims object.
#[derive(Debug, Serialize)]
struct Claims {
    pub aud: String,
    pub exp: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
}

/// Current unix timestamp in seconds.
fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

pub struct VapidSigner {}

impl VapidSigner {
    /// Create a signature with a given key. Sets the audience from the
    /// endpoint host and the expiry to twelve hours.
    pub fn sign(key: VapidKey, endpoint: &Uri, sub: Option<&str>) -> Result<VapidSignature, WebPushError> {
        let scheme = endpoint.scheme_str().ok_or(WebPushError::InvalidClaims)?;
        let host = endpoint.host().ok_or(WebPushError::InvalidClaims)?;

        let claims = Claims {
            aud: format!("{}://{}", scheme, host),
            exp: now() + 12 * 60 * 60,
            sub: sub.map(str::to_string),
        };

        log::trace!("Using jwt: {:?}", claims);

        let auth_k = key.public_key();

        //Generate JWT signature
        let encoding_key: EncodingKey = key.encoding_key()?;
        let header = Header::new(Algorithm::ES256);
        let auth_t = encode(&header, &claims, &encoding_key).map_err(|_| WebPushError::InvalidClaims)?;

        Ok(VapidSignature { auth_t, auth_k })
    }
}

#[cfg(test)]
mod tests {}
