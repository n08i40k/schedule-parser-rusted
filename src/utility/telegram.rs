use aws_lc_rs::signature::{ED25519, UnparsedPublicKey};
use base64::{Engine, engine::DecodePaddingMode, engine::general_purpose};
use derive_more::{Display, Error};
use hex_literal::hex;
use serde::Deserialize;
use std::collections::HashMap;

/// Длина подписи Ed25519 в байтах.
const ED25519_SIGNATURE_LENGTH: usize = 64;

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
        Self {
            // Decode every field exactly once before checking Telegram's signature.
            data_map: form_urlencoded::parse(data.as_bytes())
                .into_owned()
                .collect(),
        }
    }

    pub fn verify(&self, bot_id: i64, test_dc: bool) -> Result<(), VerifyError> {
        //noinspection ALL
        const TELEGRAM_PUBLIC_KEY: [[u8; 32]; 2] = [
            hex!("e7bf03a2fa4602af4580703d88dda5bb59f32ed8b02a56c187fe7d34caed242d"),
            hex!("40055058a4ee38156a06562e52eece92a771bcd8346a8c4615cb7376eddf72ec"),
        ];

        let verifying_key =
            UnparsedPublicKey::new(&ED25519, TELEGRAM_PUBLIC_KEY[if test_dc { 1 } else { 0 }]);

        self.verify_with_key(bot_id, &verifying_key)
    }

    fn verify_with_key(
        &self,
        bot_id: i64,
        verifying_key: &UnparsedPublicKey<[u8; 32]>,
    ) -> Result<(), VerifyError> {
        let signature = {
            let raw = self
                .data_map
                .get("signature")
                .ok_or(VerifyError::NoSignature)?;

            // Telegram clients may send base64url with or without padding.
            let decoder = general_purpose::GeneralPurpose::new(
                &base64::alphabet::URL_SAFE,
                general_purpose::GeneralPurposeConfig::new()
                    .with_decode_padding_mode(DecodePaddingMode::Indifferent),
            );
            let bytes = decoder.decode(raw).map_err(|_| VerifyError::BadSignature)?;

            if bytes.len() != ED25519_SIGNATURE_LENGTH {
                return Err(VerifyError::BadSignature);
            }

            bytes
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
            .verify(data_check_string.as_bytes(), signature.as_slice())
            .map_err(|_| VerifyError::IntegrityCheckFailed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_lc_rs::signature::{Ed25519KeyPair, KeyPair};

    const BOT_ID: i64 = 12345678;
    const USER: &str = r#"{"id":1234567890123,"first_name":"Иван + 100%20"}"#;
    const CHAT: &str = r#"{"id":-123456789,"type":"group","title":"A & B"}"#;

    fn signed_init_data(padded: bool) -> (WebAppInitDataMap, UnparsedPublicKey<[u8; 32]>) {
        let key_pair = Ed25519KeyPair::from_seed_unchecked(&[42; 32]).unwrap();
        // Independent, explicit signing payload: no production parsing or sorting helpers.
        let payload = format!(
            "{BOT_ID}:WebAppData\nauth_date=1700000000\nchat={CHAT}\nquery_id=AA+BB/CC=\nuser={USER}"
        );
        let signature = key_pair.sign(payload.as_bytes());
        let encoder = if padded {
            general_purpose::URL_SAFE
        } else {
            general_purpose::URL_SAFE_NO_PAD
        };
        let signature = encoder.encode(signature.as_ref());
        let query = form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("user", USER),
                ("signature", signature.as_str()),
                ("query_id", "AA+BB/CC="),
                ("hash", "ignored"),
                ("chat", CHAT),
                ("auth_date", "1700000000"),
            ])
            .finish();
        let public_key: [u8; 32] = key_pair.public_key().as_ref().try_into().unwrap();
        (
            WebAppInitDataMap::from_str(query),
            UnparsedPublicKey::new(&ED25519, public_key),
        )
    }

    #[test]
    fn verifies_encoded_fields_and_both_signature_padding_variants() {
        for padded in [false, true] {
            let (data, key) = signed_init_data(padded);
            assert_eq!(data.data_map["user"], USER);
            assert_eq!(data.data_map["chat"], CHAT);
            assert_eq!(data.data_map["query_id"], "AA+BB/CC=");
            assert_eq!(
                serde_json::from_str::<WebAppUser>(&data.data_map["user"])
                    .unwrap()
                    .id,
                1234567890123,
            );
            assert!(data.verify_with_key(BOT_ID, &key).is_ok());
        }
    }

    #[test]
    fn accepts_percent_encoded_spaces_without_decoding_twice() {
        let data = WebAppInitDataMap::from_str(
            "user=%7B%22id%22%3A1%2C%22first_name%22%3A%22A%20B%2BC%2520%22%7D".into(),
        );
        assert_eq!(data.data_map["user"], r#"{"id":1,"first_name":"A B+C%20"}"#);
    }

    #[test]
    fn rejects_tampered_fields_and_wrong_bot() {
        let (mut data, key) = signed_init_data(true);
        assert!(matches!(
            data.verify_with_key(BOT_ID + 1, &key),
            Err(VerifyError::IntegrityCheckFailed)
        ));
        data.data_map.insert("chat".into(), "{}".into());
        assert!(matches!(
            data.verify_with_key(BOT_ID, &key),
            Err(VerifyError::IntegrityCheckFailed)
        ));
    }

    #[test]
    fn rejects_missing_and_malformed_signatures() {
        let (mut data, key) = signed_init_data(false);
        for signature in ["!", "AA", ""] {
            data.data_map.insert("signature".into(), signature.into());
            assert!(matches!(
                data.verify_with_key(BOT_ID, &key),
                Err(VerifyError::BadSignature)
            ));
        }
        data.data_map.remove("signature");
        assert!(matches!(
            data.verify_with_key(BOT_ID, &key),
            Err(VerifyError::NoSignature)
        ));
    }
}
