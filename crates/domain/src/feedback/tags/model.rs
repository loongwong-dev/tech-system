//! 标签目录、语气权重与输入校验。

use serde::{Deserialize, Serialize};

/// 标签所属维度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TagDimension {
    /// 课堂表现。
    Classroom,
    /// 作业。
    Homework,
    /// 知识点。
    Knowledge,
    /// 进步。
    Progress,
    /// 习惯。
    Habit,
}

impl TagDimension {
    /// 按选择器顺序排列的全部维度。
    pub const ALL: [Self; 5] = [
        Self::Classroom,
        Self::Homework,
        Self::Knowledge,
        Self::Progress,
        Self::Habit,
    ];

    /// 面向教师与前端的中文名，同时用作维度标识。
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Classroom => "课堂表现",
            Self::Homework => "作业",
            Self::Knowledge => "知识点",
            Self::Progress => "进步",
            Self::Habit => "习惯",
        }
    }
}

/// 标签的语气权重，决定生成时的措辞强度。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TagTone {
    /// 正面。
    Positive,
    /// 中性。
    Neutral,
    /// 待改进。
    Negative,
}

/// 一个内置标签及其权重。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagDef {
    /// 标签名，同时是落库值与接口字段值。
    pub name: &'static str,
    /// 所属维度。
    pub dimension: TagDimension,
    /// 语气权重。
    pub tone: TagTone,
}

/// 内置标签目录，顺序即前端选择器的展示顺序。
///
/// 赋值顺序固定为 requirements 第 4 节的维度顺序；新增标签必须同时补测试与文档。
pub const TAGS: [TagDef; 17] = [
    TagDef {
        name: "专注投入",
        dimension: TagDimension::Classroom,
        tone: TagTone::Positive,
    },
    TagDef {
        name: "积极参与",
        dimension: TagDimension::Classroom,
        tone: TagTone::Positive,
    },
    TagDef {
        name: "偶有走神",
        dimension: TagDimension::Classroom,
        tone: TagTone::Negative,
    },
    TagDef {
        name: "需督促",
        dimension: TagDimension::Classroom,
        tone: TagTone::Negative,
    },
    TagDef {
        name: "按时完成",
        dimension: TagDimension::Homework,
        tone: TagTone::Positive,
    },
    TagDef {
        name: "质量优秀",
        dimension: TagDimension::Homework,
        tone: TagTone::Positive,
    },
    TagDef {
        name: "部分未完成",
        dimension: TagDimension::Homework,
        tone: TagTone::Negative,
    },
    TagDef {
        name: "未提交",
        dimension: TagDimension::Homework,
        tone: TagTone::Negative,
    },
    TagDef {
        name: "掌握扎实",
        dimension: TagDimension::Knowledge,
        tone: TagTone::Positive,
    },
    // 基本掌握与稳步提升是刻意保留的中性档：教师既不表扬也不指出问题时使用。
    TagDef {
        name: "基本掌握",
        dimension: TagDimension::Knowledge,
        tone: TagTone::Neutral,
    },
    TagDef {
        name: "存在薄弱点",
        dimension: TagDimension::Knowledge,
        tone: TagTone::Negative,
    },
    TagDef {
        name: "明显进步",
        dimension: TagDimension::Progress,
        tone: TagTone::Positive,
    },
    TagDef {
        name: "稳步提升",
        dimension: TagDimension::Progress,
        tone: TagTone::Neutral,
    },
    TagDef {
        name: "需加强",
        dimension: TagDimension::Progress,
        tone: TagTone::Negative,
    },
    TagDef {
        name: "主动提问",
        dimension: TagDimension::Habit,
        tone: TagTone::Positive,
    },
    TagDef {
        name: "需培养习惯",
        dimension: TagDimension::Habit,
        tone: TagTone::Negative,
    },
    TagDef {
        name: "笔记规范",
        dimension: TagDimension::Habit,
        tone: TagTone::Positive,
    },
];

/// 按名称查找内置标签。
#[must_use]
pub fn find(name: &str) -> Option<&'static TagDef> {
    TAGS.iter().find(|tag| tag.name == name)
}

/// 标签输入校验错误。
#[derive(Debug, thiserror::Error)]
pub enum TagInputError {
    /// 标签不在内置目录中。
    #[error("未知标签")]
    Unknown {
        /// 提交的标签名。
        name: String,
    },
    /// 同一维度出现多个标签。
    #[error("同一维度只能选择一个标签")]
    Duplicate {
        /// 冲突的维度名。
        dimension: &'static str,
    },
}

/// 校验标签组合：必须全部属于内置目录，且每个维度至多一个。
///
/// 一节课在同一维度给出两个标签会让语气与话术自相矛盾，因此直接拒绝。
///
/// # Errors
/// 出现未知标签返回 `Unknown`；同一维度重复返回 `Duplicate`。
pub fn validate(tags: &[String]) -> Result<(), TagInputError> {
    let mut seen: Vec<TagDimension> = Vec::with_capacity(TagDimension::ALL.len());
    for name in tags {
        let Some(tag) = find(name) else {
            return Err(TagInputError::Unknown { name: name.clone() });
        };
        if seen.contains(&tag.dimension) {
            return Err(TagInputError::Duplicate {
                dimension: tag.dimension.label(),
            });
        }
        seen.push(tag.dimension);
    }
    Ok(())
}

/// 由标签组合推导语气权重，规则固定：任一待改进即 `negative`，否则任一正面即
/// `positive`，其余 `neutral`（空组合也是 `neutral`）。
///
/// 目录外的标签不参与推导：写入路径已由 [`validate`] 拦下，这里只保证历史数据
/// 不会让读取接口失败。
#[must_use]
pub fn derive_tone(tags: &[String]) -> TagTone {
    let mut tone = TagTone::Neutral;
    for name in tags {
        match find(name).map(|tag| tag.tone) {
            Some(TagTone::Negative) => return TagTone::Negative,
            Some(TagTone::Positive) => tone = TagTone::Positive,
            Some(TagTone::Neutral) | None => {}
        }
    }
    tone
}

#[cfg(test)]
mod tests {
    use super::{TAGS, TagDimension, TagInputError, TagTone, derive_tone, find, validate};

    fn tags(names: &[&str]) -> Vec<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    #[test]
    fn catalog_covers_every_dimension_and_stays_unique() {
        assert_eq!(TAGS.len(), 17);
        let mut names: Vec<&str> = TAGS.iter().map(|tag| tag.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), TAGS.len(), "标签名必须唯一");
        for dimension in TagDimension::ALL {
            assert!(
                TAGS.iter().any(|tag| tag.dimension == dimension),
                "维度 {} 没有标签",
                dimension.label()
            );
        }
        assert!(find("专注投入").is_some());
        assert!(find("不存在的标签").is_none());
    }

    #[test]
    fn tone_table_is_derived_from_weights() {
        let cases: [(Vec<String>, TagTone); 8] = [
            (tags(&[]), TagTone::Neutral),
            (tags(&["专注投入"]), TagTone::Positive),
            (tags(&["基本掌握"]), TagTone::Neutral),
            (tags(&["稳步提升"]), TagTone::Neutral),
            (tags(&["偶有走神"]), TagTone::Negative),
            (tags(&["专注投入", "按时完成"]), TagTone::Positive),
            (tags(&["专注投入", "存在薄弱点"]), TagTone::Negative),
            (tags(&["基本掌握", "需加强"]), TagTone::Negative),
        ];
        for (input, expected) in cases {
            assert_eq!(derive_tone(&input), expected, "输入 {input:?}");
        }
    }

    #[test]
    fn unknown_tags_are_ignored_by_tone_derivation() {
        assert_eq!(derive_tone(&tags(&["不是内置标签"])), TagTone::Neutral);
    }

    #[test]
    fn validate_rejects_unknown_tag_and_dimension_conflict() {
        assert!(validate(&tags(&["专注投入", "按时完成"])).is_ok());
        assert!(matches!(
            validate(&tags(&["专注投入", "不存在"])),
            Err(TagInputError::Unknown { .. })
        ));
        assert!(matches!(
            validate(&tags(&["专注投入", "偶有走神"])),
            Err(TagInputError::Duplicate {
                dimension: "课堂表现"
            })
        ));
    }
}
