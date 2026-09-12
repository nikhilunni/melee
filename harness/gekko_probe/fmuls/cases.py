"""Independent operand construction; no predicted results from an emulator."""
import os,struct
D=lambda x:struct.unpack('>Q',struct.pack('>d',x))[0]
F=lambda x:struct.unpack('>f',struct.pack('>f',x))[0]
U=lambda x:struct.unpack('>f',struct.pack('>I',x))[0]
def cases():
 rows=[]
 def add(name,a,c,q=4.0):rows.append(dict(name=name,a=f'{a:016x}',c=f'{c:016x}',sqsum=f'{D(q):016x}'))
 # Normal frC rounding boundary: both signs, retained low-bit parity, carry.
 for sign in [0,1]:
  for e in [0x3fe,0x3ff,0x400]:
   for top in [0,1,0xfffffe,0xffffff]:
    for delta in [-1,0,1]:
     c=(sign<<63)|(e<<52)|(top<<28)+(1<<27)+delta
     for a in [D(1.0),D(1.0000000596046448),D(1.234567890123)]:add(f'normal-{sign}-{e}-{top}-{delta}',a,c)
 # Scaled subnormals yield normal f32 products, exposing multiplier rounding.
 for high in [24,25,26,35,50,51]:
  discard=max(0,high+1-25)
  for delta in [-1,0,1]:
   mag=(1<<high)+(1<<discard)//2+delta
   for sign in [0,1]:add(f'subnormal-{high}-{delta}-{sign}',D(2.0**1000),(sign<<63)|mag)
 for a,c in [(0.0,1.0),(-0.0,1.0),(1.0,-0.0),(2.0**-126,0.5),(2.0**-149,0.5),(2.0**127,2.0)]:add('boundary',D(a),D(c))
 # Guest obtains frsqrte itself: no host/emulator estimate is baked in.
 qs=[F(U(x)*U(x)) for x in [0x40f6ea32,0x40f6ea33,0x40f6ea34,0x41169113,0x41169114]]
 if os.environ.get('FMULS_SQSUM_BITS'):qs.append(U(int(os.environ['FMULS_SQSUM_BITS'],0)))
 for q in qs:add('estimate-sqsum',D(1.0),D(1.0),q)
 return rows
