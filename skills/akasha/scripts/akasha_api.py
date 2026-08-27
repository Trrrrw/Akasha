#!/usr/bin/env python3
"""Call Akasha's public read-only HTTP API without third-party dependencies."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.parse import quote, urlencode, urlsplit, urlunsplit
from urllib.request import Request, urlopen

DEFAULT_BASE_URL = "https://akasha.trrw.cn"
DEFAULT_ACCEPT = (
    "application/json, application/rss+xml, text/calendar, application/xml, "
    "text/xml, text/plain;q=0.8, */*;q=0.1"
)
ALLOWED_EXACT_PATHS = {"/healthz", "/openapi.json"}
ALLOWED_API_PREFIX = "/api/v1/"
USER_AGENT = "akasha-agent-skill/1.0"


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Call Akasha public read-only endpoints",
        epilog=(
            "example: %(prog)s /api/v1/games/ys/news "
            "--query source=web_cn --query tag=活动 --query limit=10"
        ),
    )
    parser.add_argument(
        "--base-url",
        default=os.environ.get("AKASHA_BASE_URL", DEFAULT_BASE_URL),
        help=(
            "Akasha root URL (default: AKASHA_BASE_URL or "
            f"{DEFAULT_BASE_URL})"
        ),
    )
    parser.add_argument(
        "--timeout",
        type=float,
        default=30.0,
        help="request timeout in seconds (default: 30)",
    )
    parser.add_argument(
        "--accept",
        default=DEFAULT_ACCEPT,
        help="HTTP Accept header",
    )
    parser.add_argument(
        "--query",
        action="append",
        default=[],
        metavar="KEY=VALUE",
        help="query parameter; repeat to preserve repeated keys",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="write the original response bytes to this file",
    )
    parser.add_argument(
        "path",
        help="public path such as /api/v1/games; query strings are not allowed",
    )
    return parser


def validate_base_url(value: str) -> str:
    value = value.strip().rstrip("/")
    parsed = urlsplit(value)
    if parsed.scheme not in {"http", "https"} or not parsed.netloc:
        raise ValueError("base URL must be an absolute http or https URL")
    if parsed.username or parsed.password:
        raise ValueError("base URL must not contain credentials")
    if parsed.query or parsed.fragment:
        raise ValueError("base URL must not contain a query or fragment")
    return urlunsplit((parsed.scheme, parsed.netloc, parsed.path.rstrip("/"), "", ""))


def validate_path(value: str) -> str:
    if not value.startswith("/"):
        raise ValueError("path must start with /")
    if "?" in value or "#" in value:
        raise ValueError("put query parameters in repeated --query options")
    if any(segment == ".." for segment in value.split("/")):
        raise ValueError("path must not contain .. segments")
    if value in ALLOWED_EXACT_PATHS:
        return value
    if not value.startswith(ALLOWED_API_PREFIX):
        raise ValueError("only /api/v1/**, /healthz, and /openapi.json are allowed")
    if value.startswith("/api/v1/admin/") or value == "/api/v1/admin":
        raise ValueError("admin endpoints are outside this public read-only skill")
    return value


def parse_query(values: list[str]) -> list[tuple[str, str]]:
    pairs: list[tuple[str, str]] = []
    for value in values:
        if "=" not in value:
            raise ValueError(f"query parameter must use KEY=VALUE: {value!r}")
        key, item = value.split("=", 1)
        if not key:
            raise ValueError("query parameter key must not be empty")
        pairs.append((key, item))
    return pairs


def build_url(base_url: str, path: str, query: list[tuple[str, str]]) -> str:
    encoded_path = quote(path, safe="/%:@!$&'()*+,;=-._~")
    url = f"{base_url}{encoded_path}"
    if query:
        url = f"{url}?{urlencode(query)}"
    return url


def response_charset(content_type: str) -> str:
    for part in content_type.split(";")[1:]:
        key, separator, value = part.strip().partition("=")
        if separator and key.lower() == "charset" and value.strip():
            return value.strip().strip('"')
    return "utf-8"


def render_body(body: bytes, content_type: str) -> str:
    text = body.decode(response_charset(content_type), errors="replace")
    media_type = content_type.partition(";")[0].strip().lower()
    if media_type == "application/json" or media_type.endswith("+json"):
        try:
            return json.dumps(json.loads(text), ensure_ascii=False, indent=2)
        except json.JSONDecodeError:
            pass
    return text


def error_message(error: HTTPError) -> str:
    body = error.read()
    content_type = error.headers.get("Content-Type", "")
    detail = render_body(body, content_type).strip() if body else ""
    retry_after = error.headers.get("Retry-After")
    message = f"HTTP {error.code} {error.reason}"
    if retry_after:
        message += f"; Retry-After: {retry_after}"
    if detail:
        message += f"\n{detail}"
    return message


def configure_text_streams() -> None:
    for stream in (sys.stdout, sys.stderr):
        reconfigure = getattr(stream, "reconfigure", None)
        if reconfigure is not None:
            reconfigure(encoding="utf-8", errors="replace")


def main() -> int:
    configure_text_streams()
    parser = build_parser()
    args = parser.parse_args()

    try:
        base_url = validate_base_url(args.base_url)
        path = validate_path(args.path)
        query = parse_query(args.query)
        url = build_url(base_url, path, query)
    except ValueError as error:
        parser.error(str(error))

    request = Request(
        url,
        headers={"Accept": args.accept, "User-Agent": USER_AGENT},
        method="GET",
    )

    try:
        with urlopen(request, timeout=args.timeout) as response:
            body = response.read()
            content_type = response.headers.get("Content-Type", "")
    except HTTPError as error:
        print(error_message(error), file=sys.stderr)
        return 3
    except (URLError, TimeoutError, OSError) as error:
        print(f"request failed: {error}", file=sys.stderr)
        return 4

    if args.output is not None:
        args.output.write_bytes(body)
        print(f"saved {len(body)} bytes to {args.output}", file=sys.stderr)
    elif body:
        rendered = render_body(body, content_type)
        sys.stdout.write(rendered)
        if rendered and not rendered.endswith("\n"):
            sys.stdout.write("\n")

    return 0


if __name__ == "__main__":
    raise SystemExit(main())
