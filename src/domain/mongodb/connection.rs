use crate::services::config::DatabaseConfig;
use anyhow::Result;
use mongodb::Client;
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};

const USERINFO_ENCODE: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

pub async fn connect(cfg: DatabaseConfig) -> Result<Client> {
    let uri = get_mongo_uri(cfg)?;
    let mut options = mongodb::options::ClientOptions::parse(&uri).await?;
    options.server_selection_timeout = Some(std::time::Duration::from_secs(3));
    options.connect_timeout = Some(std::time::Duration::from_secs(3));
    let client = Client::with_options(options)?;
    Ok(client)
}

pub fn select_mongo_path() -> std::path::PathBuf {
    "/usr/local/mongodb/bin".to_string().into()
}

pub fn get_mongo_uri(cfg: DatabaseConfig) -> Result<String> {
    Ok(build_mongo_uri(&cfg, true))
}

pub fn build_mongo_uri(cfg: &DatabaseConfig, include_db: bool) -> String {
    let is_srv = cfg.port == 0;
    let scheme = if is_srv { "mongodb+srv" } else { "mongodb" };
    let has_auth = !cfg.username.is_empty() && !cfg.password.is_empty();

    let credentials = if has_auth {
        format!(
            "{}:{}@",
            utf8_percent_encode(&cfg.username, USERINFO_ENCODE),
            utf8_percent_encode(&cfg.password, USERINFO_ENCODE)
        )
    } else {
        String::new()
    };

    let authority = if is_srv {
        cfg.host.clone()
    } else {
        format!("{}:{}", cfg.host, cfg.port)
    };

    let path = if include_db {
        format!("/{}", cfg.database)
    } else {
        "/".to_string()
    };

    let query = if has_auth { "?authSource=admin" } else { "" };

    format!("{}://{}{}{}{}", scheme, credentials, authority, path, query)
}

pub fn extract_db_name(dry_output: &str) -> Option<String> {
    let mut dbs = std::collections::HashSet::new();
    for line in dry_output.lines() {
        if let Some(pos) = line.find("archive prelude ") {
            let rest = &line[pos + "archive prelude ".len()..];
            if let Some(dot) = rest.find('.') {
                let db = &rest[..dot];
                dbs.insert(db.to_string());
            }
        }
    }
    dbs.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::config::{DatabaseConfig, DbType};
    use std::collections::HashMap;

    fn cfg(host: &str, port: u16, user: &str, pass: &str) -> DatabaseConfig {
        DatabaseConfig {
            name: "t".into(),
            database: "mydb".into(),
            db_type: DbType::MongoDB,
            username: user.into(),
            password: pass.into(),
            port,
            host: host.into(),
            generated_id: "id".into(),
            path: String::new(),
            max_packet_size: String::new(),
            volume_name: String::new(),
            container_name: None,
            options: HashMap::new(),
        }
    }

    #[test]
    fn standard_with_auth() {
        let c = cfg("localhost", 27017, "user", "pass");
        assert_eq!(
            build_mongo_uri(&c, true),
            "mongodb://user:pass@localhost:27017/mydb?authSource=admin"
        );
    }

    #[test]
    fn standard_no_auth() {
        let c = cfg("localhost", 27017, "", "");
        assert_eq!(build_mongo_uri(&c, true), "mongodb://localhost:27017/mydb");
    }

    #[test]
    fn srv_with_auth() {
        let c = cfg("cluster.example.mongodb.net", 0, "user", "pass");
        assert_eq!(
            build_mongo_uri(&c, true),
            "mongodb+srv://user:pass@cluster.example.mongodb.net/mydb?authSource=admin"
        );
    }

    #[test]
    fn srv_no_db_for_dryrun() {
        let c = cfg("cluster.example.mongodb.net", 0, "user", "pass");
        assert_eq!(
            build_mongo_uri(&c, false),
            "mongodb+srv://user:pass@cluster.example.mongodb.net/?authSource=admin"
        );
    }

    #[test]
    fn encodes_special_chars_in_credentials() {
        let c = cfg("cluster.example.mongodb.net", 0, "user", "p@ss:w/rd?");
        assert_eq!(
            build_mongo_uri(&c, true),
            "mongodb+srv://user:p%40ss%3Aw%2Frd%3F@cluster.example.mongodb.net/mydb?authSource=admin"
        );
    }
}
