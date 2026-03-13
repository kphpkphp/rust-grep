use std::sync::OnceLock;

pub struct Config {
    pub page_size: usize,
}

pub static CONFIG: OnceLock<Config> = OnceLock::new();

pub fn init(){
    let config = Config {
        page_size: 20,
    };
    CONFIG.set(config).ok();
}

pub fn get_config() -> &'static Config {
    CONFIG.get().expect("Settings 必须先初始化")
}