//! 序号阻止输入改走又改回时采用旧结果。
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Request {
    pub revision: u64,

    pub context: String,

    pub pinyin: String,

    pub texts: Vec<String>,
}
