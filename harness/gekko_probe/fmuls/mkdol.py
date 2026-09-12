"""Standalone, original instruction probe; imports only repo ISA encoders."""
import sys
from pathlib import Path
sys.dont_write_bytecode = True
sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import mkdol as M
TEXT=0x80003100
CTRL=0x80400000
INPUT=0x80410000
OUTPUT=0x80450000

def build():
 a=M.Asm(TEXT)
 for w in [M.mfmsr(3),M.ori(3,3,0x2000),M.mtmsr(3),M.isync()]:a.emit(w)
 a.load_addr(10,CTRL);a.load_addr(4,0xA11FE025);a.emit(M.stw(4,10,20))
 a.load_addr(4,0xC0DE0025);a.label('wait');a.emit(M.lwz(3,10,0));a.emit(M.cmpw(3,4));a.branch('bne','wait')
 # Reset FPSCR to round-nearest, non-flushing; host supplies exact zero/half.
 a.emit(M.lfd(0,10,24));a.emit((63<<26)|(255<<17)|(0<<11)|(711<<1))
 a.emit(M.lfd(8,10,32));a.emit(M.lwz(5,10,8));a.emit(M.cmpwi(5,0));a.branch('beq','done')
 a.emit(M.mtctr(5));a.load_addr(6,INPUT);a.load_addr(7,OUTPUT);a.label('loop')
 a.emit(M.lfd(1,6,0));a.emit(M.lfd(2,6,8));a.emit(M.lfd(7,6,16))
 a.emit(M.a_form(59,3,1,0,2,25));a.emit(M.stfs(3,7,0))
 a.emit(M.a_form(59,4,2,0,1,25));a.emit(M.stfs(4,7,4))
 a.emit(M.frsqrte(5,7));a.emit(M.stfd(5,7,8))
 a.emit(M.a_form(59,6,5,0,5,25));a.emit(M.stfs(6,7,16))
 a.emit(M.a_form(59,6,5,0,8,25));a.emit(M.stfs(6,7,20))
 a.emit(M.addi(6,6,24));a.emit(M.addi(7,7,24));a.branch('bdnz','loop')
 a.label('done');a.emit(M.sync());a.load_addr(4,0xC0DEF025);a.emit(M.stw(4,10,4));a.label('spin');a.branch('b','spin');a.resolve();return a.words
if __name__=='__main__':
 words=build();Path('/tmp/melee-fmuls-probe/probe.dol').write_bytes(M.make_dol(words,TEXT))
 Path('/tmp/melee-fmuls-probe/instructions.txt').write_text('\n'.join(f'{TEXT+i*4:08x} {w:08x}' for i,w in enumerate(words))+'\n')
 print('wrote probe.dol',len(words),'instructions')
