use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RustFsConfig {
    pub data_path: String,
    pub port: Option<u16>,
    pub host: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub console_enable: bool,
}

impl Default for RustFsConfig {
    fn default() -> Self {
        Self {
            data_path: String::new(),
            port: Some(9000),
            host: Some("127.0.0.1".to_string()),
            access_key: Some("rustfsadmin".to_string()),
            secret_key: Some("rustfsadmin".to_string()),
            console_enable: false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum LogType {
    App,
    RustFS,
}

#[derive(Debug, Deserialize)]
pub struct CommandResponse {
    pub success: bool,
    pub message: String,
}
