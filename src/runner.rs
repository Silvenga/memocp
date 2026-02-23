use crate::config::Config;
use crate::db::Db;
use std::error::Error;

pub struct Runner {
    config: Config,
    db: Db,
}

impl Runner {
    pub async fn new(config: Config) -> Result<Self, Box<dyn Error>> {
        let db = Db::open_file(config.state_path.clone()).await?;
        Ok(Self { config, db })
    }

    pub async fn run(&self) {}
}
