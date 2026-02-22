use redb::Database;
use tracing::debug;

pub struct Db {
    db: Database,
}

impl Db {
    pub fn open(path: String) -> Result<Self, redb::Error> {
        debug!("Acquiring lock of state database at {}.", path);
        let db = Database::create(path)?;
        Ok(Self { db })
    }
}
