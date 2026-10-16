"""Coverage table for the scene scripts' task keys (docs/new-game.md §2,
"Mapping the whole exe"): per task type and key, its use count in the
scenes, whether the eboot reads it ("TaskList/Task%d/<key>" in the Ghidra
export; y? = only the leaf name matches) and whether the converter names it.

python3 -I research/oretsuba/coverage.py <extract>/SCRIPT \
    research/oretsuba/local/ghidra/decomp crates/conv-engines/src/mages > cov.tsv
"""
import re,sys,glob,os,collections
scripts,decomp,conv=sys.argv[1:4]
use=collections.Counter()   # (type, key) -> count
tuse=collections.Counter()
for f in glob.glob(scripts+'/*'):
    b=os.path.basename(f)
    if not re.match(r'[A-Z]\d',b): continue
    txt=open(f,encoding='cp932',errors='replace').read()
    stack=[];pend=None;task=None
    for line in txt.splitlines():
        line=re.sub(r'//.*','',line).strip()
        if not line: continue
        m=re.match(r'\[(\w+)\]',line)
        if m: pend=m.group(1);continue
        if line.startswith('{'):
            stack.append(pend or '?')
            if re.match(r'Task\d+$',pend or '') and len(stack)==2: task={'type':None,'keys':[]}
            pend=None;continue
        if line.startswith('}'):
            if len(stack)==2 and task:
                ty=task['type'] or '?'; tuse[ty]+=1
                for k in set(task['keys']): use[(ty,k)]+=1
                task=None
            stack.pop();continue
        m=re.match(r':(\w+)\s*(.*)',line)
        if m and task is not None:
            k=m.group(1)
            if k=='Tasktype': task['type']=m.group(2).strip().strip('"'); continue
            sub='/'.join(re.sub(r'\d+$','%d',s) for s in stack[2:])
            task['keys'].append((sub+'/' if sub else '')+k)
eb=set()
for f in glob.glob(decomp+'/*.c'):
    for s in re.findall(r'"TaskList/Task%d/([^"]*)"',open(f,errors='replace').read()):
        eb.add(s)
cv=set()
for f in glob.glob(conv+'/*.rs'):
    cv|=set(re.findall(r'"([A-Z][A-Za-z0-9_]*)"',open(f).read()))
def leaf(k): return k.split('/')[-1]
print('type\tkey\tuses\teboot\tconv')
for (ty,k),n in sorted(use.items(),key=lambda x:(-tuse[x[0][0]],x[0][0],-x[1],x[0][1])):
    e='y' if k in eb else ('y?' if leaf(k) in {leaf(x) for x in eb} else '-')
    c='y' if leaf(k) in cv else '-'
    print(ty,k,n,e,c,sep='\t')
