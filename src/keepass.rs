use crate::error::Result;
use keepass::{
    config::DatabaseConfig,
    db::{self, Group, NodePtr},
    group_get_children, node_is_group, search_node_by_uuid, Database, DatabaseKey, Uuid,
};
use std::fs::File;

#[derive(Debug)]
pub struct KpDb {
    pub db: Option<Database>,
    pub db_path: Option<String>,
    pub password: Option<String>,
    pub key_file: Option<String>,
}

impl Default for KpDb {
    fn default() -> Self {
        Self {
            db: Some(Database::new(DatabaseConfig::default())),
            db_path: None,
            password: None,
            key_file: None,
        }
    }
}

impl KpDb {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn open(db_path: &str, password: Option<&str>, key_file: Option<&str>) -> Result<Self> {
        let mut kpdb = Self::new();
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
        let mut key_file = self.key_file.as_ref().and_then(|f| File::open(f).ok());
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

    pub fn delete_node(&mut self, uuid: Uuid) -> Result<()> {
        let db = self.db.as_mut().ok_or("No database")?;
        let node = db.remove_node_by_uuid(uuid)?;
        log::trace!("node: {:?} deleted", node.borrow().get_title());
        Ok(())
    }

    pub fn create_new_group(&mut self, parent: Uuid) -> Result<NodePtr> {
        let db = self.db.as_ref().ok_or("No database")?;
        let group = db.create_new_group(parent, 0)?;
        log::trace!("group: {:?} added", group.borrow().get_uuid());
        Ok(group)
    }

    pub fn create_new_entry(&mut self, parent: Uuid) -> Result<NodePtr> {
        let db = self.db.as_ref().ok_or("No database")?;
        let entry = db.create_new_entry(parent, 0)?;
        log::trace!("entry: {:?} added", entry.borrow().get_uuid());
        Ok(entry)
    }

    pub fn get_root(&self) -> Option<db::NodePtr> {
        self.db.as_ref().map(|db| db.root.clone())
    }

    pub fn get_groups(&self, parent: &db::NodePtr) -> Vec<db::NodePtr> {
        let mut groups = Vec::new();
        group_get_children(parent).unwrap().iter().for_each(|node| {
            if node_is_group(node) {
                groups.push(node.clone());
            }
        });
        groups
    }

    pub fn get_entries(&self, parent: &db::NodePtr) -> Vec<db::NodePtr> {
        let mut entries = Vec::new();
        group_get_children(parent).unwrap().iter().for_each(|node| {
            if !node_is_group(node) {
                entries.push(node.clone());
            }
        });
        entries
    }

    pub fn get_item(&self, path: &[&str]) -> Option<db::NodePtr> {
        self.get_root().and_then(|root| Group::get(&root, path))
    }

    pub fn get_node_by_id(&self, id: Uuid) -> Option<db::NodePtr> {
        self.get_root().and_then(|root| search_node_by_uuid(&root, id))
    }
}

#[test]
fn test_demo_db() {
    use crate::error::Error;
    use keepass::{db::Entry, Node, NodeIterator};
    let block = || {
        dotenvy::dotenv().ok();

        let db_path = std::env::var("DB_PATH")?;
        let password = std::env::var("PASSWORD").ok();
        let key_file = std::env::var("KEY_FILE").ok();

        let kpdb = KpDb::open(&db_path, password.as_deref(), key_file.as_deref())?;
        // Iterate over all `Group`s and `Entry`s
        for node in NodeIterator::new(&kpdb.get_root().unwrap()) {
            if node_is_group(&node) {
                println!("Saw group '{}'", node.borrow().get_title().unwrap());
            } else if let Some(entry) = node.borrow().as_any().downcast_ref::<Entry>() {
                let title = entry.get_title().unwrap_or("(no title)");
                let user = entry.get_username().unwrap_or("(no username)");
                let pass = entry.get_password().unwrap_or("(no password)");
                println!("Entry '{}': '{}' : '{}'", title, user, pass);
            }
        }
        Ok::<(), Error>(())
    };
    assert_eq!(block().is_ok(), true);
}
