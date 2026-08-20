mod sr;
mod ys;
mod zzz;

use sea_orm::sea_query::{Alias, Expr, Func};

pub(crate) use sr::list as list_sr;
pub(crate) use sr::list_entries as list_sr_entries;
pub(crate) use ys::list as list_ys;
pub(crate) use ys::list_entries as list_ys_entries;
pub(crate) use zzz::list as list_zzz;
pub(crate) use zzz::list_entries as list_zzz_entries;

/// 构造 SQLite JSON 字段读取表达式
fn json_field(column: Expr, path: &'static str) -> Expr {
    Expr::expr(Func::cust(Alias::new("json_extract")).arg(column).arg(path))
}
