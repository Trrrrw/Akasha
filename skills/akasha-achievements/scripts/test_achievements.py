"""Regression checks: run with uv run python -m unittest discover -s scripts."""
import contextlib
import http.server
import importlib.util
import json
from pathlib import Path
import tempfile
import threading
import unittest

spec = importlib.util.spec_from_file_location('achievements', Path(__file__).with_name('achievements.py'))
a = importlib.util.module_from_spec(spec)
spec.loader.exec_module(a)

def record(completed=True, stamp=1):
    return {'completed': completed, 'date': '', 'updatedAt': stamp, 'device': 'test'}

class ProgressTests(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.file = Path(self.tmp.name) / '中文进度.json'
        self.catalog = Path(self.tmp.name) / 'catalog.json'
        a.write(self.file, {'format': 1, 'records': {'sr:99': record()}})
        a.write(self.catalog, {'game': 'ys', 'items': [{'id': '10', 'name': '阶段', 'target': 10}, {'id': '30', 'name': '阶段', 'target': 30, 'previous_id': '10'}]})

    def test_all_uiaf_states_preserve_partial_and_unknown_time(self):
        source = {'info': {'uiaf_version': 'v1.1'}, 'list': [{'id': i + 1, 'current': i * 7, 'status': i, 'timestamp': a.UNKNOWN} for i in range(4)]}
        result = a.import_data(source, a.validate(a.read(self.file)))
        self.assertEqual([p['completed'] for k, p in result['records'].items() if k.startswith('ys:')], [False, False, True, True])
        self.assertEqual(a.export_uiaf(result)['list'], source['list'])
        self.assertIn('sr:99', result['records'])

    def test_v10_is_complete_without_claiming_reward(self):
        result = a.import_data({'info': {'uiaf_version': 'v1.0'}, 'list': [{'id': 1, 'current': 0, 'timestamp': a.UNKNOWN}]}, {'format': 1, 'records': {}})
        self.assertEqual(result['records']['ys:1']['uiaf']['status'], 2)
        self.assertEqual(result['records']['ys:1']['date'], '')

    def test_stage_edits_are_scoped_unknown_date_and_backed_up(self):
        a.main(['set', '--file', str(self.file), '--catalog', str(self.catalog), '--id', '10', '--completed'])
        result = a.read(self.file)
        self.assertEqual(result['records']['ys:10']['date'], '')
        self.assertNotIn('ys:30', result['records'])
        self.assertEqual(a.export_uiaf(result, a.read(self.catalog))['list'][0]['current'], 10)
        self.assertEqual(len(list(self.file.parent.glob('*.bak-*'))), 1)
        a.main(['set', '--file', str(self.file), '--catalog', str(self.catalog), '--id', '10', '--incomplete'])
        self.assertFalse(a.read(self.file)['records']['ys:10']['completed'])

    def test_invalid_id_date_and_duplicate_import_do_not_mutate(self):
        before = self.file.read_bytes()
        for extra in (['--id', '999', '--completed'], ['--id', '10', '--completed', '--date', '2026-02-31']):
            with self.assertRaises(ValueError):
                a.main(['set', '--file', str(self.file), '--catalog', str(self.catalog)] + extra)
        source = Path(self.tmp.name) / 'bad.json'
        a.write(source, {'info': {'uiaf_version': 'v1.1'}, 'list': [{'id': 10, 'status': 2, 'current': 1, 'timestamp': 0}] * 2})
        with self.assertRaises(ValueError):
            a.main(['import', '--file', str(self.file), '--input', str(source)])
        self.assertEqual(self.file.read_bytes(), before)
        self.assertEqual(list(self.file.parent.glob('*.bak-*')), [])

    def test_find_reports_ambiguous_stages_without_edit(self):
        before = self.file.read_bytes()
        result = a.main(['find', '--catalog', str(self.catalog), '--query', '阶段'])
        self.assertEqual(len(result['items']), 2)
        self.assertEqual(self.file.read_bytes(), before)

    def test_cancelled_records_win_and_invalid_numbers_rejected(self):
        old = {'format': 1, 'records': {'ys:10': record(True, 1)}}
        new = {'format': 1, 'records': {'ys:10': record(False, 2)}}
        self.assertFalse(a.merge(new, old)['records']['ys:10']['completed'])
        for value in (True, -1, 1.5, float('nan'), '2'):
            bad = {'format': 1, 'records': {'ys:10': {**record(), 'updatedAt': value}}}
            with self.assertRaises(ValueError): a.validate(bad)

    def test_init_export_do_not_overwrite(self):
        with self.assertRaises(ValueError): a.main(['init', '--file', str(self.file)])
        with self.assertRaises(ValueError): a.main(['export', '--file', str(self.file), '--output', str(self.file)])

    def test_public_catalog_only_get_and_complete_pagination(self):
        paths = []
        class Handler(http.server.BaseHTTPRequestHandler):
            def do_GET(self):
                paths.append(self.path)
                if self.path.endswith('achievement-groups'):
                    value = {'items': [{'id': '1', 'total': 2}]}
                else:
                    offset = 'offset=0' in self.path
                    value = {'total': 2, 'items': [{'id': '10' if offset else '30', 'name': '阶段'}]}
                body = json.dumps(value).encode()
                self.send_response(200); self.end_headers(); self.wfile.write(body)
            def log_message(self, *args): pass
        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Handler)
        thread = threading.Thread(target=server.serve_forever, daemon=True); thread.start()
        try:
            result = a.fetch_catalog('ys', f'http://127.0.0.1:{server.server_port}')
            self.assertEqual(len(result['items']), 2)
            self.assertEqual(len(paths), 3)
            self.assertTrue(all(path.startswith('/api/v1/games/ys/') for path in paths))
        finally: server.shutdown(); server.server_close(); thread.join()
        for base in ('https://example.com/api/v1/admin', 'http://example.com', 'https://name:secret@example.com'):
            with self.assertRaises(ValueError): a.fetch_catalog('ys', base)

if __name__ == '__main__': unittest.main()
