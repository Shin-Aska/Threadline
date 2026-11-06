use crate::jose::{
    create_signed_jwt,
    jws::RegisteredHeader,
    jwt::{Claims, PublicClaims, RegisteredClaims},
};
use atrium_common::store::{Store, memory::MemoryStore};
use atrium_xrpc::{
    HttpClient,
    http::{Request, Response},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use jose_jwa::{Algorithm, Signing};
use jose_jwk::{Jwk, Key, crypto};
use rand::{
    rngs::SmallRng,
    {RngCore, SeedableRng},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use thiserror::Error;

const JWT_HEADER_TYP_DPOP: &str = "dpop+jwt";

#[derive(Deserialize)]
struct ErrorResponse {
    error: String,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("crypto error: {0:?}")]
    JwkCrypto(crypto::Error),
    #[error("key does not match any alg supported by the server")]
    UnsupportedKey,
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),
}

type Result<T> = core::result::Result<T, Error>;

pub struct DpopClient<T, S = MemoryStore<String, String>>
where
    S: Store<String, String>,
{
    inner: Arc<T>,
    pub(crate) key: Key,
    nonces: S,
    is_auth_server: bool,
}

impl<T> DpopClient<T> {
    pub fn new(
        key: Key,
        http_client: Arc<T>,
        is_auth_server: bool,
        supported_algs: &Option<Vec<String>>,
    ) -> Result<Self> {
        crate::p256_jwk::secret_key(&key).map_err(Error::JwkCrypto)?;
        if let Some(algs) = supported_algs {
            if !algs.iter().any(|alg| alg == "ES256") {
                return Err(Error::UnsupportedKey);
            }
        }
        let nonces = MemoryStore::<String, String>::default();
        Ok(Self { inner: http_client, key, nonces, is_auth_server })
    }
}

impl<T, S> DpopClient<T, S>
where
    S: Store<String, String>,
{
    fn build_proof(
        &self,
        htm: String,
        htu: String,
        ath: Option<String>,
        nonce: Option<String>,
    ) -> Result<String> {
        let secret_key = crate::p256_jwk::secret_key(&self.key).map_err(Error::JwkCrypto)?;
        let mut header = RegisteredHeader::from(Algorithm::Signing(Signing::Es256));
        header.typ = Some(JWT_HEADER_TYP_DPOP.into());
        header.jwk = Some(Jwk {
            key: crate::p256_jwk::public_jwk(&secret_key.public_key()),
            prm: Default::default(),
        });
        let claims = Claims {
            registered: RegisteredClaims {
                jti: Some(Self::generate_jti()),
                iat: Some(Utc::now().timestamp()),
                ..Default::default()
            },
            public: PublicClaims { htm: Some(htm), htu: Some(htu), ath, nonce },
        };
        Ok(create_signed_jwt(secret_key.into(), header.into(), claims)?)
    }

    fn is_use_dpop_nonce_error(&self, response: &Response<Vec<u8>>) -> bool {
        // https://datatracker.ietf.org/doc/html/rfc9449#name-authorization-server-provid
        if self.is_auth_server {
            if response.status() == 400 {
                if let Ok(res) = serde_json::from_slice::<ErrorResponse>(response.body()) {
                    return res.error == "use_dpop_nonce";
                };
            }
        }
        // https://datatracker.ietf.org/doc/html/rfc6750#section-3
        // https://datatracker.ietf.org/doc/html/rfc9449#name-resource-server-provided-no
        else if response.status() == 401 {
            if let Some(www_auth) =
                response.headers().get("WWW-Authenticate").and_then(|v| v.to_str().ok())
            {
                return www_auth.starts_with("DPoP")
                    && www_auth.contains(r#"error="use_dpop_nonce""#);
            }
        }
        false
    }
    // https://datatracker.ietf.org/doc/html/rfc9449#section-4.2
    fn generate_jti() -> String {
        let mut rng = SmallRng::from_entropy();
        let mut bytes = [0u8; 12];
        rng.fill_bytes(&mut bytes);
        URL_SAFE_NO_PAD.encode(bytes)
    }
}

impl<T, S> HttpClient for DpopClient<T, S>
where
    T: HttpClient + Send + Sync + 'static,
    S: Store<String, String> + Send + Sync + 'static,
    S::Error: std::error::Error + Send + Sync + 'static,
{
    async fn send_http(
        &self,
        mut request: Request<Vec<u8>>,
    ) -> core::result::Result<Response<Vec<u8>>, Box<dyn std::error::Error + Send + Sync + 'static>>
    {
        let uri = request.uri();
        let nonce_key = uri.authority().unwrap().to_string();
        let htm = request.method().to_string();
        let htu = uri.to_string();
        // https://datatracker.ietf.org/doc/html/rfc9449#section-4.2
        let ath = request
            .headers()
            .get("Authorization")
            .filter(|v| v.to_str().is_ok_and(|s| s.starts_with("DPoP ")))
            .map(|auth| URL_SAFE_NO_PAD.encode(Sha256::digest(&auth.as_bytes()[5..])));

        let init_nonce = self.nonces.get(&nonce_key).await?;
        let init_proof =
            self.build_proof(htm.clone(), htu.clone(), ath.clone(), init_nonce.clone())?;
        request.headers_mut().insert("DPoP", init_proof.parse()?);
        let response = self.inner.send_http(request.clone()).await?;

        let next_nonce =
            response.headers().get("DPoP-Nonce").and_then(|v| v.to_str().ok()).map(String::from);
        match &next_nonce {
            Some(s) if next_nonce != init_nonce => {
                // Store the fresh nonce for future requests
                self.nonces.set(nonce_key, s.clone()).await?;
            }
            _ => {
                // No nonce was returned or it is the same as the one we sent. No need to
                // update the nonce store, or retry the request.
                return Ok(response);
            }
        }

        if !self.is_use_dpop_nonce_error(&response) {
            return Ok(response);
        }
        let next_proof = self.build_proof(htm, htu, ath, next_nonce)?;
        request.headers_mut().insert("DPoP", next_proof.parse()?);
        let response = self.inner.send_http(request).await?;
        Ok(response)
    }
}

impl<T> Clone for DpopClient<T> {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            key: self.key.clone(),
            nonces: self.nonces.clone(),
            is_auth_server: self.is_auth_server,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::signature::{ECDSA_P256_SHA256_FIXED, UnparsedPublicKey};
    use serde_json::Value;
    use std::sync::Mutex;

    fn verify_jwt(jwt: &str) -> (Value, Value) {
        let parts: Vec<_> = jwt.split('.').collect();
        assert_eq!(parts.len(), 3);
        let header: Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
        let claims = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
        let key = &header["jwk"];
        assert_eq!(header["alg"], "ES256");
        assert_eq!(header["typ"], "dpop+jwt");
        assert_eq!(key["crv"], "P-256");
        assert!(key.get("d").is_none());
        let mut public = vec![4];
        public.extend(URL_SAFE_NO_PAD.decode(key["x"].as_str().unwrap()).unwrap());
        public.extend(URL_SAFE_NO_PAD.decode(key["y"].as_str().unwrap()).unwrap());
        let signature = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
        assert_eq!(signature.len(), 64);
        UnparsedPublicKey::new(&ECDSA_P256_SHA256_FIXED, public)
            .verify(format!("{}.{}", parts[0], parts[1]).as_bytes(), &signature)
            .unwrap();
        (header, claims)
    }

    #[test]
    fn saved_and_generated_keys_sign_interoperable_proofs_without_disclosing_secrets() {
        for key in
            [crate::tests::dpop_key(), crate::utils::generate_key(&["ES256".into()]).unwrap()]
        {
            let saved = serde_json::to_string(&key).unwrap();
            let restored = serde_json::from_str(&saved).unwrap();
            let client = DpopClient::new(restored, Arc::new(()), false, &None).unwrap();
            let proof = client
                .build_proof(
                    "POST".into(),
                    "https://pds.example/xrpc".into(),
                    Some("hash".into()),
                    Some("nonce".into()),
                )
                .unwrap();
            let (header, claims) = verify_jwt(&proof);
            let Key::Ec(ec) = key else { panic!("expected EC") };
            assert_eq!(header["jwk"]["x"], URL_SAFE_NO_PAD.encode(&ec.x));
            assert_eq!(header["jwk"]["y"], URL_SAFE_NO_PAD.encode(&ec.y));
            assert_eq!(claims["htm"], "POST");
            assert_eq!(claims["htu"], "https://pds.example/xrpc");
            assert_eq!(claims["ath"], "hash");
            assert_eq!(claims["nonce"], "nonce");
            assert!(claims["iat"].as_i64().is_some());
            assert!(claims["jti"].as_str().is_some_and(|jti| !jti.is_empty()));
        }
    }

    #[test]
    fn unsupported_or_public_only_keys_fail_before_a_request() {
        let Key::Ec(mut ec) = crate::tests::dpop_key() else { panic!("expected EC") };
        ec.d = None;
        assert!(matches!(
            DpopClient::new(Key::Ec(ec), Arc::new(()), false, &None),
            Err(Error::JwkCrypto(crypto::Error::NotPrivate))
        ));
        assert!(matches!(
            DpopClient::new(
                crate::tests::dpop_key(),
                Arc::new(()),
                false,
                &Some(vec!["RS256".into()])
            ),
            Err(Error::UnsupportedKey)
        ));
        let unsupported_key = serde_json::from_str(r#"{"kty":"oct","k":"AQID"}"#).unwrap();
        assert!(matches!(
            DpopClient::new(unsupported_key, Arc::new(()), false, &Some(vec!["ES256".into()])),
            Err(Error::JwkCrypto(crypto::Error::Unsupported))
        ));
    }

    struct ChallengeClient {
        proofs: Mutex<Vec<String>>,
        auth_server: bool,
    }

    impl HttpClient for ChallengeClient {
        async fn send_http(
            &self,
            request: Request<Vec<u8>>,
        ) -> core::result::Result<
            Response<Vec<u8>>,
            Box<dyn std::error::Error + Send + Sync + 'static>,
        > {
            let mut proofs = self.proofs.lock().unwrap();
            proofs.push(request.headers()["DPoP"].to_str().unwrap().to_owned());
            if proofs.len() == 1 {
                Ok(Response::builder()
                    .status(if self.auth_server { 400 } else { 401 })
                    .header("DPoP-Nonce", "server-nonce")
                    .header("WWW-Authenticate", r#"DPoP error="use_dpop_nonce""#)
                    .body(br#"{"error":"use_dpop_nonce"}"#.to_vec())
                    .unwrap())
            } else {
                Ok(Response::new(Vec::new()))
            }
        }
    }

    #[tokio::test]
    async fn authorization_and_resource_nonce_retries_keep_the_same_signing_key() {
        for auth_server in [true, false] {
            let transport =
                Arc::new(ChallengeClient { proofs: Mutex::new(Vec::new()), auth_server });
            let client = DpopClient::new(
                crate::tests::dpop_key(),
                Arc::clone(&transport),
                auth_server,
                &None,
            )
            .unwrap();
            let response = client
                .send_http(
                    Request::builder()
                        .method("POST")
                        .uri("https://issuer.example/token")
                        .header("Authorization", "DPoP access-token")
                        .body(Vec::new())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), 200);
            let proofs = transport.proofs.lock().unwrap();
            assert_eq!(proofs.len(), 2);
            let (first_header, first_claims) = verify_jwt(&proofs[0]);
            let (retry_header, retry_claims) = verify_jwt(&proofs[1]);
            assert_eq!(first_header["jwk"], retry_header["jwk"]);
            assert!(first_claims.get("nonce").is_none());
            assert_eq!(retry_claims["nonce"], "server-nonce");
            assert_ne!(first_claims["jti"], retry_claims["jti"]);
            assert_eq!(
                retry_claims["ath"],
                URL_SAFE_NO_PAD.encode(Sha256::digest(b"access-token"))
            );
        }
    }
}
