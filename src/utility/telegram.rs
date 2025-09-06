use base64::Engine;
use derive_more::{Display, Error};
use ed25519_dalek::Verifier;
use hex_literal::hex;
use serde::Deserialize;
use std::collections::HashMap;

pub struct WebAppInitDataMap {
    pub data_map: HashMap<String, String>,
}

#[derive(Deserialize)]
pub struct WebAppUser {
    pub id: i64,
}

#[derive(Clone, Debug, Display, Error)]
pub enum VerifyError {
    #[display("No signature found.")]
    NoSignature,

    #[display("The provided signature was corrupted.")]
    BadSignature,

    #[display("The expected signature does not match the actual one.")]
    IntegrityCheckFailed,
}

impl WebAppInitDataMap {
    pub fn from_str(data: String) -> Self {
        let mut this = Self {
            data_map: HashMap::new(),
        };

        data.split('&')
            .map(|kv| kv.split_once('=').unwrap_or((kv, "")))
            .for_each(|(key, value)| {
                this.data_map.insert(key.to_string(), value.to_string());
            });

        if let Some(user) = this.data_map.get_mut("user") {
            *user = percent_encoding::percent_decode_str(&*user)
                .decode_utf8_lossy()
                .to_string();
        }

        this
    }

    pub fn verify(&self, bot_id: i64, test_dc: bool) -> Result<(), VerifyError> {
        //noinspection ALL
        const TELEGRAM_PUBLIC_KEY: [[u8; 32]; 2] = [
            hex!("e7bf03a2fa4602af4580703d88dda5bb59f32ed8b02a56c187fe7d34caed242d"),
            hex!("40055058a4ee38156a06562e52eece92a771bcd8346a8c4615cb7376eddf72ec"),
        ];

        let verifying_key = ed25519_dalek::VerifyingKey::from_bytes(
            &TELEGRAM_PUBLIC_KEY[if test_dc { 1 } else { 0 }],
        )
        .unwrap();

        let signature = {
            let raw = self
                .data_map
                .get("signature")
                .ok_or(VerifyError::NoSignature)?;

            let bytes = base64::prelude::BASE64_URL_SAFE_NO_PAD
                .decode(raw)
                .map_err(|_| VerifyError::BadSignature)?;

            ed25519_dalek::Signature::from_slice(bytes.as_slice())
                .map_err(|_| VerifyError::BadSignature)?
        };

        let data_check_string = format!("{}:WebAppData\n{}", bot_id, {
            let mut vec = self
                .data_map
                .iter()
                .filter(|(key, _)| !["hash", "signature"].iter().any(|variant| variant == key))
                .map(|(key, value)| format!("{}={}", key, value))
                .collect::<Vec<String>>();
            vec.sort();
            vec.join("\n")
        });

        verifying_key
            .verify(data_check_string.as_bytes(), &signature)
            .map_err(|_| VerifyError::IntegrityCheckFailed)
    }
}
