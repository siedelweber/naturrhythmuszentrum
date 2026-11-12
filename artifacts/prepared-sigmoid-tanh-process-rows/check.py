# SPDX-License-Identifier: AGPL-3.0-only
"""Reproduce and check existing eight-field engineering rows from retained processes."""
import argparse,hashlib,json,tomllib,decimal,collections,copy,re,shlex
from pathlib import Path
D=decimal.Decimal
PREFIX='sigmoid-tanh-recorded-'
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def accounting(root,commit,date):
 log=root/'accounting/0.txt';text=log.read_text();result=[]
 receipt=json.loads((root/'accounting/commands.json').read_text());assert all(x['exit']==0 for x in receipt)
 assert '2 passed; 0 failed' in text
 command=shlex.join(receipt[0]['argv'])
 for family in ('SIGMOID','TANH'):
  line=lambda suffix:next(l for l in text.splitlines() if l.startswith(family+'_'+suffix+' '))
  def usage(s):return dict((k,int(v)) for k,v in re.findall(r'(\w+): (\d+)',s))
  cold=usage(line('PREPARATION').split('after=')[1].split('counts=')[0]);warm=usage(line('WARM').split('before_batch=')[1].split('after_batch=')[0]);after=usage(line('WARM').split('after_batch=')[1].split('counts=')[0]);replay=usage(line('REPLAY').split('counts=')[0]);inventory=dict((k,int(v)) for k,v in re.findall(r'(\w+)=(\d+)',line('INVENTORY')))
  quantities={'cold-work':(cold['work'],'Work'),'cold-arithmetic-steps':(cold['arithmetic_steps'],'steps'),'warm-work':(warm['work'],'Work'),'warm-arithmetic-steps':(warm['arithmetic_steps'],'steps'),'replay-work':(replay['work'],'Work'),'replay-arithmetic-steps':(replay['arithmetic_steps'],'steps'),'owner-bytes':(int(re.search(r'owned=(\d+)',line('PREPARATION'))[1]),'bytes'),'graph-bytes':(inventory['graph'],'bytes'),'judgment-bytes':(inventory['judgments'],'bytes'),'input-bytes':(inventory['input'],'bytes'),'result-bytes':(inventory['output'],'bytes'),'three-input-batch-admission-work':(after['work']-warm['work'],'Work')}
  for name,(value,unit) in quantities.items():
   result.append(dict(id=PREFIX+'accounting-'+family.lower()+'-'+name,metric=family.lower()+' '+name,value=str(value),unit=unit,date=date,commit=commit,command=command,source=f'Pinned build test-profile accounting; original broad domain and 40-bit target; test owner_{family.lower()}_replay_and_accounting; resource-boundary tests also pass; Work is a witnessed build count, not a physical-cost bound; native evaluation leaves all ledger counters unchanged; batch Work is admission, not exact evaluation; full before/after ledgers and counts in artifacts/prepared-sigmoid-tanh-process-rows/accounting/0.txt SHA-256 {sha(log)}'))
 return result
def records(root):
 groups=json.loads((root/'groups.json').read_text());assert len(groups)==4
 assert {(g['count'],g['mode']) for g in groups}=={(n,m) for n in (256,4096) for m in ('disabled','enabled')}
 result=[];contexts={};same_pair=len({g['pair'] for g in groups})==1
 for group in groups:
  folder=root/group['name'];ps=json.loads((folder/'processes.json').read_text());assert [p['ordinal'] for p in ps]==list(range(6))
  assert len({p['pid'] for p in ps})==6
  estimates=collections.defaultdict(list);readings=[]
  for p in ps:
   assert p['cpu']==group['cpu'] and p['count']==group['count'] and p['mode']==group['mode']
   assert p['warm_up_pid']==(0 if p['ordinal']==0 else ps[0]['pid'])
   raw=folder/p['stdout'];rows=[json.loads(line) for line in raw.read_text().splitlines()]
   if p['exit']==0:assert len(rows)==(1 if p['ordinal']==0 else 20)
   for row in rows:
    assert set(row)=={'id','metric','value','unit','date','commit','command','source'}
    assert row['command']==p['command'] and row['commit']==p['commit']
    assert f"pid={p['pid']};" in row['source'] and f"ordinal={p['ordinal']};" in row['source'] and f"cpu={p['cpu']};" in row['source']
    if p['ordinal']==0:
     assert row['id'] in ('warm-up','collection-failure');continue
    assert row['id']!='warm-up'
    readings.append((p,row,raw))
    if row['unit']=='s':estimates[row['id']].append((p['ordinal'],row['value']))
  for p,rawrow,raw in readings:
   row=dict(rawrow);row['id']=PREFIX+group['name']+'-p'+str(p['ordinal'])+'-'+rawrow['id']
   stat=''
   if rawrow['unit']=='s':
    vals=estimates[rawrow['id']];numbers=[D(v) for _,v in vals]
    stat=f'; process estimates={json.dumps(vals,separators=(",",":"))}; minimum={min(numbers)}; maximum={max(numbers)}; spread={max(numbers)-min(numbers)}; available_estimates={len(vals)} of 5; all measured processes retained'
   before=group['before'];after=group['after']
   scope='all four groups share one core pair' if same_pair else 'groups use different core pairs: no direct cross-group latency comparison with an assumed matching environment'
   evidence=rawrow['source'].split('; boundary=')[0]
   # Detailed snapshots and result bits remain byte-for-byte in the hashed raw output.
   pair=before['pairs'][group['pair']]
   post=dict(pair_idle_percent={str(n):group['group_interval_idle_percent']['cpu'+str(n)] for n in pair},frequency_before={str(n):before['freq']['cpu'+str(n)] for n in pair},frequency_after={str(n):after['freq']['cpu'+str(n)] for n in pair},pressure_before=before['pressure'],pressure_after=after['pressure'])
   row['source']=evidence+f'; artifact=artifacts/prepared-sigmoid-tanh-process-rows/{group["name"]}/{raw.name}'+f' SHA-256 {sha(raw)}; runner=groups.json SHA-256 {sha(root/"groups.json")}; process manifest={group["name"]}/processes.json SHA-256 {sha(folder/"processes.json")}; boundary={rawrow["metric"]}; raw artifact retains complete resource snapshots and result bits; group load before={before["load"][0]} after={after["load"][0]}; state={json.dumps(post,separators=(",",":"))}; admission decisions=decisions.jsonl SHA-256 {sha(root/"decisions.jsonl")}; {scope}; post-group counters include this group own work and do not identify interference'+stat
   row['source']+='; admission bound: preceding 30s both siblings idle >=99% excluding iowait; CPU PSI some avg10 <=5%; memory some avg10 <=0.1% and full avg10 zero; policy present and unchanged; account otherwise idle; load recorded only'
   # Source is a Markdown table cell; preserve literal punctuation through rendering.
   row['source']=row['source'].replace('[',r'\[').replace(']',r'\]').replace('<',r'\<')
   assert not any(c in row['source'] for c in '\n\r|')
   result.append(row)
 assert len({r['id'] for r in result})==len(result)
 result.extend(accounting(root,result[0]['commit'],result[0]['date']))
 return result
def old_check(root,input_file):
 expected=records(root);actual=[r for r in tomllib.loads(input_file.read_text())['measurements'] if r['id'].startswith(PREFIX)]
 assert len(actual)==len(expected) and len({r['id'] for r in actual})==len(actual),'missing or duplicate recorded row'
 assert {r['id']:r for r in actual}=={r['id']:r for r in expected},'recorded row differs from retained process'
 return len(expected)

def semantic_source(text):
 """Remove only publication references and the neutralized private binding."""
 text=re.sub(r'admission and state in [^;]+','admission and state in EVIDENCE',text)
 for field in ('artifact','runner','process manifest','admission decisions'):
  text=re.sub(re.escape(field)+r'=[^;]+',field+'=REFERENCE',text)
 text=re.sub(r'full before/after ledgers and counts in [^;]+',
             'full before/after ledgers and counts in REFERENCE',text)
 return text

def validate_set(root):
 manifest=json.loads((root/'manifest.json').read_text())
 files={str(p.relative_to(root)) for p in root.rglob('*') if p.is_file()}
 assert files==set(manifest['files'])|{'manifest.json'},'file inventory differs'
 for name,row in manifest['files'].items():
  assert sha(root/name)==row['sha256'],f'digest mismatch: {name}'
  assert (root/name).stat().st_size==row['bytes'],f'length mismatch: {name}'
 decisions=[json.loads(l) for l in (root/'decisions.jsonl').read_text().splitlines()]
 assert len(decisions)==8 and decisions[0]['kind']=='baseline'
 assert sum(d.get('chosen_pair') is not None for d in decisions)==4
 assert sum(d.get('kind')=='decision' and d.get('chosen_pair') is None for d in decisions)==3
 previous=decisions[0]['state']
 def pressure(state,field,kind):
  line=next(l for l in state['pressure'][field].splitlines() if l.startswith(kind+' '))
  return float(line.split('avg10=')[1].split()[0])
 groups=json.loads((root/'groups.json').read_text())
 for i,d in enumerate(decisions[1:],1):
  assert json.loads((root/f'admissions/{i}.json').read_text())==d
  current=d['state']
  # Intervening group completion starts the next thirty-second interval.
  if d['previous_at']!=previous['at']:
   previous=next(g['after'] for g in groups if g['after']['at']==d['previous_at'])
  for cpu,values in current['cpus'].items():
   delta=[a-b for a,b in zip(values,previous['cpus'][cpu])]
   assert 100*delta[3]/sum(delta)==d['idle_percent'][cpu]
  pressure_ok=pressure(current,'cpu','some')<=5 and pressure(current,'memory','some')<=.1 and pressure(current,'memory','full')==0
  assert pressure_ok==d['pressure_ok']
  assert d['account_ok']==(not current['other_account_work'])
  for pair,cs in current['pairs'].items():
   stable=current['boost'] is not None and current['boost']==previous['boost']
   for c in cs:
    for k,v in current['freq']['cpu'+str(c)].items():
     if k!='scaling_cur_freq':stable=stable and v is not None and v==previous['freq']['cpu'+str(c)][k]
   assert bool(stable)==d['pairs'][pair]['frequency_stable']
   assert d['pairs'][pair]['admit']==bool(stable and pressure_ok and d['account_ok'] and min(d['idle_percent']['cpu'+str(c)] for c in cs)>=99)
  if d['chosen_pair']:assert d['pairs'][d['chosen_pair']]['admit']
  previous=current
 for g in groups:
  d=next(d for d in decisions if d.get('group')==g['name'] and d.get('chosen_pair'))
  assert d['state']==g['before'] and d['chosen_pair']==g['pair']
 return manifest

def published_records(root,evidence_commit=None):
 rows=records(root)
 if evidence_commit is None:return rows
 assert re.fullmatch(r'[0-9a-f]{40}',evidence_commit),'full evidence commit required'
 address='https://github.com/siedelweber/naturrhythmuszentrum/blob/'+evidence_commit+'/'
 directory='artifacts/prepared-sigmoid-tanh-process-rows/'
 groups=json.loads((root/'groups.json').read_text())
 for row in rows:
  if '-accounting-' in row['id']:
   paths=['accounting/0.txt','accounting/commands.json']
  else:
   group=next(g for g in groups if row['id'].startswith(PREFIX+g['name']+'-p'))
   ordinal=int(row['id'].split(group['name']+'-p',1)[1].split('-',1)[0])
   state=json.loads((root/group['name']/'state.json').read_text())
   paths=[group['name']+'/'+str(ordinal)+'.jsonl',group['name']+'/processes.json',state['admission']]
  row['source']+='; public evidence only, not offered for merge: '+', '.join(address+directory+p+' SHA-256 '+sha(root/p) for p in paths)
 return rows

def check(root,input_file,evidence_commit=None):
 validate_set(root)
 expected={r['id']:r for r in published_records(root,evidence_commit)}
 rows=[r for r in tomllib.loads(input_file.read_text())['measurements'] if r['id'].startswith(PREFIX)]
 assert len(rows)==len(expected)==424 and len({r['id'] for r in rows})==424
 refusals=collections.Counter()
 for actual in rows:
  row=expected[actual['id']]
  for field in ('id','metric','value','unit','date','commit'):
   assert actual[field]==row[field],f'{actual["id"]}: {field} differs'
  if actual['command']!=row['command']:
   assert '-accounting-' in actual['id'],'phase command differs'
   # Both old and published commands must select exactly the same test filter.
   assert all(s in actual['command'] for s in ('cargo','test','nrz-matrix','prepared_expm1','replay_and_accounting','--nocapture'))
   refusals['accounting-command']+=1
  if actual['source']!=row['source']:
   assert semantic_source(actual['source'])==semantic_source(row['source']),f'{actual["id"]}: non-citation source differs'
   refusals['accounting-citation' if '-accounting-' in actual['id'] else 'phase-citations']+=1
 print('EVIDENCE-CONSISTENT processes=24 measured=20 warm-ups=4 decisions=7 refused=3 rows=424')
 for reason,count in sorted(refusals.items()):print(f'REFUSAL {reason} rows={count}')
 print('OTHER-REFUSALS 0')
 return 1 if refusals else 0

if __name__=='__main__':
 parser=argparse.ArgumentParser(description='Join retained process evidence to engineering rows.')
 parser.add_argument('input',type=Path)
 parser.add_argument('--evidence-commit',help='public evidence commit used by the regenerated row citations')
 args=parser.parse_args()
 raise SystemExit(check(Path(__file__).resolve().parent,args.input,args.evidence_commit))
