use crate::error::Result;
use keepass::{db::NodeRef, Database, DatabaseKey};
use std::fs::File;

pub fn demo_db(db_path: &str, password: Option<&str>, key_file: Option<&str>) -> Result<()> {
    let db_path = std::path::Path::new(db_path);

    let mut _key_file: File;
    let db_key = match (password, key_file) {
        (None, None) => {
            let info = "Password or key file must be specified";
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, info).into());
        }
        (Some(password), None) => DatabaseKey::with_password(password),
        (None, Some(key_file)) => {
            let key_file = std::path::Path::new(key_file);
            _key_file = File::open(key_file)?;
            DatabaseKey::with_keyfile(&mut _key_file)
        }
        (Some(password), Some(key_file)) => {
            let key_file = std::path::Path::new(key_file);
            _key_file = File::open(key_file)?;
            DatabaseKey::with_password_and_keyfile(password, &mut _key_file)
        }
    };

    let db = Database::open(&mut File::open(db_path)?, db_key)?;

    // Iterate over all `Group`s and `Entry`s
    for node in &db.root {
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

    Ok(())
}

#[test]
fn test_demo_db() {
    use crate::error::Error;
    let block = || {
        dotenvy::dotenv()?;

        let db_path = dotenvy::var("DB_PATH")?;
        let password = dotenvy::var("PASSWORD")?;
        // let key_file = dotenvy::var("KEY_FILE")?;
        demo_db(&db_path, Some(&password), None)?;
        Ok::<(), Error>(())
    };
    assert_eq!(block().is_ok(), true);
}
