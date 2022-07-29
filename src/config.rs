use envconfig::Envconfig;

#[derive(Envconfig)]
pub struct Config {
    #[envconfig(from = "LISTEN_ADDRESS", default = "0.0.0.0:8989")]
    pub listen_address: String,

    #[envconfig(from = "BASIC_AUTH_USERNAME", default = "")]
    pub basic_auth_username: String,

    #[envconfig(from = "BASIC_AUTH_PASSWORD", default = "")]
    pub basic_auth_password: String,

    #[envconfig(from = "ENABLE_AUTH", default = "false")]
    pub enable_auth: bool,

    #[envconfig(from = "UPLOADS_FOLDER", default = "./uploads")]
    pub storage_folder: String,
}

#[derive(Debug, Clone)]
pub enum AuthConfig {
    Enabled { username: String, password: String },
    Disabled,
}
