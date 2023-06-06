use crate::error::Result;
use keepass::{config::DatabaseConfig, db, db::NodeRef, Database, DatabaseKey};
use std::fs::File;

#[derive(Debug, Default)]
pub struct KpDb {
    pub db: Option<Database>,
    pub db_path: Option<String>,
    pub password: Option<String>,
    pub key_file: Option<String>,
}

impl KpDb {
    pub fn new() -> Self {
        let mut kpdb = Self::default();
        kpdb.db = Some(Database::new(DatabaseConfig::default()));
        kpdb
    }

    pub fn open(db_path: &str, password: Option<&str>, key_file: Option<&str>) -> Result<Self> {
        let mut kpdb = Self::default();
        kpdb.db_path = Some(db_path.to_string());
        kpdb.password = password.map(|s| s.to_string());
        kpdb.key_file = key_file.map(|s| s.to_string());

        let _db_path = std::path::Path::new(db_path);

        let db_key = kpdb.build_db_key()?;

        let db = Database::open(&mut File::open(_db_path)?, db_key)?;
        kpdb.db = Some(db);
        Ok(kpdb)
    }

    fn build_db_key(&self) -> Result<DatabaseKey> {
        let mut key_file = self.key_file.clone();
        let mut key_file = key_file.as_mut().and_then(|f| File::open(f).ok());
        let key_file = key_file.as_mut().map(|kf| kf as &mut dyn std::io::Read);

        let mut db_key = DatabaseKey::new();
        if let Some(ref password) = self.password {
            if !password.is_empty() {
                db_key = db_key.with_password(password);
            }
        }
        if let Some(key_file) = key_file {
            db_key = db_key.with_keyfile(key_file)?;
        }

        Ok(db_key)
    }

    pub fn get_root(&self) -> Option<&db::Group> {
        self.db.as_ref().map(|db| &db.root)
    }

    pub fn get_groups<'a>(&'a self, parent: &'a db::Group) -> Vec<&'a db::Group> {
        let mut groups = Vec::new();
        for node in parent {
            if let NodeRef::Group(g) = node {
                groups.push(g);
            }
        }
        groups
    }

    pub fn get_entries<'a>(&'a self, parent: &'a db::Group) -> Vec<&'a db::Entry> {
        let mut entries = Vec::new();
        for node in parent {
            if let NodeRef::Entry(e) = node {
                entries.push(e);
            }
        }
        entries
    }

    pub fn get_item(&self, path: &[&str]) -> Option<db::NodeRef> {
        self.get_root().and_then(|root| root.get(path))
    }
}

#[test]
fn test_demo_db() {
    use crate::error::Error;
    use keepass::db::NodeRef;
    let block = || {
        dotenvy::dotenv()?;

        let db_path = dotenvy::var("DB_PATH")?;
        let password = dotenvy::var("PASSWORD")?;
        // let key_file = dotenvy::var("KEY_FILE")?;

        let kpdb = KpDb::open(&db_path, Some(&password), None)?;
        // Iterate over all `Group`s and `Entry`s
        for node in kpdb.get_root().unwrap() {
            match node {
                NodeRef::Group(g) => {
                    println!("Saw group '{0}'", g.name);
                }
                NodeRef::Entry(e) => {
                    let title = e.get_title().unwrap_or("(no title)");
                    let user = e.get_username().unwrap_or("(no username)");
                    let pass = e.get_password().unwrap_or("(no password)");
                    println!("Entry '{0}': '{1}' : '{2}'", title, user, pass);
                }
            }
        }
        Ok::<(), Error>(())
    };
    assert_eq!(block().is_ok(), true);
}
