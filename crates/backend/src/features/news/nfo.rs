use akasha_application::news::{NewsSeries, NewsSummary};
use anyhow::Context;
use serde::Serialize;

use super::{china_timezone, video_duration_seconds};
use crate::http::response::public_asset_url;

const XML_DECLARATION: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>";
const NFO_STUDIO: &str = "Akasha";
const FILENAME_SEGMENT_LIMIT: usize = 48;
const NFO_PLOT_CSS: &str = "* { white-space: pre; }";

/// 可直接作为 HTTP 文件响应返回的 NFO 文档
pub(super) struct NfoDocument {
    /// 完整 XML 文本
    pub(super) xml: String,
    /// 只包含安全 ASCII 字符的下载文件名
    pub(super) filename: String,
}

/// Kodi 和 Jellyfin 使用的单视频 Movie NFO 根节点
#[derive(Serialize)]
#[serde(rename = "movie")]
struct MovieNfo {
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    plot: Option<String>,
    premiered: String,
    uniqueid: UniqueIdentifier,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumb: Option<String>,
    #[serde(rename = "tag")]
    tags: Vec<String>,
    studio: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    fileinfo: Option<MediaFileInfo>,
}

/// Kodi 和 Jellyfin 使用的 TV Show NFO 根节点
#[derive(Serialize)]
#[serde(rename = "tvshow")]
struct TvShowNfo {
    title: String,
    season: u32,
    episode: u64,
    premiered: String,
    uniqueid: UniqueIdentifier,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumb: Option<String>,
    #[serde(rename = "tag")]
    tags: Vec<String>,
    studio: &'static str,
}

/// Kodi 和 Jellyfin 使用的单集 NFO 根节点
#[derive(Serialize)]
#[serde(rename = "episodedetails")]
struct EpisodeNfo {
    title: String,
    showtitle: String,
    season: u32,
    episode: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    plot: Option<String>,
    aired: String,
    uniqueid: UniqueIdentifier,
    #[serde(skip_serializing_if = "Option::is_none")]
    thumb: Option<String>,
    studio: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    fileinfo: Option<MediaFileInfo>,
}

/// 生成视频新闻单集 NFO 所需的剧集上下文
pub(super) struct EpisodeNfoContext {
    game_id: String,
    source_id: String,
    series: NewsSeries,
    season: u32,
    episode: u32,
}

impl EpisodeNfoContext {
    /// 创建一个已经由 HTTP 层校验季集编号的单集上下文
    pub(super) fn new(
        game_id: &str,
        source_id: &str,
        series: NewsSeries,
        season: u32,
        episode: u32,
    ) -> Self {
        Self {
            game_id: game_id.to_owned(),
            source_id: source_id.to_owned(),
            series,
            season,
            episode,
        }
    }
}

/// 标记 Akasha 新闻主键的 NFO 唯一标识
#[derive(Serialize)]
struct UniqueIdentifier {
    #[serde(rename = "@type")]
    identifier_type: &'static str,
    #[serde(rename = "@default")]
    is_default: bool,
    #[serde(rename = "$text")]
    value: String,
}

/// NFO 媒体文件信息
#[derive(Serialize)]
struct MediaFileInfo {
    streamdetails: StreamDetails,
}

/// NFO 媒体流信息
#[derive(Serialize)]
struct StreamDetails {
    video: VideoStreamDetails,
}

/// NFO 视频流信息
#[derive(Serialize)]
struct VideoStreamDetails {
    durationinseconds: u64,
}

/// 将一条独立视频新闻转换为 Movie NFO 文档
pub(super) fn build_movie(
    game_id: &str,
    source_id: &str,
    news: NewsSummary,
    game_cover: Option<String>,
    asset_base_url: &str,
) -> anyhow::Result<NfoDocument> {
    let NewsSummary {
        id,
        title,
        publish_time,
        cover,
        tags,
        video_duration_ms,
        intro,
        ..
    } = news;

    // 先把来源内容规范化为 NFO 支持的纯文本和绝对资源地址
    let fallback_title = format!("news-{id}");
    let title = non_empty_xml_text(&title).unwrap_or(fallback_title);
    let plot = intro.as_deref().map(render_plot).transpose()?.flatten();
    let thumb = public_asset_url(asset_base_url, cover.or(game_cover))
        .and_then(|value| non_empty_xml_text(&value));
    let tags = tags
        .iter()
        .filter_map(|tag| non_empty_xml_text(tag))
        .collect();

    // 使用来源和新闻主键生成跨媒体库稳定的唯一标识
    let movie = MovieNfo {
        title,
        plot,
        premiered: publish_time
            .with_timezone(&china_timezone())
            .format("%Y-%m-%d")
            .to_string(),
        uniqueid: unique_identifier([game_id, source_id, id.as_str()]),
        thumb,
        tags,
        studio: NFO_STUDIO,
        fileinfo: media_file_info(video_duration_ms),
    };

    let filename = format!(
        "akasha-{}-{}-{}.nfo",
        safe_filename_segment(game_id),
        safe_filename_segment(source_id),
        safe_filename_segment(&id)
    );

    serialize_document(&movie, filename, "序列化视频新闻 Movie NFO 失败")
}

/// 将一个视频标签转换为 TV Show NFO 文档
pub(super) fn build_series(
    game_id: &str,
    source_id: &str,
    series: NewsSeries,
    asset_base_url: &str,
) -> anyhow::Result<NfoDocument> {
    let title = series_title(&series.game_name, &series.tag_name);
    let thumb =
        public_asset_url(asset_base_url, series.cover).and_then(|value| non_empty_xml_text(&value));
    let tv_show = TvShowNfo {
        title,
        season: 1,
        episode: series.episode_count,
        premiered: series
            .premiered
            .with_timezone(&china_timezone())
            .format("%Y-%m-%d")
            .to_string(),
        uniqueid: unique_identifier([game_id, source_id, "tag", series.tag_name.as_str()]),
        thumb,
        tags: non_empty_xml_text(&series.tag_name).into_iter().collect(),
        studio: NFO_STUDIO,
    };

    serialize_document(
        &tv_show,
        "tvshow.nfo".to_owned(),
        "序列化标签 TV Show NFO 失败",
    )
}

/// 将一条标签内的视频新闻转换为 Episode NFO 文档
pub(super) fn build_episode(
    context: EpisodeNfoContext,
    news: NewsSummary,
    asset_base_url: &str,
) -> anyhow::Result<NfoDocument> {
    let NewsSummary {
        id,
        title,
        publish_time,
        cover,
        video_duration_ms,
        intro,
        ..
    } = news;

    // 单集封面缺失时使用该标签剧集的代表封面
    let title = non_empty_xml_text(&title).unwrap_or_else(|| format!("news-{id}"));
    let plot = intro.as_deref().map(render_plot).transpose()?.flatten();
    let thumb = public_asset_url(asset_base_url, cover.or(context.series.cover.clone()))
        .and_then(|value| non_empty_xml_text(&value));
    let episode_nfo = EpisodeNfo {
        title,
        showtitle: series_title(&context.series.game_name, &context.series.tag_name),
        season: context.season,
        episode: context.episode,
        plot,
        aired: publish_time
            .with_timezone(&china_timezone())
            .format("%Y-%m-%d")
            .to_string(),
        uniqueid: unique_identifier([
            context.game_id.as_str(),
            context.source_id.as_str(),
            id.as_str(),
        ]),
        thumb,
        studio: NFO_STUDIO,
        fileinfo: media_file_info(video_duration_ms),
    };
    let filename = format!(
        "akasha-{}-{}-{}-s{:02}e{:02}.nfo",
        safe_filename_segment(&context.game_id),
        safe_filename_segment(&context.source_id),
        safe_filename_segment(&id),
        context.season,
        context.episode
    );

    serialize_document(&episode_nfo, filename, "序列化视频新闻 Episode NFO 失败")
}

/// 使用统一 XML 声明序列化一种 NFO 根节点
fn serialize_document(
    value: &impl Serialize,
    filename: String,
    error_context: &'static str,
) -> anyhow::Result<NfoDocument> {
    // quick-xml 负责转义文本，显式添加媒体服务器普遍识别的 XML 声明
    let body = quick_xml::se::to_string(value).context(error_context)?;
    let xml = format!("{XML_DECLARATION}\n{body}\n");

    Ok(NfoDocument { xml, filename })
}

/// 创建 Akasha 命名空间中的稳定 NFO 唯一标识
fn unique_identifier<'a>(parts: impl IntoIterator<Item = &'a str>) -> UniqueIdentifier {
    let value = parts
        .into_iter()
        .map(sanitize_xml_text)
        .collect::<Vec<_>>()
        .join(":");

    UniqueIdentifier {
        identifier_type: "akasha",
        is_default: true,
        value,
    }
}

/// 将毫秒时长转换为 NFO 视频流信息
fn media_file_info(video_duration_ms: Option<i64>) -> Option<MediaFileInfo> {
    let durationinseconds = video_duration_ms
        .and_then(video_duration_seconds)
        .filter(|duration| *duration > 0)?;

    Some(MediaFileInfo {
        streamdetails: StreamDetails {
            video: VideoStreamDetails { durationinseconds },
        },
    })
}

/// 使用游戏名和标签名生成媒体库中的剧集标题
fn series_title(game_name: &str, tag_name: &str) -> String {
    let game_prefix = format!("《{}》", sanitize_xml_text(game_name).trim());
    let tag_name = non_empty_xml_text(tag_name).unwrap_or_else(|| "视频合集".to_owned());

    if tag_name.starts_with(&game_prefix) {
        tag_name
    } else {
        format!("{game_prefix}{tag_name}")
    }
}

/// 将新闻简介转换为保留来源换行且不重新排版的纯文本
fn render_plot(value: &str) -> anyhow::Result<Option<String>> {
    let plain_text = html2text::config::plain_no_decorate()
        .add_css(NFO_PLOT_CSS)
        .context("配置新闻简介空白规则失败")?
        .string_from_read(value.as_bytes(), usize::MAX)
        .context("转换新闻简介失败")?;
    Ok(non_empty_xml_text(plain_text.trim()))
}

/// 清除 XML 1.0 不允许的控制字符
fn sanitize_xml_text(value: &str) -> String {
    value
        .chars()
        .filter(|character| is_xml_character(*character))
        .collect()
}

/// 仅在清理后的 XML 文本非空时返回内容
fn non_empty_xml_text(value: &str) -> Option<String> {
    let value = sanitize_xml_text(value);
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

/// 判断字符是否属于 XML 1.0 允许的字符范围
fn is_xml_character(character: char) -> bool {
    matches!(character, '\u{9}' | '\u{A}' | '\u{D}')
        || ('\u{20}'..='\u{D7FF}').contains(&character)
        || ('\u{E000}'..='\u{FFFD}').contains(&character)
        || ('\u{10000}'..='\u{10FFFF}').contains(&character)
}

/// 将外部标识压缩为适合 Content-Disposition 的文件名片段
fn safe_filename_segment(value: &str) -> String {
    let mut segment = String::with_capacity(value.len().min(FILENAME_SEGMENT_LIMIT));
    let mut previous_was_separator = false;

    for character in value.chars().take(FILENAME_SEGMENT_LIMIT) {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
            segment.push(character);
            previous_was_separator = false;
        } else if !previous_was_separator && !segment.is_empty() {
            segment.push('-');
            previous_was_separator = true;
        }
    }

    let segment = segment.trim_matches('-');
    if segment.is_empty() {
        "unknown".to_owned()
    } else {
        segment.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use akasha_application::news::{NewsSeries, NewsSummary};
    use chrono::{FixedOffset, TimeZone};

    use super::{
        EpisodeNfoContext, build_episode, build_movie, build_series, render_plot,
        safe_filename_segment,
    };

    /// 构造用于 NFO 序列化测试的视频新闻
    fn video_news() -> NewsSummary {
        let timezone = FixedOffset::east_opt(8 * 60 * 60).expect("应创建 UTC+8 时区");

        NewsSummary {
            id: "video-1".to_owned(),
            source_id: "web_cn".to_owned(),
            title: "角色 <PV> & 幕后\u{1}".to_owned(),
            publish_time: timezone
                .with_ymd_and_hms(2026, 8, 10, 12, 0, 0)
                .single()
                .expect("应创建测试时间"),
            source_url: "https://example.com/video-1".to_owned(),
            cover: Some("/assets/video-1.jpg".to_owned()),
            news_type: "video".to_owned(),
            tags: vec!["角色 & PV".to_owned()],
            characters: Vec::new(),
            video_url: Some("https://video.example.com/video-1.mp4".to_owned()),
            video_duration_ms: Some(154_633),
            intro: Some("<p>第一段 <strong>简介</strong></p><p>第二段</p>".to_owned()),
        }
    }

    /// 构造用于 TV Show 和 Episode NFO 测试的标签剧集
    fn news_series() -> NewsSeries {
        let timezone = FixedOffset::east_opt(8 * 60 * 60).expect("应创建 UTC+8 时区");

        NewsSeries {
            tag_name: "动画短片".to_owned(),
            game_name: "崩坏：星穹铁道".to_owned(),
            cover: Some("/assets/series.jpg".to_owned()),
            premiered: timezone
                .with_ymd_and_hms(2026, 8, 1, 12, 0, 0)
                .single()
                .expect("应创建测试时间"),
            episode_count: 2,
        }
    }

    /// NFO 包含媒体库需要的核心字段并安全转义来源文本
    #[test]
    fn serializes_movie_nfo_metadata() {
        let document = build_movie(
            "ys",
            "web_cn",
            video_news(),
            None,
            "https://assets.example.com",
        )
        .expect("应生成 NFO");

        assert!(document.xml.starts_with("<?xml version=\"1.0\""));
        assert!(document.xml.contains("<movie>"));
        assert!(
            document
                .xml
                .contains("<title>角色 &lt;PV&gt; &amp; 幕后</title>")
        );
        assert!(
            document.xml.contains("<plot>第一段 简介\n\n第二段</plot>"),
            "实际 NFO：{}",
            document.xml
        );
        assert!(document.xml.contains("<premiered>2026-08-10</premiered>"));
        assert!(
            document.xml.contains(
                "<uniqueid type=\"akasha\" default=\"true\">ys:web_cn:video-1</uniqueid>"
            )
        );
        assert!(
            document
                .xml
                .contains("<thumb>https://assets.example.com/assets/video-1.jpg</thumb>")
        );
        assert!(document.xml.contains("<tag>角色 &amp; PV</tag>"));
        assert!(
            document
                .xml
                .contains("<durationinseconds>155</durationinseconds>")
        );
        assert_eq!(document.filename, "akasha-ys-web_cn-video-1.nfo");
    }

    /// 缺少条目封面时使用游戏封面的公开绝对地址
    #[test]
    fn uses_game_cover_as_fallback() {
        let mut news = video_news();
        news.cover = None;

        let document = build_movie(
            "ys",
            "web_cn",
            news,
            Some("/assets/game.jpg".to_owned()),
            "https://assets.example.com",
        )
        .expect("应生成 NFO");

        assert!(
            document
                .xml
                .contains("<thumb>https://assets.example.com/assets/game.jpg</thumb>")
        );
    }

    /// 外部标识不能向响应头注入控制字符或路径分隔符
    #[test]
    fn sanitizes_download_filename_segments() {
        assert_eq!(safe_filename_segment("../a\r\n中文:b"), "a-b");
        assert_eq!(safe_filename_segment("中文"), "unknown");
    }

    /// 标签剧集 NFO 使用 tvshow 根节点和组合后的剧集标题
    #[test]
    fn serializes_tv_show_nfo_metadata() {
        let document = build_series("sr", "web_cn", news_series(), "https://assets.example.com")
            .expect("应生成 TV Show NFO");

        assert!(document.xml.contains("<tvshow>"));
        assert!(
            document
                .xml
                .contains("<title>《崩坏：星穹铁道》动画短片</title>")
        );
        assert!(document.xml.contains("<season>1</season>"));
        assert!(document.xml.contains("<episode>2</episode>"));
        assert!(document.xml.contains("<premiered>2026-08-01</premiered>"));
        assert_eq!(document.filename, "tvshow.nfo");
    }

    /// 标签内视频 NFO 使用 episodedetails 根节点和前端传入的季集编号
    #[test]
    fn serializes_episode_nfo_metadata() {
        let context = EpisodeNfoContext::new("sr", "web_cn", news_series(), 1, 2);
        let document = build_episode(context, video_news(), "https://assets.example.com")
            .expect("应生成 Episode NFO");

        assert!(document.xml.contains("<episodedetails>"));
        assert!(
            document
                .xml
                .contains("<showtitle>《崩坏：星穹铁道》动画短片</showtitle>")
        );
        assert!(document.xml.contains("<season>1</season>"));
        assert!(document.xml.contains("<episode>2</episode>"));
        assert!(document.xml.contains("<aired>2026-08-10</aired>"));
        assert_eq!(document.filename, "akasha-sr-web_cn-video-1-s01e02.nfo");
    }

    /// 无标签纯文本简介保留已有换行和空行
    #[test]
    fn preserves_plain_text_plot_line_breaks() {
        let plot = render_plot("第一行\n第二行\n\n第三行")
            .expect("应转换简介")
            .expect("简介不应为空");

        assert_eq!(plot, "第一行\n第二行\n\n第三行");
    }

    /// HTML 文本节点换行和段落边界都会保留
    #[test]
    fn preserves_html_plot_line_breaks() {
        let plot = render_plot("<p>第一行\n第二行</p><p>第三行</p>")
            .expect("应转换简介")
            .expect("简介不应为空");

        assert_eq!(plot, "第一行\n第二行\n\n第三行");
    }

    /// 超长简介不会按固定显示宽度插入人工换行
    #[test]
    fn does_not_wrap_long_plot_lines() {
        let source = "一段包含空格的长简介 ".repeat(20);
        let plot = render_plot(&source)
            .expect("应转换简介")
            .expect("简介不应为空");

        assert!(!plot.contains('\n'));
        assert_eq!(plot, source.trim());
    }
}
