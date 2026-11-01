//! 标签服务：内置目录输出与学员标签继承。
//!
//! 目录是纯数据，不查库；继承查询在租户事务内完成，跨租户学员视为不存在。

use uuid::Uuid;

use crate::feedback::draft::repository as draft_repository;
use crate::feedback::tags::model::{TAGS, TagDimension, TagTone, derive_tone};
use crate::feedback::tags::repository;

/// 目录中的一个标签。
#[derive(Debug, Clone, Copy)]
pub struct CatalogTag {
    /// 标签名。
    pub name: &'static str,
    /// 语气权重。
    pub tone: TagTone,
}

/// 目录中的一个维度。
#[derive(Debug, Clone)]
pub struct CatalogDimension {
    /// 维度名。
    pub name: &'static str,
    /// 该维度下的标签，顺序与常量目录一致。
    pub tags: Vec<CatalogTag>,
}

/// 学员上次使用的标签组合。
#[derive(Debug, Clone)]
pub struct InheritedTags {
    /// 标签组合，没有历史时为空。
    pub tags: Vec<String>,
    /// 由标签推导出的语气权重。
    pub tone: TagTone,
}

/// 标签查询错误。
#[derive(Debug, thiserror::Error)]
pub enum TagQueryError {
    /// 开启或提交租户事务失败。
    #[error("标签事务失败")]
    Transaction {
        /// 底层数据库错误。
        #[source]
        source: db::DbError,
    },
    /// 查询失败。
    #[error("标签查询失败")]
    Database {
        /// 底层数据库错误。
        #[source]
        source: sqlx::Error,
    },
}

/// 列出内置标签目录，按维度分组。
#[must_use]
pub fn list_catalog() -> Vec<CatalogDimension> {
    TagDimension::ALL
        .iter()
        .map(|dimension| CatalogDimension {
            name: dimension.label(),
            tags: TAGS
                .iter()
                .filter(|tag| tag.dimension == *dimension)
                .map(|tag| CatalogTag {
                    name: tag.name,
                    tone: tag.tone,
                })
                .collect(),
        })
        .collect()
}

/// 返回学员最近一次使用的标签组合。
///
/// 返回 `None` 表示学员不存在或不属于当前租户；学员存在但没有反馈时返回
/// `Some(InheritedTags { tags: vec![], tone: Neutral })`。
///
/// # Errors
/// 开启事务、查询或提交失败时返回 [`TagQueryError`]。
pub async fn latest_student_tags(
    pool: &db::TenantPool,
    tenant_id: Uuid,
    student_id: Uuid,
) -> Result<Option<InheritedTags>, TagQueryError> {
    let mut transaction = pool.begin(tenant_id).await.map_err(transaction_error)?;
    if !draft_repository::student_exists(transaction.transaction_mut(), tenant_id, student_id)
        .await
        .map_err(database)?
    {
        return Ok(None);
    }
    let tags = repository::latest_tags(transaction.transaction_mut(), tenant_id, student_id)
        .await
        .map_err(database)?
        .unwrap_or_default();
    let tone = derive_tone(&tags);
    transaction.commit().await.map_err(transaction_error)?;
    Ok(Some(InheritedTags { tags, tone }))
}

fn transaction_error(source: db::DbError) -> TagQueryError {
    TagQueryError::Transaction { source }
}

fn database(source: sqlx::Error) -> TagQueryError {
    TagQueryError::Database { source }
}
