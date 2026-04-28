import json
from pathlib import Path
manifest=json.loads(Path('evals/dirac-comparison/reference/manifest.json').read_text())
map_task={e['task']:e for e in manifest if e['agent']=='dirac'}
for task in ['DynamicCache','IOverlayWidget','addLogging','datadict','extensionswb_service','latency','sendRequest','stoppingcriteria']:
    p=Path(f'evals/dirac-comparison/tasks/{task}.json')
    if not p.exists():
        continue
    data=json.loads(p.read_text())
    e=map_task.get(task)
    if e:
        data['dirac_reference']={
            'patch': e['patch_path'],
            'metadata': f"evals/dirac-comparison/reference/metadata/dirac/{e['name']}.json",
            'source_url': e['source_url'],
        }
    p.write_text(json.dumps(data,indent=2)+"\n")
