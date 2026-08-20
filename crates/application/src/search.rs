use std::{collections::HashSet, fmt};

/// 公开文本查询允许的最大原始字符数
pub const MAX_TEXT_QUERY_CHARS: usize = 200;
/// 文本查询允许的最大 AND 条件组数
pub const MAX_TEXT_QUERY_GROUPS: usize = 8;
/// 单个条件组允许的最大 OR 分支数
pub const MAX_TEXT_QUERY_ALTERNATIVES_PER_GROUP: usize = 4;
/// 文本查询允许的最大 OR 分支总数
pub const MAX_TEXT_QUERY_ALTERNATIVES: usize = 16;
/// 单个查询词允许的最大字符数
pub const MAX_TEXT_QUERY_TERM_CHARS: usize = 64;

/// 已解析为 AND 条件组的文本查询
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextQuery {
    pub groups: Vec<TextQueryGroup>,
}

impl TextQuery {
    /// 解析支持空格 AND、竖线 OR、减号排除、引号短语和反斜杠转义的查询
    pub fn parse(value: &str) -> Result<Self, TextQueryParseError> {
        if value.chars().count() > MAX_TEXT_QUERY_CHARS {
            return Err(TextQueryParseError::new(format!(
                "q must not exceed {MAX_TEXT_QUERY_CHARS} characters"
            )));
        }

        let mut parser = TextQueryParser::default();
        for ch in value.chars() {
            parser.push(ch)?;
        }
        parser.finish()
    }

    /// 查询是否不包含任何有效条件
    pub fn is_empty(&self) -> bool {
        self.groups.is_empty()
    }
}

/// 一组可选文本，其中所有分支使用 OR，组与组之间使用 AND
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextQueryGroup {
    pub excluded: bool,
    pub alternatives: Vec<String>,
}

/// 文本查询语法或复杂度错误
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextQueryParseError {
    message: String,
}

impl TextQueryParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for TextQueryParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TextQueryParseError {}

#[derive(Debug, Default)]
struct TextQueryParser {
    groups: Vec<TextQueryGroup>,
    alternatives: Vec<String>,
    current: String,
    excluded: bool,
    group_started: bool,
    in_quotes: bool,
    escaped: bool,
    alternative_count: usize,
}

impl TextQueryParser {
    fn push(&mut self, ch: char) -> Result<(), TextQueryParseError> {
        if self.escaped {
            self.current.push(ch);
            self.group_started = true;
            self.escaped = false;
            return Ok(());
        }

        if ch == '\\' {
            self.escaped = true;
            self.group_started = true;
            return Ok(());
        }

        if ch == '"' {
            self.in_quotes = !self.in_quotes;
            self.group_started = true;
            return Ok(());
        }

        if !self.in_quotes {
            if ch.is_whitespace() {
                if self.group_started {
                    self.finish_group()?;
                }
                return Ok(());
            }

            if ch == '|' {
                self.finish_alternative()?;
                self.group_started = true;
                return Ok(());
            }

            if ch == '-' && !self.group_started {
                self.excluded = true;
                self.group_started = true;
                return Ok(());
            }
        }

        self.current.push(ch);
        self.group_started = true;
        Ok(())
    }

    fn finish(mut self) -> Result<TextQuery, TextQueryParseError> {
        if self.escaped {
            return Err(TextQueryParseError::new(
                "q must not end with an escape character",
            ));
        }
        if self.in_quotes {
            return Err(TextQueryParseError::new("q contains an unclosed quote"));
        }
        if self.group_started {
            self.finish_group()?;
        }

        Ok(TextQuery {
            groups: self.groups,
        })
    }

    fn finish_alternative(&mut self) -> Result<(), TextQueryParseError> {
        let value = self.current.trim();
        if value.is_empty() {
            return Err(TextQueryParseError::new(
                "q contains an empty OR alternative",
            ));
        }
        if value.chars().count() > MAX_TEXT_QUERY_TERM_CHARS {
            return Err(TextQueryParseError::new(format!(
                "q terms must not exceed {MAX_TEXT_QUERY_TERM_CHARS} characters"
            )));
        }

        self.alternatives.push(value.to_owned());
        self.current.clear();
        self.alternative_count += 1;
        if self.alternatives.len() > MAX_TEXT_QUERY_ALTERNATIVES_PER_GROUP {
            return Err(TextQueryParseError::new(format!(
                "q groups must not contain more than {MAX_TEXT_QUERY_ALTERNATIVES_PER_GROUP} alternatives"
            )));
        }
        if self.alternative_count > MAX_TEXT_QUERY_ALTERNATIVES {
            return Err(TextQueryParseError::new(format!(
                "q must not contain more than {MAX_TEXT_QUERY_ALTERNATIVES} alternatives"
            )));
        }
        Ok(())
    }

    fn finish_group(&mut self) -> Result<(), TextQueryParseError> {
        self.finish_alternative()?;

        let mut seen = HashSet::new();
        self.alternatives
            .retain(|value| seen.insert(value.to_lowercase()));
        self.groups.push(TextQueryGroup {
            excluded: self.excluded,
            alternatives: std::mem::take(&mut self.alternatives),
        });
        if self.groups.len() > MAX_TEXT_QUERY_GROUPS {
            return Err(TextQueryParseError::new(format!(
                "q must not contain more than {MAX_TEXT_QUERY_GROUPS} groups"
            )));
        }

        self.current.clear();
        self.excluded = false;
        self.group_started = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{TextQuery, TextQueryGroup};

    #[test]
    fn parses_boolean_groups_phrases_and_escapes() {
        let parsed = TextQuery::parse(r#"版本 前瞻|直播 -"问题修复" 标题\|文本"#)
            .expect("应解析受支持的查询语法");

        assert_eq!(
            parsed.groups,
            vec![
                TextQueryGroup {
                    excluded: false,
                    alternatives: vec!["版本".to_owned()],
                },
                TextQueryGroup {
                    excluded: false,
                    alternatives: vec!["前瞻".to_owned(), "直播".to_owned()],
                },
                TextQueryGroup {
                    excluded: true,
                    alternatives: vec!["问题修复".to_owned()],
                },
                TextQueryGroup {
                    excluded: false,
                    alternatives: vec!["标题|文本".to_owned()],
                },
            ]
        );
    }

    #[test]
    fn accepts_exclusion_only_queries() {
        let parsed = TextQuery::parse("-修复|补偿").expect("应允许只有排除条件的查询");

        assert_eq!(parsed.groups.len(), 1);
        assert!(parsed.groups[0].excluded);
        assert_eq!(parsed.groups[0].alternatives, ["修复", "补偿"]);
    }

    #[test]
    fn rejects_malformed_queries() {
        assert!(TextQuery::parse("版本|").is_err());
        assert!(TextQuery::parse("\"版本").is_err());
        assert!(TextQuery::parse("版本\\").is_err());
        assert!(TextQuery::parse("-").is_err());
    }

    #[test]
    fn treats_blank_query_as_empty() {
        assert!(TextQuery::parse("  \t ").expect("空查询应有效").is_empty());
    }
}
