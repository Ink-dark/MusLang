//! 词法器：grammar.ebnf §7（词法层）+ §8（保留字）。
//!
//! 临时裁定见 crate 根文档 DEVIATIONS：`0b`/`0o`/`0x` 前缀纳入、嵌套块注释、
//! 文档注释作为 trivia 忽略、`unsafe`/`dyn` 暂按普通标识符。

use crate::diag::{Diagnostic, Span};
use crate::token::{Token, TokenKind, keyword};

pub fn lex(src: &str) -> (Vec<Token>, Vec<Diagnostic>) {
    Lexer::new(src).run()
}

struct Lexer<'s> {
    src: &'s str,
    bytes: &'s [u8],
    pos: usize,
    tokens: Vec<Token>,
    diags: Vec<Diagnostic>,
}

impl<'s> Lexer<'s> {
    fn new(src: &'s str) -> Self {
        Self {
            src,
            bytes: src.as_bytes(),
            pos: 0,
            tokens: Vec::new(),
            diags: Vec::new(),
        }
    }

    fn run(mut self) -> (Vec<Token>, Vec<Diagnostic>) {
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            match b {
                b' ' | b'\t' | b'\r' | b'\n' => self.pos += 1,
                b'/' if self.peek() == Some(b'/') => self.line_comment(),
                b'/' if self.peek() == Some(b'*') => self.block_comment(),
                b'"' => self.string_lit(),
                b'\'' => self.char_or_lifetime(),
                b'r' if self.raw_string_ahead() => self.raw_string_lit(),
                b'0'..=b'9' => self.number_lit(),
                b'A'..=b'Z' | b'a'..=b'z' | b'_' => self.ident(),
                _ => self.punct(),
            }
        }
        self.push(TokenKind::Eof, "");
        (self.tokens, self.diags)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    fn starts_ident(b: u8) -> bool {
        b.is_ascii_alphabetic() || b == b'_'
    }

    fn is_ident_cont(b: u8) -> bool {
        b.is_ascii_alphanumeric() || b == b'_'
    }

    fn error(&mut self, start: usize, message: impl Into<String>) {
        self.diags
            .push(Diagnostic::new(Span::new(start, self.pos), message));
    }

    fn push(&mut self, kind: TokenKind, text: &str) {
        let start = self.pos - text.len();
        self.tokens.push(Token {
            kind,
            text: text.to_string(),
            span: Span::new(start, self.pos),
        });
    }

    fn push_slice(&mut self, kind: TokenKind, text: &str, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            text: text.to_string(),
            span: Span::new(start, end),
        });
    }

    // ---- 注释 ----

    fn line_comment(&mut self) {
        while self.pos < self.bytes.len() && self.bytes[self.pos] != b'\n' {
            self.pos += 1;
        }
    }

    fn block_comment(&mut self) {
        let start = self.pos;
        self.pos += 2;
        let mut depth = 1usize;
        while self.pos < self.bytes.len() && depth > 0 {
            match self.bytes[self.pos] {
                b'/' if self.peek() == Some(b'*') => {
                    depth += 1;
                    self.pos += 2;
                }
                b'*' if self.peek() == Some(b'/') => {
                    depth -= 1;
                    self.pos += 2;
                }
                _ => self.pos += 1,
            }
        }
        if depth > 0 {
            self.pos = self.bytes.len();
            self.error(start, "未闭合的块注释");
        }
    }

    // ---- 标识符 / 保留字 ----

    fn ident(&mut self) {
        let start = self.pos;
        self.pos += 1;
        while self.pos < self.bytes.len() && Self::is_ident_cont(self.bytes[self.pos]) {
            self.pos += 1;
        }
        let text = &self.src[start..self.pos];
        let kind = keyword(text).unwrap_or(TokenKind::Ident);
        self.push_slice(kind, text, start, self.pos);
    }

    // ---- 字符字面量 vs 生命周期 ----

    fn char_or_lifetime(&mut self) {
        let start = self.pos;
        self.pos += 1; // '
        match self.bytes.get(self.pos) {
            Some(b'\\') => {
                self.pos += 1;
                if !self.escape(start) {
                    self.pos = self.bytes.len();
                    return;
                }
                if self.bytes.get(self.pos) == Some(&b'\'') {
                    self.pos += 1;
                    let text = &self.src[start..self.pos];
                    self.push_slice(TokenKind::CharLit, text, start, self.pos);
                } else {
                    self.error(start, "字符字面量缺少收尾引号 '");
                    self.pos = self.bytes.len();
                }
            }
            Some(&b) if Self::starts_ident(b) => {
                let run_start = self.pos;
                self.pos += 1;
                while self.pos < self.bytes.len() && Self::is_ident_cont(self.bytes[self.pos]) {
                    self.pos += 1;
                }
                if self.bytes.get(self.pos) == Some(&b'\'') {
                    // 'x'：单字符字面量（run 必须恰好一个 Unicode 标量）
                    let content = &self.src[run_start..self.pos];
                    if content.chars().count() != 1 {
                        self.error(start, "字符字面量只能包含一个字符");
                    }
                    self.pos += 1;
                    let text = &self.src[start..self.pos];
                    self.push_slice(TokenKind::CharLit, text, start, self.pos);
                } else {
                    // 'a / 'static：生命周期
                    let text = &self.src[start..self.pos];
                    self.push_slice(TokenKind::Lifetime, text, start, self.pos);
                }
            }
            _ => {
                self.error(start, "无法识别的 ' 起始记号");
                self.pos = self.bytes.len();
            }
        }
    }

    /// 解析 `\` 之后的转义序列；返回 false 表示致命（已报错且源耗尽）。
    fn escape(&mut self, lit_start: usize) -> bool {
        if self.pos >= self.bytes.len() {
            self.error(lit_start, "转义序列在源码末尾截断");
            return false;
        }
        let b = self.bytes[self.pos];
        self.pos += 1;
        match b {
            b'n' | b'r' | b't' | b'0' | b'\\' | b'\'' | b'"' => true,
            b'x' => {
                let ok = self.pos + 2 <= self.bytes.len()
                    && self.bytes[self.pos..self.pos + 2]
                        .iter()
                        .all(|&h| h.is_ascii_hexdigit());
                if ok {
                    self.pos += 2;
                    true
                } else {
                    self.error(lit_start, "\\x 需要两位十六进制数字");
                    false
                }
            }
            b'u' => {
                if self.bytes.get(self.pos) != Some(&b'{') {
                    self.error(lit_start, "\\u 需要形如 \\u{1F600} 的花括号形式");
                    return false;
                }
                self.pos += 1;
                let digits_start = self.pos;
                while self.pos < self.bytes.len()
                    && (self.bytes[self.pos].is_ascii_hexdigit() || self.bytes[self.pos] == b'_')
                {
                    self.pos += 1;
                }
                let n = self.pos - digits_start;
                if self.bytes.get(self.pos) != Some(&b'}') || n == 0 || n > 6 {
                    self.error(lit_start, "\\u{...} 需要 1~6 位十六进制数字并以 } 收尾");
                    return false;
                }
                self.pos += 1;
                true
            }
            _ => {
                self.pos -= 1;
                self.error(lit_start, "无法识别的转义序列");
                // 吃掉一个字符避免死循环
                self.pos += 1;
                true
            }
        }
    }

    // ---- 字符串 ----

    fn string_lit(&mut self) {
        let start = self.pos;
        self.pos += 1; // "
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'"' => {
                    self.pos += 1;
                    let text = &self.src[start..self.pos];
                    self.push_slice(TokenKind::StrLit, text, start, self.pos);
                    return;
                }
                b'\n' => {
                    self.error(start, "字符串字面量未闭合（遇到换行）");
                    let text = &self.src[start..self.pos];
                    self.push_slice(TokenKind::StrLit, text, start, self.pos);
                    return;
                }
                b'\\' => {
                    self.pos += 1;
                    if !self.escape(start) {
                        let text = &self.src[start..self.pos.min(self.bytes.len())];
                        self.push_slice(TokenKind::StrLit, text, start, self.pos);
                        return;
                    }
                }
                _ => self.pos += 1,
            }
        }
        self.error(start, "字符串字面量未闭合（源码末尾）");
        let text = &self.src[start..];
        self.push_slice(TokenKind::StrLit, text, start, self.pos);
    }

    fn raw_string_ahead(&self) -> bool {
        // r" 或 r#"……（r 后跟 0 个以上 # 再跟 "）
        let mut i = self.pos + 1;
        while self.bytes.get(i) == Some(&b'#') {
            i += 1;
        }
        self.bytes.get(i) == Some(&b'"')
    }

    fn raw_string_lit(&mut self) {
        let start = self.pos;
        self.pos += 1; // r
        let mut hashes = 0usize;
        while self.bytes.get(self.pos) == Some(&b'#') {
            hashes += 1;
            self.pos += 1;
        }
        self.pos += 1; // "
        loop {
            match self.bytes.get(self.pos) {
                None => {
                    self.error(start, "原始字符串字面量未闭合（源码末尾）");
                    let text = &self.src[start..];
                    self.push_slice(TokenKind::RawStrLit, text, start, self.pos);
                    return;
                }
                Some(b'"') => {
                    let mut end = self.pos + 1;
                    let mut seen = 0usize;
                    while seen < hashes && self.bytes.get(end) == Some(&b'#') {
                        seen += 1;
                        end += 1;
                    }
                    if seen == hashes {
                        let text = &self.src[start..end];
                        self.push_slice(TokenKind::RawStrLit, text, start, end);
                        self.pos = end;
                        return;
                    }
                    self.pos += 1;
                }
                Some(_) => self.pos += 1,
            }
        }
    }

    // ---- 数字 ----

    fn number_lit(&mut self) {
        let start = self.pos;
        let mut is_float = false;

        // 进制前缀
        let radix = if self.bytes[self.pos] == b'0' {
            match self.peek() {
                Some(b'x') => {
                    self.pos += 2;
                    Some(16)
                }
                Some(b'o') => {
                    self.pos += 2;
                    Some(8)
                }
                Some(b'b') => {
                    self.pos += 2;
                    Some(2)
                }
                _ => None,
            }
        } else {
            None
        };

        if let Some(radix) = radix {
            self.consume_digits(radix);
            if self.pos == start + 2 {
                self.error(start, "进制前缀后缺少数字");
            }
        } else {
            self.consume_digits(10);
        }

        // 小数部分：仅当 '.' 后紧跟数字（避免吃掉 `1..2` 的区间点与 `1.foo`）
        if radix.is_none()
            && self.bytes.get(self.pos) == Some(&b'.')
            && matches!(self.peek(), Some(b'0'..=b'9'))
        {
            is_float = true;
            self.pos += 1;
            self.consume_digits(10);
        }

        // 指数部分
        if radix.is_none() && matches!(self.bytes.get(self.pos), Some(b'e') | Some(b'E')) {
            let exp_at = self.pos;
            self.pos += 1;
            if matches!(self.bytes.get(self.pos), Some(b'+') | Some(b'-')) {
                self.pos += 1;
            }
            let d0 = self.pos;
            self.consume_digits(10);
            if self.pos == d0 {
                self.pos = exp_at;
                // 不是指数（如 0xE 之类已被 radix 分支处理；此处可能是后缀 e…）
                // grammar 未定义裸 e 后缀 → 交给后缀校验报错
            } else {
                is_float = true;
            }
        }

        // 后缀
        if self.pos < self.bytes.len() && Self::is_ident_cont(self.bytes[self.pos]) {
            let sfx_start = self.pos;
            while self.pos < self.bytes.len() && Self::is_ident_cont(self.bytes[self.pos]) {
                self.pos += 1;
            }
            let sfx = &self.src[sfx_start..self.pos];
            match sfx {
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" | "u8" | "u16" | "u32" | "u64"
                | "u128" | "usize"
                    if !is_float => {}
                "f32" | "f64" => is_float = true,
                _ => {
                    self.error(sfx_start, format!("无法识别的字面量后缀 {sfx:?}"));
                }
            }
        }

        let text = &self.src[start..self.pos];
        let kind = if is_float {
            TokenKind::FloatLit
        } else {
            TokenKind::IntLit
        };
        self.push_slice(kind, text, start, self.pos);
    }

    fn consume_digits(&mut self, radix: u8) {
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            if b == b'_' {
                self.pos += 1;
                continue;
            }
            if (b as char).is_digit(radix as u32) {
                self.pos += 1;
            } else {
                // 越界数字 / 后缀字母：留待后缀校验统一报错
                break;
            }
        }
    }

    // ---- 符号 ----

    fn punct(&mut self) {
        let start = self.pos;
        let rest = &self.bytes[self.pos..];
        let (kind, len) = if rest.starts_with(b"<<=") {
            (TokenKind::ShlEq, 3)
        } else if rest.starts_with(b">>=") {
            (TokenKind::ShrEq, 3)
        } else if rest.starts_with(b"..=") {
            (TokenKind::DotDotEq, 3)
        } else if rest.starts_with(b"&&") {
            (TokenKind::AmpAmp, 2)
        } else if rest.starts_with(b"||") {
            (TokenKind::PipePipe, 2)
        } else if rest.starts_with(b"<<") {
            (TokenKind::Shl, 2)
        } else if rest.starts_with(b">>") {
            (TokenKind::Shr, 2)
        } else if rest.starts_with(b"+=") {
            (TokenKind::PlusEq, 2)
        } else if rest.starts_with(b"-=") {
            (TokenKind::MinusEq, 2)
        } else if rest.starts_with(b"*=") {
            (TokenKind::StarEq, 2)
        } else if rest.starts_with(b"/=") {
            (TokenKind::SlashEq, 2)
        } else if rest.starts_with(b"%=") {
            (TokenKind::PercentEq, 2)
        } else if rest.starts_with(b"&=") {
            (TokenKind::AmpEq, 2)
        } else if rest.starts_with(b"|=") {
            (TokenKind::PipeEq, 2)
        } else if rest.starts_with(b"^=") {
            (TokenKind::CaretEq, 2)
        } else if rest.starts_with(b"==") {
            (TokenKind::EqEq, 2)
        } else if rest.starts_with(b"!=") {
            (TokenKind::Ne, 2)
        } else if rest.starts_with(b"<=") {
            (TokenKind::Le, 2)
        } else if rest.starts_with(b">=") {
            (TokenKind::Ge, 2)
        } else if rest.starts_with(b"..") {
            (TokenKind::DotDot, 2)
        } else if rest.starts_with(b"::") {
            (TokenKind::ColonColon, 2)
        } else if rest.starts_with(b"->") {
            (TokenKind::Arrow, 2)
        } else if rest.starts_with(b"=>") {
            (TokenKind::FatArrow, 2)
        } else {
            let k = match rest[0] {
                b'&' => TokenKind::Amp,
                b'|' => TokenKind::Pipe,
                b'^' => TokenKind::Caret,
                b'+' => TokenKind::Plus,
                b'-' => TokenKind::Minus,
                b'*' => TokenKind::Star,
                b'/' => TokenKind::Slash,
                b'%' => TokenKind::Percent,
                b'=' => TokenKind::Eq,
                b'<' => TokenKind::Lt,
                b'>' => TokenKind::Gt,
                b'!' => TokenKind::Bang,
                b'?' => TokenKind::Question,
                b'.' => TokenKind::Dot,
                b',' => TokenKind::Comma,
                b';' => TokenKind::Semi,
                b':' => TokenKind::Colon,
                b'#' => TokenKind::Hash,
                b'@' => TokenKind::At,
                b'(' => TokenKind::LParen,
                b')' => TokenKind::RParen,
                b'{' => TokenKind::LBrace,
                b'}' => TokenKind::RBrace,
                b'[' => TokenKind::LBracket,
                b']' => TokenKind::RBracket,
                other => {
                    let _ = other;
                    let end = self.src[self.pos..]
                        .chars()
                        .next()
                        .map_or(self.pos + 1, |c| self.pos + c.len_utf8());
                    self.pos = end;
                    self.error(
                        start,
                        format!("无法识别的字符 {:?}", &self.src[start..self.pos]),
                    );
                    return;
                }
            };
            (k, 1)
        };
        self.pos += len;
        let text = &self.src[start..self.pos];
        self.push_slice(kind, text, start, self.pos);
    }
}
