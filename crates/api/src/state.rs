//! 路由共享状态。

use db::Db;

/// 路由层共享状态。
#[derive(Clone)]
pub struct AppState {
    db: Db,
}

impl AppState {
    /// 使用数据库连接池构造状态。
    #[must_use]
    pub fn new(db: Db) -> Self {
        Self { db }
    }

    pub(crate) fn db(&self) -> &Db {
        &self.db
    }
}
