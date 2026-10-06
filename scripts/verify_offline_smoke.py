#!/usr/bin/env python3
"""Exercise the real Rust CLI in a fresh synthetic home, with zero external calls."""
import argparse
import json
import subprocess
import tempfile
from pathlib import Path

parser = argparse.ArgumentParser()
parser.add_argument('--core', required=True, type=Path, help='Built director-next-core binary')
args = parser.parse_args()
core = args.core.resolve(strict=True)
with tempfile.TemporaryDirectory(prefix='director-next-smoke-') as temporary:
    home = Path(temporary) / 'next-home'
    def call(action, **kwargs):
        result = subprocess.run([str(core), str(home)], input=json.dumps({'action': action, **kwargs}),
            text=True, capture_output=True, timeout=30)
        assert result.returncode == 0, result.stderr
        return json.loads(result.stdout)
    policy = call('getExecutionPolicy')
    assert policy['mode'] == 'offline_prepare_only' and policy['liveExecutionAllowed'] is False
    project = call('projects.create', title='Synthetic smoke project')
    source = '第1集\n场景：门口\n甲（OS）：等一下🙂。\n第2集\n场景：院子\n乙：我回来了。\n'
    workspace = call('source.import', projectId=project['id'], text=source)
    assert workspace['source']['text'] == source
    assert len(workspace['source']['episodes']) == 2
    restarted = call('workspace.get', projectId=project['id'])
    assert restarted['source'] == workspace['source']
    for action in ['foundation.run', 'storyboard.run', 'images.run', 'videos.submit', 'textCli.status', 'providers.refresh']:
        blocked = subprocess.run([str(core), str(home)], input=json.dumps({'action': action, 'projectId': project['id']}),
            text=True, capture_output=True, timeout=30)
        assert blocked.returncode != 0 and 'OFFLINE_EXECUTION_BLOCKED:' in blocked.stderr, action
    assert call('workspace.get', projectId=project['id'])['jobs'] == []
    print(json.dumps({'status':'passed', 'separateProcessCalls':11, 'sourceEpisodes':2,
        'sourcePreserved':True, 'restartPreserved':True, 'externalActionsBlocked':6, 'generationJobs':0}))
