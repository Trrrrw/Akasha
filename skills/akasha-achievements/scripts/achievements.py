#!/usr/bin/env python3
"""Local achievement progress and read-only public catalog CLI (stdlib only)."""
import argparse
import datetime as dt
import json
import os
from pathlib import Path
import shutil
import sys
import tempfile
import time
import urllib.error
import urllib.parse
import urllib.request
import uuid

GAMES = ('ys', 'sr', 'zzz')
UNKNOWN = 253402271999
SAFE = 9007199254740991
TZ = dt.timezone(dt.timedelta(hours=8))

def integer(value, maximum=SAFE):
    return type(value) is int and 0 <= value <= maximum

def day(value):
    if not isinstance(value, str):
        raise ValueError('日期必须为字符串')
    if value:
        if len(value) != 10 or dt.date.fromisoformat(value).isoformat() != value:
            raise ValueError('日期必须为 YYYY-MM-DD')
    return value

def dumps(value):
    return json.dumps(value, ensure_ascii=False, separators=(',', ':'), allow_nan=False)

def read(path):
    with Path(path).open(encoding='utf-8-sig') as stream:
        def pairs(entries):
            result = {}
            for key, value in entries:
                if key in result:
                    raise ValueError('JSON 包含重复字段')
                result[key] = value
            return result
        return json.load(stream, object_pairs_hook=pairs, parse_constant=lambda _: (_ for _ in ()).throw(ValueError('非法 JSON 数值')))

def validate(value):
    if not isinstance(value, dict) or type(value.get('format')) is not int or value['format'] != 1 or not isinstance(value.get('records'), dict):
        raise ValueError('不支持的本站备份格式')
    if len(value['records']) > 100000:
        raise ValueError('进度文件过大')
    records = {}
    for key, p in value['records'].items():
        game, sep, ident = key.partition(':')
        if game not in GAMES or not sep or not ident or len(ident) > 128 or not isinstance(p, dict):
            raise ValueError('进度键无效')
        if type(p.get('completed')) is not bool or not integer(p.get('updatedAt')) or not isinstance(p.get('device'), str) or not 1 <= len(p['device']) <= 128:
            raise ValueError('进度字段无效')
        result = {'completed': p['completed'], 'date': day(p.get('date'))}
        if 'note' in p:
            if not isinstance(p['note'], str) or len(p['note']) > 10000:
                raise ValueError('旧备注字段无效')
            result['note'] = p['note']
        result.update(updatedAt=p['updatedAt'], device=p['device'])
        if 'uiaf' in p:
            u = p['uiaf']
            if not isinstance(u, dict) or not integer(u.get('current')) or not integer(u.get('status'), 3) or not integer(u.get('timestamp'), UNKNOWN) or p['completed'] != (u['status'] >= 2):
                raise ValueError('UIAF 元信息无效')
            result['uiaf'] = {k: u[k] for k in ('current', 'status', 'timestamp')}
        records[key] = result
    return {'format': 1, 'records': records}

def merge(*documents):
    result = {'format': 1, 'records': {}}
    for document in documents:
        for key, p in document['records'].items():
            old = result['records'].get(key)
            rank = lambda v: (v['updatedAt'], dumps(v).encode('utf-16-be', 'surrogatepass'))
            if old is None or rank(p) > rank(old):
                result['records'][key] = p.copy()
    return result

def write(path, value, replace=False):
    path = Path(path)
    if path.exists() and not replace:
        raise ValueError(f'目标已存在，禁止覆盖：{path}')
    path.parent.mkdir(parents=True, exist_ok=True)
    text = json.dumps(value, ensure_ascii=False, indent=2, allow_nan=False) + '\n'
    if replace and path.exists():
        backup = path.with_name(path.name + '.bak-' + dt.datetime.now().strftime('%Y%m%d%H%M%S') + '-' + uuid.uuid4().hex[:8])
        shutil.copy2(path, backup)
        print(f'备份：{backup}', file=sys.stderr)
    fd, name = tempfile.mkstemp(prefix='.' + path.name + '-', dir=path.parent)
    try:
        with os.fdopen(fd, 'w', encoding='utf-8', newline='\n') as stream:
            stream.write(text)
            stream.flush()
            os.fsync(stream.fileno())
        if replace:
            os.replace(name, path)
        else:
            # 排他创建防止检查后出现的新文件被覆盖
            with path.open('x', encoding='utf-8', newline='\n') as target:
                target.write(text)
    finally:
        if os.path.exists(name):
            os.unlink(name)

def timestamp(date):
    value = int(dt.datetime.combine(dt.date.fromisoformat(date), dt.time(), TZ).timestamp()) if date else UNKNOWN
    if not integer(value, UNKNOWN):
        raise ValueError('完成日期无法导出为 UIAF 时间戳')
    return value

def date_from_timestamp(value):
    return dt.datetime.fromtimestamp(value, TZ).date().isoformat() if value != UNKNOWN else ''

def import_data(value, current):
    if isinstance(value, dict) and value.get('format') == 1:
        return merge(current, validate(value))
    info = value.get('info') if isinstance(value, dict) else None
    if not isinstance(info, dict) or info.get('uiaf_version') not in ('v1.0', 'v1.1') or not isinstance(value.get('list'), list) or len(value['list']) > 100000:
        raise ValueError('不支持的 UIAF 文件')
    incoming = {'format': 1, 'records': {}}
    now = int(time.time() * 1000)
    for entry in value['list']:
        if not isinstance(entry, dict) or not integer(entry.get('id')) or entry['id'] == 0 or not integer(entry.get('current')) or not integer(entry.get('timestamp'), UNKNOWN):
            raise ValueError('UIAF 记录字段无效')
        status = entry.get('status') if info['uiaf_version'] == 'v1.1' else 2
        if not integer(status, 3):
            raise ValueError('UIAF 状态无效')
        key = 'ys:' + str(entry['id'])
        if key in incoming['records']:
            raise ValueError('UIAF 包含重复 ID')
        old = current['records'].get(key, {})
        record = {'completed': status >= 2, 'date': date_from_timestamp(entry['timestamp']) if status >= 2 else ''}
        if 'note' in old:
            record['note'] = old['note']
        record.update(updatedAt=max(now, old.get('updatedAt', 0) + 1), device='agent-uiaf-import')
        record['uiaf'] = {'current': entry['current'], 'status': status, 'timestamp': entry['timestamp']}
        incoming['records'][key] = record
    return merge(current, validate(incoming))

def load_catalog(path):
    catalog = read(path)
    if not isinstance(catalog, dict) or catalog.get('game') not in GAMES or not isinstance(catalog.get('items'), list):
        raise ValueError('目录格式无效')
    seen = set()
    for item in catalog['items']:
        if not isinstance(item, dict) or not isinstance(item.get('id'), str) or not item['id'] or item['id'] in seen or not isinstance(item.get('name'), str):
            raise ValueError('目录 ID 或名称无效')
        seen.add(item['id'])
    return catalog

def fetch_catalog(game, base):
    url = urllib.parse.urlsplit(base)
    if url.username or url.password or url.query or url.fragment or url.path not in ('', '/') or not (url.scheme == 'https' or url.scheme == 'http' and url.hostname in ('localhost', '127.0.0.1', '::1')) or not url.hostname:
        raise ValueError('API 地址必须为根 HTTPS 地址或本机 HTTP，不允许凭据、路径或查询参数')
    base = base.rstrip('/')
    def get(path):
        class NoRedirect(urllib.request.HTTPRedirectHandler):
            def redirect_request(self, *args, **kwargs):
                raise ValueError('API 重定向被拒绝，请使用实际根地址')
        request = urllib.request.Request(base + '/api/v1' + path, headers={'Accept': 'application/json', 'User-Agent': 'akasha-achievements-skill/1'})
        with urllib.request.build_opener(NoRedirect).open(request, timeout=30) as response:
            body = response.read(20_000_001)
            if len(body) > 20_000_000:
                raise ValueError('API 响应过大')
            return json.loads(body.decode('utf-8'))
    groups = get(f'/games/{game}/achievement-groups')['items']
    items, total = [], None
    while True:
        page = get(f'/games/{game}/achievements?limit=100&offset={len(items)}')
        if not integer(page.get('total'), 100000) or total is not None and total != page['total'] or not isinstance(page.get('items'), list):
            raise ValueError('目录分页无效或正在更新')
        total = page['total']
        if not page['items'] and len(items) < total:
            raise ValueError('目录分页不完整')
        items.extend(page['items'])
        if len(items) >= total:
            break
    if len(items) != total or len({item['id'] for item in items}) != total or sum(g['total'] for g in groups) != total:
        raise ValueError('目录总数不一致，请重新加载')
    return {'game': game, 'items': items, 'groups': groups, 'fetchedAt': int(time.time() * 1000)}

def export_uiaf(document, catalog=None):
    targets = {a['id']: a.get('target') for a in catalog['items']} if catalog else {}
    result = []
    for key, p in document['records'].items():
        if not key.startswith('ys:'):
            continue
        ident = key[3:]
        if not ident.isascii() or not ident.isdecimal() or not integer(int(ident)) or int(ident) == 0:
            raise ValueError('原神 ID 无法导出为 UIAF 整数')
        u = p.get('uiaf', {})
        target = targets.get(ident)
        result.append({'id': int(ident), 'current': u.get('current', target if p['completed'] and integer(target) else 0), 'timestamp': u.get('timestamp', timestamp(p['date']) if p['completed'] else UNKNOWN), 'status': u.get('status', 2 if p['completed'] else 1)})
    return {'info': {'export_app': 'akasha-achievements-skill', 'export_app_version': '1.0', 'export_timestamp': int(time.time()), 'uiaf_version': 'v1.1'}, 'list': sorted(result, key=lambda p: p['id'])}

def parser():
    p = argparse.ArgumentParser(description=__doc__)
    sub = p.add_subparsers(dest='command', required=True)
    for name in ('init', 'validate', 'import', 'export', 'set', 'status'):
        command = sub.add_parser(name)
        command.add_argument('--file', required=True)
        if name == 'import':
            command.add_argument('--input', required=True)
        if name == 'export':
            command.add_argument('--output', required=True)
            command.add_argument('--format', choices=('backup', 'uiaf'), default='backup')
            command.add_argument('--catalog')
        if name in ('set', 'status'):
            command.add_argument('--catalog', required=True)
        if name == 'set':
            command.add_argument('--id', action='append', required=True)
            state = command.add_mutually_exclusive_group(required=True)
            state.add_argument('--completed', action='store_true')
            state.add_argument('--incomplete', action='store_true')
            command.add_argument('--date')
    cat = sub.add_parser('catalog')
    cat.add_argument('--game', choices=GAMES, required=True)
    cat.add_argument('--output', required=True)
    cat.add_argument('--base-url', default='https://akasha.trrw.cn')
    find = sub.add_parser('find')
    find.add_argument('--catalog', required=True)
    find.add_argument('--query', required=True)
    return p

def main(argv=None):
    args = parser().parse_args(argv)
    if args.command == 'init':
        write(args.file, {'format': 1, 'records': {}})
        return {'file': str(Path(args.file).resolve()), 'records': 0}
    if args.command == 'catalog':
        catalog = fetch_catalog(args.game, args.base_url)
        write(args.output, catalog)
        return {'game': args.game, 'items': len(catalog['items']), 'file': str(Path(args.output).resolve())}
    if args.command == 'find':
        catalog = load_catalog(args.catalog)
        q = args.query.strip().casefold()
        if not q:
            raise ValueError('查询不能为空')
        return {'game': catalog['game'], 'items': [a for a in catalog['items'] if q in str(a.get('name', '')).casefold() or q in str(a.get('description', '')).casefold() or q == a['id']]}
    document = validate(read(args.file))
    if args.command == 'import':
        incoming = read(args.input)
        merged = import_data(incoming, document)
        changed = sum(document['records'].get(key) != value for key, value in merged['records'].items())
        write(args.file, merged, replace=True)
        return {'changed': changed, 'records': len(merged['records']), 'completed': sum(p['completed'] for p in merged['records'].values())}
    if args.command == 'export':
        catalog = load_catalog(args.catalog) if args.catalog else None
        if args.format == 'uiaf' and catalog and catalog['game'] != 'ys':
            raise ValueError('UIAF 目标值需要原神目录')
        write(args.output, export_uiaf(document, catalog) if args.format == 'uiaf' else document)
        return {'file': str(Path(args.output).resolve()), 'format': args.format}
    if args.command in ('set', 'status'):
        catalog = load_catalog(args.catalog)
        game = catalog['game']
        ids = {a['id'] for a in catalog['items']}
        if args.command == 'status':
            return {'game': game, 'catalog_total': len(ids), 'completed': sum(document['records'].get(game + ':' + ident, {}).get('completed', False) for ident in ids), 'unknown_records': [key for key in document['records'] if key.startswith(game + ':') and key[3:] not in ids]}
        if any(ident not in ids for ident in args.id):
            raise ValueError('指定 ID 不在当前目录中，未修改进度')
        if args.date is not None:
            day(args.date)
        if args.incomplete and args.date:
            raise ValueError('未完成成就不能设置完成日期')
        now = int(time.time() * 1000)
        for ident in set(args.id):
            key = game + ':' + ident
            old = document['records'].get(key, {})
            record = dict(old)
            record.update(completed=args.completed, date=(args.date if args.date is not None else old.get('date', '')) if args.completed else '', updatedAt=max(now, old.get('updatedAt', 0) + 1), device='agent-local')
            if args.completed != old.get('completed'):
                record.pop('uiaf', None)
            elif args.date is not None and 'uiaf' in record:
                record['uiaf'] = {**record['uiaf'], 'timestamp': timestamp(record['date'])}
            document['records'][key] = record
        write(args.file, validate(document), replace=True)
        return {'changed_ids': sorted(set(args.id)), 'game': game, 'completed': args.completed}
    return {'valid': True, 'records': len(document['records'])}

if __name__ == '__main__':
    for stream in (sys.stdout, sys.stderr):
        if hasattr(stream, 'reconfigure'):
            stream.reconfigure(encoding='utf-8')
    try:
        print(json.dumps(main(), ensure_ascii=False, allow_nan=False))
    except (ValueError, TypeError, KeyError, OSError, OverflowError, urllib.error.URLError) as error:
        print(f'错误：{error}', file=sys.stderr)
        sys.exit(1)
