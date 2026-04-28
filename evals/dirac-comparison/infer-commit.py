#!/usr/bin/env python3
from __future__ import annotations
import argparse, json, subprocess, sys
from pathlib import Path

def run(args, cwd=None):
    return subprocess.run(args, cwd=cwd, text=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)

def main():
    ap=argparse.ArgumentParser()
    ap.add_argument('task')
    ap.add_argument('--limit', type=int, default=10000)
    args=ap.parse_args()
    root=Path('evals/dirac-comparison')
    spec=json.loads((root/'tasks'/f'{args.task}.json').read_text())
    ref=json.loads(Path(spec['dirac_reference']['metadata']).read_text())
    checkout=root/'worktrees'/args.task
    if not (checkout/'.git').exists():
        r=run(['git','clone',spec['repo'],str(checkout)])
        sys.stderr.write(r.stdout+r.stderr)
        if r.returncode: return r.returncode
    subprocess.run(['git','fetch','--all','--tags','--prune'], cwd=checkout, check=False)
    specs=[]
    for f,i in zip(ref['changed_files'], ref['indexes']):
        old=i['old_blob']
        if old.startswith('0'): continue
        specs.append((f['old_path'], old))
    candidates=run(['git','rev-list','--all','--']+[p for p,_ in specs], cwd=checkout).stdout.splitlines()[:args.limit]
    for c in candidates:
        ok=True
        for path, blob in specs:
            r=run(['git','rev-parse',f'{c}:{path}'], cwd=checkout)
            actual=r.stdout.strip()[:len(blob)] if r.returncode==0 else ''
            if actual != blob:
                ok=False; break
        if ok:
            print(c)
            show=run(['git','show','-s','--format=%H %ci %s',c], cwd=checkout)
            print(show.stdout.strip(), file=sys.stderr)
            return 0
    print('no-match', file=sys.stderr)
    return 1
if __name__=='__main__':
    raise SystemExit(main())
