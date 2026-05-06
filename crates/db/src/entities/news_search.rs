use sea_orm::{ConnectionTrait, DbErr, Statement, Value};

#[derive(Clone, Debug)]
pub struct SearchKey {
    pub remote_id: String,
    pub game_code: String,
    pub source: String,
}

pub struct SearchQuery<'a> {
    pub q: &'a str,
    pub game: Option<&'a str>,
    pub source: Option<&'a str>,
    pub is_video: Option<bool>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

#[derive(Clone, Copy)]
enum Scope {
    All,
    Title,
    Tag,
    Game,
    Source,
    Category,
}

struct SearchGroup {
    negated: bool,
    terms: Vec<SearchTerm>,
}

struct SearchTerm {
    scope: Scope,
    value: String,
}

#[derive(Default)]
struct SearchFilters {
    game: Option<String>,
    source: Option<String>,
}

pub async fn search(query: SearchQuery<'_>) -> Result<Vec<SearchKey>, DbErr> {
    let (mut sql, mut values) = build_search_sql(&query, SearchSelect::Keys).await?;

    sql.push_str(" ORDER BY n.publish_time DESC");

    if let Some(limit) = query.limit {
        sql.push_str(" LIMIT ?");
        values.push((limit as i64).into());
    }

    if let Some(offset) = query.offset {
        sql.push_str(" OFFSET ?");
        values.push((offset as i64).into());
    }

    query_keys(sql, values).await
}

pub async fn count(query: SearchQuery<'_>) -> Result<u64, DbErr> {
    let (sql, values) = build_search_sql(&query, SearchSelect::Count).await?;

    query_count(sql, values).await
}

enum SearchSelect {
    Keys,
    Count,
}

async fn build_search_sql(
    query: &SearchQuery<'_>,
    select: SearchSelect,
) -> Result<(String, Vec<Value>), DbErr> {
    let mut groups = parse_query(query.q);
    let mut filters = SearchFilters {
        game: query.game.map(ToOwned::to_owned),
        source: query.source.map(ToOwned::to_owned),
    };

    groups.retain_mut(|group| extract_filter_group(group, &mut filters));
    resolve_game_filters(&mut groups, &mut filters).await?;

    let mut sql = match select {
        SearchSelect::Keys => String::from(
            r#"
            SELECT
                n.remote_id AS remote_id,
                n.game_code AS game_code,
                n.source AS source
            FROM news_items AS n
            WHERE 1 = 1
            "#,
        ),
        SearchSelect::Count => String::from(
            r#"
            SELECT COUNT(*) AS total
            FROM news_items AS n
            WHERE 1 = 1
            "#,
        ),
    };
    let mut values: Vec<Value> = Vec::new();

    if let Some(game) = filters.game {
        sql.push_str(" AND n.game_code = ?");
        values.push(game.into());
    }

    if let Some(source) = filters.source {
        sql.push_str(" AND n.source = ?");
        values.push(source.into());
    }

    if let Some(is_video) = query.is_video {
        sql.push_str(" AND n.is_video = ?");
        values.push(is_video.into());
    }

    for group in &groups {
        push_group_clause(&mut sql, &mut values, group);
    }

    Ok((sql, values))
}

async fn resolve_game_filters(
    groups: &mut [SearchGroup],
    filters: &mut SearchFilters,
) -> Result<(), DbErr> {
    if let Some(game) = filters.game.take() {
        filters.game = Some(
            super::games::Entity::resolve_game_code(&game)
                .await?
                .unwrap_or_else(|| "\0".to_string()),
        );
    }

    for group in groups {
        for term in &mut group.terms {
            if matches!(term.scope, Scope::Game) {
                term.value = super::games::Entity::resolve_game_code(&term.value)
                    .await?
                    .unwrap_or_else(|| "\0".to_string());
            }
        }
    }

    Ok(())
}

fn parse_query(q: &str) -> Vec<SearchGroup> {
    split_query_tokens(q)
        .into_iter()
        .filter_map(|raw_token| {
            let (negated, token) = raw_token
                .strip_prefix('-')
                .map(|value| (true, value))
                .unwrap_or((false, raw_token.as_str()));
            let terms = parse_terms(token);

            if terms.is_empty() {
                None
            } else {
                Some(SearchGroup { negated, terms })
            }
        })
        .collect()
}

fn split_query_tokens(q: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut in_quote = false;

    for ch in q.chars() {
        match ch {
            '"' => {
                in_quote = !in_quote;
            }
            ch if ch.is_whitespace() && !in_quote => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn parse_terms(token: &str) -> Vec<SearchTerm> {
    let mut scope_override = None;
    let mut rest = token;

    if let Some((scope, value)) = token.split_once(':') {
        scope_override = match scope {
            "title" => Some(Scope::Title),
            "tag" => Some(Scope::Tag),
            "game" => Some(Scope::Game),
            "source" => Some(Scope::Source),
            "category" => Some(Scope::Category),
            _ => None,
        };

        if scope_override.is_some() {
            rest = value;
        }
    }

    rest.split('|')
        .filter_map(|value| {
            let value = value.trim();
            if value.is_empty() {
                return None;
            }

            Some(SearchTerm {
                scope: scope_override.unwrap_or(Scope::All),
                value: value.to_string(),
            })
        })
        .collect()
}

fn extract_filter_group(group: &mut SearchGroup, filters: &mut SearchFilters) -> bool {
    if group.negated || group.terms.len() != 1 {
        return true;
    }

    let term = &group.terms[0];
    match term.scope {
        Scope::Game => {
            filters.game = Some(term.value.clone());
            false
        }
        Scope::Source => {
            filters.source = Some(term.value.clone());
            false
        }
        _ => true,
    }
}

fn push_group_clause(sql: &mut String, values: &mut Vec<Value>, group: &SearchGroup) {
    if group.negated {
        sql.push_str(" AND NOT (");
    } else {
        sql.push_str(" AND (");
    }

    for (index, term) in group.terms.iter().enumerate() {
        if index > 0 {
            sql.push_str(" OR ");
        }

        push_term_clause(sql, values, term);
    }

    sql.push(')');
}

fn push_term_clause(sql: &mut String, values: &mut Vec<Value>, term: &SearchTerm) {
    match term.scope {
        Scope::All => {
            sql.push_str(
                r#"
                (
                    n.title LIKE ?
                    OR EXISTS (
                        SELECT 1
                        FROM news_tags_link AS ntl
                        WHERE ntl.news_remote_id = n.remote_id
                            AND ntl.news_game_belong = n.game_code
                            AND ntl.news_source_belong = n.source
                            AND ntl.tag_title LIKE ?
                    )
                )
                "#,
            );
            let pattern = like_pattern(&term.value);
            values.push(pattern.clone().into());
            values.push(pattern.into());
        }
        Scope::Title => {
            sql.push_str("n.title LIKE ?");
            values.push(like_pattern(&term.value).into());
        }
        Scope::Tag => {
            sql.push_str(
                r#"
                EXISTS (
                    SELECT 1
                    FROM news_tags_link AS ntl
                    WHERE ntl.news_remote_id = n.remote_id
                        AND ntl.news_game_belong = n.game_code
                        AND ntl.news_source_belong = n.source
                        AND ntl.tag_title LIKE ?
                )
                "#,
            );
            values.push(like_pattern(&term.value).into());
        }
        Scope::Game => {
            sql.push_str("n.game_code = ?");
            values.push(term.value.clone().into());
        }
        Scope::Source => {
            sql.push_str("n.source = ?");
            values.push(term.value.clone().into());
        }
        Scope::Category => {
            sql.push_str(
                r#"
                EXISTS (
                    SELECT 1
                    FROM news_categories_link AS ncl
                    WHERE ncl.news_remote_id = n.remote_id
                        AND ncl.news_game_belong = n.game_code
                        AND ncl.news_source_belong = n.source
                        AND ncl.category_title LIKE ?
                )
                "#,
            );
            values.push(like_pattern(&term.value).into());
        }
    }
}

fn like_pattern(keyword: &str) -> String {
    format!("%{keyword}%")
}

async fn query_keys(sql: String, values: Vec<Value>) -> Result<Vec<SearchKey>, DbErr> {
    let conn = crate::pool();
    let sql = numbered_placeholders(&sql);
    let rows = conn
        .query_all_raw(Statement::from_sql_and_values(
            conn.get_database_backend(),
            sql,
            values,
        ))
        .await?;
    let mut keys = Vec::with_capacity(rows.len());

    for row in rows {
        keys.push(SearchKey {
            remote_id: row.try_get("", "remote_id")?,
            game_code: row.try_get("", "game_code")?,
            source: row.try_get("", "source")?,
        });
    }

    Ok(keys)
}

async fn query_count(sql: String, values: Vec<Value>) -> Result<u64, DbErr> {
    let conn = crate::pool();
    let sql = numbered_placeholders(&sql);
    let Some(row) = conn
        .query_one_raw(Statement::from_sql_and_values(
            conn.get_database_backend(),
            sql,
            values,
        ))
        .await?
    else {
        return Ok(0);
    };

    let total: i64 = row.try_get("", "total")?;

    Ok(total as u64)
}

fn numbered_placeholders(sql: &str) -> String {
    let mut next = 1;
    let mut output = String::with_capacity(sql.len());

    for ch in sql.chars() {
        if ch == '?' {
            output.push('$');
            output.push_str(&next.to_string());
            next += 1;
        } else {
            output.push(ch);
        }
    }

    output
}
