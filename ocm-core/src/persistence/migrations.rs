use refinery::config::{Config, ConfigDbType};

mod embedded {
    use refinery::embed_migrations;
    embed_migrations!("migrations");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all("data")?;

    let db_path = "data/ocm-impl.db";

    let mut config = Config::new(ConfigDbType::Sqlite).set_db_path(db_path);
    embedded::migrations::runner().run(&mut config)?;

    println!("OCM database initialized at {}", db_path);
    Ok(())
}
