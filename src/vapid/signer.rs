use std::{
    collections::BTreeMap,
    time::{SystemTime, UNIX_EPOCH},
};

use http::uri::Uri;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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

/// JWT claims object. Custom claims are implemented as a map.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Claims {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aud: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(flatten)]
    pub custom: BTreeMap<String, Value>,
}

impl Claims {
    /// Creates claims with the default twelve hour expiry.
    pub(crate) fn with_default_expiry() -> Self {
        Claims {
            exp: Some(now() + 12 * 60 * 60),
            ..Claims::default()
        }
    }
}

/// Current unix timestamp in seconds.
pub(crate) fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
}

pub struct VapidSigner {}

impl VapidSigner {
    /// Create a signature with a given key. Sets the default audience from the
    /// endpoint host and sets the expiry in twelve hours. Values can be
    /// overwritten by adding the `aud` and `exp` claims.
    pub fn sign(key: VapidKey, endpoint: &Uri, mut claims: Claims) -> Result<VapidSignature, WebPushError> {
        if !claims.custom.contains_key("aud") {
            //Add audience if not provided.
            let audience = format!("{}://{}", endpoint.scheme_str().unwrap(), endpoint.host().unwrap());
            claims.aud = Some(audience);
        } else {
            //Use provided claims if given. This is here to avoid breaking changes.
            let aud = claims.custom.get("aud").unwrap().clone();
            //NOTE: This as_str is needed, else \" gets added around the string
            claims.aud = Some(aud.as_str().ok_or(WebPushError::InvalidClaims)?.to_string());
            claims.custom.remove("aud");
        }

        //Override the exp claim if provided in custom. Must then remove from custom to avoid printing
        //Twice, as this is just for backwards compatibility.
        if claims.custom.contains_key("exp") {
            let exp = claims.custom.get("exp").unwrap().clone();
            claims.exp = Some(now() + exp.as_u64().ok_or(WebPushError::InvalidClaims)?);
            claims.custom.remove("exp");
        }

        // Add sub if not provided as some browsers (like firefox) require it even though the API doesn't say its needed >:[
        if !claims.custom.contains_key("sub") {
            claims.sub = Some("mailto:example@example.com".to_string());
        }

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
