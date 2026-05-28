use devtool_core::error::{PluginError, PluginResult};
use devtool_core::plugin::Plugin;
use devtool_core::types::*;
use md5::Digest as Md5Digest;
use sha2::{Sha256, Sha512};

struct CryptoPlugin;

impl Plugin for CryptoPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            name: "crypto".into(),
            version: "0.1.0".into(),
            description: "Encryption, decryption, encoding, and hashing utilities".into(),
            author: "DevTool Team".into(),
            category: PluginCategory::Security,
            actions: vec![
                PluginAction {
                    name: "base64-encode".into(),
                    description: "Encode text to Base64".into(),
                    params: vec![],
                },
                PluginAction {
                    name: "base64-decode".into(),
                    description: "Decode Base64 to text".into(),
                    params: vec![],
                },
                PluginAction {
                    name: "hex-encode".into(),
                    description: "Encode text to hex string".into(),
                    params: vec![],
                },
                PluginAction {
                    name: "hex-decode".into(),
                    description: "Decode hex string to text".into(),
                    params: vec![],
                },
                PluginAction {
                    name: "hash".into(),
                    description: "Hash input with specified algorithm (sha256, sha512, sha1, md5)".into(),
                    params: vec![ActionParam {
                        name: "algo".into(),
                        description: "Hash algorithm: sha256, sha512, sha1, md5".into(),
                        required: true,
                        default_value: Some("sha256".into()),
                        param_type: ParamType::String,
                    }],
                },
                PluginAction {
                    name: "hmac".into(),
                    description: "Compute HMAC with specified algorithm and key".into(),
                    params: vec![
                        ActionParam {
                            name: "algo".into(),
                            description: "HMAC algorithm: sha256, sha512".into(),
                            required: true,
                            default_value: Some("sha256".into()),
                            param_type: ParamType::String,
                        },
                        ActionParam {
                            name: "key".into(),
                            description: "Secret key for HMAC".into(),
                            required: true,
                            default_value: None,
                            param_type: ParamType::String,
                        },
                    ],
                },
                PluginAction {
                    name: "url-encode".into(),
                    description: "URL-encode the input string".into(),
                    params: vec![],
                },
                PluginAction {
                    name: "url-decode".into(),
                    description: "URL-decode the input string".into(),
                    params: vec![],
                },
            ],
        }
    }

    fn execute(&self, input: PluginInput) -> PluginResult<PluginOutput> {
        match input.action.as_str() {
            "base64-encode" => self.base64_encode(&input),
            "base64-decode" => self.base64_decode(&input),
            "hex-encode" => self.hex_encode(&input),
            "hex-decode" => self.hex_decode(&input),
            "hash" => self.hash(&input),
            "hmac" => self.hmac_compute(&input),
            "url-encode" => self.url_encode(&input),
            "url-decode" => self.url_decode(&input),
            _ => Err(PluginError::InvalidAction(input.action)),
        }
    }

    fn tui_view(&self) -> Option<TuiViewDef> {
        Some(TuiViewDef {
            title: "Crypto".into(),
            component_type: TuiComponentType::Form,
        })
    }

    fn web_handlers(&self) -> Vec<WebHandlerDef> {
        vec![]
    }
}

impl CryptoPlugin {
    fn get_data(&self, input: &PluginInput) -> PluginResult<String> {
        input.input_data.clone().ok_or_else(|| {
            PluginError::MissingParam("input_data (provide data via --input or -f)".into())
        })
    }

    fn base64_encode(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let data = self.get_data(input)?;
        let encoded = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, data.as_bytes());
        Ok(PluginOutput { success: true, data: encoded, error: None, metadata: None })
    }

    fn base64_decode(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let data = self.get_data(input)?;
        let decoded = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, data.as_bytes())
            .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let text = String::from_utf8(decoded).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        Ok(PluginOutput { success: true, data: text, error: None, metadata: None })
    }

    fn hex_encode(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let data = self.get_data(input)?;
        Ok(PluginOutput { success: true, data: hex::encode(data.as_bytes()), error: None, metadata: None })
    }

    fn hex_decode(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let data = self.get_data(input)?;
        let bytes = hex::decode(data.trim()).map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
        let text = String::from_utf8(bytes).map_err(|e| PluginError::ExecutionFailed(format!("Not valid UTF-8: {}", e)))?;
        Ok(PluginOutput { success: true, data: text, error: None, metadata: None })
    }

    fn hash(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let data = self.get_data(input)?;
        let algo = input.params.get("algo").map(|s| s.as_str()).unwrap_or("sha256");
        let result = match algo {
            "sha256" => hex::encode(Sha256::digest(data.as_bytes())),
            "sha512" => hex::encode(Sha512::digest(data.as_bytes())),
            "sha1" => hex::encode(sha1::Sha1::digest(data.as_bytes())),
            "md5" => hex::encode(md5::Md5::digest(data.as_bytes())),
            _ => return Err(PluginError::InvalidAction(format!("Unknown algorithm: {}", algo))),
        };
        Ok(PluginOutput { success: true, data: result, error: None, metadata: None })
    }

    fn hmac_compute(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let data = self.get_data(input)?;
        let algo = input.params.get("algo").map(|s| s.as_str()).unwrap_or("sha256");
        let key = input.params.get("key")
            .ok_or_else(|| PluginError::MissingParam("key".into()))?;
        let result = match algo {
            "sha256" => {
                use hmac::{Hmac, Mac};
                use sha2::Sha256;
                let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes())
                    .map_err(|e| PluginError::ExecutionFailed(e.to_string()))?;
                mac.update(data.as_bytes());
                hex::encode(mac.finalize().into_bytes())
            }
            _ => return Err(PluginError::InvalidAction(format!("Unknown HMAC algorithm: {}", algo))),
        };
        Ok(PluginOutput { success: true, data: result, error: None, metadata: None })
    }

    fn url_encode(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let data = self.get_data(input)?;
        Ok(PluginOutput { success: true, data: urlencoding(&data), error: None, metadata: None })
    }

    fn url_decode(&self, input: &PluginInput) -> PluginResult<PluginOutput> {
        let data = self.get_data(input)?;
        Ok(PluginOutput { success: true, data: urldecoding(&data), error: None, metadata: None })
    }
}

fn urlencoding(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => (b as char).to_string(),
            b' ' => "+".to_string(),
            _ => format!("%{:02X}", b),
        })
        .collect::<Vec<_>>()
        .join("")
}

fn urldecoding(s: &str) -> String {
    let mut result = Vec::new();
    let mut bytes = s.bytes();
    while let Some(b) = bytes.next() {
        match b {
            b'+' => result.push(b' '),
            b'%' => {
                let h = bytes.next().unwrap_or(b'0');
                let l = bytes.next().unwrap_or(b'0');
                let hex = u8::from_str_radix(
                    &String::from_utf8_lossy(&[h, l]), 16
                ).unwrap_or(0);
                result.push(hex);
            }
            _ => result.push(b),
        }
    }
    String::from_utf8_lossy(&result).into_owned()
}

#[no_mangle]
pub extern "C" fn _plugin_create() -> Box<dyn Plugin> {
    Box::new(CryptoPlugin)
}
