//! 验证集成测试目标可以链接共享 crate。

#[test]
fn workspace_name_is_available() {
    assert_eq!(shared::WORKSPACE_NAME, "tech-system");
}
