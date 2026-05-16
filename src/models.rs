use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PAPER_FIELDS: [&str; 12] = [
    "概述",
    "心得",
    "核心貢獻",
    "關鍵內容",
    "可放入 Introduction 的角度",
    "可放入 Related Work 的角度",
    "方法摘要",
    "資料集摘要",
    "結果摘要",
    "限制",
    "與本研究的關係",
    "可引用句子",
];

pub const ALGORITHM_FIELDS: [&str; 9] = [
    "公式",
    "描述",
    "公式與描述",
    "輸入",
    "輸出",
    "使用原因",
    "限制",
    "LaTeX 原始碼",
    "與本研究的關係",
];

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum SectionType {
    Literature,
    Method,
    Results,
    Discussion,
    GeneralNote,
    Mixed,
}

impl SectionType {
    pub fn all() -> [Self; 6] {
        [
            Self::Literature,
            Self::Method,
            Self::Results,
            Self::Discussion,
            Self::GeneralNote,
            Self::Mixed,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Literature => "Literature",
            Self::Method => "Method",
            Self::Results => "Results",
            Self::Discussion => "Discussion",
            Self::GeneralNote => "General Note",
            Self::Mixed => "Mixed",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum BlockType {
    Paper,
    Algorithm,
    Image,
    Table,
    Note,
}

impl BlockType {
    pub fn all() -> [Self; 5] {
        [
            Self::Paper,
            Self::Algorithm,
            Self::Image,
            Self::Table,
            Self::Note,
        ]
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Paper => "Paper",
            Self::Algorithm => "Algorithm",
            Self::Image => "Image",
            Self::Table => "Table",
            Self::Note => "Note",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SortRule {
    pub field: String,
    pub order: SortOrder,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum SortOrder {
    Asc,
    Desc,
}

impl SortOrder {
    pub fn label(self) -> &'static str {
        match self {
            Self::Asc => "Asc",
            Self::Desc => "Desc",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Project {
    pub project_id: String,
    pub project_name: String,
    #[serde(default)]
    #[serde(alias = "source_folder")]
    pub folder_id: String,
    pub sections: Vec<Section>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Section {
    pub section_id: String,
    #[serde(default)]
    pub idx: i64,
    pub title: String,
    pub section_type: SectionType,
    #[serde(default)]
    pub left_display_field: String,
    #[serde(default)]
    pub source_folder: String,
    #[serde(default)]
    pub display_field: String,
    #[serde(default)]
    pub sort_rules: Vec<SortRule>,
    #[serde(default)]
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Block {
    pub block_id: String,
    #[serde(default)]
    pub idx: i64,
    pub block_type: BlockType,
    pub source_id: String,
    #[serde(default)]
    pub display_field: String,
    #[serde(default)]
    pub display_order: i64,
    #[serde(default)]
    pub custom_sentence: String,
    #[serde(default)]
    pub note: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Material {
    pub id: String,
    pub kind: BlockType,
    pub name: String,
    #[serde(default)]
    pub fields: BTreeMap<String, String>,
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    #[serde(default)]
    pub file_path: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Library {
    pub projects: Vec<Project>,
    pub papers: Vec<Material>,
    pub algorithms: Vec<Material>,
    pub images: Vec<Material>,
    pub tables: Vec<Material>,
    pub notes: Vec<Material>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectData {
    pub project: Project,
    pub papers: Vec<Material>,
    pub algorithms: Vec<Material>,
    pub images: Vec<Material>,
    pub tables: Vec<Material>,
    pub notes: Vec<Material>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectEntry {
    pub project_id: String,
    pub project_name: String,
    #[serde(default)]
    pub folder_id: String,
    #[serde(default)]
    pub local_path: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct ProjectsIndex {
    pub projects: Vec<ProjectEntry>,
}

impl Library {
    pub fn materials(&self, kind: BlockType) -> &[Material] {
        match kind {
            BlockType::Paper => &self.papers,
            BlockType::Algorithm => &self.algorithms,
            BlockType::Image => &self.images,
            BlockType::Table => &self.tables,
            BlockType::Note => &self.notes,
        }
    }

    pub fn materials_mut(&mut self, kind: BlockType) -> &mut Vec<Material> {
        match kind {
            BlockType::Paper => &mut self.papers,
            BlockType::Algorithm => &mut self.algorithms,
            BlockType::Image => &mut self.images,
            BlockType::Table => &mut self.tables,
            BlockType::Note => &mut self.notes,
        }
    }

    pub fn material(&self, kind: BlockType, id: &str) -> Option<&Material> {
        self.materials(kind).iter().find(|m| m.id == id)
    }

}
