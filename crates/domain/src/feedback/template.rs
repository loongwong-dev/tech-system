//! 标签到模板句子的映射。
//!
//! 模板路径是 AI 不可用时的降级方案：只复述教师输入，不引入新事实。

use crate::feedback::model::Feedback;

/// 模板规则版本；更换规则时必须更新，便于回溯。
pub(crate) const PROMPT_VERSION: &str = "template-v1";

/// 生成反馈正文。
pub(crate) fn render(feedback: &Feedback) -> String {
    let mut sentences = Vec::new();
    for tag in &feedback.knowledge_tags {
        let phrase = tag_phrase(tag);
        if !sentences.contains(&phrase) {
            sentences.push(phrase);
        }
    }

    let notes = feedback
        .notes
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let notes = notes.trim_end_matches(['。', '！', '!', '？', '?']);
    if !notes.is_empty() {
        // 教师原文是事实来源，只清理空白与句末标点后原样拼接。
        sentences.push(format!("{notes}。"));
    }
    if sentences.is_empty() {
        sentences.push("本节课整体表现稳定。".to_owned());
    }
    sentences.push("建议课后结合课堂练习及时订正，针对薄弱点多做巩固。".to_owned());

    let text = sentences.join("");
    truncate_chars(&text, 200)
}

fn tag_phrase(tag: &str) -> String {
    match tag {
        "专注投入" => "课堂投入度高，能持续跟随讲解。".to_owned(),
        "偶有走神" => "课堂注意力有时会分散，需要提醒。".to_owned(),
        "积极参与" => "课堂参与积极。".to_owned(),
        "需督促" => "课堂需要更多督促。".to_owned(),
        "按时完成" => "作业按时完成。".to_owned(),
        "质量优秀" => "作业完成质量较好。".to_owned(),
        "部分未完成" => "作业有部分未完成。".to_owned(),
        "未提交" => "作业尚未提交。".to_owned(),
        "掌握扎实" => "对本节知识点掌握扎实。".to_owned(),
        "基本掌握" => "基本掌握本节内容。".to_owned(),
        "存在薄弱点" => "对部分知识点还存在薄弱环节。".to_owned(),
        "明显进步" => "相较之前有明显进步。".to_owned(),
        "稳步提升" => "保持稳步提升。".to_owned(),
        "需加强" => "还需要更多练习巩固。".to_owned(),
        "主动提问" => "课堂能主动提问。".to_owned(),
        "需培养习惯" => "学习习惯还需要培养。".to_owned(),
        "笔记规范" => "课堂笔记规范。".to_owned(),
        other => format!("{}。", other.trim_end_matches('。')),
    }
}

fn truncate_chars(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    text.chars().take(max).collect()
}
