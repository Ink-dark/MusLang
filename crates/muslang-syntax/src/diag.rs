//! 源码位置与诊断信息。

/// 字节偏移区间（`start` 含，`end` 不含）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self {
            start: start as u32,
            end: end as u32,
        }
    }

    /// 合并两个区间，覆盖 `self` 与 `other`。
    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub span: Span,
    pub message: String,
}

impl Diagnostic {
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }

    /// 渲染为 `行:列: 消息`（行列均从 1 计，按字节偏移换算）。
    pub fn render(&self, src: &str) -> String {
        let (line, col) = line_col(src, self.span.start as usize);
        format!("{line}:{col}: {}", self.message)
    }
}

/// 把字节偏移换算为 1-based 行列号。
pub fn line_col(src: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(src.len());
    let before = &src.as_bytes()[..offset];
    let line = before.iter().filter(|&&b| b == b'\n').count() + 1;
    let col = match before.iter().rposition(|&b| b == b'\n') {
        Some(pos) => offset - pos,
        None => offset + 1,
    };
    (line, col)
}
