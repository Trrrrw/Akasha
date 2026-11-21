use akasha_application::{
    news::{NewsSearchMatch, NewsSearchPart},
    search::TextQuery,
};
use sea_orm::{
    Condition, ConnectionTrait, DatabaseConnection, DbBackend, DbErr, Statement, TransactionTrait,
    sea_query::Expr,
};

const BACKFILL_BATCH_SIZE: u64 = 250;
const EXCERPT_BEFORE: usize = 32;
const EXCERPT_AFTER: usize = 64;

/// FTS5 虚拟表及其维护触发器需要直接使用 SQLite DDL
pub(crate) async fn initialize(conn: &DatabaseConnection) -> Result<(), DbErr> {
    conn.execute_unprepared(
        "CREATE TABLE IF NOT EXISTS news_search_document (
            search_id INTEGER PRIMARY KEY,
            game_id TEXT NOT NULL,
            source_id TEXT NOT NULL,
            news_id TEXT NOT NULL,
            title TEXT NOT NULL,
            body TEXT NOT NULL,
            UNIQUE (game_id, source_id, news_id),
            FOREIGN KEY (game_id, source_id, news_id)
                REFERENCES news (game_id, source_id, id) ON DELETE CASCADE
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS news_search_fts USING fts5(
            title, body, content='news_search_document', content_rowid='search_id',
            tokenize='trigram case_sensitive 1'
        );
        CREATE TRIGGER IF NOT EXISTS news_search_insert AFTER INSERT ON news_search_document BEGIN
            INSERT INTO news_search_fts (rowid, title, body)
                VALUES (new.search_id, new.title, new.body);
        END;
        CREATE TRIGGER IF NOT EXISTS news_search_delete AFTER DELETE ON news_search_document BEGIN
            INSERT INTO news_search_fts (news_search_fts, rowid, title, body)
                VALUES ('delete', old.search_id, old.title, old.body);
        END;
        CREATE TRIGGER IF NOT EXISTS news_search_update AFTER UPDATE ON news_search_document BEGIN
            INSERT INTO news_search_fts (news_search_fts, rowid, title, body)
                VALUES ('delete', old.search_id, old.title, old.body);
            INSERT INTO news_search_fts (rowid, title, body)
                VALUES (new.search_id, new.title, new.body);
        END;",
    )
    .await?;
    loop {
        let txn = conn.begin().await?;
        let rows = txn
            .query_all_raw(Statement::from_sql_and_values(
                DbBackend::Sqlite,
                "SELECT n.game_id, n.source_id, n.id, n.title, n.intro
                FROM news n WHERE NOT EXISTS (
                    SELECT 1 FROM news_search_document d
                    WHERE (d.game_id, d.source_id, d.news_id) = (n.game_id, n.source_id, n.id)
                )
                ORDER BY n.game_id, n.source_id, n.id LIMIT ?",
                [BACKFILL_BATCH_SIZE.into()],
            ))
            .await?;
        for row in &rows {
            upsert(
                &txn,
                &row.try_get::<String>("", "game_id")?,
                &row.try_get::<String>("", "source_id")?,
                &row.try_get::<String>("", "id")?,
                &row.try_get::<String>("", "title")?,
                row.try_get::<Option<String>>("", "intro")?.as_deref(),
            )
            .await?;
        }
        txn.commit().await?;
        if rows.len() < BACKFILL_BATCH_SIZE as usize {
            return Ok(());
        }
    }
}

pub(super) async fn upsert(
    conn: &impl ConnectionTrait,
    game: &str,
    source: &str,
    id: &str,
    title: &str,
    intro: Option<&str>,
) -> Result<(), DbErr> {
    let body = plain_body(intro)?.to_ascii_lowercase();
    conn.execute_raw(Statement::from_sql_and_values(
        DbBackend::Sqlite,
        "INSERT INTO news_search_document (game_id, source_id, news_id, title, body)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT (game_id, source_id, news_id) DO UPDATE SET
            title = excluded.title, body = excluded.body
        WHERE title <> excluded.title OR body <> excluded.body",
        [
            game.into(),
            source.into(),
            id.into(),
            title.to_ascii_lowercase().into(),
            body.into(),
        ],
    ))
    .await?;
    Ok(())
}

fn plain_body(intro: Option<&str>) -> Result<String, DbErr> {
    let Some(intro) = intro.filter(|text| !text.is_empty()) else {
        return Ok(String::new());
    };
    let text = html2text::config::plain_no_decorate()
        .add_css("img, iframe, video, script, style { display: none }")
        .and_then(|config| config.string_from_read(intro.as_bytes(), usize::MAX))
        .map_err(|error| DbErr::Custom(format!("新闻正文转纯文本失败: {error}")))?;
    Ok(text.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// 按组匹配标题或正文。短词用 INSTR 保持字面量语义，长词走 FTS5
pub(super) fn condition(query: &TextQuery) -> Condition {
    let mut conditions = Condition::all();
    for group in &query.groups {
        let mut group_condition = Condition::any();
        for term in &group.alternatives {
            let term = term.to_ascii_lowercase();
            let expression = if term.chars().count() >= 3 && !term.contains('\0') {
                let phrase = format!("\"{}\"", term.replace('"', "\"\""));
                Expr::cust_with_values(
                    "(news.game_id, news.source_id, news.id) IN (
                        SELECT game_id, source_id, news_id FROM news_search_document
                        WHERE search_id IN (SELECT rowid FROM news_search_fts WHERE news_search_fts MATCH ?)
                    )",
                    [phrase],
                )
            } else {
                Expr::cust_with_values(
                    "(news.game_id, news.source_id, news.id) IN (
                        SELECT game_id, source_id, news_id FROM news_search_document
                        WHERE instr(title, ?) > 0 OR instr(body, ?) > 0
                    )",
                    [term.clone(), term],
                )
            };
            group_condition = group_condition.add(expression);
        }
        conditions = conditions.add(if group.excluded {
            group_condition.not()
        } else {
            group_condition
        });
    }
    conditions
}

pub(super) fn highlight_terms(query: &TextQuery) -> Vec<String> {
    query
        .groups
        .iter()
        .filter(|group| !group.excluded)
        .flat_map(|group| &group.alternatives)
        .map(|term| term.to_ascii_lowercase())
        .collect()
}

pub(super) fn excerpts(
    terms: &[String],
    title: &str,
    intro: Option<&str>,
) -> Result<Option<NewsSearchMatch>, DbErr> {
    let title = highlighted_parts(title, terms, false);
    let body = highlighted_parts(&plain_body(intro)?, terms, true);
    Ok(if title.is_some() || body.is_some() {
        Some(NewsSearchMatch { title, body })
    } else {
        None
    })
}

fn highlighted_parts(text: &str, terms: &[String], excerpt: bool) -> Option<Vec<NewsSearchPart>> {
    let lower = text.to_ascii_lowercase();
    let first = terms
        .iter()
        .filter_map(|term| lower.find(term).map(|start| (start, start + term.len())))
        .min_by_key(|&(start, end)| (start, std::cmp::Reverse(end)))?;
    let (start, end) = if excerpt {
        let start = text[..first.0]
            .char_indices()
            .rev()
            .nth(EXCERPT_BEFORE - 1)
            .map_or(0, |(position, _)| position);
        let end = text[first.1..]
            .char_indices()
            .nth(EXCERPT_AFTER)
            .map_or(text.len(), |(position, _)| first.1 + position);
        (start, end)
    } else {
        (0, text.len())
    };
    let mut matches = terms
        .iter()
        .flat_map(|term| {
            lower[start..end]
                .match_indices(term)
                .map(|(position, matched)| (start + position, start + position + matched.len()))
        })
        .collect::<Vec<_>>();
    matches.sort_unstable();

    let mut parts = Vec::new();
    let mut cursor = start;
    if start > 0 {
        parts.push(NewsSearchPart {
            text: "…".to_owned(),
            highlight: false,
        });
    }
    for (begin, finish) in matches {
        if finish <= cursor {
            continue;
        }
        if begin > cursor {
            parts.push(NewsSearchPart {
                text: text[cursor..begin].to_owned(),
                highlight: false,
            });
        }
        if begin < cursor
            && let Some(previous) = parts.last_mut()
        {
            previous.text.push_str(&text[cursor..finish]);
        } else {
            parts.push(NewsSearchPart {
                text: text[begin..finish].to_owned(),
                highlight: true,
            });
        }
        cursor = finish;
    }
    if cursor < end || end < text.len() {
        let mut trailing = text[cursor..end].to_owned();
        if end < text.len() {
            trailing.push('…');
        }
        parts.push(NewsSearchPart {
            text: trailing,
            highlight: false,
        });
    }
    Some(parts)
}
