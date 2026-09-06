//! 语法器：grammar.ebnf §1-§6 的递归下降实现。
//!
//! 错误策略：`PResult<T>` + 结构性 bail；`parse_program` / `parse_block`
//! 在 Err 后跳到下一个条目/语句边界继续解析，一次编译输出全部诊断。

use crate::ast::*;
use crate::diag::{Diagnostic, Span};
use crate::token::{Token, TokenKind as T};

struct Bail;
type PResult<T> = Result<T, Bail>;

/// 宏白名单（grammar.ebnf §6：阶段一 = 方案 C，仅编译器内建宏）。
const MACRO_WHITELIST: [&str; 4] = ["panic", "assert", "assert_eq", "unimplemented"];

pub fn parse(src: &str) -> (Program, Vec<Diagnostic>) {
    let (tokens, mut diags) = crate::lexer::lex(src);
    let mut p = Parser {
        tokens,
        pos: 0,
        diags: Vec::new(),
    };
    let program = match p.parse_program() {
        Ok(prog) => prog,
        Err(Bail) => Program {
            inner_attrs: Vec::new(),
            items: Vec::new(),
        },
    };
    diags.append(&mut p.diags);
    (program, diags)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    diags: Vec<Diagnostic>,
}

impl Parser {
    // ------------------------------------------------------------ 基础

    fn kind(&self) -> T {
        self.tokens[self.pos].kind
    }

    fn nth_kind(&self, n: usize) -> T {
        self.tokens.get(self.pos + n).map_or(T::Eof, |t| t.kind)
    }

    fn at(&self, k: T) -> bool {
        self.kind() == k
    }

    fn eat(&mut self, k: T) -> bool {
        if self.at(k) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn bump(&mut self) -> Token {
        let t = self.tokens[self.pos].clone();
        if self.pos + 1 < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn peek_text(&self) -> &str {
        &self.tokens[self.pos].text
    }

    fn error_here(&mut self, msg: impl Into<String>) {
        let span = self.tokens[self.pos.min(self.tokens.len() - 1)].span;
        self.diags.push(Diagnostic::new(span, msg));
    }

    fn expect(&mut self, k: T, what: &str) -> PResult<Token> {
        if self.at(k) {
            Ok(self.bump())
        } else {
            self.error_here(format!("expected {what}, found {}", self.peek_text()));
            Err(Bail)
        }
    }

    fn expect_ident(&mut self, what: &str) -> PResult<String> {
        if self.at(T::Ident) {
            Ok(self.bump().text)
        } else {
            self.error_here(format!("expected {what}, found {}", self.peek_text()));
            Err(Bail)
        }
    }

    fn span_from(&self, start: usize) -> Span {
        let start = self.tokens[start].span.start;
        let end = self.tokens[self.pos.saturating_sub(1)].span.end;
        Span::new(start as usize, end as usize)
    }

    /// 闭合泛型：`>` 或把 `>>` 劈成两个 `>`（吃半个）。
    fn expect_gt(&mut self, what: &str) -> PResult<()> {
        match self.kind() {
            T::Gt => {
                self.bump();
                Ok(())
            }
            T::Shr => {
                let tok = &mut self.tokens[self.pos];
                tok.kind = T::Gt;
                tok.text = ">".to_string();
                tok.span.end -= 1;
                Ok(())
            }
            _ => {
                self.error_here(format!("expected {what}, found {}", self.peek_text()));
                Err(Bail)
            }
        }
    }

    // ------------------------------------------------------------ 顶层

    fn parse_program(&mut self) -> PResult<Program> {
        let mut inner_attrs = Vec::new();
        let mut items = Vec::new();
        let mut seen_item = false;
        loop {
            if self.at(T::Hash) && self.nth_kind(1) == T::Bang {
                if seen_item {
                    self.error_here("内部属性 #![...] 只能出现在文件头部");
                }
                inner_attrs.push(self.parse_attr(true)?);
                continue;
            }
            if self.at(T::Eof) {
                break;
            }
            match self.parse_item() {
                Ok(item) => {
                    seen_item = true;
                    items.push(item);
                }
                Err(Bail) => self.recover_to_item_start(),
            }
        }
        Ok(Program { inner_attrs, items })
    }

    fn recover_to_item_start(&mut self) {
        loop {
            match self.kind() {
                T::Eof | T::RBrace => break,
                T::Fn
                | T::Struct
                | T::Enum
                | T::Trait
                | T::Impl
                | T::Mod
                | T::Use
                | T::Extern
                | T::Const
                | T::Static
                | T::Pub
                | T::Async
                | T::TypeKw
                | T::Hash => break,
                _ => {
                    self.bump();
                }
            }
        }
        self.eat(T::RBrace);
    }

    fn parse_item(&mut self) -> PResult<Item> {
        let start = self.pos;
        let mut attrs = Vec::new();
        while self.at(T::Hash) {
            attrs.push(self.parse_attr(false)?);
        }
        let vis = self.parse_visibility()?;
        let kind = self.parse_item_kind()?;
        let span = self.span_from(start);
        Ok(Item {
            kind,
            attrs,
            vis,
            span,
        })
    }

    fn parse_attr(&mut self, inner: bool) -> PResult<Attribute> {
        let start = self.pos;
        self.expect(T::Hash, "`#`")?;
        if inner {
            self.expect(T::Bang, "`!`")?;
        }
        self.expect(T::LBracket, "`[`")?;
        let mut path = vec![self.expect_ident("属性路径")?];
        while self.eat(T::ColonColon) {
            path.push(self.expect_ident("属性路径段")?);
        }
        let mut payload = Vec::new();
        let mut depth = 0usize;
        while !self.at(T::Eof) && (!self.at(T::RBracket) || depth > 0) {
            let t = self.bump();
            match t.kind {
                T::LParen | T::LBracket | T::LBrace => depth += 1,
                T::RParen | T::RBracket | T::RBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
            payload.push(AttrToken::from_token(&t));
        }
        self.expect(T::RBracket, "`]`")?;
        Ok(Attribute {
            inner,
            path,
            payload,
            span: self.span_from(start),
        })
    }

    fn parse_visibility(&mut self) -> PResult<Visibility> {
        if !self.eat(T::Pub) {
            return Ok(Visibility::Private);
        }
        if !self.eat(T::LParen) {
            return Ok(Visibility::Pub);
        }
        let vis = if self.eat(T::Crate) {
            Visibility::PubCrate
        } else if self.eat(T::SelfKw) {
            Visibility::PubSelf
        } else if self.eat(T::Super) {
            Visibility::PubSuper
        } else if self.eat(T::In) {
            let mut segs = vec![self.expect_ident("pub(in path)")?];
            while self.eat(T::ColonColon) {
                segs.push(self.expect_ident("pub(in path)")?);
            }
            Visibility::PubIn(segs)
        } else {
            self.error_here("expected crate|self|super|in");
            return Err(Bail);
        };
        self.expect(T::RParen, "`)`")?;
        Ok(vis)
    }

    fn parse_item_kind(&mut self) -> PResult<ItemKind> {
        match self.kind() {
            T::Fn | T::Async => Ok(ItemKind::Fn(self.parse_fn()?)),
            T::Struct => Ok(ItemKind::Struct(self.parse_struct()?)),
            T::Enum => Ok(ItemKind::Enum(self.parse_enum()?)),
            T::Trait => Ok(ItemKind::Trait(self.parse_trait()?)),
            T::Impl => Ok(ItemKind::Impl(self.parse_impl()?)),
            T::Mod => Ok(ItemKind::Mod(self.parse_mod()?)),
            T::Use => Ok(ItemKind::Use(self.parse_use()?)),
            T::Extern => Ok(ItemKind::ExternBlock(self.parse_extern()?)),
            T::TypeKw => Ok(ItemKind::TypeAlias(self.parse_type_alias()?)),
            T::Const => Ok(ItemKind::Const(self.parse_const()?)),
            T::Static => Ok(ItemKind::Static(self.parse_static()?)),
            _ => {
                self.error_here(format!("expected item, found {}", self.peek_text()));
                Err(Bail)
            }
        }
    }

    // ------------------------------------------------------------ 条目

    fn parse_generics(&mut self) -> PResult<Vec<GenericParam>> {
        if !self.eat(T::Lt) {
            return Ok(Vec::new());
        }
        let mut params = Vec::new();
        loop {
            if matches!(self.kind(), T::Gt | T::Shr) {
                break;
            }
            let pstart = self.pos;
            if self.at(T::Lifetime) {
                let name = self.bump().text;
                let bounds = self.parse_opt_bounds()?;
                params.push(GenericParam {
                    name,
                    bounds,
                    span: self.span_from(pstart),
                });
            } else {
                let name = self.expect_ident("泛型参数")?;
                let bounds = self.parse_opt_bounds()?;
                params.push(GenericParam {
                    name,
                    bounds,
                    span: self.span_from(pstart),
                });
            }
            if !self.eat(T::Comma) {
                break;
            }
        }
        self.expect_gt("`>`")?;
        Ok(params)
    }

    fn parse_opt_bounds(&mut self) -> PResult<Vec<GenericBound>> {
        if !self.eat(T::Colon) {
            return Ok(Vec::new());
        }
        self.parse_bounds()
    }

    fn parse_bounds(&mut self) -> PResult<Vec<GenericBound>> {
        let mut bounds = vec![self.parse_bound()?];
        while self.eat(T::Plus) {
            bounds.push(self.parse_bound()?);
        }
        Ok(bounds)
    }

    fn parse_bound(&mut self) -> PResult<GenericBound> {
        if self.at(T::Lifetime) {
            Ok(GenericBound::Lifetime(self.bump().text))
        } else {
            Ok(GenericBound::Trait(self.parse_path_type()?))
        }
    }

    fn parse_where_clause(&mut self) -> PResult<Vec<WherePred>> {
        if !self.eat(T::Where) {
            return Ok(Vec::new());
        }
        let mut preds = Vec::new();
        loop {
            // where 子句以 `{` / `;` 收尾；逗号既可分隔也可尾置
            if self.at(T::LBrace) || self.at(T::Semi) {
                break;
            }
            let subject = if self.at(T::Lifetime) {
                let name = self.bump().text;
                WhereSubject::Lifetime(name)
            } else {
                WhereSubject::Type(self.parse_type()?)
            };
            self.expect(T::Colon, "`:`（where 谓词）")?;
            let bounds = self.parse_bounds()?;
            preds.push(WherePred { subject, bounds });
            if !self.eat(T::Comma) {
                break;
            }
        }
        Ok(preds)
    }

    fn parse_fn(&mut self) -> PResult<FnItem> {
        let start = self.pos;
        let is_async = self.eat(T::Async);
        self.expect(T::Fn, "`fn`")?;
        let name = self.expect_ident("函数名")?;
        let generics = self.parse_generics()?;
        let params = self.parse_fn_params()?;
        let ret = if self.eat(T::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let where_clause = self.parse_where_clause()?;
        let body = if self.at(T::LBrace) {
            Some(self.parse_block()?)
        } else {
            self.expect(T::Semi, "`{` 或 `;`")?;
            None
        };
        let span = self.span_from(start);
        Ok(FnItem {
            name,
            generics,
            params,
            ret,
            where_clause,
            body,
            is_async,
            span,
        })
    }

    fn parse_fn_params(&mut self) -> PResult<Vec<Param>> {
        self.expect(T::LParen, "`(`")?;
        let mut params = Vec::new();
        if self.eat(T::RParen) {
            return Ok(params);
        }
        loop {
            let pstart = self.pos;
            let mut attrs = Vec::new();
            while self.at(T::Hash) {
                attrs.push(self.parse_attr(false)?);
            }
            let kind = self.parse_param_kind()?;
            params.push(Param {
                kind,
                attrs,
                span: self.span_from(pstart),
            });
            if !self.eat(T::Comma) {
                break;
            }
            if self.at(T::RParen) {
                break;
            }
        }
        self.expect(T::RParen, "`)`")?;
        Ok(params)
    }

    fn parse_param_kind(&mut self) -> PResult<ParamKind> {
        // mut self / mut self: T
        if self.at(T::Mut) && self.nth_kind(1) == T::SelfKw {
            self.bump();
            self.bump();
            if self.eat(T::Colon) {
                let ty = self.parse_type()?;
                Ok(ParamKind::SelfTyped {
                    ty: Box::new(ty),
                    is_mut: true,
                })
            } else {
                Ok(ParamKind::SelfVal { is_mut: true })
            }
        } else if self.at(T::SelfKw) {
            self.bump();
            if self.eat(T::Colon) {
                let ty = self.parse_type()?;
                Ok(ParamKind::SelfTyped {
                    ty: Box::new(ty),
                    is_mut: false,
                })
            } else {
                Ok(ParamKind::SelfVal { is_mut: false })
            }
        } else if self.at(T::Amp) {
            self.bump();
            let lifetime = if self.at(T::Lifetime) {
                Some(self.bump().text)
            } else {
                None
            };
            let is_mut = self.eat(T::Mut);
            self.expect(T::SelfKw, "`self`")?;
            Ok(ParamKind::SelfRef { is_mut, lifetime })
        } else {
            let pat = self.parse_pattern()?;
            self.expect(T::Colon, "`:`")?;
            let ty = self.parse_type()?;
            Ok(ParamKind::Pattern { pat, ty })
        }
    }

    fn parse_struct(&mut self) -> PResult<StructItem> {
        let start = self.pos;
        self.expect(T::Struct, "`struct`")?;
        let name = self.expect_ident("结构体名")?;
        let generics = self.parse_generics()?;
        let where_clause = self.parse_where_clause()?;
        let fields = if self.eat(T::Semi) {
            Vec::new()
        } else {
            self.expect(T::LBrace, "`{` 或 `;`")?;
            let fields = self.parse_fields()?;
            self.expect(T::RBrace, "`}`")?;
            fields
        };
        let span = self.span_from(start);
        Ok(StructItem {
            name,
            generics,
            fields,
            where_clause,
            span,
        })
    }

    fn parse_fields(&mut self) -> PResult<Vec<Field>> {
        let mut fields = Vec::new();
        while !self.at(T::RBrace) {
            let fstart = self.pos;
            let vis = self.parse_visibility()?;
            let name = self.expect_ident("字段名")?;
            self.expect(T::Colon, "`:`")?;
            let ty = self.parse_type()?;
            fields.push(Field {
                vis,
                name,
                ty,
                span: self.span_from(fstart),
            });
            if !self.eat(T::Comma) {
                break;
            }
        }
        Ok(fields)
    }

    fn parse_enum(&mut self) -> PResult<EnumItem> {
        let start = self.pos;
        self.expect(T::Enum, "`enum`")?;
        let name = self.expect_ident("枚举名")?;
        let generics = self.parse_generics()?;
        let where_clause = self.parse_where_clause()?;
        self.expect(T::LBrace, "`{`")?;
        let mut variants = Vec::new();
        while !self.at(T::RBrace) {
            let vstart = self.pos;
            let vis = self.parse_visibility()?;
            let vname = self.expect_ident("变体名")?;
            let payload = if self.eat(T::LParen) {
                let mut tys = Vec::new();
                while !self.at(T::RParen) {
                    tys.push(self.parse_type()?);
                    if !self.eat(T::Comma) {
                        break;
                    }
                }
                self.expect(T::RParen, "`)`")?;
                Some(VariantPayload::Tuple(tys))
            } else if self.at(T::LBrace) {
                self.bump();
                let fields = self.parse_fields()?;
                self.expect(T::RBrace, "`}`")?;
                Some(VariantPayload::Struct(fields))
            } else {
                None
            };
            variants.push(Variant {
                vis,
                name: vname,
                payload,
                span: self.span_from(vstart),
            });
            if !self.eat(T::Comma) {
                break;
            }
        }
        self.expect(T::RBrace, "`}`")?;
        let span = self.span_from(start);
        Ok(EnumItem {
            name,
            generics,
            where_clause,
            variants,
            span,
        })
    }

    fn parse_trait(&mut self) -> PResult<TraitItem> {
        let start = self.pos;
        self.expect(T::Trait, "`trait`")?;
        let name = self.expect_ident("trait 名")?;
        let generics = self.parse_generics()?;
        let where_clause = self.parse_where_clause()?;
        self.expect(T::LBrace, "`{`")?;
        let mut members = Vec::new();
        while !self.at(T::RBrace) && !self.at(T::Eof) {
            match self.parse_item() {
                Ok(item) => members.push(item),
                Err(Bail) => self.recover_stmt(),
            }
        }
        self.expect(T::RBrace, "`}`")?;
        let span = self.span_from(start);
        Ok(TraitItem {
            name,
            generics,
            where_clause,
            members,
            span,
        })
    }

    fn parse_impl(&mut self) -> PResult<ImplItem> {
        let start = self.pos;
        self.expect(T::Impl, "`impl`")?;
        let generics = self.parse_generics()?;
        let first = self.parse_type()?;
        let (trait_path, self_ty) = if self.eat(T::For) {
            let path = match first.kind {
                TypeKind::Path(p) => p,
                _ => {
                    self.error_here("impl trait 形式的首类型必须是路径");
                    return Err(Bail);
                }
            };
            (Some(path), self.parse_type()?)
        } else {
            (None, first)
        };
        let where_clause = self.parse_where_clause()?;
        self.expect(T::LBrace, "`{`")?;
        let mut members = Vec::new();
        while !self.at(T::RBrace) && !self.at(T::Eof) {
            match self.parse_item() {
                Ok(item) => members.push(item),
                Err(Bail) => self.recover_stmt(),
            }
        }
        self.expect(T::RBrace, "`}`")?;
        let span = self.span_from(start);
        Ok(ImplItem {
            trait_path,
            self_ty,
            generics,
            where_clause,
            members,
            span,
        })
    }

    fn parse_mod(&mut self) -> PResult<ModItem> {
        let start = self.pos;
        self.expect(T::Mod, "`mod`")?;
        let name = self.expect_ident("模块名")?;
        let body = if self.eat(T::Semi) {
            None
        } else {
            self.expect(T::LBrace, "`{` 或 `;`")?;
            let mut items = Vec::new();
            while !self.at(T::RBrace) && !self.at(T::Eof) {
                match self.parse_item() {
                    Ok(item) => items.push(item),
                    Err(Bail) => self.recover_to_item_start(),
                }
            }
            self.expect(T::RBrace, "`}`")?;
            Some(items)
        };
        let span = self.span_from(start);
        Ok(ModItem { name, body, span })
    }

    fn parse_use(&mut self) -> PResult<UseItem> {
        let start = self.pos;
        self.expect(T::Use, "`use`")?;
        let tree = self.parse_use_tree()?;
        self.expect(T::Semi, "`;`")?;
        let span = self.span_from(start);
        Ok(UseItem { tree, span })
    }

    fn parse_use_tree(&mut self) -> PResult<UseTree> {
        if self.eat(T::Star) {
            return Ok(UseTree::Glob);
        }
        if self.at(T::LBrace) {
            let nested = self.parse_use_nested()?;
            return Ok(UseTree::Nested(nested));
        }
        let name = self.parse_use_segment_name()?;
        if self.eat(T::ColonColon) {
            let rest = self.parse_use_tree()?;
            Ok(UseTree::Path {
                prefix: name,
                rest: Box::new(rest),
            })
        } else {
            let alias = if self.eat(T::As) {
                if self.at(T::Ident) {
                    Some(self.bump().text)
                } else {
                    self.error_here("expected 别名标识符（含保留别名 `default`，D-20）");
                    return Err(Bail);
                }
            } else {
                None
            };
            Ok(UseTree::Name { name, alias })
        }
    }

    fn parse_use_nested(&mut self) -> PResult<Vec<UseTree>> {
        self.expect(T::LBrace, "`{`")?;
        let mut trees = Vec::new();
        while !self.at(T::RBrace) {
            trees.push(self.parse_use_tree()?);
            if !self.eat(T::Comma) {
                break;
            }
        }
        self.expect(T::RBrace, "`}`")?;
        Ok(trees)
    }

    fn parse_use_segment_name(&mut self) -> PResult<String> {
        match self.kind() {
            T::Ident => Ok(self.bump().text),
            T::SelfKw | T::Super | T::Crate => Ok(self.bump().text),
            _ => {
                self.error_here(format!("expected use 路径段, found {}", self.peek_text()));
                Err(Bail)
            }
        }
    }

    fn parse_extern(&mut self) -> PResult<ExternBlock> {
        let start = self.pos;
        self.expect(T::Extern, "`extern`")?;
        let abi = if self.at(T::StrLit) {
            let t = self.bump();
            Some(t.text)
        } else {
            None
        };
        self.expect(T::LBrace, "`{`")?;
        let mut fns = Vec::new();
        while !self.at(T::RBrace) && !self.at(T::Eof) {
            let fstart = self.pos;
            while self.at(T::Hash) {
                self.parse_attr(false)?;
            }
            self.expect(T::Fn, "`fn`")?;
            let name = self.expect_ident("extern 函数名")?;
            let params = self.parse_extern_params()?;
            let ret = if self.eat(T::Arrow) {
                Some(self.parse_type()?)
            } else {
                None
            };
            self.expect(T::Semi, "`;`")?;
            fns.push(ExternFn {
                name,
                params,
                ret,
                span: self.span_from(fstart),
            });
        }
        self.expect(T::RBrace, "`}`")?;
        let span = self.span_from(start);
        Ok(ExternBlock { abi, fns, span })
    }

    fn parse_extern_params(&mut self) -> PResult<Vec<(Pattern, Type)>> {
        self.expect(T::LParen, "`(`")?;
        let mut params = Vec::new();
        while !self.at(T::RParen) {
            let pat = self.parse_pattern()?;
            self.expect(T::Colon, "`:`")?;
            let ty = self.parse_type()?;
            params.push((pat, ty));
            if !self.eat(T::Comma) {
                break;
            }
        }
        self.expect(T::RParen, "`)`")?;
        Ok(params)
    }

    fn parse_type_alias(&mut self) -> PResult<TypeAliasItem> {
        let start = self.pos;
        self.expect(T::TypeKw, "`type`")?;
        let name = self.expect_ident("类型别名")?;
        let generics = self.parse_generics()?;
        self.expect(T::Eq, "`=`")?;
        let ty = self.parse_type()?;
        self.expect(T::Semi, "`;`")?;
        let span = self.span_from(start);
        Ok(TypeAliasItem {
            name,
            generics,
            ty,
            span,
        })
    }

    fn parse_const(&mut self) -> PResult<ConstItem> {
        let start = self.pos;
        self.expect(T::Const, "`const`")?;
        let name = self.expect_ident("常量名")?;
        self.expect(T::Colon, "`:`")?;
        let ty = self.parse_type()?;
        self.expect(T::Eq, "`=`")?;
        let value = self.parse_expr()?;
        self.expect(T::Semi, "`;`")?;
        let span = self.span_from(start);
        Ok(ConstItem {
            name,
            ty,
            value,
            span,
        })
    }

    fn parse_static(&mut self) -> PResult<StaticItem> {
        let start = self.pos;
        self.expect(T::Static, "`static`")?;
        let is_mut = self.eat(T::Mut);
        let name = self.expect_ident("静态量名")?;
        self.expect(T::Colon, "`:`")?;
        let ty = self.parse_type()?;
        self.expect(T::Eq, "`=`")?;
        let value = self.parse_expr()?;
        self.expect(T::Semi, "`;`")?;
        let span = self.span_from(start);
        Ok(StaticItem {
            name,
            is_mut,
            ty,
            value,
            span,
        })
    }

    // ------------------------------------------------------------ 类型

    fn parse_type(&mut self) -> PResult<Type> {
        let start = self.pos;
        let kind = match self.kind() {
            T::Amp => {
                self.bump();
                let lifetime = if self.at(T::Lifetime) {
                    Some(self.bump().text)
                } else {
                    None
                };
                let is_mut = self.eat(T::Mut);
                let inner = Box::new(self.parse_type()?);
                TypeKind::Ref {
                    lifetime,
                    is_mut,
                    inner,
                }
            }
            T::Star => {
                self.bump();
                if self.eat(T::Anyopaque) {
                    TypeKind::Ptr(PtrKind::Anyopaque)
                } else {
                    let allowzero = self.eat(T::Allowzero);
                    let is_mut = if self.eat(T::Const) {
                        false
                    } else {
                        self.expect(T::Mut, "`const` 或 `mut`")?;
                        true
                    };
                    let inner = Box::new(self.parse_type()?);
                    match (allowzero, is_mut) {
                        (false, false) => TypeKind::Ptr(PtrKind::Const(inner)),
                        (false, true) => TypeKind::Ptr(PtrKind::Mut(inner)),
                        (true, false) => TypeKind::Ptr(PtrKind::AllowzeroConst(inner)),
                        (true, true) => TypeKind::Ptr(PtrKind::AllowzeroMut(inner)),
                    }
                }
            }
            T::LBracket => {
                self.bump();
                let elem = Box::new(self.parse_type()?);
                if self.eat(T::Semi) {
                    let len = Box::new(self.parse_expr()?);
                    self.expect(T::RBracket, "`]`")?;
                    TypeKind::Array { elem, len }
                } else {
                    self.expect(T::RBracket, "`;` 或 `]`")?;
                    TypeKind::Slice(elem)
                }
            }
            T::LParen => {
                self.bump();
                let mut tys = Vec::new();
                let mut trailing_comma = false;
                while !self.at(T::RParen) {
                    tys.push(self.parse_type()?);
                    if !self.eat(T::Comma) {
                        trailing_comma = false;
                        break;
                    }
                    trailing_comma = true;
                }
                self.expect(T::RParen, "`)`")?;
                match tys.len() {
                    0 => TypeKind::Tuple(Vec::new()),
                    1 if !trailing_comma => tys.pop().unwrap().kind,
                    _ => TypeKind::Tuple(tys),
                }
            }
            T::Bang => {
                self.bump();
                TypeKind::Never
            }
            T::Ident if self.peek_text() == "_" => {
                self.bump();
                TypeKind::Inferred
            }
            _ => TypeKind::Path(self.parse_path_type()?),
        };
        let span = self.span_from(start);
        Ok(Type { kind, span })
    }

    fn parse_path_type(&mut self) -> PResult<PathType> {
        let start = self.pos;
        let mut segments = Vec::new();
        let first = self.parse_path_segment()?;
        segments.push(first);
        while self.eat(T::ColonColon) {
            segments.push(self.parse_path_segment()?);
        }
        let span = self.span_from(start);
        Ok(PathType { segments, span })
    }

    fn parse_path_segment(&mut self) -> PResult<PathSegment> {
        let name = match self.kind() {
            T::Ident => self.bump().text,
            T::SelfKw | T::Super | T::Crate => self.bump().text,
            _ => {
                self.error_here(format!("expected 路径段, found {}", self.peek_text()));
                return Err(Bail);
            }
        };
        let mut generic_args = Vec::new();
        if self.at(T::Lt) {
            self.bump();
            generic_args = self.parse_generic_args()?;
        } else if self.at(T::ColonColon) && self.nth_kind(1) == T::Lt {
            self.bump(); // ::
            self.bump(); // <
            generic_args = self.parse_generic_args()?;
        }
        Ok(PathSegment { name, generic_args })
    }

    fn parse_generic_args(&mut self) -> PResult<Vec<Type>> {
        let mut args = Vec::new();
        while !matches!(self.kind(), T::Gt | T::Shr) {
            args.push(self.parse_type()?);
            if !self.eat(T::Comma) {
                break;
            }
        }
        self.expect_gt("`>`")?;
        Ok(args)
    }

    // ------------------------------------------------------------ 模式

    fn parse_pattern(&mut self) -> PResult<Pattern> {
        let start = self.pos;
        let kind = match self.kind() {
            T::IntLit => {
                let text = self.bump().text;
                self.try_finish_range(text, false)?
            }
            T::CharLit => {
                let text = self.bump().text;
                self.try_finish_range(text, true)?
            }
            T::FloatLit => PatternKind::Float(self.bump().text),
            T::StrLit => PatternKind::Str(self.bump().text),
            T::RawStrLit => PatternKind::Str(self.bump().text),
            T::True | T::False => PatternKind::Bool(self.eat(T::True)),
            T::Amp => {
                self.bump();
                let inner = self.parse_pattern()?;
                PatternKind::Ref(Box::new(inner))
            }
            T::Mut => {
                self.bump();
                PatternKind::Binding {
                    reference: false,
                    is_mut: true,
                    name: self.expect_ident("绑定名")?,
                }
            }
            T::LParen => {
                self.bump();
                let mut pats = Vec::new();
                while !self.at(T::RParen) {
                    pats.push(self.parse_pattern()?);
                    if !self.eat(T::Comma) {
                        break;
                    }
                }
                self.expect(T::RParen, "`)`")?;
                PatternKind::Tuple(pats)
            }
            T::Ident | T::SelfKw | T::Super | T::Crate => {
                if self.peek_text() == "_" && self.at(T::Ident) {
                    self.bump();
                    PatternKind::Wildcard
                } else {
                    let path = self.parse_path_type()?;
                    match self.kind() {
                        T::LBrace => {
                            self.bump();
                            let mut fields = Vec::new();
                            while !self.at(T::RBrace) {
                                let fname = self.expect_ident("字段模式")?;
                                let sub = if self.eat(T::Colon) {
                                    Some(self.parse_pattern()?)
                                } else {
                                    None
                                };
                                fields.push((fname, sub));
                                if !self.eat(T::Comma) {
                                    break;
                                }
                            }
                            self.expect(T::RBrace, "`}`")?;
                            PatternKind::Struct { path, fields }
                        }
                        T::LParen => {
                            self.bump();
                            let mut args = Vec::new();
                            while !self.at(T::RParen) {
                                args.push(self.parse_pattern()?);
                                if !self.eat(T::Comma) {
                                    break;
                                }
                            }
                            self.expect(T::RParen, "`)`")?;
                            PatternKind::Enum { path, args }
                        }
                        _ => PatternKind::Enum {
                            path,
                            args: Vec::new(),
                        },
                    }
                }
            }
            _ => {
                self.error_here(format!("expected 模式, found {}", self.peek_text()));
                return Err(Bail);
            }
        };
        let span = self.span_from(start);
        Ok(Pattern { kind, span })
    }

    /// 字面量后若跟 `..` / `..=`，补完 range pattern。
    fn try_finish_range(&mut self, start: String, is_char: bool) -> PResult<PatternKind> {
        let inclusive = match self.kind() {
            T::DotDot => false,
            T::DotDotEq => true,
            _ => return Ok(Self::lit_pattern(&start, is_char)),
        };
        // 仅 int / char 字面量可作为终点（Float 不行）；端点是 range op 的下一个 token
        if !matches!(self.nth_kind(1), T::IntLit | T::CharLit) {
            return Ok(Self::lit_pattern(&start, is_char));
        }
        self.bump();
        let end = self.bump().text;
        Ok(PatternKind::Range {
            start,
            inclusive,
            end,
        })
    }

    fn lit_pattern(start: &str, is_char: bool) -> PatternKind {
        if is_char {
            PatternKind::Char(start.to_string())
        } else {
            PatternKind::Int(start.to_string())
        }
    }

    // ------------------------------------------------------------ 语句与块

    fn parse_block(&mut self) -> PResult<Block> {
        let start = self.pos;
        self.expect(T::LBrace, "`{`")?;
        let mut stmts = Vec::new();
        let mut tail: Option<Box<Expr>> = None;
        loop {
            match self.kind() {
                T::RBrace => {
                    self.bump();
                    break;
                }
                T::Eof => {
                    self.error_here("块未闭合（遇到文件结束）");
                    return Err(Bail);
                }
                T::Semi => {
                    self.bump();
                    stmts.push(Stmt {
                        kind: StmtKind::Empty,
                        span: self.span_from(self.pos - 1),
                    });
                }
                T::Fn
                | T::Struct
                | T::Enum
                | T::Trait
                | T::Impl
                | T::Mod
                | T::Use
                | T::Extern
                | T::Const
                | T::Static
                | T::TypeKw
                | T::Pub
                | T::Hash => {
                    let item = self.parse_item()?;
                    let span = item.span;
                    stmts.push(Stmt {
                        kind: StmtKind::Item(Box::new(item)),
                        span,
                    });
                }
                T::Async if self.nth_kind(1) == T::Fn => {
                    let item = self.parse_item()?;
                    let span = item.span;
                    stmts.push(Stmt {
                        kind: StmtKind::Item(Box::new(item)),
                        span,
                    });
                }
                T::Let => {
                    let s = self.parse_let_stmt()?;
                    stmts.push(s);
                }
                T::Defer => {
                    let sstart = self.pos;
                    self.bump();
                    let body = self.parse_expr_or_block()?;
                    self.expect(T::Semi, "`;`")?;
                    stmts.push(Stmt {
                        kind: StmtKind::Defer(body),
                        span: self.span_from(sstart),
                    });
                }
                T::Errdefer => {
                    let sstart = self.pos;
                    self.bump();
                    let body = self.parse_expr_or_block()?;
                    self.expect(T::Semi, "`;`")?;
                    stmts.push(Stmt {
                        kind: StmtKind::Errdefer(body),
                        span: self.span_from(sstart),
                    });
                }
                _ => {
                    let estart = self.pos;
                    match self.parse_expr() {
                        Ok(expr) => {
                            if self.at(T::Semi) {
                                self.bump();
                                stmts.push(Stmt {
                                    kind: StmtKind::Expr {
                                        expr,
                                        requires_semi: true,
                                    },
                                    span: self.span_from(estart),
                                });
                            } else if self.at(T::RBrace) {
                                tail = Some(Box::new(expr));
                            } else if expr_is_block_like(&expr) {
                                // 块状表达式作语句可省略分号（Rust 惯例；DEVIATIONS #14）
                                stmts.push(Stmt {
                                    kind: StmtKind::Expr {
                                        expr,
                                        requires_semi: false,
                                    },
                                    span: self.span_from(estart),
                                });
                            } else {
                                self.error_here("expected `;` 或 `}`");
                                self.recover_stmt();
                            }
                        }
                        Err(Bail) => self.recover_stmt(),
                    }
                }
            }
        }
        let span = self.span_from(start);
        Ok(Block { stmts, tail, span })
    }

    fn recover_stmt(&mut self) {
        while !matches!(self.kind(), T::Semi | T::RBrace | T::Eof) {
            self.bump();
        }
        self.eat(T::Semi);
    }

    fn parse_expr_or_block(&mut self) -> PResult<ExprOrBlock> {
        if self.at(T::LBrace) {
            Ok(ExprOrBlock::Block(self.parse_block()?))
        } else {
            Ok(ExprOrBlock::Expr(self.parse_expr()?))
        }
    }

    fn parse_let_stmt(&mut self) -> PResult<Stmt> {
        let start = self.pos;
        self.expect(T::Let, "`let`")?;
        let pat = self.parse_pattern()?;
        let ty = if self.eat(T::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let init = if self.eat(T::Eq) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        self.expect(T::Semi, "`;`")?;
        let span = self.span_from(start);
        Ok(Stmt {
            kind: StmtKind::Let { pat, ty, init },
            span,
        })
    }

    // ------------------------------------------------------------ 表达式

    fn parse_expr(&mut self) -> PResult<Expr> {
        self.parse_assign_expr()
    }

    fn parse_assign_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let lhs = self.parse_range_expr()?;
        let op = match self.kind() {
            T::Eq => Some(AssignOp::Assign),
            T::PlusEq => Some(AssignOp::Add),
            T::MinusEq => Some(AssignOp::Sub),
            T::StarEq => Some(AssignOp::Mul),
            T::SlashEq => Some(AssignOp::Div),
            T::PercentEq => Some(AssignOp::Rem),
            T::AmpEq => Some(AssignOp::BitAnd),
            T::PipeEq => Some(AssignOp::BitOr),
            T::CaretEq => Some(AssignOp::BitXor),
            T::ShlEq => Some(AssignOp::Shl),
            T::ShrEq => Some(AssignOp::Shr),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let rhs = self.parse_assign_expr()?; // 右结合
            let span = self.span_from(start);
            return Ok(Expr {
                kind: ExprKind::Assign {
                    op,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            });
        }
        Ok(lhs)
    }

    fn parse_range_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let lhs = self.parse_or_expr()?;
        let op = match self.kind() {
            T::DotDot => RangeOp::Exclusive,
            T::DotDotEq => RangeOp::Inclusive,
            _ => return Ok(lhs),
        };
        self.bump();
        let end = if matches!(
            self.kind(),
            T::Semi | T::RBrace | T::RParen | T::Comma | T::RBracket | T::Eof | T::FatArrow
        ) {
            None
        } else {
            Some(Box::new(self.parse_or_expr()?))
        };
        let span = self.span_from(start);
        Ok(Expr {
            kind: ExprKind::Range {
                op,
                start: Some(Box::new(lhs)),
                end,
            },
            span,
        })
    }

    fn parse_or_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let mut lhs = self.parse_and_expr()?;
        while self.eat(T::PipePipe) {
            let rhs = self.parse_and_expr()?;
            let span = self.span_from(start);
            lhs = Expr {
                kind: ExprKind::Binary {
                    op: BinOp::Or,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_and_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let mut lhs = self.parse_cmp_expr()?;
        while self.eat(T::AmpAmp) {
            let rhs = self.parse_cmp_expr()?;
            let span = self.span_from(start);
            lhs = Expr {
                kind: ExprKind::Binary {
                    op: BinOp::And,
                    lhs: Box::new(lhs),
                    rhs: Box::new(rhs),
                },
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_cmp_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let lhs = self.parse_bit_or_expr()?;
        let op = match self.kind() {
            T::EqEq => BinOp::Eq,
            T::Ne => BinOp::Ne,
            T::Lt => BinOp::Lt,
            T::Gt => BinOp::Gt,
            T::Le => BinOp::Le,
            T::Ge => BinOp::Ge,
            _ => return Ok(lhs),
        };
        self.bump();
        let rhs = self.parse_bit_or_expr()?; // 单次比较，非结合
        let span = self.span_from(start);
        Ok(Expr {
            kind: ExprKind::Binary {
                op,
                lhs: Box::new(lhs),
                rhs: Box::new(rhs),
            },
            span,
        })
    }

    fn parse_bit_or_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let mut lhs = self.parse_bit_xor_expr()?;
        while self.eat(T::Pipe) {
            let rhs = self.parse_bit_xor_expr()?;
            let span = self.span_from(start);
            lhs = bin(BinOp::BitOr, lhs, rhs, span);
        }
        Ok(lhs)
    }

    fn parse_bit_xor_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let mut lhs = self.parse_bit_and_expr()?;
        while self.eat(T::Caret) {
            let rhs = self.parse_bit_and_expr()?;
            let span = self.span_from(start);
            lhs = bin(BinOp::BitXor, lhs, rhs, span);
        }
        Ok(lhs)
    }

    fn parse_bit_and_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let mut lhs = self.parse_shift_expr()?;
        while self.eat(T::Amp) {
            let rhs = self.parse_shift_expr()?;
            let span = self.span_from(start);
            lhs = bin(BinOp::BitAnd, lhs, rhs, span);
        }
        Ok(lhs)
    }

    fn parse_shift_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let mut lhs = self.parse_add_expr()?;
        loop {
            let op = match self.kind() {
                T::Shl => BinOp::Shl,
                T::Shr => BinOp::Shr,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_add_expr()?;
            let span = self.span_from(start);
            lhs = bin(op, lhs, rhs, span);
        }
        Ok(lhs)
    }

    fn parse_add_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let mut lhs = self.parse_mul_expr()?;
        loop {
            let op = match self.kind() {
                T::Plus => BinOp::Add,
                T::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_mul_expr()?;
            let span = self.span_from(start);
            lhs = bin(op, lhs, rhs, span);
        }
        Ok(lhs)
    }

    fn parse_mul_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let mut lhs = self.parse_cast_expr()?;
        loop {
            let op = match self.kind() {
                T::Star => BinOp::Mul,
                T::Slash => BinOp::Div,
                T::Percent => BinOp::Rem,
                _ => break,
            };
            self.bump();
            let rhs = self.parse_cast_expr()?;
            let span = self.span_from(start);
            lhs = bin(op, lhs, rhs, span);
        }
        Ok(lhs)
    }

    fn parse_cast_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let expr = self.parse_unary_expr()?;
        if self.eat(T::As) {
            let ty = Box::new(self.parse_type()?);
            let span = self.span_from(start);
            return Ok(Expr {
                kind: ExprKind::Cast {
                    expr: Box::new(expr),
                    ty,
                },
                span,
            });
        }
        Ok(expr)
    }

    /// 前缀 `-` `!` `*` 与借位 `&` `&mut` `&&`（DEVIATIONS #10：借位并入一元层）。
    fn parse_unary_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let op = match self.kind() {
            T::Minus => Some(UnOp::Neg),
            T::Bang => Some(UnOp::Not),
            T::Star => Some(UnOp::Deref),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let expr = self.parse_unary_expr()?;
            let span = self.span_from(start);
            return Ok(Expr {
                kind: ExprKind::Unary {
                    op,
                    expr: Box::new(expr),
                },
                span,
            });
        }
        // 借位前缀
        if self.at(T::Amp) || self.at(T::AmpAmp) {
            let double = self.eat(T::AmpAmp);
            if !double {
                self.bump(); // &
            }
            let is_mut = !double && self.eat(T::Mut);
            let inner = self.parse_unary_expr()?;
            let span = self.span_from(start);
            let inner = if double {
                Expr {
                    kind: ExprKind::Borrow {
                        is_mut: false,
                        inner: Box::new(inner),
                    },
                    span,
                }
            } else {
                inner
            };
            return Ok(Expr {
                kind: ExprKind::Borrow {
                    is_mut,
                    inner: Box::new(inner),
                },
                span,
            });
        }
        self.parse_postfix_expr()
    }

    fn parse_postfix_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let mut expr = self.parse_primary_expr()?;
        loop {
            let kind = match self.kind() {
                T::Question => ExprKind::Try(Box::new(expr)),
                T::DotDot | T::DotDotEq => break,
                T::Dot => {
                    self.bump();
                    match self.kind() {
                        T::Await => ExprKind::Await(Box::new(expr)),
                        T::IntLit => {
                            let idx: u32 = self.bump().text.parse().map_err(|_| {
                                self.error_here("元组字段索引过大");
                                Bail
                            })?;
                            ExprKind::TupleField {
                                expr: Box::new(expr),
                                index: idx,
                            }
                        }
                        T::Ident => ExprKind::Field {
                            expr: Box::new(expr),
                            name: self.bump().text,
                        },
                        _ => {
                            self.error_here("expected `.` 之后的字段名 / await / 索引");
                            return Err(Bail);
                        }
                    }
                }
                T::LParen => {
                    self.bump();
                    let mut args = Vec::new();
                    while !self.at(T::RParen) {
                        args.push(self.parse_expr()?);
                        if !self.eat(T::Comma) {
                            break;
                        }
                    }
                    self.expect(T::RParen, "`)`")?;
                    ExprKind::Call {
                        callee: Box::new(expr),
                        args,
                    }
                }
                T::LBracket => {
                    self.bump();
                    let index = self.parse_expr()?;
                    self.expect(T::RBracket, "`]`")?;
                    ExprKind::Index {
                        expr: Box::new(expr),
                        index: Box::new(index),
                    }
                }
                _ => break,
            };
            self.bump_if_postfix_consumed(&kind);
            let span = self.span_from(start);
            expr = Expr { kind, span };
        }
        Ok(expr)
    }

    /// Question / Dot / LParen / LBracket 的消费在分支外统一处理：
    /// 分支内已消费 `.` 与 `(` `[` 的开头，这里补上单 token 后缀。
    fn bump_if_postfix_consumed(&mut self, kind: &ExprKind) {
        match kind {
            ExprKind::Try(_) | ExprKind::Await(_) => {
                self.bump(); // ? 或 await
            }
            _ => {}
        }
    }

    fn parse_primary_expr(&mut self) -> PResult<Expr> {
        let start = self.pos;
        let kind = match self.kind() {
            T::IntLit => ExprKind::Int(self.bump().text),
            T::FloatLit => ExprKind::Float(self.bump().text),
            T::CharLit => ExprKind::Char(self.bump().text),
            T::StrLit => ExprKind::Str(self.bump().text),
            T::RawStrLit => ExprKind::RawStr(self.bump().text),
            T::True => {
                self.bump();
                ExprKind::Bool(true)
            }
            T::False => {
                self.bump();
                ExprKind::Bool(false)
            }
            T::Ident | T::SelfKw | T::Super | T::Crate => {
                if self.at(T::Ident) && self.nth_kind(1) == T::Bang {
                    let name = self.bump().text;
                    self.parse_macro_body(&name)?
                } else {
                    ExprKind::Path(self.parse_path_type()?)
                }
            }
            T::LParen => {
                self.bump();
                let mut exprs = Vec::new();
                let mut trailing_comma = false;
                while !self.at(T::RParen) {
                    exprs.push(self.parse_expr()?);
                    if !self.eat(T::Comma) {
                        trailing_comma = false;
                        break;
                    }
                    trailing_comma = true;
                }
                self.expect(T::RParen, "`)`")?;
                match exprs.len() {
                    0 => ExprKind::Tuple(Vec::new()),
                    1 if !trailing_comma => ExprKind::Group(Box::new(exprs.pop().unwrap())),
                    _ => ExprKind::Tuple(exprs),
                }
            }
            T::LBracket => {
                self.bump();
                if self.at(T::RBracket) {
                    self.bump();
                    ExprKind::Array(Vec::new())
                } else {
                    let first = self.parse_expr()?;
                    if self.eat(T::Semi) {
                        let len = self.parse_expr()?;
                        self.expect(T::RBracket, "`]`")?;
                        ExprKind::ArrayRepeat {
                            elem: Box::new(first),
                            len: Box::new(len),
                        }
                    } else {
                        let mut exprs = vec![first];
                        while self.eat(T::Comma) {
                            if self.at(T::RBracket) {
                                break;
                            }
                            exprs.push(self.parse_expr()?);
                        }
                        self.expect(T::RBracket, "`,` 或 `]`")?;
                        ExprKind::Array(exprs)
                    }
                }
            }
            T::LBrace => ExprKind::Block(self.parse_block()?),
            T::If => self.parse_if()?,
            T::Match => self.parse_match()?,
            T::Loop => {
                self.bump();
                ExprKind::Loop(self.parse_block()?)
            }
            T::While => self.parse_while()?,
            T::For => self.parse_for()?,
            T::Return => {
                self.bump();
                let e = if self.expr_can_start() {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                ExprKind::Return(e)
            }
            T::Break => {
                self.bump();
                let e = if self.expr_can_start() {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                ExprKind::Break(e)
            }
            T::Continue => {
                self.bump();
                ExprKind::Continue
            }
            T::Pipe | T::PipePipe | T::Move => self.parse_closure()?,
            T::Async => {
                self.bump();
                let is_move = self.eat(T::Move);
                ExprKind::AsyncBlock {
                    is_move,
                    body: self.parse_block()?,
                }
            }
            T::At => {
                self.bump();
                let name = self.expect_ident("@ 内建名")?;
                self.expect(T::LParen, "`(`")?;
                let mut args = Vec::new();
                while !self.at(T::RParen) {
                    args.push(self.parse_expr()?);
                    if !self.eat(T::Comma) {
                        break;
                    }
                }
                self.expect(T::RParen, "`)`")?;
                ExprKind::Builtin { name, args }
            }
            _ => {
                self.error_here(format!("expected 表达式, found {}", self.peek_text()));
                return Err(Bail);
            }
        };
        let span = self.span_from(start);
        Ok(Expr { kind, span })
    }

    fn expr_can_start(&self) -> bool {
        matches!(
            self.kind(),
            T::IntLit
                | T::FloatLit
                | T::CharLit
                | T::StrLit
                | T::RawStrLit
                | T::True
                | T::False
                | T::Ident
                | T::SelfKw
                | T::Super
                | T::Crate
                | T::LParen
                | T::LBracket
                | T::LBrace
                | T::If
                | T::Match
                | T::Loop
                | T::While
                | T::For
                | T::Return
                | T::Break
                | T::Continue
                | T::Pipe
                | T::PipePipe
                | T::Move
                | T::Async
                | T::At
                | T::Minus
                | T::Bang
                | T::Star
                | T::Amp
                | T::AmpAmp
        )
    }

    fn parse_macro_body(&mut self, name: &str) -> PResult<ExprKind> {
        self.expect(T::Bang, "`!`")?;
        let span = self.tokens[self.pos].span;
        if !MACRO_WHITELIST.contains(&name) {
            self.diags.push(Diagnostic::new(
                span,
                format!(
                    "宏 {name}! 不在 MVP 白名单（仅 panic!/assert!/assert_eq!/unimplemented!）"
                ),
            ));
        }
        let body = match self.kind() {
            T::LParen => {
                self.bump();
                let toks = self.collect_attr_tokens(T::RParen)?;
                MacroBody::Paren(toks)
            }
            T::LBracket => {
                self.bump();
                let toks = self.collect_attr_tokens(T::RBracket)?;
                MacroBody::Bracket(toks)
            }
            T::LBrace => {
                self.bump();
                let mut raw = Vec::new();
                let mut depth = 0usize;
                while !self.at(T::Eof) {
                    if self.at(T::LBrace) {
                        depth += 1;
                    } else if self.at(T::RBrace) {
                        if depth == 0 {
                            break;
                        }
                        depth -= 1;
                    }
                    raw.push(self.bump());
                }
                self.expect(T::RBrace, "`}`")?;
                MacroBody::Brace(raw)
            }
            _ => {
                self.error_here("expected `(` / `[` / `{`（宏体）");
                return Err(Bail);
            }
        };
        Ok(ExprKind::Macro {
            name: name.to_string(),
            body,
        })
    }

    fn collect_attr_tokens(&mut self, close: T) -> PResult<Vec<AttrToken>> {
        let mut toks = Vec::new();
        let mut depth = 0usize;
        while !self.at(T::Eof) && (self.kind() != close || depth > 0) {
            let t = self.bump();
            match t.kind {
                T::LParen | T::LBracket | T::LBrace => depth += 1,
                T::RParen | T::RBracket | T::RBrace => depth = depth.saturating_sub(1),
                _ => {}
            }
            toks.push(AttrToken::from_token(&t));
        }
        self.expect(close, "闭合括号")?;
        Ok(toks)
    }

    fn parse_if(&mut self) -> PResult<ExprKind> {
        self.expect(T::If, "`if`")?;
        let cond = Box::new(self.parse_expr()?);
        let then = self.parse_block()?;
        let els = if self.eat(T::Else) {
            if self.at(T::If) {
                Some(Box::new(Expr {
                    kind: self.parse_if()?,
                    span: self.span_from(self.pos),
                }))
            } else {
                Some(Box::new(Expr {
                    kind: ExprKind::Block(self.parse_block()?),
                    span: self.span_from(self.pos),
                }))
            }
        } else {
            None
        };
        Ok(ExprKind::If { cond, then, els })
    }

    fn parse_match(&mut self) -> PResult<ExprKind> {
        self.expect(T::Match, "`match`")?;
        let scrutinee = Box::new(self.parse_expr()?);
        self.expect(T::LBrace, "`{`")?;
        let mut arms = Vec::new();
        while !self.at(T::RBrace) && !self.at(T::Eof) {
            let pat = self.parse_pattern()?;
            let guard = if self.eat(T::If) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            self.expect(T::FatArrow, "`=>`")?;
            let body = self.parse_expr()?;
            arms.push(MatchArm { pat, guard, body });
            if !self.eat(T::Comma) {
                break;
            }
        }
        self.expect(T::RBrace, "`}`")?;
        Ok(ExprKind::Match { scrutinee, arms })
    }

    fn parse_while(&mut self) -> PResult<ExprKind> {
        self.expect(T::While, "`while`")?;
        if self.eat(T::Let) {
            let pat = self.parse_pattern()?;
            self.expect(T::Eq, "`=`")?;
            let expr = Box::new(self.parse_expr()?);
            let body = self.parse_block()?;
            return Ok(ExprKind::WhileLet { pat, expr, body });
        }
        let cond = Box::new(self.parse_expr()?);
        let body = self.parse_block()?;
        Ok(ExprKind::While { cond, body })
    }

    fn parse_for(&mut self) -> PResult<ExprKind> {
        self.expect(T::For, "`for`")?;
        let pat = self.parse_pattern()?;
        self.expect(T::In, "`in`")?;
        let iter = Box::new(self.parse_expr()?);
        let body = self.parse_block()?;
        Ok(ExprKind::For { pat, iter, body })
    }

    fn parse_closure(&mut self) -> PResult<ExprKind> {
        let is_move = self.eat(T::Move);
        let mut params = Vec::new();
        if self.eat(T::PipePipe) {
            // 零参闭包 || body
        } else {
            self.expect(T::Pipe, "`|`")?;
            while !self.at(T::Pipe) {
                let pat = self.parse_pattern()?;
                if self.eat(T::Colon) {
                    self.parse_type()?;
                }
                params.push(pat);
                if !self.eat(T::Comma) {
                    break;
                }
            }
            self.expect(T::Pipe, "`|`")?;
        }
        let ret = if self.eat(T::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };
        let body = Box::new(self.parse_expr()?);
        Ok(ExprKind::Closure {
            is_move,
            params,
            ret,
            body,
        })
    }
}

fn bin(op: BinOp, lhs: Expr, rhs: Expr, span: Span) -> Expr {
    Expr {
        kind: ExprKind::Binary {
            op,
            lhs: Box::new(lhs),
            rhs: Box::new(rhs),
        },
        span,
    }
}

/// 块状表达式：语句位置可省略分号。
fn expr_is_block_like(e: &Expr) -> bool {
    matches!(
        e.kind,
        ExprKind::If { .. }
            | ExprKind::Match { .. }
            | ExprKind::Loop { .. }
            | ExprKind::While { .. }
            | ExprKind::WhileLet { .. }
            | ExprKind::For { .. }
            | ExprKind::Block { .. }
            | ExprKind::AsyncBlock { .. }
    )
}
