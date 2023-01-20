//!
//!
//!
pub mod entity;

use sea_orm::DatabaseConnection;



#[derive(Clone)]
pub struct DataSource {
    pub sea_orm: DatabaseConnection,
}