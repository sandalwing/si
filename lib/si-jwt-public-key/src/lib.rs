use core::str;
use si_events::{UserPk, WorkspacePk};
use si_std::CanonicalFile;
use std::sync::Arc;

use base64::{engine::general_purpose, Engine};
use jwt_simple::{common::VerificationOptions, prelude::*};
use serde::{Deserialize, Serialize};
use telemetry::prelude::*;
use thiserror::Error;
use tokio::{fs, io::AsyncReadExt, task::JoinError};

#[remain::sorted]
#[derive(Error, Debug)]
pub enum JwtPublicSigningKeyError {
    #[error("failed to decode base64 string: {0}")]
    Base64Decode(#[from] base64::DecodeError),
    #[error("invalid bearer token")]
    BearerToken,
    #[error("error creating jwt from config")]
    FromConfig,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JWT error: {0}")]
    Jwt(#[from] jwt_simple::Error),
    #[error("{0}")]
    TaskJoin(#[from] JoinError),
    #[error("Unsupported JWT signing algorithm: {0}")]
    UnsupportedAlgo(String),
    #[error("failed to build string from utf8: {0}")]
    Utf8(#[from] std::str::Utf8Error),
    #[error("failure to verify token: {0}")]
    Verify(String),
    #[error("failure to verify against secondary token: first error: {0}, second error: {1}")]
    VerifySecondaryFail(String, String),
}

pub type JwtKeyResult<T> = Result<T, JwtPublicSigningKeyError>;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct JwtConfig {
    pub key_file: Option<CanonicalFile>,
    pub key_base64: Option<String>,
    pub algo: JwtAlgo,
}

impl JwtConfig {
    pub async fn to_pem(self) -> JwtKeyResult<String> {
        Ok(match (self.key_file.as_ref(), self.key_base64.as_deref()) {
            (None, Some(key_base64)) => {
                let buf = general_purpose::STANDARD.decode(key_base64)?;
                str::from_utf8(&buf)?.to_string()
            }
            (Some(key_file), None) => {
                let mut file = fs::File::open(key_file).await?;
                let mut buf = String::new();
                file.read_to_string(&mut buf).await?;

                buf
            }
            _ => Err(JwtPublicSigningKeyError::FromConfig)?,
        })
    }

    pub async fn into_verify(self) -> JwtKeyResult<Arc<dyn JwtPublicKeyVerify>> {
        let algo = self.algo;
        let pem = self.to_pem().await?;

        Ok(match algo {
            JwtAlgo::ES256 => {
                Arc::new(ES256PublicKey::from_pem(&pem)?) as Arc<dyn JwtPublicKeyVerify>
            }
            JwtAlgo::RS256 => {
                Arc::new(RS256PublicKey::from_pem(&pem)?) as Arc<dyn JwtPublicKeyVerify>
            }
        })
    }
}

#[derive(Deserialize, Serialize, Debug, Clone, Eq, PartialEq)]
pub struct SiJwtClaims {
    pub user_pk: UserPk,
    pub workspace_pk: WorkspacePk,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub enum JwtAlgo {
    #[default]
    ES256,
    RS256,
}

pub trait JwtPublicKeyVerify: std::fmt::Debug + Send + Sync {
    fn algo(&self) -> JwtAlgo;
    fn verify(
        &self,
        token: &str,
        options: Option<VerificationOptions>,
    ) -> JwtKeyResult<JWTClaims<SiJwtClaims>>;
}

impl JwtPublicKeyVerify for RS256PublicKey {
    fn algo(&self) -> JwtAlgo {
        JwtAlgo::RS256
    }

    fn verify(
        &self,
        token: &str,
        options: Option<VerificationOptions>,
    ) -> JwtKeyResult<JWTClaims<SiJwtClaims>> {
        self.verify_token(token, options)
            .map_err(|err| JwtPublicSigningKeyError::Verify(format!("{err}")))
    }
}

impl JwtPublicKeyVerify for ES256PublicKey {
    fn algo(&self) -> JwtAlgo {
        JwtAlgo::ES256
    }

    fn verify(
        &self,
        token: &str,
        options: Option<VerificationOptions>,
    ) -> JwtKeyResult<JWTClaims<SiJwtClaims>> {
        self.verify_token(token, options)
            .map_err(|err| JwtPublicSigningKeyError::Verify(format!("{err}")))
    }
}

#[derive(Clone, Debug)]
pub struct JwtPublicSigningKeyChain {
    primary: Arc<dyn JwtPublicKeyVerify>,
    secondary: Option<Arc<dyn JwtPublicKeyVerify>>,
}

impl JwtPublicSigningKeyChain {
    pub async fn from_config(
        primary: JwtConfig,
        secondary: Option<JwtConfig>,
    ) -> JwtKeyResult<Self> {
        Ok(Self {
            primary: primary.into_verify().await?,
            secondary: match secondary {
                Some(jwt_cfg) => Some(jwt_cfg.into_verify().await?),
                None => None,
            },
        })
    }

    /// Attempt to verify that this token was signed by either the primary or
    /// secondary key(s)
    pub fn verify_token(
        &self,
        token: &str,
        options: Option<VerificationOptions>,
    ) -> JwtKeyResult<JWTClaims<SiJwtClaims>> {
        match self.primary.verify(token, options.clone()) {
            Ok(claims) => Ok(claims),
            Err(err) => match self.secondary.as_ref() {
                Some(secondary) => match secondary.verify(token, options) {
                    Ok(claims) => Ok(claims),
                    Err(second_err) => Err(JwtPublicSigningKeyError::VerifySecondaryFail(
                        err.to_string(),
                        second_err.to_string(),
                    )),
                },
                None => Err(err),
            },
        }
    }
}

#[instrument(level = "debug", skip_all)]
pub async fn validate_bearer_token(
    public_key: JwtPublicSigningKeyChain,
    bearer_token: impl AsRef<str>,
) -> JwtKeyResult<JWTClaims<SiJwtClaims>> {
    let bearer_token = bearer_token.as_ref();
    let token = if let Some(token) = bearer_token.strip_prefix("Bearer ") {
        token.to_string()
    } else {
        return Err(JwtPublicSigningKeyError::BearerToken);
    };

    let claims =
        tokio::task::spawn_blocking(move || public_key.verify_token(&token, None)).await??;

    Ok(claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn validate_with_primary_rs256() {
        println!("generating key...");
        let key_pair = RS256KeyPair::generate(2048).expect("generate key pair");
        println!("done");

        let pub_key = key_pair.public_key();
        let pub_key_pem = pub_key.to_pem().expect("get pub key pem");
        let pub_key_base64 = general_purpose::STANDARD.encode(pub_key_pem);

        let si_claim = SiJwtClaims {
            user_pk: UserPk::generate(),
            workspace_pk: WorkspacePk::generate(),
        };

        let claims = JWTClaims {
            issued_at: None,
            expires_at: None,
            invalid_before: None,
            issuer: None,
            subject: None,
            audiences: None,
            jwt_id: None,
            nonce: None,
            custom: si_claim.clone(),
        };

        let signed = key_pair.sign(claims).expect("sign the key");
        let bearer_token = format!("Bearer {signed}");

        let primary_cfg = JwtConfig {
            key_file: None,
            key_base64: Some(pub_key_base64),
            algo: JwtAlgo::RS256,
        };

        let key_chain = JwtPublicSigningKeyChain::from_config(primary_cfg, None)
            .await
            .expect("make key chain");

        let claims = validate_bearer_token(key_chain, &bearer_token)
            .await
            .expect("shoudl validate");

        assert_eq!(si_claim, claims.custom);
    }

    #[tokio::test]
    async fn validate_with_primary_es256() {
        println!("generating key...");
        let key_pair = ES256KeyPair::generate();
        let key_pair_2 = ES256KeyPair::generate();
        println!("done");

        let pub_key = key_pair.public_key();
        let pub_key_pem = pub_key.to_pem().expect("get pub key pem");
        let pub_key_base64 = general_purpose::STANDARD.encode(pub_key_pem);

        let si_claim = SiJwtClaims {
            user_pk: UserPk::generate(),
            workspace_pk: WorkspacePk::generate(),
        };

        let claims = JWTClaims {
            issued_at: None,
            expires_at: None,
            invalid_before: None,
            issuer: None,
            subject: None,
            audiences: None,
            jwt_id: None,
            nonce: None,
            custom: si_claim.clone(),
        };

        let signed = key_pair.sign(claims.clone()).expect("sign the key");
        let bearer_token = format!("Bearer {signed}");

        let primary_cfg = JwtConfig {
            key_file: None,
            key_base64: Some(pub_key_base64),
            algo: JwtAlgo::ES256,
        };

        let key_chain = JwtPublicSigningKeyChain::from_config(primary_cfg, None)
            .await
            .expect("make key chain");

        let claims = validate_bearer_token(key_chain.clone(), &bearer_token)
            .await
            .expect("shoudl validate");

        assert_eq!(si_claim, claims.custom);

        // Just confirm it fails with the wrong key
        let signed_bad = key_pair_2.sign(claims).expect("sign the key");
        let bearer_bad = format!("Bearer {signed_bad}");
        let result = validate_bearer_token(key_chain, &bearer_bad).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn validate_with_secondary_rs256() {
        println!("generating keys...");
        let key_pair_es256 = ES256KeyPair::generate();
        let key_pair_rs256 = RS256KeyPair::generate(2048).expect("generate rs256 key");
        println!("done");

        let pub_key_es256 = key_pair_es256.public_key();
        let pub_key_pem = pub_key_es256.to_pem().expect("get pub key pem");
        let pub_key_base64_es256 = general_purpose::STANDARD.encode(pub_key_pem);

        let pub_key_rs256 = key_pair_rs256.public_key();
        let pub_key_pem = pub_key_rs256.to_pem().expect("get pub key pem");
        let pub_key_base64_rs256 = general_purpose::STANDARD.encode(pub_key_pem);

        let si_claim = SiJwtClaims {
            user_pk: UserPk::generate(),
            workspace_pk: WorkspacePk::generate(),
        };

        let claims = JWTClaims {
            issued_at: None,
            expires_at: None,
            invalid_before: None,
            issuer: None,
            subject: None,
            audiences: None,
            jwt_id: None,
            nonce: None,
            custom: si_claim.clone(),
        };

        let signed = key_pair_rs256.sign(claims.clone()).expect("sign the key");
        let bearer_token = format!("Bearer {signed}");

        let primary_cfg = JwtConfig {
            key_file: None,
            key_base64: Some(pub_key_base64_es256),
            algo: JwtAlgo::ES256,
        };

        let secondary_cfg = JwtConfig {
            key_file: None,
            key_base64: Some(pub_key_base64_rs256),
            algo: JwtAlgo::RS256,
        };

        let key_chain = JwtPublicSigningKeyChain::from_config(primary_cfg, Some(secondary_cfg))
            .await
            .expect("make key chain");

        let claims = validate_bearer_token(key_chain.clone(), &bearer_token)
            .await
            .expect("shoudl validate");

        assert_eq!(si_claim, claims.custom);
    }
}