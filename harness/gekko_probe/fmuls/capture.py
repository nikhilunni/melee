"""Dolphin --script host; all artifacts remain under /tmp."""
import os,sys,json,traceback
sys.dont_write_bytecode=True
sys.path.insert(0, os.path.dirname(os.path.abspath(globals().get("__file__") or sys._getframe().f_code.co_filename)))
from cases import cases
from dolphin import event,memory
OUT=os.environ.get('FMULS_PROBE_OUT','/tmp/melee-fmuls-probe/results')
os.makedirs(OUT,exist_ok=True)
rows=cases();state={'frame':0,'started':False}
def status(s):
 with open(OUT+'/status.txt','a') as f:f.write(s+'\n')
def frame():
 try:
  state['frame']+=1
  if not state['started']:
   if memory.read_u32(0x80400014)!=0xA11FE025:
    if state['frame']>300:raise RuntimeError('guest boot timeout')
    return
   assert len(rows)*24<0x40000
   memory.write_u64(0x80400018,0);memory.write_u64(0x80400020,0x3fe0000000000000)
   for i,r in enumerate(rows):
    for j,k in enumerate(['a','c','sqsum']):memory.write_u64(0x80410000+i*24+j*8,int(r[k],16))
    for j in range(6):memory.write_u32(0x80450000+i*24+j*4,0xdeaddead)
   memory.write_u32(0x80400008,len(rows));memory.write_u32(0x80400004,0);memory.write_u32(0x80400000,0xC0DE0025)
   state['started']=True;status('started '+str(len(rows)));return
  if memory.read_u32(0x80400004)==0xC0DEF025:
   with open(OUT+'/fmuls.jsonl','w') as f:
    for i,r in enumerate(rows):
     p=0x80450000+i*24
     r.update(result=f'{memory.read_u32(p):08x}',swapped=f'{memory.read_u32(p+4):08x}',estimate=f'{memory.read_u64(p+8):016x}',estimate_square=f'{memory.read_u32(p+16):08x}',estimate_half=f'{memory.read_u32(p+20):08x}')
     f.write(json.dumps(r)+'\n')
   status('DONE');sys.stdout.flush();os._exit(0)
  if state['frame']>900:raise RuntimeError('execution timeout')
 except Exception:
  status(traceback.format_exc());os._exit(1)
event.on_frameadvance(frame)
status('loaded')
