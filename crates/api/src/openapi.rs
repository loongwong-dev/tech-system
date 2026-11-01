//! `OpenAPI` 文档装配。
//!
//! 由 `utoipa` 在编译期生成；提交到仓库的 `openapi.json` 必须通过
//! `bun run generate:api` 更新，不得手改。

use utoipa::OpenApi;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "教培教师 AI 反馈平台 API",
        version = "0.1.0",
        description = "C 端与管理端 HTTP 契约。生成物由 utoipa 与 openapi-typescript 维护。"
    ),
    paths(
        crate::routes::health::healthz,
        crate::routes::health::readyz,
        crate::routes::auth::register,
        crate::routes::auth::login,
        crate::routes::auth::logout,
        crate::routes::auth::switch_tenant,
        crate::routes::auth::tenants,
        crate::routes::me::me,
        crate::routes::students::index,
        crate::routes::students::create,
        crate::routes::students::update,
        crate::routes::students::archive,
        crate::routes::students::delete,
        crate::routes::classes::index,
        crate::routes::classes::create,
        crate::routes::classes::update,
        crate::routes::classes::archive,
        crate::routes::classes::delete,
        crate::routes::lessons::index,
        crate::routes::lessons::create,
        crate::routes::generation::generate_feedback,
        crate::routes::generation::generate_lesson,
        crate::routes::generation::job_progress,
    ),
    components(schemas(
        crate::FieldError,
        crate::error::ProblemDetails,
        crate::routes::auth::RegisterRequest,
        crate::routes::auth::RegisterResponse,
        crate::routes::auth::LoginRequest,
        crate::routes::auth::LoginResponse,
        crate::routes::auth::TenantSwitchRequest,
        crate::routes::auth::TenantSummaryResponse,
        crate::routes::auth::TenantListResponse,
        crate::routes::me::MeResponse,
        crate::routes::me::QuotaResponse,
        crate::routes::students::CreateStudentRequest,
        crate::routes::students::UpdateStudentRequest,
        crate::routes::students::DeleteStudentRequest,
        crate::routes::students::StudentResponse,
        crate::routes::students::StudentListResponse,
        crate::routes::students::DeletedStudentResponse,
        crate::routes::classes::CreateClassRequest,
        crate::routes::classes::UpdateClassRequest,
        crate::routes::classes::ClassResponse,
        crate::routes::classes::ClassListResponse,
        crate::routes::lessons::CreateLessonRequest,
        crate::routes::lessons::LessonStatus,
        crate::routes::lessons::LessonResponse,
        crate::routes::lessons::LessonListResponse,
        crate::routes::generation::BatchJobResponse,
    )),
    tags(
        (name = "auth", description = "注册、登录与租户会话"),
        (name = "account", description = "当前账号与租户"),
        (name = "students", description = "学员管理"),
        (name = "classes", description = "班级管理"),
        (name = "lessons", description = "课次管理"),
        (name = "generation", description = "反馈生成与任务进度"),
        (name = "health", description = "健康检查")
    )
)]
struct ApiDoc;

/// 生成格式化后的 `OpenAPI` JSON。
///
/// # Errors
/// 序列化内部文档失败时返回 `serde_json::Error`。
pub fn openapi_json() -> Result<String, serde_json::Error> {
    let mut document = ApiDoc::openapi();
    if let Some(components) = document.components.as_mut() {
        components.add_security_scheme(
            "sid",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new("sid"))),
        );
    }
    document.to_pretty_json()
}
